//! Negative source controls stay compiled; mutated selection proofs never run.
use super::*;
use crate::native_artifact::program::prepare::numeric_plan::{Exit, ExitKind};
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const SOURCE: &str = include_str!("../../../../tests/fixtures/native/guarded-packed-read.mec");

fn compile(source: &str) -> JitProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57494e44), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{source}");
    let artifact = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    JitProgramArtifact::compile_with_limits(
        &artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap()
}

#[test]
fn source_window_facts_refuse_unproved_paths_members_and_boundaries() {
    let native = compile(SOURCE);
    let metadata = native.descriptor().metadata();
    let facts = metadata.plan.facts();
    let proof = metadata
        .closed_tails
        .regions
        .iter()
        .filter_map(|r| r.numeric.as_ref())
        .find(|p| derive(&facts, p).iter().any(|p| p.source_window))
        .unwrap();
    let pack = derive(&facts, proof)
        .into_iter()
        .find(|p| p.source_window)
        .unwrap();
    let mut members = vec![pack.first];
    let mut site = pack.first;
    loop {
        let &[prior] = facts.predecessors(site as u32).unwrap() else {
            panic!("current guard chain")
        };
        members.push(prior as usize);
        site = prior as usize;
        if matches!(proof.action(site), Some(Operation::Length { .. })) {
            break;
        }
    }
    assert!(members.len() >= 6);
    let refuses = |p: &Proof| {
        !derive(&facts, p)
            .iter()
            .any(|p| p.first == pack.first && p.source_window)
    };
    for member in members {
        let mut missing = proof.clone();
        missing.sites.retain(|&s| s != member);
        assert!(refuses(&missing), "missing selected source {member}");
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
            assert!(refuses(&frontier), "cold endpoint {member}");
        }
        for field in ["header", "initialize", "latch", "finish", "exit"] {
            let mut phase = serde_json::json!({"loops":[{"iteration":0,"header":0,"initialize":0,"latch":0,"finish":0,"exit":0,"parent":null,"counter":0,"first_visit":null,"compute":0,"comparison":0,"trips":null}],"cold":[],"boundary_work":0,"compute_bound":{"Constant":0},"comparison_bound":{"Constant":0},"checked_children":false});
            phase["loops"][0][field] = serde_json::json!(member);
            let mut frontier = proof.clone();
            frontier.phase = Some(serde_json::from_value(phase).unwrap());
            assert!(refuses(&frontier), "funding {field} {member}");
        }
    }
    let original_arm = "| true => read-word(xs, at) | false => 0u64";
    let wrong_array = SOURCE
        .replace("walk(xs<[u8]>,", "walk(xs<[u8]>, ys<[u8]>,")
        .replace("sequence/length(xs)", "sequence/length(ys)")
        .replace("walk(xs, at +", "walk(xs, ys, at +")
        .replace("walk(bytes<[u8]>,", "walk(bytes<[u8]>, other<[u8]>,");
    for (name, source) in [
        ("wrapping subtraction", SOURCE.replace("available - 4u64", "bits/wrapping-sub(available, 4u64)")),
        ("shorter window", SOURCE.replace("available - 4u64", "available - 3u64")),
        ("false arm", SOURCE.replace(original_arm, "| false => read-word(xs, at) | true => 0u64")),
        ("different array", wrong_array),
        ("merged length", SOURCE.replace("available := sequence/length(xs)", "available := ((left > 1u64)? | true => sequence/length(xs) | false => sequence/length(xs) - 1u64)")),
    ] {
        assert_ne!(source, SOURCE);
        let native = compile(&source);
        let metadata = native.descriptor().metadata();
        let packs = metadata.closed_tails.regions.iter().filter_map(|r| r.numeric.as_ref())
            .flat_map(|p| derive(&metadata.plan.facts(), p)).collect::<Vec<_>>();
        assert!(!packs.is_empty(), "{name} must test refusal inside a compiled packed region");
        assert!(!packs.iter().any(|p| p.source_window), "{name}");
    }
}
