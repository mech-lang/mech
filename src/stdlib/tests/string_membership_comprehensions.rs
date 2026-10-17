//! Allocation-free maintained String membership composes with ordinary control.
#![cfg(feature = "full_compiler")]

use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, ReactiveInstanceId, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs, ResidentValueMut,
    ResidentValueRef, ValueData, snapshot::SequenceView,
};
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ResidentActivationError, ResidentExecutionError, activate,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicUsize, Ordering};

const DIRECT: &str = "[signal<string> ∈ {\"\", \"é\", \"x\", \"é\", \"🧑‍🚀\"} | ignored <- [1u64]]\n";

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x534d454d), Revision(1), source).unwrap(),
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
        .unwrap();
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|value| value.kind == SourceDocumentOutputKind::Program)
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

fn check(source: &str, cases: &[(&str, bool)]) {
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x534d454d, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (text, expected) in cases {
            let input = [(*text).to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap();
            let actual = instance.copied_output(output).unwrap();
            let ValueData::Matrix(matrix) = actual.data() else {
                panic!("{actual:?}")
            };
            let SequenceView::Bool(values) = matrix.elements() else {
                panic!("{actual:?}")
            };
            assert_eq!(values, &[*expected]);
        }
    }
}

#[test]
fn direct_and_nested_string_membership_preserve_raw_keys_and_live_replacements() {
    let cases = [
        ("é", true),
        ("é", true),
        ("É", false),
        ("🧑‍🚀", true),
        ("", true),
        ("x", true),
        ("y", false),
        ("x", true),
    ];
    check(DIRECT, &cases);
    check(
        "Has(query<string>) => (found<bool>) := found := query ∈ {\"\", \"é\", \"x\", \"é\", \"🧑‍🚀\"}.\n[Has(signal<string>) | ignored <- [1u64]]\n",
        &cases,
    );
    check(
        "empty<{string}> := {}\n[signal<string> ∈ empty | ignored <- [1u64]]\n",
        &[("", false), ("é", false)],
    );
}

#[test]
fn denied_large_candidate_retains_published_value_and_recovers() {
    let (artifacts, output) = compile(DIRECT);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x534d454e, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let input = ["é".to_owned()];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&input),
            }])
            .unwrap();
        let before = instance
            .copied_output(output)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let epoch = instance.published_epoch();
        let large = ["x".repeat(mech_core::RESIDENT_MAX_COMPARISON_WORK as usize + 1)];
        let error = instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&large),
            }])
            .unwrap_err();
        assert!(
            matches!(
                error,
                ResidentExecutionError::Kernel {
                    error: ResidentKernelError::ResourceLimit {
                        phase: mech_core::ResidentResourcePhase::Admission,
                        dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                        required: 65_910,
                        limit: 65_536,
                    },
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before
        );
        let recovered = ["y".to_owned()];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&recovered),
            }])
            .unwrap();
        let actual = instance.copied_output(output).unwrap();
        let ValueData::Matrix(matrix) = actual.data() else {
            panic!("{actual:?}")
        };
        let SequenceView::Bool(values) = matrix.elements() else {
            panic!("{actual:?}")
        };
        assert_eq!(values, &[false]);
    }
}

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
    Ok(BoundResidentKernel::new(unclassified_execute, Box::new([])))
}

#[test]
fn same_name_unclassified_membership_is_rejected_before_any_execution() {
    let (artifacts, _) = compile(DIRECT);
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::new();
    let mut replaced = false;
    for (key, entry) in maintained.resident_factories() {
        let replace =
            key.module_path.as_ref() == ["set".to_owned()] && key.operation_name == "element-of";
        replaced |= replace;
        builder
            .insert_resident_factory(
                key.module_path.iter().cloned(),
                key.operation_name.clone(),
                entry.implementation_memory,
                if replace {
                    unclassified_bind
                } else {
                    entry.factory
                },
            )
            .unwrap();
    }
    assert!(replaced);
    let catalog = builder.build().unwrap();
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        assert!(matches!(
            activate(
                ReactiveInstanceId::new(0x534d454f, ordinal as u32),
                artifact,
                &catalog,
                &ActivationFacts::default()
            ),
            Err(ResidentActivationError::UnsupportedControlLayout { .. })
        ));
    }
    assert_eq!(REPLACEMENT_EXECUTIONS.load(Ordering::SeqCst), 0);
}
