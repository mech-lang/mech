use super::*;

#[test]
fn composition_finite_boundaries_preserve_publication_contents_and_retry() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = huffman_inputs(&artifact, 3, vec![0, 1, 1, 2], &memory);
        let before = contents(&args);
        let mut published = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut published, mode.execution(), &memory)
            .unwrap();
        let expected = published
            .iter()
            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for dimension in [
            "control",
            "library",
            "comparison",
            "memory",
            "nodes",
            "transport",
        ] {
            let trial = |allowance| {
                let mut policy = mode.execution();
                let mut budget = memory.clone();
                match dimension {
                    "control" => policy.execution.max_work = allowance,
                    "library" => policy.library.max_compute_work = Some(allowance),
                    "comparison" => policy.library.max_comparison_work = Some(allowance),
                    "memory" => budget = memory.with_limits(allowance, u64::MAX),
                    "nodes" => budget = memory.with_limits(u64::MAX, allowance),
                    "transport" => policy.execution.max_transport_bytes = allowance,
                    _ => unreachable!(),
                }
                let mut outputs = published.clone();
                let (result, counts) = observe::run(&jit, &args, &mut outputs, policy, &budget);
                if result.is_ok() {
                    assert_eq!(
                        outputs
                            .iter()
                            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        expected
                    );
                } else {
                    for (value, prior) in outputs.iter().zip(&published) {
                        assert!(
                            value
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(prior.as_ref().unwrap())
                        );
                        assert_eq!(
                            value.as_ref().unwrap().canonical_data_draft().unwrap(),
                            prior.as_ref().unwrap().canonical_data_draft().unwrap()
                        );
                    }
                }
                assert_eq!(
                    outputs
                        .iter()
                        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    expected
                );
                drop(outputs);
                assert_eq!(contents(&args), before);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                (result.is_ok(), counts.selected)
            };
            // Find the actual first successful finite allowance, preserving the
            // same published roots throughout. Selection can decline lawfully.
            let mut lo = 0;
            let mut hi = 1_000_000_000u64;
            assert!(trial(hi).0, "{dimension}");
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if trial(mid).0 {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            let exact = trial(lo);
            assert!(exact.0);
            if lo > 0 {
                assert!(!trial(lo - 1).0);
            }
            assert!(trial(1_000_000_000).0);
            println!(
                "{}",
                serde_json::json!({"witness":"composition-finite-boundary", "dimension":dimension, "minimum":lo, "exact_selected_fragments":exact.1, "zero_spending":lo==0})
            );
        }
        drop((args, published));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
