//! A phase guard may decline, but it cannot change source errors or grant view
//! authority. These controls exercise the direct child route and image loader.
use super::*;
#[test]
fn nested_numeric_phase_keeps_assertion_bounds_and_arithmetic_order() {
    for source in sources() {
        let source = source.replace("values[at + 1u64]", "values[at + 4u64]");
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            qualify_nested_inventory(&jit);
            for restored in [false, true] {
                let mut published = vec![None; artifact.outputs().len()];
                for (steps, allowed, divisor, rejection) in [
                    (0, false, 0, None),
                    (1, false, 0, Some("Integrity")),
                    (1, true, 0, Some("Index")),
                    (0, false, 0, None),
                ] {
                    let args = arguments(
                        &artifact,
                        &memory,
                        steps,
                        true,
                        allowed,
                        u64::MAX,
                        divisor,
                        0,
                    );
                    let before = contents(&args);
                    let previous = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = if restored {
                        observe::run_restored(
                            &jit,
                            &args,
                            &mut published,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                    };
                    if let Some(rejection) = rejection {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains(rejection), "{error}");
                        for (now, old) in published.iter().zip(&previous) {
                            assert!(
                                now.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            );
                        }
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                    }
                    assert_eq!(contents(&args), before);
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
fn nested_numeric_phase_declines_unknown_progress_mutation_and_collection() {
    let no_progress = SOURCE.replace(
        "phase-candidates(values, end + 1u64,",
        "phase-candidates(values, end,",
    );
    let mutation = SOURCE.replace("word<u64> := (more? | true => values[at + 1u64] | false => 0u32)","changed := sequence/append(values, 0u32)\n  word<u64> := (more? | true => changed[at + 1u64] | false => 0u32)");
    for source in [&no_progress, &mutation, multiple_children::SOURCE] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            assert!(
                !jit.descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .any(|r| r.iteration == 0
                        && r.numeric
                            .as_ref()
                            .is_some_and(|p| p.phase.is_some() && p.exits.is_empty()))
            );
        }
    }
}
#[test]
fn nested_numeric_phase_images_cannot_grant_membership_storage_or_debits() {
    for artifact in sources().iter().flat_map(|s| artifacts(s)) {
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
                "/descriptor/closed_tails/regions/0/numeric/phase/loops/1/counter",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/loops/1/header",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/loops/1/compute",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/phase/compute_bound",
                serde_json::json!({"Constant":0}),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/sites/0",
                serde_json::json!(0),
            ),
            (
                "/descriptor/closed_tails/regions/0/numeric/arrays/0/field",
                serde_json::json!(99999),
            ),
        ] {
            let mut bad = image.clone();
            *bad.pointer_mut(path).unwrap() = replacement;
            assert!(
                super::super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&bad).unwrap()
                ),
                "{path}"
            );
        }
    }
}

#[test]
fn nested_numeric_phase_settles_nonempty_prefix_before_ordinary_retry() {
    use crate::native_artifact::program::prepare::numeric_phase::Bound;
    fn bound(expr: &Bound, length: u64) -> u64 {
        match expr {
            Bound::Constant(n) => *n,
            Bound::Length(_) | Bound::BufferLength(_) => length,
            Bound::Add(a, b) => bound(a, length).checked_add(bound(b, length)).unwrap(),
            Bound::Multiply(a, b) => bound(a, length).checked_mul(bound(b, length)).unwrap(),
            Bound::Maximum(a, b) => bound(a, length).max(bound(b, length)),
            Bound::Root(_) => panic!("witness envelope uses the current node length"),
        }
    }
    let source =
        include_str!("../../../../../tests/fixtures/native/numeric-nested-phase-growing.mec");
    let source = source.replace("305257u64", "16u64");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = inputs(
            &artifact,
            &[
                ("nodes", Data::Matrix([Data::U64(9)].into())),
                ("steps", Data::U64(3)),
                ("allowed", Data::Bool(true)),
                ("deny-at", Data::U64(u64::MAX)),
                ("divisor", Data::U64(1)),
            ],
            &memory,
        );
        let before = contents(&args);
        let mut published = vec![None; artifact.outputs().len()];
        let (result, _) = observe::run(&jit, &args, &mut published, mode.execution(), &memory);
        result.unwrap();
        // Measure this same prior-publication coexistence, including its live
        // alias, before choosing a finite account for the following visit.
        let prior = published.clone();
        let (result, unlimited) =
            observe::run(&jit, &args, &mut published, mode.execution(), &memory);
        result.unwrap();
        drop(prior);
        let phase = jit.descriptor().metadata().closed_tails.regions[0]
            .numeric
            .as_ref()
            .unwrap()
            .phase
            .as_ref()
            .unwrap();
        let before_entry = u64::MAX - unlimited.phase_initial_fuel[0];
        let after_write = u64::MAX - unlimited.phase_write_fuel[0];
        let limit = (before_entry + bound(&phase.compute_bound, 1)).max(after_write);
        assert!(limit < after_write + bound(&phase.compute_bound, 2));
        for restored in [false, true] {
            let mut policy = mode.execution();
            policy.execution.max_work = limit;
            let prior = published.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut published, policy, &memory)
            } else {
                observe::run(&jit, &args, &mut published, policy, &memory)
            };
            assert!(
                counts
                    .phase_settlements
                    .iter()
                    .any(|&(mode, roots, children, _)| mode == 5 && roots == 1 && children > 0)
            );
            assert!(
                counts
                    .phase_settlements
                    .iter()
                    .any(|&(mode, _, _, _)| mode == 1),
                "result={result:?} limit={limit} entry={before_entry} write={after_write} counts={counts:?}"
            );
            println!(
                "{}",
                serde_json::json!({"witness":"native-prefix-fallback","restored":restored,"limit":limit,"succeeded":result.is_ok(),"settlements":counts.phase_settlements})
            );
            if result.is_err() {
                for (new, old) in published.iter().zip(&prior) {
                    assert!(
                        new.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    );
                }
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            jit.evaluate_into(&args, &mut published, mode.execution(), &memory)
                .unwrap();
            assert_eq!(
                published
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([9, 9, 28, 75].map(Data::U64).into())
            );
            assert_eq!(contents(&args), before);
        }
        drop((args, published));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
