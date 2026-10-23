//! Actual maintained map projection inside ordinary String Set construction.
#![cfg(feature = "full_compiler")]

use std::sync::atomic::{AtomicUsize, Ordering};

use mech_core::{
    BoundResidentKernel, FunctionCatalog, FunctionCatalogBuilder, ReactiveInstanceId,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentValueMut, ResidentValueRef, Value, ValueData,
};
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ResidentActivationError, activate,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d4b4559), Revision(1), source).unwrap(),
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

fn check(source: &str, cases: &[&str], expected: impl Fn(usize, &Value)) {
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x4d4b4559, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (turn, text) in cases.iter().enumerate() {
            let input = [(*text).to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap_or_else(|error| panic!("{source}: turn {turn}: {error:?}"));
            expected(turn, &instance.copied_output(output).unwrap());
        }
    }
}

const DIRECT: &str = "+> map\n+> text\ncounts := map/histogram(text/lines(signal<string>))\n{key | key <- map/keys(counts)}\n";

#[test]
fn direct_keys_set_uses_actual_projection_for_empty_unicode_and_changed_cardinality() {
    let cases = ["", "é\né\né\n🧑‍🚀", "new\nnew", ""];
    let expected: &[&[&str]] = &[&[], &["é", "é", "🧑‍🚀"], &["new"], &[]];
    check(DIRECT, &cases, |turn, value| {
        let ValueData::Set(set) = value.data() else {
            panic!("{value:?}")
        };
        let actual = set
            .elements()
            .iter()
            .map(|item| {
                let ValueData::String(text) = item.data() else {
                    panic!("{item:?}")
                };
                text.as_ref()
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected[turn]);
    });
}

#[test]
fn nested_function_keys_set_composes_with_lookup_format_and_string_collection() {
    let source = "+> map\n+> number\n+> text\nlookup(counts<{string:u64}>, query<string>) => (output<string>) :=\n  keys := {key | key <- map/keys(counts)}\n  count := (query ∈ keys)? | true => counts{query} | false => 0u64.\n  output := number/format(count, 0u64) + \"\\t\" + query + \"\\n\".\ncounts := map/histogram(text/lines(signal<string>))\nqueries := [\"é\" \"é\" \"🧑‍🚀\" \"missing\"]\ntext/join([lookup(counts, query) | query <- queries], \"\")\n";
    let expected = [
        "0\té\n0\té\n0\t🧑‍🚀\n0\tmissing\n",
        "2\té\n1\té\n1\t🧑‍🚀\n0\tmissing\n",
        "0\té\n0\té\n0\t🧑‍🚀\n0\tmissing\n",
    ];
    check(
        source,
        &["", "é\né\né\n🧑‍🚀", "new\nnew"],
        |turn, value| {
            assert!(
                matches!(value.data(), ValueData::String(text) if text.as_ref() == expected[turn]),
                "{value:?}"
            );
        },
    );
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

fn replacement_catalog() -> FunctionCatalog {
    let maintained = mech_stdlib::source_catalog();
    let mut builder = FunctionCatalogBuilder::new();
    let mut replaced = false;
    for (key, entry) in maintained.resident_factories() {
        let replace =
            key.module_path.as_ref() == ["map".to_owned()] && key.operation_name == "keys";
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
    builder.build().unwrap()
}

#[test]
fn same_name_unclassified_keys_replacement_is_rejected_without_execution() {
    let (artifacts, _) = compile(DIRECT);
    let catalog = replacement_catalog();
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        assert!(matches!(
            activate(
                ReactiveInstanceId::new(0x4d4b455a, ordinal as u32),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            ),
            Err(ResidentActivationError::UnsupportedControlLayout { .. })
        ));
    }
    assert_eq!(REPLACEMENT_EXECUTIONS.load(Ordering::SeqCst), 0);
}
