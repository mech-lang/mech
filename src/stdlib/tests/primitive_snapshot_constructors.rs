//! Exact maintained fixed-scalar constructors retain ordinary collection admission.
#![cfg(feature = "full_compiler")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, ReactiveInstanceId, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentValueMut, ResidentValueRef, Value, ValueData,
};
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentActivationError, activate,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

// Preserve the original collection fixture. Its typed constant matrix can be
// folded into one artifact constant, so it alone cannot prove constructor
// replacement admission.
const SOURCE: &str = "+> number\n{char + number/format(index, 0u64) | char <- signal<string>, index <- [1u64 2u64]}\n";

// A generator yields one grapheme at a time, so length(char) is always one.
// This preserves the same outputs while forcing the constructor to depend on
// a live local. The artifact inventory and bind counter below prove selection.
const LIVE_CONSTRUCTOR_SOURCE: &str = "+> number\n+> sequence\n{char + number/format(index, 0u64) | char <- signal<string>, index <- [sequence/length(char) 2u64]}\n";

fn compile() -> ([ProgramArtifact; 2], usize) {
    compile_source(SOURCE)
}

fn compile_source(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5053434f), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
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

fn instance(artifact: &ProgramArtifact, ordinal: usize) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x5053434f, ordinal as u32),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn run(instance: &mut ReactiveInstance, text: String) -> Result<(), String> {
    assert_eq!(instance.plan.inputs.len(), 1);
    let slot = instance.plan.inputs[0].slot;
    let values = [text];
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&values),
        }])
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn assert_set(value: &Value, expected: &[&str]) {
    let ValueData::Set(set) = value.data() else {
        panic!("{value:?}")
    };
    let actual = set
        .elements()
        .iter()
        .map(|item| {
            let ValueData::String(value) = item.data() else {
                panic!("{item:?}")
            };
            value.as_ref()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn nested_fixed_scalar_constructor_composes_with_live_unicode_and_empty_strings() {
    let (artifacts, output) = compile();
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (text, expected) in [
            ("", &[][..]),
            ("éA", &["A1", "A2", "é1", "é2"][..]),
            ("AA", &["A1", "A2"][..]),
            ("e\u{301}", &["e\u{301}1", "e\u{301}2"][..]),
            ("", &[][..]),
        ] {
            run(&mut instance, text.into()).unwrap();
            assert_set(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn finite_collection_denial_retains_publication_and_recovers() {
    let (artifacts, output) = compile();
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        run(&mut instance, "A".into()).unwrap();
        let before = instance.copied_output(output).unwrap();
        let epoch = instance.published_epoch();
        assert!(run(&mut instance, "x".repeat(65_536)).is_err());
        assert_eq!(instance.published_epoch(), epoch);
        assert!(
            instance
                .copied_output(output)
                .unwrap()
                .shares_frozen_storage(&before)
        );
        run(&mut instance, "B".into()).unwrap();
        assert_set(&instance.copied_output(output).unwrap(), &["B1", "B2"]);
    }
}

static REPLACEMENT_BINDS: AtomicUsize = AtomicUsize::new(0);
static REPLACEMENT_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);

struct ConstructorDelegate(BoundResidentKernel);

fn unclassified_execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    struct Forward<'a>(&'a dyn ResidentKernelInputs);
    impl ResidentKernelInputs for Forward<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index)
        }
    }
    REPLACEMENT_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
    // If a gate regression reaches this executor, it runs the genuine bound
    // constructor rather than manufacture an unrelated execution failure.
    kernel
        .retained_state::<ConstructorDelegate>()
        .unwrap()
        .0
        .execute(&Forward(inputs), output)
}
fn unclassified_bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let catalog = mech_stdlib::source_catalog();
    let maintained = catalog
        .resident_factory(&["matrix".to_owned()], "horzcat")
        .unwrap();
    let genuine = (maintained.factory())(request)?;
    assert_eq!(
        genuine.work_accounting(),
        ResidentKernelWorkAccounting::RuntimeMetered
    );
    let metadata = genuine.snapshot_output().unwrap().clone();
    assert_eq!(metadata.schema, request.output.schema_id);
    assert_eq!(metadata.schema_key, request.output.schema_key);
    assert_eq!(metadata.shape, request.output.shape_instance);
    let schemas = genuine.snapshot_schemas().unwrap().clone();
    let parameters = genuine.parameters().to_vec().into_boxed_slice();
    // Reattaching the unchanged semantic metadata legitimately clears the
    // finished implementation's work declaration while preserving its plan.
    let genuine = genuine.with_snapshot_output(metadata.clone());
    assert_eq!(
        genuine.work_accounting(),
        ResidentKernelWorkAccounting::Unclassified
    );
    let replacement = BoundResidentKernel::new(unclassified_execute, parameters)
        .with_snapshot_output(metadata)
        .with_snapshot_schemas(schemas)
        .with_retained_state(Arc::new(ConstructorDelegate(genuine)));
    assert_eq!(
        replacement.work_accounting(),
        ResidentKernelWorkAccounting::Unclassified
    );
    REPLACEMENT_BINDS.fetch_add(1, Ordering::SeqCst);
    Ok(replacement)
}

#[test]
fn same_name_unclassified_constructor_is_rejected_before_execution() {
    let (artifacts, output) = compile_source(LIVE_CONSTRUCTOR_SOURCE);
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::new();
    let mut replaced = false;
    for (key, entry) in maintained.resident_factories() {
        let replace =
            key.module_path.as_ref() == ["matrix".to_owned()] && key.operation_name == "horzcat";
        replaced |= replace;
        builder
            .insert_resident_factory(
                key.module_path.iter().cloned(),
                key.operation_name.clone(),
                entry.implementation_memory(),
                if replace {
                    unclassified_bind
                } else {
                    entry.factory()
                },
            )
            .unwrap();
    }
    assert!(replaced);
    let catalog = builder.build().unwrap();
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let operations = artifact
            .operation_references()
            .into_iter()
            .map(|operation| operation.canonical_name())
            .collect::<Vec<_>>();
        assert!(
            operations.iter().any(|name| name == "matrix/horzcat"),
            "live constructor was folded or routed elsewhere: {operations:?}"
        );
        // The exact dynamic fixture must activate and retain the original
        // Unicode outputs with the maintained catalog before substitution.
        let mut positive = instance(artifact, ordinal);
        run(&mut positive, "éA".into()).unwrap();
        assert_set(
            &positive.copied_output(output).unwrap(),
            &["A1", "A2", "é1", "é2"],
        );

        let binds_before = REPLACEMENT_BINDS.load(Ordering::SeqCst);
        assert_eq!(REPLACEMENT_EXECUTIONS.load(Ordering::SeqCst), 0);
        match activate(
            ReactiveInstanceId::new(0x50534350, ordinal as u32),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        ) {
            Err(error) => assert!(
                matches!(
                    &error,
                    ResidentActivationError::UnsupportedControlLayout { .. }
                ),
                "unexpected admission error: {error:?}; operations: {operations:?}"
            ),
            Ok(_) => panic!("unclassified replacement was unexpectedly admitted"),
        }
        assert!(
            REPLACEMENT_BINDS.load(Ordering::SeqCst) > binds_before,
            "same-name replacement factory was never selected: {operations:?}"
        );
        assert_eq!(REPLACEMENT_EXECUTIONS.load(Ordering::SeqCst), 0);
    }
    assert_eq!(REPLACEMENT_EXECUTIONS.load(Ordering::SeqCst), 0);
}
