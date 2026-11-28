//! Rare source branches retain native execution under fixed finite fuel.
use super::*;
use mech_core::snapshot::F64Bits;

const SOURCE: &str = r#"+> stats
#Occasional(initial<[f64]:4,3>, rounds<u64>, trigger<u64>) => <[f64]:4,3>
  | :Step(data<[f64]:4,3>, pass<u64>)
  | :Done(data<[f64]:4,3>).
#Occasional(initial, rounds, trigger) -> :Step(initial,0u64)
  :Step(data,pass)
    | pass >= rounds -> :Done(data)
    | pass == trigger -> {
        total := stats/sum/column([data[k,1u64] | k <- 1u64..=4u64])[1u64]
        next := [[data[i,j] + total | i <- 1u64..=4u64]' | j <- 1u64..=3u64]
      } -> :Step(next,pass + 1u64)
    | * -> :Step(data,pass + 1u64)
  :Done(data) => data.
#Occasional(input<[f64]:4,3>,rounds<u64>,trigger<u64>)
"#;

#[test]
fn numeric_summary_rare_branches_pay_only_reached_maps_and_folds_with_finite_fuel() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|r| {
                r.numeric
                    .as_ref()
                    .is_some_and(|p| p.phase.as_ref().is_some_and(|s| s.summaries.len() == 2))
            })
            .expect("both exact source operations share the state-machine phase");
        let phase = region.numeric.as_ref().unwrap().phase.as_ref().unwrap();
        let memory = mode.memory_budget();
        let mut limits = mode.execution();
        limits.execution.max_work = 1_000_000_000_000;
        limits.library.max_compute_work = Some(1_000_000_000_000);
        limits.library.max_comparison_work = Some(1_000_000_000_000);
        let original = (1..=12)
            .map(|v| Data::F64(F64Bits::from_f64(v as f64)))
            .collect::<Vec<_>>();
        let mut outputs = vec![None; artifact.outputs().len()];
        for restored in [false, true] {
            for (rounds, trigger) in [
                (0, 0),
                (4096, 0),
                (4096, 2048),
                (4096, 4095),
                (4096, u64::MAX),
                (1, 0),
            ] {
                let args = super::numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", Data::Matrix(original.clone().into()), &[4, 3]),
                        ("rounds", Data::U64(rounds), &[]),
                        ("trigger", Data::U64(trigger), &[]),
                    ],
                );
                let before = contents(&args);
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, limits, &memory)
                } else {
                    observe::run(&jit, &args, &mut outputs, limits, &memory)
                };
                let report = result.unwrap();
                let ran = trigger < rounds;
                let expected = (1..=12)
                    .map(|v| Data::F64(F64Bits::from_f64(v as f64 + if ran { 22.0 } else { 0.0 })))
                    .collect::<Vec<_>>();
                // Physical source payloads are row-major: the first column is 1,4,7,10.
                assert_eq!(
                    outputs
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into())
                );
                assert_eq!(contents(&args), before);
                for debit in &phase.summaries {
                    assert_eq!(
                        counts
                            .phase_summary_attempts
                            .get(&debit.site)
                            .copied()
                            .unwrap_or(0),
                        u64::from(ran),
                        "{counts:?}"
                    );
                    assert_eq!(
                        counts
                            .admits
                            .get(&(debit.site as u64))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "summary must stay native: {counts:?}"
                    );
                }
                assert!(
                    report.visits < 100,
                    "thousands of scalar steps must remain native: {report:?}"
                );
            }
        }
        outputs.fill(None);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_summary_fuel_miss_preserves_original_replay_retry_and_authenticated_counters() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let (region_id, region) = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .enumerate()
            .find(|(_, r)| {
                r.numeric
                    .as_ref()
                    .is_some_and(|p| p.phase.as_ref().is_some_and(|s| s.summaries.len() == 2))
            })
            .unwrap();
        let phase = region.numeric.as_ref().unwrap().phase.as_ref().unwrap();
        let memory = mode.memory_budget();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "input",
                    Data::Matrix(
                        (1..=12)
                            .map(|v| Data::F64(F64Bits::from_f64(v as f64)))
                            .collect(),
                    ),
                    &[4, 3],
                ),
                ("rounds", Data::U64(1), &[]),
                ("trigger", Data::U64(0), &[]),
            ],
        );
        let before = contents(&args);
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut output = vec![None; artifact.outputs().len()];
        let (result, initial) = observe::run(&jit, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        let expected = contents(&output.iter().flatten().cloned().collect::<Vec<_>>());
        let entry_spent = u64::MAX - initial.phase_initial_fuel[0];
        output.fill(None);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained
        );
        for restored in [false, true] {
            let mut limits = mode.execution();
            // Entry and the whole replay fit, but the most expensive summary
            // cannot start. Its original constructor path must remain usable.
            limits.execution.max_work = entry_spent
                + phase.replay_credit
                + phase.loops[0].compute
                + phase.summaries.iter().map(|s| s.compute).max().unwrap() / 2;
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut output, limits, &memory)
            } else {
                observe::run(&jit, &args, &mut output, limits, &memory)
            };
            result.unwrap();
            assert!(
                counts.phases > 0,
                "decline must happen inside the admitted phase: {counts:?}"
            );
            assert!(
                phase.summaries.iter().any(|s| counts
                    .admits
                    .get(&(s.site as u64))
                    .copied()
                    .unwrap_or(0)
                    > 0),
                "the exact original operation must replay: {counts:?}"
            );
            assert!(
                phase.summaries.iter().any(|s| counts
                    .phase_summary_attempts
                    .get(&s.site)
                    .copied()
                    .unwrap_or(0)
                    == 0),
                "an unfunded attempt must not be charged: {counts:?}"
            );
            assert_eq!(
                contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
            output.fill(None);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            let (result, counts) =
                observe::run(&jit, &args, &mut output, mode.execution(), &memory);
            result.unwrap();
            for s in &phase.summaries {
                assert_eq!(counts.phase_summary_attempts[&s.site], 1);
            }
            assert_eq!(
                contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                expected
            );
            assert_eq!(contents(&args), before);
            output.fill(None);
        }
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let base = format!("/descriptor/closed_tails/regions/{region_id}/numeric/phase/summaries");
        for i in 0..phase.summaries.len() {
            for field in ["site", "counter", "compute", "comparison"] {
                let mut forged = image.clone();
                let word = forged.pointer_mut(&format!("{base}/{i}/{field}")).unwrap();
                *word = (word.as_u64().unwrap() + 1).into();
                assert!(
                    super::super::tests::rejects_malformed_helper_image(
                        &serde_json::to_vec(&forged).unwrap()
                    ),
                    "{i}/{field}"
                );
            }
        }
        let mut forged = image.clone();
        *forged.pointer_mut(&base).unwrap() = serde_json::json!([]);
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&forged).unwrap()
        ));
        drop(args);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
