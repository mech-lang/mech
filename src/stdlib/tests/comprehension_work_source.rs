//! Ordinary artifact execution of collection bodies with implementation-owned work accounting.
#![cfg(feature = "full_compiler")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use mech_core::{
    BoundResidentKernel, DimensionExpr, FunctionCatalog, FunctionCatalogBuilder, IntegerWidth,
    ReactiveInstanceId, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelFactory, ResidentKernelInputs, ResidentKernelWorkAccounting,
    ResidentSnapshotOutput, ResidentValueMut, SchemaBody, Value, ValueCell, ValueData,
    snapshot::{SequenceView, ValueDataDraft},
};
use mech_engine::resident::{
    ActivatedTurnStep, ActivationFacts, CapturedValueInput, ResidentActivationError, activate,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434f4c57), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (program.compile_artifact().unwrap(), output)
}

fn strings(values: &[&str]) -> Value {
    matrix(
        SchemaBody::String,
        values
            .iter()
            .map(|value| ValueDataDraft::String((*value).to_owned()))
            .collect(),
    )
}

fn integers(values: &[u64]) -> Value {
    matrix(
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        values.iter().copied().map(ValueDataDraft::U64).collect(),
    )
}

fn matrix(element: SchemaBody, values: Vec<ValueDataDraft>) -> Value {
    ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: Box::new([
                DimensionExpr::Constant(1),
                DimensionExpr::Constant(values.len() as u64),
            ]),
        },
        ValueDataDraft::Matrix(values.into_boxed_slice()),
    )
    .unwrap()
    .snapshot()
    .unwrap()
}

fn string_values(value: &Value) -> Vec<&str> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("{value:?}")
    };
    match matrix.elements() {
        SequenceView::String(values) => values.iter().map(|value| value.as_ref()).collect(),
        other => panic!("{other:?}"),
    }
}

fn u64_values(value: &Value) -> Vec<u64> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("{value:?}")
    };
    let SequenceView::U64(values) = matrix.elements() else {
        panic!("{value:?}")
    };
    values.to_vec()
}

fn live(source: &str, inputs: &[Value], expected: impl Fn(usize, &Value)) {
    let (original, output) = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x434f4c57, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(instance.plan.inputs.len(), 1);
        let input = instance.plan.inputs[0].clone();
        for (turn, value) in inputs.iter().enumerate() {
            let admitted = value
                .rebind(input.schema, &input.shape, artifact.schemas())
                .unwrap();
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &admitted,
                }])
                .and_then(|prepared| prepared.publish())
                .unwrap_or_else(|error| panic!("{source}, turn {turn}: {error:?}"));
            expected(turn, &instance.copied_output(output).unwrap());
        }
    }
}

#[test]
fn string_producing_body_operations_preserve_unicode_and_exact_integer_formatting() {
    live(
        "ys := [s + \"!\" | s <- xs<[string]:1,2>]\nys\n",
        &[strings(&["é", ""]), strings(&["é", "🧑‍🚀"])],
        |turn, value| assert_eq!(string_values(value), [["é!", "!"], ["é!", "🧑‍🚀!"]][turn]),
    );
    live(
        "+> number\nys := [number/format(x, 0u64) | x <- xs<[u64]:1,3>]\nys\n",
        &[
            integers(&[0, 9_007_199_254_740_993, u64::MAX]),
            integers(&[19, 2, 7]),
        ],
        |turn, value| {
            assert_eq!(
                string_values(value),
                [
                    ["0", "9007199254740993", "18446744073709551615"],
                    ["19", "2", "7"],
                ][turn]
            )
        },
    );
}

#[test]
fn string_selection_and_filters_keep_grapheme_and_lazy_body_semantics() {
    live(
        "ys := [s[1u64..=2u64] | s <- xs<[string]:1,2>]\nys\n",
        &[strings(&["é👩🏽‍💻Z", "🇺🇸A"]), strings(&["ab", "xy"])],
        |turn, value| assert_eq!(string_values(value), [["é👩🏽‍💻", "🇺🇸A"], ["ab", "xy"]][turn]),
    );
    live(
        "ys := [s[99u64] | s <- xs<[string]:1,2>, false]\nys\n",
        &[strings(&["", "x"]), strings(&["a", ""])],
        |_, value| assert!(string_values(value).is_empty()),
    );
}

#[test]
fn u64_filters_compare_exactly_above_f64_integer_precision() {
    live(
        "ys := [x | x <- xs<[u64]:1,4>, x > 9007199254740992u64]\nys\n",
        &[
            integers(&[9_007_199_254_740_992, 9_007_199_254_740_993, 0, u64::MAX]),
            integers(&[19, u64::MAX, 7, 2]),
        ],
        |turn, value| {
            assert_eq!(
                u64_values(value),
                [vec![9_007_199_254_740_993, u64::MAX], vec![u64::MAX],][turn]
            )
        },
    );
}

#[test]
fn string_set_bodies_compose_with_existing_membership_and_empty_output() {
    live(
        "keys := {s + \"\" | s <- xs<[string]:1,3>}\n\"é\" ∈ keys\n",
        &[strings(&["é", "é", ""]), strings(&["é", "A", "A"])],
        |turn, value| {
            assert!(
                matches!(value.data(), ValueData::Bool(actual) if *actual == (turn == 0)),
                "{value:?}"
            )
        },
    );
    live(
        "{s + \"!\" | s <- xs<[string]:1,2>, false}\n",
        &[strings(&["", "é"]), strings(&["a", "a"])],
        |_, value| {
            assert!(
                matches!(value.data(), ValueData::Set(set) if set.elements().is_empty()),
                "{value:?}"
            )
        },
    );
    live(
        "+> number\n{number/format(number/parse-u64(s), 0u64) | s <- xs<[string]:1,2>}\n",
        &[
            strings(&["19", "019"]),
            strings(&["0", "18446744073709551615"]),
        ],
        |turn, value| {
            let expected = if turn == 0 {
                vec!["19"]
            } else {
                vec!["0", "18446744073709551615"]
            };
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Set(
                    expected
                        .into_iter()
                        .map(|text| ValueDataDraft::String(text.to_owned()))
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                )
            );
        },
    );
    live(
        "+> number\n{number/format(x, 0u64) | x <- [number/parse-u64(s) | s <- xs<[string]:1,2>]}\n",
        &[
            strings(&["19", "019"]),
            strings(&["0", "18446744073709551615"]),
        ],
        |turn, value| {
            let expected = if turn == 0 {
                vec!["19"]
            } else {
                vec!["0", "18446744073709551615"]
            };
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Set(
                    expected
                        .into_iter()
                        .map(|text| ValueDataDraft::String(text.to_owned()))
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                )
            );
        },
    );
    live(
        "Suffix(text<string>) => <string>\n  | text => text + \"!\".\n{Suffix(s) | s <- xs<[string]:1,2>}\n",
        &[strings(&["é", "é"]), strings(&["A", ""])],
        |turn, value| {
            let expected = if turn == 0 {
                vec!["é!"]
            } else {
                vec!["!", "A!"]
            };
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Set(
                    expected
                        .into_iter()
                        .map(|text| ValueDataDraft::String(text.to_owned()))
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                )
            );
        },
    );
}

#[test]
fn a_later_string_selection_error_preserves_the_previous_published_collection() {
    let (original, output) = compile("ys := [s[2u64] | s <- xs<[string]:1,2>]\nys\n");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x434f4c58, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let input = instance.plan.inputs[0].clone();
        let before = strings(&["ab", "cd"])
            .rebind(input.schema, &input.shape, artifact.schemas())
            .unwrap();
        instance
            .prepare_turn_values(&[CapturedValueInput {
                slot: input.slot,
                value: &before,
            }])
            .and_then(|prepared| prepared.publish())
            .unwrap();
        assert_eq!(
            string_values(&instance.copied_output(output).unwrap()),
            ["b", "d"]
        );
        let published = instance
            .copied_output(output)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let bad = strings(&["ef", ""])
            .rebind(input.schema, &input.shape, artifact.schemas())
            .unwrap();
        assert!(
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &bad
                }])
                .and_then(|prepared| prepared.publish())
                .is_err()
        );
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            published
        );
    }
}

static REPLACEMENT_BINDS: AtomicUsize = AtomicUsize::new(0);
static REPLACEMENT_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);

fn unclassified_execute(
    _: &BoundResidentKernel,
    _: &dyn ResidentKernelInputs,
    _: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    REPLACEMENT_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
    Err(ResidentKernelError::InvalidInput)
}

fn unclassified_bind(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    REPLACEMENT_BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(BoundResidentKernel::new(unclassified_execute, Box::new([])))
}

fn mutated_concat_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    REPLACEMENT_BINDS.fetch_add(1, Ordering::SeqCst);
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog
        .resident_factory(&["string".into()], "concat")
        .unwrap();
    let delegate = (entry.factory)(request)?;
    assert_eq!(
        delegate.work_accounting(),
        ResidentKernelWorkAccounting::RuntimeMetered
    );
    Ok(delegate.with_retained_state(Arc::new(19usize)))
}

fn mutated_comparison_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    REPLACEMENT_BINDS.fetch_add(1, Ordering::SeqCst);
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog.resident_factory(&["compare".into()], "gt").unwrap();
    let delegate = (entry.factory)(request)?;
    assert_eq!(
        delegate.work_accounting(),
        ResidentKernelWorkAccounting::RuntimeMetered
    );
    Ok(delegate.with_snapshot_schemas(request.schemas.clone()))
}

fn cloned_concat_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog
        .resident_factory(&["string".into()], "concat")
        .unwrap();
    Ok((entry.factory)(request)?.clone())
}

#[test]
fn a_genuine_unchanged_delegated_kernel_keeps_its_actual_accounting_and_behavior() {
    let (original, output) = compile("[s + \"!\" | s <- xs<[string]:1,2>]\n");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let catalog = replacement_catalog("string", "concat", cloned_concat_bind);
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x434f4c5b, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let input = instance.plan.inputs[0].clone();
        let value = strings(&["é", ""])
            .rebind(input.schema, &input.shape, artifact.schemas())
            .unwrap();
        instance
            .prepare_turn_values(&[CapturedValueInput {
                slot: input.slot,
                value: &value,
            }])
            .and_then(|prepared| prepared.publish())
            .unwrap();
        assert_eq!(
            string_values(&instance.copied_output(output).unwrap()),
            ["é!", "!"]
        );
    }
}

fn replacement_catalog(
    module: &str,
    operation: &str,
    factory: ResidentKernelFactory,
) -> FunctionCatalog {
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::new();
    let mut replaced = false;
    for (key, entry) in maintained.resident_factories() {
        let replacement =
            key.module_path.as_ref() == [module.to_owned()] && key.operation_name == operation;
        replaced |= replacement;
        builder
            .insert_resident_factory(
                key.module_path.iter().cloned(),
                key.operation_name.clone(),
                entry.implementation_memory,
                if replacement { factory } else { entry.factory },
            )
            .unwrap();
    }
    assert!(replaced);
    builder.build().unwrap()
}

#[test]
fn a_same_name_or_mutated_delegate_does_not_gain_an_implementation_accounting_claim() {
    let cases: &[(&str, &str, &str, ResidentKernelFactory)] = &[
        (
            "[s + \"!\" | s <- xs<[string]:1,2>]\n",
            "string",
            "concat",
            unclassified_bind,
        ),
        (
            "[s + \"!\" | s <- xs<[string]:1,2>]\n",
            "string",
            "concat",
            mutated_concat_bind,
        ),
        (
            "[x | x <- xs<[u64]:1,2>, x > 9007199254740992u64]\n",
            "compare",
            "gt",
            unclassified_bind,
        ),
        (
            "[x | x <- xs<[u64]:1,2>, x > 9007199254740992u64]\n",
            "compare",
            "gt",
            mutated_comparison_bind,
        ),
        // A newly admitted String Set must not inherit the old arbitrary
        // Snapshot-output work=0 path for an unclassified intermediate call.
        (
            "+> number\n{number/format(number/parse-u64(s), 0u64) | s <- xs<[string]:1,2>}\n",
            "number",
            "parse-u64",
            unclassified_bind,
        ),
        (
            "Suffix(text<string>) => <string>\n  | text => text + \"!\".\n{Suffix(s) | s <- xs<[string]:1,2>}\n",
            "string",
            "concat",
            unclassified_bind,
        ),
        (
            "+> number\n{number/format(x, 0u64) | x <- [number/parse-u64(s) | s <- xs<[string]:1,2>]}\n",
            "number",
            "parse-u64",
            unclassified_bind,
        ),
    ];
    for (source, module, operation, factory) in cases {
        let (original, _) = compile(source);
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        let catalog = replacement_catalog(module, operation, *factory);
        for artifact in [&original, &decoded] {
            REPLACEMENT_BINDS.store(0, Ordering::SeqCst);
            REPLACEMENT_EXECUTIONS.store(0, Ordering::SeqCst);
            let result = activate(
                ReactiveInstanceId::new(0x434f4c59, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            );
            let error = result
                .err()
                .expect("unclassified collection body must be rejected");
            assert!(
                matches!(
                    error,
                    ResidentActivationError::UnsupportedControlLayout { .. }
                ),
                "{source}: {error:?}"
            );
            assert!(REPLACEMENT_BINDS.load(Ordering::SeqCst) > 0);
            assert_eq!(REPLACEMENT_EXECUTIONS.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn unchanged_bound_kernel_clones_keep_accounting_but_semantic_builders_clear_it() {
    let (artifact, output) = compile("text<string> + \"!\"\n");
    let instance = activate(
        ReactiveInstanceId::new(0x434f4c5a, 0),
        &artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let kernel = instance
        .plan
        .steps
        .iter()
        .find_map(|step| match step {
            ActivatedTurnStep::Kernel(node) => Some(&node.kernel),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        kernel.work_accounting(),
        ResidentKernelWorkAccounting::RuntimeMetered
    );
    assert_eq!(
        kernel.clone().work_accounting(),
        ResidentKernelWorkAccounting::RuntimeMetered
    );
    let schema_id = artifact.outputs()[output].schema;
    let schema = artifact.schemas().get(schema_id).unwrap();
    let mutations = [
        kernel.clone().with_retained_state(Arc::new(1usize)),
        kernel
            .clone()
            .with_snapshot_schemas(artifact.schemas().clone()),
        kernel.clone().with_snapshot_output(ResidentSnapshotOutput {
            schema: schema_id,
            schema_key: schema.key(),
            shape: schema.instantiate_shape(Box::new([])).unwrap(),
            exact_cardinality: Some(1),
            maximum_cardinality: Some(1),
        }),
        kernel
            .clone()
            .with_bound_call(kernel.bound_call().unwrap().clone()),
    ];
    for changed in mutations {
        assert_eq!(
            changed.work_accounting(),
            ResidentKernelWorkAccounting::Unclassified
        );
    }
    assert_eq!(
        BoundResidentKernel::new(unclassified_execute, Box::new([])).work_accounting(),
        ResidentKernelWorkAccounting::Unclassified
    );
}
