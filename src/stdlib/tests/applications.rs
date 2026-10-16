//! Ordinary expression functions, lexical applications and bounded iteration.
#![cfg(feature = "full_compiler")]

use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, ReactiveInstanceId,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentValueMut, ResidentValueRef, SchemaBody, Value, ValueData, ValueDataDraft,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn syntax(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4150504c), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn compile(source: &str, catalog: Arc<FunctionCatalog>) -> ([ProgramArtifact; 2], usize) {
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax(source), catalog)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn instance(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    ordinal: usize,
) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x4150504c, ordinal as u32),
        artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn input(
    artifact: &ProgramArtifact,
    name: &str,
    extents: Option<[u64; 2]>,
    data: ValueDataDraft,
) -> Value {
    let input = artifact
        .inputs()
        .iter()
        .find(|input| mech_engine::decode_source_input_name(&input.name).as_deref() == Some(name))
        .unwrap();
    let schema = artifact.schemas().get(input.schema).unwrap();
    let shape_values = extents
        .map(|extents| {
            mech_core::shape_for_resolved_extents(schema, &extents)
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice()
        })
        .unwrap_or_default();
    ValueDraft {
        schema: input.schema,
        shape_values,
        data,
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap()
}

fn scalar(artifact: &ProgramArtifact, name: &str, value: u64) -> Value {
    input(artifact, name, None, ValueDataDraft::U64(value))
}

fn boolean(artifact: &ProgramArtifact, name: &str, value: bool) -> Value {
    input(artifact, name, None, ValueDataDraft::Bool(value))
}

fn turn(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    named: &[(&str, Value)],
) -> Result<(), String> {
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let declaration = artifact
                .inputs()
                .iter()
                .find(|declaration| declaration.slot == input.artifact_slot)
                .unwrap();
            let name = mech_engine::decode_source_input_name(&declaration.name).unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &named.iter().find(|(key, _)| *key == name).unwrap().1,
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|prepared| prepared.publish())
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn assert_scalar(value: &Value, expected: u64) {
    assert!(
        matches!(value.data(), ValueData::U64(actual) if *actual == expected),
        "{value:?}"
    );
}

fn assert_matrix(value: &Value, extents: [u64; 2], data: ValueDataDraft) {
    let schemas = value.schemas().unwrap();
    let SchemaBody::Matrix { dimensions, .. } = schemas.get(value.schema()).unwrap().body() else {
        panic!("Matrix result")
    };
    assert_eq!(
        dimensions
            .iter()
            .map(|dimension| value.shape().resolve_dimension(dimension).unwrap())
            .collect::<Vec<_>>(),
        extents
    );
    assert_eq!(value.canonical_data_draft().unwrap(), data);
}

#[test]
fn expression_result_functions_accept_full_expression_seeds_and_ordered_stages() {
    for (source, expected) in [
        (
            "inc(x<u64>) => <u64> := x + 1u64\ntwice(x<u64>) => <u64> := x * 2u64\n(1u64 + 2u64) |> inc |> twice\n",
            8,
        ),
        (
            "inc(x<u64>) => <u64> := x + 1u64\n(flag<bool>? | true => 2u64 | false => 5u64) |> inc\n",
            3,
        ),
    ] {
        let catalog = mech_stdlib::source_catalog();
        let (artifacts, output) = compile(source, Arc::clone(&catalog));
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = instance(artifact, &catalog, ordinal);
            let named = artifact
                .inputs()
                .iter()
                .map(|declaration| {
                    let name = mech_engine::decode_source_input_name(&declaration.name).unwrap();
                    assert_eq!(name, "flag");
                    ("flag", boolean(artifact, "flag", true))
                })
                .collect::<Vec<_>>();
            turn(&mut instance, artifact, &named).unwrap();
            assert_scalar(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn defining_document_globals_resolve_at_invocation_without_caller_shadowing() {
    let source = "step(x<u64>) => <u64> := x + A\ncaller(A<u64>, x<u64>) => <u64> := x |> step\nA := 7u64\ncaller(99u64, 1u64)\n";
    let catalog = mech_stdlib::source_catalog();
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        turn(&mut instance, artifact, &[]).unwrap();
        assert_scalar(&instance.copied_output(output).unwrap(), 8);
    }
    for (source, expected) in [
        ("echo(A<u64>) => <u64> := A\nA := 7u64\necho(99u64)\n", 99),
        (
            "step(x<u64>) => <u64> := x + copy\n~A<u64> := 7u64\ncopy := A + 0u64\n1u64 |3> step\n",
            22,
        ),
    ] {
        let (artifacts, output) = compile(source, Arc::clone(&catalog));
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = instance(artifact, &catalog, ordinal);
            turn(&mut instance, artifact, &[]).unwrap();
            assert_scalar(&instance.copied_output(output).unwrap(), expected);
        }
    }
    for (source, code) in [
        (
            "step(x<u64>) => <u64> := x + A\nstep(1u64)\nA := 7u64\n",
            "source-semantics/unbound-function-input",
        ),
        (
            "step(x<u64>) => <u64> := x + A\n~A<u64> := 7u64\nstep(1u64)\n",
            "source-semantics/mutable-function-capture",
        ),
    ] {
        let error = match CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax(source), Arc::clone(&catalog))
        {
            Ok(_) => panic!("invalid document capture accepted: {source}"),
            Err(error) => error,
        };
        assert_eq!(error.code, code);
    }
}

#[test]
fn a_complete_comprehension_seed_uses_the_normal_unary_library_declaration() {
    let source = "+> sequence\n[x + 1u64 | x <- values<[u64]>] |> sequence/length\n";
    let catalog = mech_stdlib::source_catalog();
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        for values in [vec![], vec![2, 3, 4], vec![7], vec![]] {
            let data =
                ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U64).collect());
            turn(
                &mut instance,
                artifact,
                &[(
                    "values",
                    input(artifact, "values", Some([1, values.len() as u64]), data),
                )],
            )
            .unwrap();
            assert_scalar(
                &instance.copied_output(output).unwrap(),
                values.len() as u64,
            );
        }
    }
}

#[test]
fn exact_counts_include_zero_and_exceed_the_recursion_depth_without_unrolling() {
    for count in [0, 1, 25, 64] {
        let source = format!("inc(x<u64>) => <u64> := x + 1u64\nseed<u64> |{count}> inc\n");
        let catalog = mech_stdlib::source_catalog();
        let (artifacts, output) = compile(&source, Arc::clone(&catalog));
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            assert_eq!(
                artifact
                    .nodes()
                    .iter()
                    .filter(|node| matches!(
                        node.body,
                        mech_engine::ExecutableNodeBody::Iteration(_)
                    ))
                    .count(),
                1
            );
            let mut instance = instance(artifact, &catalog, ordinal);
            for seed in [3, 9] {
                turn(
                    &mut instance,
                    artifact,
                    &[("seed", scalar(artifact, "seed", seed))],
                )
                .unwrap();
                assert_scalar(&instance.copied_output(output).unwrap(), seed + count);
            }
        }
    }
    let source = "fail(x<u64>) => <u64> := 1u64 / x\nseed<u64> |0> fail\n";
    let catalog = mech_stdlib::source_catalog();
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        turn(
            &mut instance,
            artifact,
            &[("seed", scalar(artifact, "seed", 0))],
        )
        .unwrap();
        assert_scalar(&instance.copied_output(output).unwrap(), 0);
    }
    // Closed control is genuinely evaluated once through the activation
    // dispatcher, never folded/unrolled or body-executed for count zero.
    for (source, expected) in [
        ("fail(x<u64>) => <u64> := 1u64 / x\n0u64 |0> fail\n", 0),
        ("inc(x<u64>) => <u64> := x + 1u64\n0u64 |19> inc\n", 19),
    ] {
        let catalog = mech_stdlib::source_catalog();
        let (artifacts, output) = compile(source, Arc::clone(&catalog));
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            assert!(artifact.inputs().is_empty());
            let mut instance = instance(artifact, &catalog, ordinal);
            assert_scalar(&instance.copied_output(output).unwrap(), expected);
            turn(&mut instance, artifact, &[]).unwrap();
            assert_scalar(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn nested_counted_applications_keep_separate_carriers_and_exact_lexical_captures() {
    let source = "inner(x<u64>) => <u64> := x + A\nouter(x<u64>) => <u64> := x |3> inner\nA := delta<u64> + 0u64\nseed<u64> |2> outer\n";
    let catalog = mech_stdlib::source_catalog();
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        for (seed, delta) in [(1, 3), (9, 1), (2, 4)] {
            turn(
                &mut instance,
                artifact,
                &[
                    ("seed", scalar(artifact, "seed", seed)),
                    ("delta", scalar(artifact, "delta", delta)),
                ],
            )
            .unwrap();
            assert_scalar(&instance.copied_output(output).unwrap(), seed + 6 * delta);
        }
    }
}

#[test]
fn max_count_is_finitely_denied_after_earlier_checked_errors_and_recovers() {
    let source = "checked(x<u64>) => (out<u64>) := unused := 1u64 / x\nout := x + 1u64.\n(flag<bool>? | true => seed<u64> |18446744073709551615> checked | false => seed<u64>)\n";
    let catalog = mech_stdlib::source_catalog();
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        let named = |seed, flag| {
            vec![
                ("seed", scalar(artifact, "seed", seed)),
                ("flag", boolean(artifact, "flag", flag)),
            ]
        };
        turn(&mut instance, artifact, &named(3, false)).unwrap();
        let epoch = instance.published_epoch();
        let denial = turn(&mut instance, artifact, &named(1, true)).unwrap_err();
        assert!(denial.contains("InvalidShape"), "{denial}");
        assert_eq!(instance.published_epoch(), epoch);
        assert_scalar(&instance.copied_output(output).unwrap(), 3);
        let arithmetic = turn(&mut instance, artifact, &named(0, true)).unwrap_err();
        assert!(arithmetic.contains("Arithmetic"), "{arithmetic}");
        assert_eq!(instance.published_epoch(), epoch);
        assert_scalar(&instance.copied_output(output).unwrap(), 3);
        turn(&mut instance, artifact, &named(4, false)).unwrap();
        assert_scalar(&instance.copied_output(output).unwrap(), 4);
    }
}

#[test]
fn flat_matrix_carriers_keep_current_geometry_and_column_vector_identity() {
    let catalog = mech_stdlib::source_catalog();
    let source = "grow(x<[u64]>) => <[u64]> := [x 9u64]\nvalues<[u64]> |3> grow\n";
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        for values in [vec![], vec![4, 5], vec![]] {
            let initial =
                ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U64).collect());
            let named = [(
                "values",
                input(artifact, "values", Some([1, values.len() as u64]), initial),
            )];
            turn(&mut instance, artifact, &named).unwrap();
            let expected = ValueDataDraft::Matrix(
                values
                    .iter()
                    .copied()
                    .chain([9, 9, 9])
                    .map(ValueDataDraft::U64)
                    .collect(),
            );
            assert_matrix(
                &instance.copied_output(output).unwrap(),
                [1, values.len() as u64 + 3],
                expected,
            );
        }
    }
    let source = "identity(x<[f64]>) => <[f64]> := x\nvalues<[f64]> |25> identity\n";
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        for extents in [[3, 1], [0, 1], [1, 4], [2, 0]] {
            let data = ValueDataDraft::Matrix(
                (0..extents[0] * extents[1])
                    .map(|index| ValueDataDraft::F64(F64Bits::from_f64(index as f64 + 0.5)))
                    .collect(),
            );
            turn(
                &mut instance,
                artifact,
                &[(
                    "values",
                    input(artifact, "values", Some(extents), data.clone()),
                )],
            )
            .unwrap();
            assert_matrix(&instance.copied_output(output).unwrap(), extents, data);
        }
    }
}

#[test]
fn string_carriers_preserve_owned_raw_utf8_across_scalar_and_matrix_boundaries() {
    let catalog = mech_stdlib::source_catalog();
    let source = "append(text<string>) => <string> := text + \"é\"\ntext<string> |2> append\n";
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        for text in ["", "é", "👩‍👩‍👧‍👦\r\n", "x"] {
            turn(
                &mut instance,
                artifact,
                &[(
                    "text",
                    input(
                        artifact,
                        "text",
                        None,
                        ValueDataDraft::String(text.to_owned()),
                    ),
                )],
            )
            .unwrap();
            assert_eq!(
                instance
                    .copied_output(output)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::String(format!("{text}éé")),
            );
        }
    }
    let source =
        "grow(items<[string]>) => <[string]> := [items \"🧑‍🚀\"]\nitems<[string]> |2> grow\n";
    let (artifacts, output) = compile(source, Arc::clone(&catalog));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, &catalog, ordinal);
        for strings in [vec![], vec!["é", "é", ""], vec![]] {
            let initial = ValueDataDraft::Matrix(
                strings
                    .iter()
                    .map(|text| ValueDataDraft::String((*text).to_owned()))
                    .collect(),
            );
            turn(
                &mut instance,
                artifact,
                &[(
                    "items",
                    input(artifact, "items", Some([1, strings.len() as u64]), initial),
                )],
            )
            .unwrap();
            let expected = ValueDataDraft::Matrix(
                strings
                    .iter()
                    .copied()
                    .chain(["🧑‍🚀", "🧑‍🚀"])
                    .map(|text| ValueDataDraft::String(text.to_owned()))
                    .collect(),
            );
            assert_matrix(
                &instance.copied_output(output).unwrap(),
                [1, strings.len() as u64 + 2],
                expected,
            );
        }
    }
}

static PRODUCER_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
struct BorrowedInputs<'a>(&'a dyn ResidentKernelInputs);
impl ResidentKernelInputs for BorrowedInputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.0.get(index)
    }
}
fn observed_division(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    PRODUCER_EXECUTIONS.fetch_add(1, Ordering::Relaxed);
    kernel
        .retained_state::<BoundResidentKernel>()
        .ok_or(ResidentKernelError::InvalidInput)?
        .execute(&BorrowedInputs(inputs), output)
}
fn observed_division_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let maintained = mech_stdlib::source_catalog();
    let entry = maintained
        .resident_factory(&["math".into()], "div")
        .unwrap();
    let delegate = (entry.factory)(request)?;
    let output = delegate.snapshot_output().cloned();
    let work = delegate.work_accounting();
    let mut kernel = BoundResidentKernel::new(observed_division, Box::new([]))
        .with_snapshot_schemas(request.schemas.clone())
        .with_retained_state(Arc::new(delegate))
        .with_work_accounting(work);
    if let Some(output) = output {
        kernel = kernel.with_snapshot_output(output);
    }
    Ok(kernel)
}

#[test]
fn runtime_global_and_seed_producers_execute_once_per_invocation() {
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::new();
    for (key, entry) in maintained.resident_factories() {
        builder
            .insert_resident_factory(
                key.module_path.iter().cloned(),
                key.operation_name.clone(),
                entry.implementation_memory,
                if key.module_path.as_ref() == ["math".to_owned()] && key.operation_name == "div" {
                    observed_division_bind
                } else {
                    entry.factory
                },
            )
            .unwrap();
    }
    let execution_catalog = builder.build().unwrap();
    // Source declarations remain the genuine maintained declarations. Only the
    // selected runtime leaf has a transparent test observer; it delegates all
    // arithmetic and error behavior to that exact maintained implementation.
    let source = "step(x<u64>) => <u64> := x + A\nA := 20u64 / denominator<u64>\n(10u64 / seed<u64>) |4> step\n";
    let (artifacts, output) = compile(source, Arc::clone(&maintained));
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        PRODUCER_EXECUTIONS.store(0, Ordering::Relaxed);
        let mut instance = instance(artifact, &execution_catalog, ordinal);
        assert_eq!(PRODUCER_EXECUTIONS.load(Ordering::Relaxed), 0);
        for (seed, denominator, expected) in [(2, 4, 25), (5, 2, 42)] {
            let before = PRODUCER_EXECUTIONS.load(Ordering::Relaxed);
            turn(
                &mut instance,
                artifact,
                &[
                    ("seed", scalar(artifact, "seed", seed)),
                    ("denominator", scalar(artifact, "denominator", denominator)),
                ],
            )
            .unwrap();
            assert_eq!(PRODUCER_EXECUTIONS.load(Ordering::Relaxed) - before, 2);
            assert_scalar(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn bound_document_inputs_relocate_iteration_captures_and_body_constants_recursively() {
    let catalog = mech_stdlib::source_catalog();
    for (expression, visits, matrix) in [
        ("seed<u64> |3> step", 3, false),
        (
            "(flag<bool>? | true => seed<u64> |3> step | false => seed<u64>)",
            3,
            false,
        ),
        ("[(seed |2> step) | seed <- values<[u64]>]", 2, true),
    ] {
        let source =
            format!("step(x<u64>) => <u64> := x + A + 1u64\nA := bias<u64>\n{expression}\n");
        let product = CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax(&source), Arc::clone(&catalog))
            .unwrap();
        let unbound = product.compile_artifact().unwrap();
        let binding = unbound
            .inputs()
            .iter()
            .find(|input| {
                mech_engine::decode_source_input_name(&input.name).as_deref() == Some("bias")
            })
            .unwrap()
            .input
            .get();
        let output = product
            .document_outputs()
            .iter()
            .rev()
            .find(|output| output.kind == SourceDocumentOutputKind::Program)
            .unwrap()
            .output as usize;
        for bias in [0, 11] {
            let bound = product
                .clone()
                .bind_input_constants(&[(binding, scalar(&unbound, "bias", bias))])
                .unwrap();
            let original = bound.compile_artifact().unwrap();
            assert!(original.inputs().iter().all(|input| {
                mech_engine::decode_source_input_name(&input.name).as_deref() != Some("bias")
            }));
            let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
            let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
            assert_eq!(original.revision(), decoded.revision());
            assert_eq!(
                bytes,
                mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
            );
            for (ordinal, artifact) in [original, decoded].iter().enumerate() {
                let mut instance = instance(artifact, &catalog, ordinal);
                if matrix {
                    turn(
                        &mut instance,
                        artifact,
                        &[(
                            "values",
                            input(
                                artifact,
                                "values",
                                Some([1, 2]),
                                ValueDataDraft::Matrix(
                                    vec![ValueDataDraft::U64(2), ValueDataDraft::U64(5)]
                                        .into_boxed_slice(),
                                ),
                            ),
                        )],
                    )
                    .unwrap();
                    assert_matrix(
                        &instance.copied_output(output).unwrap(),
                        [1, 2],
                        ValueDataDraft::Matrix(
                            [2, 5]
                                .into_iter()
                                .map(|seed| ValueDataDraft::U64(seed + visits * (bias + 1)))
                                .collect(),
                        ),
                    );
                } else {
                    let mut named = vec![("seed", scalar(artifact, "seed", 2))];
                    if artifact.inputs().iter().any(|input| {
                        mech_engine::decode_source_input_name(&input.name).as_deref()
                            == Some("flag")
                    }) {
                        named.push(("flag", boolean(artifact, "flag", true)));
                    }
                    turn(&mut instance, artifact, &named).unwrap();
                    assert_scalar(
                        &instance.copied_output(output).unwrap(),
                        2 + visits * (bias + 1),
                    );
                }
            }
        }
    }
}
