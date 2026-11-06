//! The source check must still reject before any packed load or publication.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

#[test]
fn source_guarded_pack_errors_match_ordinary_execution_and_recover() {
    let source = super::guarded_read_packs::source(8, true, true)
        .replace(
            "  out := (more?",
            "  divisor := (((left > 0u64) && (at == 1u64))? | true => 0u64 | false => 1u64)\n  checked := word / divisor\n  out := (more?",
        )
        .replace("bits/xor(total, word)", "bits/xor(total, checked)");
    assert!(source.contains("checked := word / divisor"));
    let mut expected_errors = [None, None, None];
    let mut expected = None;
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for (reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for fault in [0, 1, 0, 2, 0, 3, 0] {
                    let (length, at, steps) = match fault {
                        0 => (32, 0, 1),
                        1 => (7, 0, 1),
                        2 => (32, u64::MAX, 1),
                        3 => (32, 0, 2),
                        _ => unreachable!(),
                    };
                    let args = super::read_packs::arguments(
                        &artifact,
                        &memory,
                        [1, length],
                        at,
                        steps,
                        41,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut published, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut published, mode.execution(), &memory)
                    };
                    if fault == 0 {
                        result.unwrap();
                        let actual = published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap();
                        assert_eq!(actual, *expected.get_or_insert(actual.clone()));
                        if !reference {
                            assert!(
                                !counts.tail_entries.is_empty(),
                                "real generated execution required"
                            );
                        }
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains("Arithmetic"), "{error}");
                        assert_eq!(
                            error,
                            *expected_errors[fault - 1].get_or_insert(error.clone())
                        );
                        if !reference && fault == 3 {
                            assert!(
                                counts.tail_boundaries.iter().any(|row| row.2 == 3),
                                "settle the selected error after a successful packed-read prefix: {counts:?}"
                            );
                        }
                        super::read_packs::unchanged_publication(&published, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn source_guarded_pack_finite_boundaries_keep_publication_aliases_and_retry() {
    resources::finite_boundaries(
        &super::guarded_read_packs::source(8, false, false),
        |artifact, memory| super::read_packs::arguments(artifact, memory, [1, 32], 2, 3, 41),
    );
}
