use super::*;
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(256), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    [original, decoded]
}
fn prepared(artifact: &ProgramArtifact) -> PreparedNativeProgram {
    let catalog = mech_stdlib::source_catalog();
    let plan =
        mech_engine::native_artifact::lower_native_program_artifact_with_definition_table_limit(
            artifact,
            &catalog,
            u64::MAX,
        )
        .unwrap();
    PreparedNativeProgram::prepare(artifact, &catalog, plan).unwrap()
}

#[test]
fn private_fixed_nested_contract_leaves_ordinary_funding_refusal_intact() {
    let source = include_str!("../../../../tests/fixtures/native/numeric-private-fixed-nested.mec");
    for artifact in artifacts(source) {
        let prepared = prepared(&artifact);
        let metadata = prepared.metadata();
        let layout = Layout::derive(prepared.plan()).unwrap();
        let region = metadata
            .closed_tails
            .regions
            .iter()
            .find(|r| {
                r.numeric.as_ref().is_some_and(|p| {
                    p.phase
                        .as_ref()
                        .is_some_and(|s| s.checked_children && s.loops.len() == 2)
                })
            })
            .unwrap();
        let proof = region.numeric.as_ref().unwrap();
        let refusal = crate::native_artifact::program::prepare::readonly_numeric::derive_complete_for_diagnostic(
            &metadata.plan, &region.tail, &metadata.buffer_frames,
            &metadata.scalar_aggregates, &metadata.calls, &metadata.visits,
        ).unwrap_err();
        assert_eq!(
            refusal,
            "mutable children require current-account visit funding"
        );
        assert!(proof.phase.as_ref().unwrap().checked_children);
        assert_eq!(proof.exits.len(), 2);
        assert!(proof.exits.iter().all(|e| e.kind != ExitKind::Unsupported));
        let contract = PrivatePhase::derive(&prepared, region, &layout)
            .unwrap()
            .unwrap();
        assert!(contract.permits(proof));
        assert!(
            !contract.permits(&proof.clone()),
            "a copied proof has no authority at this boundary"
        );
        assert!(contract.handoffs.is_empty());
        assert_eq!(contract.packet_words(), 0);
    }
}

#[test]
fn private_fixed_nested_handoffs_are_exact_current_scalar_and_alias_exports() {
    let source = include_str!("../../../../tests/fixtures/native/numeric-nested-rebase.mec");
    for artifact in artifacts(source) {
        let prepared = prepared(&artifact);
        let layout = Layout::derive(prepared.plan()).unwrap();
        let mut handoffs = 0;
        for region in &prepared.metadata().closed_tails.regions {
            if let Some(contract) = PrivatePhase::derive(&prepared, region, &layout).unwrap() {
                let proof = region.numeric.as_ref().unwrap();
                let shadow = region.shadow.unwrap();
                let exports = contract.exports(shadow);
                for exit in &contract.handoffs {
                    handoffs += 1;
                    assert_eq!(proof.action(exit.site), Some(&Operation::Fallback));
                    assert!(
                        proof
                            .exits
                            .iter()
                            .any(|e| e.to == exit.site && e.kind == ExitKind::Unsupported)
                    );
                    assert_eq!(exports[&exit.site].len(), exit.exports.len());
                    for &(at, _) in &exports[&exit.site] {
                        assert!(at > 0 && at < contract.packet_words());
                    }
                    for &owner in &exit.owners {
                        assert!(
                            exit.outputs
                                .iter()
                                .any(|&(to, ref from)| to == owner
                                    && matches!(from, Output::Word(_)))
                        );
                    }
                }
            }
        }
        assert!(
            handoffs >= 2,
            "outer and nested collection boundaries must both export current state"
        );
    }
}

#[test]
fn private_fixed_nested_contract_declines_growing_storage() {
    let source = include_str!("../../../../tests/fixtures/native/numeric-nested-phase-growing.mec");
    for artifact in artifacts(source) {
        let prepared = prepared(&artifact);
        let layout = Layout::derive(prepared.plan()).unwrap();
        assert!(
            prepared
                .metadata()
                .closed_tails
                .regions
                .iter()
                .all(|region| {
                    PrivatePhase::derive(&prepared, region, &layout)
                        .unwrap()
                        .is_none()
                })
        );
    }
}
