#![cfg(all(
    feature = "source_default",
    feature = "resident-artifact",
    feature = "string"
))]

use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft};
use mech_engine::__resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifact(source: &str) -> mech_engine::ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x551), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap()
}

fn expected(values: &[&str]) -> ValueDataDraft {
    ValueDataDraft::Set(
        values
            .iter()
            .map(|value| ValueDataDraft::String((*value).to_owned()))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

#[test]
fn closed_string_set_comprehension_preserves_unicode_and_deduplicates() {
    let source = "{x | x <- [\"é\" \"A\" \"é\" \"é\" \"🧑‍🚀\" \"\"]}\n";
    let original = artifact(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (index, artifact) in [&original, &decoded].into_iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x551, index as u32),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{error:?}\n{artifact:#?}"));
        instance.turn(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        assert_eq!(
            output.canonical_data_draft().unwrap(),
            expected(&["", "A", "é", "é", "🧑‍🚀"])
        );
    }
}

#[test]
fn live_string_set_comprehension_rebinds_cardinality_and_recovers_after_denied_work() {
    let original = artifact("{x | x <- signal<[string]:1,6>}\n");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (index, artifact) in [&original, &decoded].into_iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x552, index as u32),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (input, result) in [
            (
                ["é", "A", "é", "é", "🧑‍🚀", ""],
                vec!["", "A", "é", "é", "🧑‍🚀"],
            ),
            (["x", "x", "x", "x", "x", "x"], vec!["x"]),
        ] {
            let input = input.map(str::to_owned);
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap();
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected(&result)
            );
        }
        // Keep the former denial fixture: tighter String-set reservation now
        // admits its 6 * 3 * (floor(log2(6)) + 1) * 1024 = 55,296 comparison
        // units under the unchanged 65,536 limit. Do not restore removed work
        // or raise a limit just to retain the obsolete rejection expectation.
        let accepted = vec!["x".repeat(1024); 6];
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&accepted),
            }])
            .unwrap();
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected(&[accepted[0].as_str()])
        );
        let before = instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let epoch = instance.published_epoch();
        // Doubling only the key width requires 110,592 comparison units. The
        // real byte-aware reservation must fail before changing publication.
        let denied = vec!["x".repeat(2048); 6];
        let error = instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&denied),
            }])
            .unwrap_err();
        assert!(
            matches!(error, mech_engine::__resident::ResidentExecutionError::Kernel {
                error: mech_core::ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                    required,
                    limit,
                    ..
                }, ..
            } if required > limit && limit == mech_core::RESIDENT_MAX_COMPARISON_WORK),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before
        );
        let recovered = ["z", "A", "z", "A", "", ""].map(str::to_owned);
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&recovered),
            }])
            .unwrap();
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected(&["", "A", "z"])
        );
    }
}
