//! Ordinary one-level block construction, including empty and reactive shapes.
#![cfg(feature = "full_compiler")]
use std::sync::{Arc, Mutex};

use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, ReactiveInstanceId,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentValueMut, ResidentValueRef, SchemaBody, Value, ValueData, ValueDataDraft,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{
    CanonicalSourceFrontend, ComprehensionKind, MatrixBlockRows, ProgramArtifact,
    SourceDocumentOutputKind,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x48424c4b), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = program.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}
fn instance(artifact: &ProgramArtifact, ordinal: usize) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x48424c4b, ordinal as u32),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}
fn input(
    artifact: &ProgramArtifact,
    name: &str,
    dimensions: Option<[u64; 2]>,
    data: ValueDataDraft,
) -> Value {
    let input = artifact
        .inputs()
        .iter()
        .find(|input| mech_engine::decode_source_input_name(&input.name).as_deref() == Some(name))
        .unwrap();
    let schema = artifact.schemas().get(input.schema).unwrap();
    let shape_values = dimensions
        .map(|dimensions| {
            mech_core::shape_for_resolved_extents(schema, &dimensions)
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
fn turn(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    named: &[(&str, Value)],
) -> Result<(), String> {
    let values = artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            &named.iter().find(|(key, _)| *key == name).unwrap().1
        })
        .collect::<Vec<_>>();
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|value| value.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: values[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|prepared| prepared.publish())
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}
fn assert_matrix(
    value: &Value,
    expected_rows: u64,
    expected_columns: u64,
    expected: ValueDataDraft,
) {
    let schema = value.schemas().unwrap();
    let SchemaBody::Matrix { dimensions, .. } = schema.get(value.schema()).unwrap().body() else {
        panic!("matrix result");
    };
    assert_eq!(
        dimensions
            .iter()
            .map(|dimension| value.shape().resolve_dimension(dimension).unwrap())
            .collect::<Vec<_>>(),
        [expected_rows, expected_columns]
    );
    assert_eq!(value.canonical_data_draft().unwrap(), expected);
}
fn f64s(values: &[f64]) -> ValueDataDraft {
    ValueDataDraft::Matrix(
        values
            .iter()
            .map(|value| ValueDataDraft::F64(F64Bits::from_f64(*value)))
            .collect(),
    )
}
fn u64s(values: &[u64]) -> ValueDataDraft {
    ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U64).collect())
}

#[test]
fn scalar_blocks_and_explicit_nested_elements_have_distinct_ordinary_semantics() {
    for (source, rows, columns, expected) in [
        ("[x + 1.0 | x <- [1.0 2.0]]\n", 1, 2, f64s(&[2.0, 3.0])),
        (
            "[[x; x + 10.0] | x <- [1.0 2.0]]\n",
            2,
            2,
            f64s(&[1.0, 2.0, 11.0, 12.0]),
        ),
        (
            "blocks<[[f64]:1,1]:1,2> := [[x] | x <- [1.0 2.0]]\nblocks\n",
            1,
            2,
            ValueDataDraft::Matrix(vec![f64s(&[1.0]), f64s(&[2.0])].into_boxed_slice()),
        ),
        (
            "blocks<[[string]:2,1]:1,2> := [[\"é\"; \"x\"] | * <- [1u64 2u64]]\nblocks\n",
            1,
            2,
            ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::Matrix(
                        vec![
                            ValueDataDraft::String("é".into()),
                            ValueDataDraft::String("x".into()),
                        ]
                        .into_boxed_slice(),
                    ),
                    ValueDataDraft::Matrix(
                        vec![
                            ValueDataDraft::String("é".into()),
                            ValueDataDraft::String("x".into()),
                        ]
                        .into_boxed_slice(),
                    ),
                ]
                .into_boxed_slice(),
            ),
        ),
        (
            "blocks<[[f64]:2,1]:1,2> := [[x; x + 10.0] | x <- [1.0 2.0]]\nblocks\n",
            1,
            2,
            ValueDataDraft::Matrix(vec![f64s(&[1.0, 11.0]), f64s(&[2.0, 12.0])].into_boxed_slice()),
        ),
        (
            "blocks<[[f64]:1,2]:1,2> := [[x x + 1.0] | x <- [1.0 3.0]]\nout<[[f64]:1,2]:1,4> := [[blocks] | * <- [1.0 2.0]]\nout\n",
            1,
            4,
            ValueDataDraft::Matrix(
                vec![
                    f64s(&[1.0, 2.0]),
                    f64s(&[3.0, 4.0]),
                    f64s(&[1.0, 2.0]),
                    f64s(&[3.0, 4.0]),
                ]
                .into_boxed_slice(),
            ),
        ),
    ] {
        let (artifacts, output) = compile(source);
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = instance(artifact, ordinal);
            turn(&mut instance, artifact, &[]).unwrap();
            assert_matrix(
                &instance.copied_output(output).unwrap(),
                rows,
                columns,
                expected.clone(),
            );
        }
    }
}

#[test]
fn block_widths_are_summed_once_and_scattered_in_row_major_order() {
    let source = "[[y | y <- [0u64 1u64 2u64], y < x] | x <- [1u64 3u64 0u64]]\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        turn(&mut instance, artifact, &[]).unwrap();
        assert_matrix(
            &instance.copied_output(output).unwrap(),
            1,
            4,
            u64s(&[0, 0, 1, 2]),
        );
    }
}

#[test]
fn all_three_row_policies_use_real_geometry_including_empty_axes() {
    for (source, policy) in [
        (
            "[[x; x] | x <- columns<[u64]>]\n",
            MatrixBlockRows::Fixed(2),
        ),
        (
            "[[row | row <- rows<[u64]>, true]' | column <- columns<[u64]>]\n",
            MatrixBlockRows::InferOrZero,
        ),
        (
            "[[row + column | row <- rows<[u64]>]' | column <- columns<[u64]>]\n",
            MatrixBlockRows::InputMatrixElements { input: 0 },
        ),
    ] {
        let (artifacts, output) = compile(source);
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let outer = artifact
                .nodes()
                .iter()
                .rev()
                .find_map(|node| match &node.body {
                    mech_engine::ExecutableNodeBody::Comprehension(control) => Some(control),
                    _ => None,
                })
                .unwrap();
            let ComprehensionKind::MatrixHorizontalBlocks { rows } = outer.kind else {
                panic!("horizontal constructor");
            };
            match policy {
                MatrixBlockRows::InputMatrixElements { .. } => {
                    assert!(matches!(rows, MatrixBlockRows::InputMatrixElements { .. }))
                }
                _ => assert_eq!(rows, policy),
            }
            let mut instance = instance(artifact, ordinal);
            for (n, m) in [(3, 2), (4, 0), (0, 3), (1, 1), (0, 0)] {
                let mut named = vec![(
                    "columns",
                    input(
                        artifact,
                        "columns",
                        Some([1, m]),
                        u64s(&(0..m).collect::<Vec<_>>()),
                    ),
                )];
                if artifact.inputs().iter().any(|input| {
                    mech_engine::decode_source_input_name(&input.name).as_deref() == Some("rows")
                }) {
                    named.push((
                        "rows",
                        input(
                            artifact,
                            "rows",
                            Some([1, n]),
                            u64s(&(0..n).collect::<Vec<_>>()),
                        ),
                    ));
                }
                turn(&mut instance, artifact, &named).unwrap();
                let (height, values) = match policy {
                    MatrixBlockRows::Fixed(_) => (2, (0..2).flat_map(|_| 0..m).collect::<Vec<_>>()),
                    MatrixBlockRows::InferOrZero => (
                        if m == 0 { 0 } else { n },
                        (0..n)
                            .flat_map(|row| std::iter::repeat_n(row, m as usize))
                            .collect(),
                    ),
                    MatrixBlockRows::InputMatrixElements { .. } => (
                        n,
                        (0..n)
                            .flat_map(|row| (0..m).map(move |column| row + column))
                            .collect(),
                    ),
                };
                assert_matrix(
                    &instance.copied_output(output).unwrap(),
                    height,
                    m,
                    u64s(&values),
                );
            }
        }
    }
}

#[test]
fn mismatched_successful_block_rows_preserve_previous_output_and_recover() {
    let source = "[[y | y <- [0u64 1u64 2u64], y < x]' | x <- signal<[u64]:1,2>]\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let signal = |values: &[u64]| input(artifact, "signal", Some([1, 2]), u64s(values));
        turn(&mut instance, artifact, &[("signal", signal(&[2, 2]))]).unwrap();
        let previous = instance.copied_output(output).unwrap();
        let epoch = instance.published_epoch();
        assert_matrix(&previous, 2, 2, u64s(&[0, 0, 1, 1]));
        let error = turn(&mut instance, artifact, &[("signal", signal(&[2, 3]))]).unwrap_err();
        assert!(error.contains("InvalidShape"), "{error}");
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            previous.canonical_data_draft().unwrap()
        );
        turn(&mut instance, artifact, &[("signal", signal(&[1, 1]))]).unwrap();
        assert_matrix(
            &instance.copied_output(output).unwrap(),
            1,
            2,
            u64s(&[0, 0]),
        );
    }
}

#[test]
fn filters_skip_the_yield_but_prior_checked_coefficients_still_fail_for_zero_iterations() {
    let (artifacts, output) = compile("[[1u64 / x; x] | x <- signal<[u64]:1,2>, x != 0u64]\n");
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (values, width, expected) in [([0, 0], 0, vec![]), ([0, 1], 1, vec![1, 1])] {
            turn(
                &mut instance,
                artifact,
                &[(
                    "signal",
                    input(artifact, "signal", Some([1, 2]), u64s(&values)),
                )],
            )
            .unwrap();
            assert_matrix(
                &instance.copied_output(output).unwrap(),
                2,
                width,
                u64s(&expected),
            );
        }
    }
    let source = "coefficient := 1u64 / divisor<u64>\n[[coefficient; x] | x <- items<[u64]>]\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let named = |divisor| {
            vec![
                (
                    "divisor",
                    input(artifact, "divisor", None, ValueDataDraft::U64(divisor)),
                ),
                ("items", input(artifact, "items", Some([1, 0]), u64s(&[]))),
            ]
        };
        turn(&mut instance, artifact, &named(1)).unwrap();
        assert_matrix(&instance.copied_output(output).unwrap(), 2, 0, u64s(&[]));
        let error = turn(&mut instance, artifact, &named(0)).unwrap_err();
        assert!(error.contains("Arithmetic"), "{error}");
        assert_matrix(&instance.copied_output(output).unwrap(), 2, 0, u64s(&[]));
        turn(&mut instance, artifact, &named(1)).unwrap();
    }
}

#[test]
fn refutable_patterns_skip_the_complete_block_yield() {
    let source = "[[1u64 / x; x] | [x,x] <- signal<[[u64]:1,2]:1,2>]\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (pairs, width, expected) in [
            ([[0, 1], [0, 2]], 0, vec![]),
            ([[0, 1], [2, 2]], 1, vec![0, 2]),
        ] {
            let data = ValueDataDraft::Matrix(pairs.iter().map(|pair| u64s(pair)).collect());
            turn(
                &mut instance,
                artifact,
                &[("signal", input(artifact, "signal", Some([1, 2]), data))],
            )
            .unwrap();
            assert_matrix(
                &instance.copied_output(output).unwrap(),
                2,
                width,
                u64s(&expected),
            );
        }
    }
}

#[test]
fn fixed_result_rows_check_only_successfully_yielded_blocks() {
    let source = "result<[u64]:3,0> := [[x; x] | x <- [1u64], enabled<bool>]\nresult\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let named = |enabled| {
            vec![(
                "enabled",
                input(artifact, "enabled", None, ValueDataDraft::Bool(enabled)),
            )]
        };
        turn(&mut instance, artifact, &named(false)).unwrap();
        assert_matrix(&instance.copied_output(output).unwrap(), 3, 0, u64s(&[]));
        let epoch = instance.published_epoch();
        let error = turn(&mut instance, artifact, &named(true)).unwrap_err();
        assert!(error.contains("InvalidShape"), "{error}");
        assert_eq!(instance.published_epoch(), epoch);
        assert_matrix(&instance.copied_output(output).unwrap(), 3, 0, u64s(&[]));
        turn(&mut instance, artifact, &named(false)).unwrap();
    }
}

#[test]
fn complete_constant_cardinality_constraints_check_actual_geometry_and_recover_atomically() {
    for (source, block_shape, kind, rows, columns, valid_count, invalid_counts, expected) in [
        (
            "result<[u64]:1,2> := [x + 1u64 | x <- items<[u64]>]\nresult\n",
            None,
            ComprehensionKind::Matrix,
            1,
            2,
            2,
            vec![1, 0],
            u64s(&[1, 2]),
        ),
        (
            "result<[u64]:1,0> := [x + 1u64 | x <- items<[u64]>]\nresult\n",
            None,
            ComprehensionKind::Matrix,
            1,
            0,
            0,
            vec![1],
            u64s(&[]),
        ),
        (
            "result<[[u64]:0,1]:1,2> := [block<[u64]:0,1> | * <- items<[u64]>]\nresult\n",
            Some([0, 1]),
            ComprehensionKind::Matrix,
            1,
            2,
            2,
            vec![1, 0],
            ValueDataDraft::Matrix(vec![u64s(&[]), u64s(&[])].into_boxed_slice()),
        ),
        (
            "result<[u64]:2,0> := [block<[u64]:2,1> | * <- items<[u64]>]\nresult\n",
            Some([2, 1]),
            ComprehensionKind::MatrixHorizontalBlocks {
                rows: MatrixBlockRows::Fixed(2),
            },
            2,
            0,
            0,
            vec![1],
            u64s(&[]),
        ),
        (
            "result<[u64]:0,4> := [block<[u64]:0,2> | * <- items<[u64]>]\nresult\n",
            Some([0, 2]),
            ComprehensionKind::MatrixHorizontalBlocks {
                rows: MatrixBlockRows::Fixed(0),
            },
            0,
            4,
            2,
            vec![1, 0],
            u64s(&[]),
        ),
    ] {
        let (artifacts, output) = compile(source);
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let actual_kind = artifact
                .nodes()
                .iter()
                .rev()
                .find_map(|node| match &node.body {
                    mech_engine::ExecutableNodeBody::Comprehension(control) => Some(control.kind),
                    _ => None,
                })
                .unwrap();
            assert_eq!(actual_kind, kind);
            let named = |count| {
                let mut named = vec![(
                    "items",
                    input(
                        artifact,
                        "items",
                        Some([1, count]),
                        u64s(&(0..count).collect::<Vec<_>>()),
                    ),
                )];
                if let Some(shape) = block_shape {
                    let count = shape[0] * shape[1];
                    named.push((
                        "block",
                        input(
                            artifact,
                            "block",
                            Some(shape),
                            u64s(&vec![0; count as usize]),
                        ),
                    ));
                }
                named
            };
            let mut instance = instance(artifact, ordinal);
            for count in invalid_counts.iter().copied() {
                turn(&mut instance, artifact, &named(valid_count)).unwrap();
                assert_matrix(
                    &instance.copied_output(output).unwrap(),
                    rows,
                    columns,
                    expected.clone(),
                );
                let epoch = instance.published_epoch();
                let error = turn(&mut instance, artifact, &named(count)).unwrap_err();
                assert!(error.contains("InvalidShape"), "{source}: {error}");
                assert_eq!(instance.published_epoch(), epoch);
                assert_matrix(
                    &instance.copied_output(output).unwrap(),
                    rows,
                    columns,
                    expected.clone(),
                );
                turn(&mut instance, artifact, &named(valid_count)).unwrap();
                assert_matrix(
                    &instance.copied_output(output).unwrap(),
                    rows,
                    columns,
                    expected.clone(),
                );
            }
        }
    }
}

#[test]
fn suspended_block_row_authority_uses_the_retained_capture_beside_fresh_live_shape() {
    let source = "+> text\n+> sequence\n#Captured(items<[string]>) => <([u64],u64)>\n  | :Start\n  | :Later.\n#Captured(items) -> :Start\n  :Start ~> :Later\n  :Later => ([[1u64 | row <- items]' | column <- [0u64], false], sequence/length(text/lines(signal<string>))).\n#Captured(text/lines(signal<string>))\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (old, fresh, retained_rows, fresh_count) in [
            ("a\nkeep", "fresh", 2, 1),
            ("a\né\nz", "new\nb\nc\nd", 3, 4),
        ] {
            let signal = |text: &str| {
                input(
                    artifact,
                    "signal",
                    None,
                    ValueDataDraft::String(text.to_owned()),
                )
            };
            turn(&mut instance, artifact, &[("signal", signal(old))]).unwrap();
            assert!(instance.has_ready_continuation());
            turn(&mut instance, artifact, &[("signal", signal(fresh))]).unwrap();
            assert!(!instance.has_ready_continuation());
            let value = instance.copied_output(output).unwrap();
            let owner = value.schemas().unwrap();
            let SchemaBody::Tuple(fields) = owner.get(value.schema()).unwrap().body() else {
                panic!("tuple schema");
            };
            let SchemaBody::Matrix { dimensions, .. } = &fields[0] else {
                panic!("matrix component");
            };
            assert_eq!(
                dimensions
                    .iter()
                    .map(|dimension| value.shape().resolve_dimension(dimension).unwrap())
                    .collect::<Vec<_>>(),
                [retained_rows, 0]
            );
            let ValueData::Tuple(parts) = value.data() else {
                panic!("tuple result");
            };
            assert!(matches!(&parts[0], ValueData::Matrix(matrix) if matrix.elements().len() == 0));
            assert!(matches!(&parts[1], ValueData::U64(actual) if *actual == fresh_count));
        }
    }
}

// Only this test catalog replaces the selected leaf with a transparent delegate.
// The log is test instrumentation; the actual maintained division computes every result/error.
static DIVISION_ORDER: Mutex<Vec<u64>> = Mutex::new(Vec::new());
static DIVISION_TEST_LOCK: Mutex<()> = Mutex::new(());

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
    let Some(ResidentValueRef::Snapshot([Some(denominator)])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ValueData::U64(denominator) = denominator.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    DIVISION_ORDER.lock().unwrap().push(*denominator);
    let delegate = kernel
        .retained_state::<BoundResidentKernel>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    delegate.execute(&BorrowedInputs(inputs), output)
}

fn observed_division_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let maintained = mech_stdlib::source_catalog();
    let entry = maintained
        .resident_factory(&["math".into()], "div")
        .unwrap();
    let delegate = (entry.factory())(request)?;
    let output = delegate.snapshot_output().cloned();
    let mut kernel = BoundResidentKernel::new(observed_division, Box::new([]))
        .with_snapshot_schemas(request.schemas.clone())
        .with_retained_state(Arc::new(delegate));
    if let Some(output) = output {
        kernel = kernel.with_snapshot_output(output);
    }
    Ok(kernel)
}

fn observed_division_catalog() -> FunctionCatalog {
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::new();
    let mut replaced = false;
    for (key, entry) in maintained.resident_factories() {
        let selected =
            key.module_path.as_ref() == ["math".to_owned()] && key.operation_name == "div";
        replaced |= selected;
        builder
            .insert_resident_factory(
                key.module_path.iter().cloned(),
                key.operation_name.clone(),
                entry.implementation_memory(),
                if selected {
                    observed_division_bind
                } else {
                    entry.factory()
                },
            )
            .unwrap();
    }
    assert!(replaced);
    builder.build().unwrap()
}

#[test]
fn successful_block_yields_evaluate_once_in_order_and_stop_at_the_first_checked_error() {
    let _test_guard = DIVISION_TEST_LOCK.lock().unwrap();
    for filtered in [true, false] {
        let source = if filtered {
            "[[12u64 / x; x] | x <- signal<[u64]:1,3>, x != 0u64]\n"
        } else {
            "[[12u64 / x; x] | x <- signal<[u64]:1,3>]\n"
        };
        let (artifacts, output) = compile(source);
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = activate(
                ReactiveInstanceId::new(0x48424c51, ordinal as u32),
                artifact,
                &observed_division_catalog(),
                &ActivationFacts::default(),
            )
            .unwrap();
            DIVISION_ORDER.lock().unwrap().clear();
            let valid = if filtered { [3, 0, 2] } else { [3, 2, 1] };
            turn(
                &mut instance,
                artifact,
                &[(
                    "signal",
                    input(artifact, "signal", Some([1, 3]), u64s(&valid)),
                )],
            )
            .unwrap();
            let expected_order = if filtered { vec![3, 2] } else { vec![3, 2, 1] };
            assert_eq!(*DIVISION_ORDER.lock().unwrap(), expected_order);
            let previous = instance.copied_output(output).unwrap();
            assert_matrix(
                &previous,
                2,
                if filtered { 2 } else { 3 },
                if filtered {
                    u64s(&[4, 6, 3, 2])
                } else {
                    u64s(&[4, 6, 12, 3, 2, 1])
                },
            );
            if !filtered {
                let epoch = instance.published_epoch();
                DIVISION_ORDER.lock().unwrap().clear();
                let error = turn(
                    &mut instance,
                    artifact,
                    &[(
                        "signal",
                        input(artifact, "signal", Some([1, 3]), u64s(&[3, 0, 2])),
                    )],
                )
                .unwrap_err();
                assert!(error.contains("Arithmetic"), "{error}");
                assert_eq!(*DIVISION_ORDER.lock().unwrap(), [3, 0]);
                assert_eq!(instance.published_epoch(), epoch);
                assert_eq!(
                    instance
                        .copied_output(output)
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    previous.canonical_data_draft().unwrap()
                );
            }
        }
    }
}

#[test]
fn constant_result_axes_check_after_ordered_yield_errors_without_early_shortcut() {
    let _test_guard = DIVISION_TEST_LOCK.lock().unwrap();
    let source = "result<[u64]:2,2> := [[12u64 / x; x] | x <- signal<[u64]>]\nresult\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x48424c52, ordinal as u32),
            artifact,
            &observed_division_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let named = |values: &[u64]| {
            vec![(
                "signal",
                input(
                    artifact,
                    "signal",
                    Some([1, values.len() as u64]),
                    u64s(values),
                ),
            )]
        };
        DIVISION_ORDER.lock().unwrap().clear();
        turn(&mut instance, artifact, &named(&[3, 2])).unwrap();
        assert_eq!(*DIVISION_ORDER.lock().unwrap(), [3, 2]);
        let epoch = instance.published_epoch();
        for (values, diagnostic, expected_order) in [
            ([3, 0, 2], "Arithmetic", vec![3, 0]),
            ([3, 2, 1], "InvalidShape", vec![3, 2, 1]),
        ] {
            DIVISION_ORDER.lock().unwrap().clear();
            let error = turn(&mut instance, artifact, &named(&values)).unwrap_err();
            assert!(error.contains(diagnostic), "{error}");
            assert_eq!(*DIVISION_ORDER.lock().unwrap(), expected_order);
            assert_eq!(instance.published_epoch(), epoch);
            assert_matrix(
                &instance.copied_output(output).unwrap(),
                2,
                2,
                u64s(&[4, 6, 3, 2]),
            );
        }
        DIVISION_ORDER.lock().unwrap().clear();
        turn(&mut instance, artifact, &named(&[3, 2])).unwrap();
        assert_eq!(*DIVISION_ORDER.lock().unwrap(), [3, 2]);
        assert_matrix(
            &instance.copied_output(output).unwrap(),
            2,
            2,
            u64s(&[4, 6, 3, 2]),
        );
    }
}

#[test]
fn retained_actual_legacy_bytes_execute_their_original_nested_and_preserved_modes() {
    for (bytes, rows, columns, expected) in [
        (
            &include_bytes!(
                "../../engine/tests/fixtures/comprehension-legacy-v1/matrix-scalar.mecb"
            )[..],
            1,
            1,
            f64s(&[3.0]),
        ),
        (
            &include_bytes!(
                "../../engine/tests/fixtures/comprehension-legacy-v1/matrix-nested.mecb"
            )[..],
            1,
            1,
            ValueDataDraft::Matrix(vec![f64s(&[3.0, 7.0])].into_boxed_slice()),
        ),
        (
            &include_bytes!(
                "../../engine/tests/fixtures/comprehension-legacy-v1/matrix-preserve-shape.mecb"
            )[..],
            2,
            1,
            f64s(&[3.0, 7.0]),
        ),
    ] {
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(bytes).unwrap();
        assert_eq!(
            mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            bytes
        );
        let mut instance = instance(&artifact, 0);
        turn(&mut instance, &artifact, &[]).unwrap();
        assert_matrix(&instance.copied_output(0).unwrap(), rows, columns, expected);
    }
    let bytes =
        include_bytes!("../../engine/tests/fixtures/comprehension-legacy-v1/set-scalar.mecb");
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(bytes).unwrap();
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        bytes
    );
    let mut instance = instance(&artifact, 0);
    turn(&mut instance, &artifact, &[]).unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::Set(vec![ValueDataDraft::F64(F64Bits::from_f64(3.0))].into_boxed_slice())
    );
}
