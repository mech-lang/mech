//! The real depth-counting prefix mutates a buffer before its cold scalar child.
use super::*;

fn source() -> String {
    format!(
        "{}\ncold-count(a<[u32]>, counts<[u64]>, root<u64>, maximum<u64>) => (out<([u32],[u64])>) :=\n  (depths, lengths) := huff-length-count-loop(a, counts, root, maximum)\n  out := (depths, lengths).\ncold-count(a<[u32]>, counts<[u64]>, root<u64>, maximum<u64>)\n",
        include_str!("../../../../../tests/fixtures/native/deflate-huffman-level12.mec")
    )
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    root: u64,
    maximum: u64,
    initial: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("a", Data::Matrix([1024, 2049, 2].map(Data::U32).into())),
            (
                "counts",
                Data::Matrix([0, initial, 0, 0, 0].map(Data::U64).into()),
            ),
            ("root", Data::U64(root)),
            ("maximum", Data::U64(maximum)),
        ],
        memory,
    )
}
fn unchanged(output: &[Option<Value>], prior: &[Option<Value>]) {
    for (actual, old) in output.iter().zip(prior) {
        let (actual, old) = (actual.as_ref().unwrap(), old.as_ref().unwrap());
        assert!(actual.shares_frozen_storage(old));
        assert_eq!(
            actual.canonical_data_draft().unwrap(),
            old.canonical_data_draft().unwrap()
        );
    }
}

#[test]
fn numeric_cold_child_preserves_post_write_state_errors_aliases_and_retry() {
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|r| r.iteration == 0)
            .unwrap_or_else(|| {
                let metadata = jit.descriptor().metadata();
                let mech_engine::native_artifact::NativeControlLoop::Tail(tail) =
                    &metadata.plan.iterations()[0]
                else {
                    panic!("root must be a Tail");
                };
                panic!(
                    "conditional scalar child must retain a native parent: {:?}",
                    crate::native_artifact::program::prepare::readonly_numeric::derive(
                        &metadata.plan,
                        tail,
                        &metadata.buffer_frames,
                        &metadata.scalar_aggregates,
                        &metadata.calls,
                        &metadata.visits
                    )
                );
            });
        let proof = region.numeric.as_ref().unwrap();
        let pending = region.cold_resume().unwrap();
        for other in &jit.descriptor().metadata().closed_tails.regions {
            if let Some(numeric) = &other.numeric {
                let shadow = other.shadow.unwrap();
                assert!(
                    !(shadow..shadow + numeric.words).contains(&pending),
                    "ordinary child scratch must not overwrite a parent's pending continuation"
                );
            }
        }
        assert_eq!(proof.exits.len(), 1);
        assert_eq!(proof.phase.as_ref().unwrap().cold.len(), 1);
        let cold_site = proof.exits[0].to as u64;
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for (name, root, maximum, initial, expected, reject, cold) in [
                ("normal", 2, 3, 2, vec![0, 1, 1, 2, 0], false, false),
                (
                    "taken-after-write",
                    2,
                    2,
                    2,
                    vec![0, 0, 4, 0, 0],
                    false,
                    true,
                ),
                ("failure-after-write", 2, 2, 1, vec![], true, true),
                (
                    "unselected-invalid-child",
                    0,
                    0,
                    2,
                    vec![0, 2, 0, 0, 0],
                    false,
                    false,
                ),
                ("retry", 2, 3, 2, vec![0, 1, 1, 2, 0], false, false),
            ] {
                let args = arguments(&artifact, &memory, root, maximum, initial);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if reject {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Arithmetic"), "{error}");
                    unchanged(&output, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    let a = if root == 0 {
                        vec![1024, 2049, 2]
                    } else {
                        vec![2048, 1025, 2]
                    };
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple(
                            vec![
                                Data::Matrix(a.into_iter().map(Data::U32).collect()),
                                Data::Matrix(expected.into_iter().map(Data::U64).collect())
                            ]
                            .into()
                        )
                    );
                }
                assert_eq!(
                    counts
                        .phase_settlements
                        .iter()
                        .filter(|row| row.0 == 6)
                        .count(),
                    usize::from(cold),
                    "{name}: {counts:?}"
                );
                assert_eq!(counts.cold_private_arrays.len(), usize::from(cold));
                assert!(
                    counts.cold_private_arrays.iter().all(|&n| n > 0),
                    "cold exit must follow actual native private writes"
                );
                if cold {
                    assert_eq!(counts.admits.get(&cold_site), Some(&1));
                } else {
                    assert!(!counts.admits.contains_key(&cold_site));
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                println!(
                    "cold-child case={name} restored={restored} selected={} cold={}",
                    counts.phases,
                    counts.cold_private_arrays.len()
                );
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_cold_child_finite_suffix_keeps_the_same_account_and_publication() {
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 2, 2, 2);
        let aliases = args.clone();
        let before = contents(&args);
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            let run = |output: &mut [Option<Value>], policy| {
                if restored {
                    observe::run_restored(&jit, &args, output, policy, &memory)
                } else {
                    observe::run(&jit, &args, output, policy, &memory)
                }
            };
            run(&mut output, mode.execution()).0.unwrap();
            let (full, _) = run(&mut output, mode.execution());
            let full = full.unwrap();
            let prior = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut finite = mode.execution();
            finite.execution.max_work = full.work - 1;
            let (denied, counts) = run(&mut output, finite);
            let error = format!("{:?}", denied.unwrap_err());
            assert!(error.contains("WorkLimit"), "{error}");
            assert_eq!(
                counts.cold_private_arrays.len(),
                1,
                "finite denial must follow the real cold exit"
            );
            assert!(counts.phase_settlements.iter().any(|row| row.0 == 6));
            unchanged(&output, &prior);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            let (retry, counts) = run(&mut output, mode.execution());
            assert_eq!(retry.unwrap().work, full.work);
            assert_eq!(counts.cold_private_arrays.len(), 1);
            drop((prior, output));
        }
        drop((args, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_cold_child_images_reconstruct_edges_versions_and_debits() {
    for artifact in artifacts(&source()) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        for (path, replacement) in [
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/from",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/to",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/exits/0/aliases",
                serde_json::json!([]),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/cold/0/initialize",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/cold/0/exit",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/loops/0/compute",
                serde_json::json!(0),
            ),
        ] {
            let mut bad = image.clone();
            let current = bad.pointer_mut(path).unwrap();
            assert_ne!(*current, replacement, "mutation must change the recipe");
            *current = replacement;
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&bad).unwrap()
                ),
                "{path}"
            );
        }
    }
}
