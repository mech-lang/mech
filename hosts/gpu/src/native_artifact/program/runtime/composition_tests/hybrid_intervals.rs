//! Nested fixed-size updates beside a real cold collection boundary.
use super::*;

fn source() -> String {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-collector-rebase.mec")
        .replacen("poke(nodes<[u16]>)", "poke-once(nodes<[u16]>)", 1);
    source.replace(
        "count(cache<[u32]>)",
        r#"poke-loop(nodes<[u16]>, at<u64>, end<u64>) => (out<[u16]>) :=
  more := at < end
  next := (more? | true => poke-once(nodes) | false => nodes)
  out := (more? | true => poke-loop(next, at + 1u64, end) | false => nodes).
poke(nodes<[u16]>) => (out<[u16]>) :=
  out := poke-loop(nodes, 0u64, 2u64).
count(cache<[u32]>)"#,
    )
}

#[test]
fn native_nested_updates_keep_a_cold_collection_exit() {
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| region.tail.parent().is_none()
                    && region.numeric.as_ref().is_some_and(|proof| {
                        proof.phase.as_ref().is_some_and(|phase| phase.loops.len() > 1)
                            && proof.exits.iter().any(|exit| matches!(
                                exit.kind,
                                crate::native_artifact::program::prepare::numeric_plan::ExitKind::Unsupported
                            ))
                    })),
            "a cold collection must not disqualify the parent's supported child loops"
        );
    }
}
