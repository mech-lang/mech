//! Perturb only selected-region evidence. Mutated proofs are never emitted.
use super::*;
use crate::native_artifact::program::prepare::numeric_plan::{Exit, ExitKind};
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[test]
fn complementary_cases_refuse_missing_members_and_region_frontiers() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x424f4f4c), Revision(1),
            "+> bits\nwalk(xs<[u8]>, at<u64>, left<u64>, total<u64>) => (out<u64>) :=\n  more := left > 0u64\n  ready := at < 4u64\n  word<u64> := (ready? | false => 0u8 | true => xs[at + 1u64])\n  out := (more? | true => walk(xs, at + 1u64, left - 1u64, bits/xor(total, word)) | false => total).\nwalk(bytes<[u8]>, at<u64>, steps<u64>, seed<u64>)\n"
        ).unwrap(), ParseConfig::default(),
    );
    let artifact = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let native = JitProgramArtifact::compile_with_limits(
        &artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = native.descriptor().metadata();
    let facts = metadata.plan.facts();
    let proof = metadata
        .closed_tails
        .regions
        .iter()
        .filter_map(|region| region.numeric.as_ref())
        .find(|proof| !derive(&facts, proof).is_empty())
        .unwrap();
    let (&site, _) = derive(&facts, proof).iter().next().unwrap();
    let &[previous] = facts.predecessors(site as u32).unwrap() else {
        panic!("sole predecessor");
    };
    let &[producer] = facts.predecessors(previous).unwrap() else {
        panic!("fresh producer");
    };
    for member in [site, previous as usize, producer as usize] {
        let mut missing = proof.clone();
        missing.sites.retain(|&at| at != member);
        assert!(!derive(&facts, &missing).contains_key(&site));
        for (from, to) in [(member, 0), (0, member)] {
            let mut frontier = proof.clone();
            frontier.exits.push(Exit {
                from,
                to,
                frames: vec![],
                aliases: vec![],
                values: vec![],
                retire_carrier: false,
                kind: ExitKind::Unsupported,
            });
            assert!(!derive(&facts, &frontier).contains_key(&site));
        }
        for field in ["header", "initialize", "latch", "finish", "exit"] {
            let mut phase = serde_json::json!({"loops": [{
                "iteration": 0, "header": 0, "initialize": 0, "latch": 0,
                "finish": 0, "exit": 0, "parent": null, "counter": 0,
                "first_visit": null, "compute": 0, "comparison": 0, "trips": null
            }], "cold": [], "boundary_work": 0, "compute_bound": {"Constant": 0},
                "comparison_bound": {"Constant": 0}, "checked_children": false});
            phase["loops"][0][field] = serde_json::json!(member);
            let mut frontier = proof.clone();
            frontier.phase = Some(serde_json::from_value(phase).unwrap());
            assert!(!derive(&facts, &frontier).contains_key(&site));
        }
    }
}
