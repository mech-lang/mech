//! Admission envelopes are checked against actual ordinary source traversals.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::numeric_phase::{StateTrips, fixed_state_trips};
use mech_core::snapshot::F64Bits;
use mech_engine::native_artifact::NativeControlLoop;

fn source(seed: &str, guard: &str, next: &str) -> String {
    format!(
        r#"+> bits
#Count(initial<[f64]:1,4>, start<u64>) => <[f64]:1,4>
  | :Step(data<[f64]:1,4>, at<u64>)
  | :Done(data<[f64]:1,4>).
#Count(initial, start) -> :Step(initial, {seed})
  :Step(data, at)
    | {guard} -> :Step(data, {next})
    | * -> :Done(data)
  :Done(data) => data.
#Count(input<[f64]:1,4>, start<u64>)
"#
    )
}

fn proof(jit: &JitProgramArtifact) -> Result<StateTrips, &'static str> {
    let m = jit.descriptor().metadata();
    let tails = m
        .plan
        .iterations()
        .iter()
        .filter_map(|control| {
            let NativeControlLoop::Tail(tail) = control else {
                return None;
            };
            Some(tail)
        })
        .collect::<Vec<_>>();
    assert_eq!(tails.len(), 1);
    fixed_state_trips(
        &m.plan.facts(),
        &m.buffer_frames,
        &m.scalar_aggregates,
        tails[0],
    )
}

#[test]
fn numeric_fsm_trip_envelopes_cover_forward_reverse_wrapping_and_zero_trip_traces() {
    let mode = NativeProgramLimitMode::NoLimits;
    let cases = [
        ("start", "at <= 8u64", "at + 1u64", 11),
        ("start", "at < 8u64", "bits/wrapping-add(at, 3u64)", 12),
        ("8u64", "at >= 1u64", "at - 1u64", 10),
        ("8u64", "at > 1u64", "bits/wrapping-sub(at, 2u64)", 10),
        ("0u64", "at >= 1u64", "at - 1u64", 2),
        ("start", "at <= 0u64", "at + 1u64", 3),
    ];
    for (seed, guard, next, maximum) in cases {
        for artifact in artifacts(&source(seed, guard, next)) {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let trips = proof(&jit).expect("proved source recurrence");
            assert_eq!(trips.maximum, maximum);
            assert_eq!(trips.tag, Some(0));
            assert_eq!(trips.field, 1);
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &mech_stdlib::source_catalog(),
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            let header = ordinary.descriptor().metadata().plan.iterations()
                [trips.iteration as usize]
                .as_ref()
                .header() as u64;
            for restored in [false, true] {
                for start in [0, 1, 4, 8, 9, u64::MAX] {
                    let args = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            (
                                "input",
                                Data::Matrix(vec![Data::F64(F64Bits::from_f64(1.0)); 4].into()),
                                &[1, 4],
                            ),
                            ("start", Data::U64(start), &[]),
                        ],
                    );
                    let before = contents(&args);
                    let (report, observed) = if restored {
                        observe::run_restored(
                            &ordinary,
                            &args,
                            &mut outputs,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&ordinary, &args, &mut outputs, mode.execution(), &memory)
                    };
                    report.unwrap();
                    let actual = observed.admits.get(&header).copied().unwrap_or(0);
                    assert!(
                        actual > 0 && actual <= trips.maximum,
                        "{seed} / {guard} / {next}, start {start}, restored {restored}: {actual} > {trips:?}"
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        contents(&outputs.iter().flatten().cloned().collect::<Vec<_>>()),
                        vec![Data::Matrix(
                            vec![Data::F64(F64Bits::from_f64(1.0)); 4].into()
                        )]
                    );
                }
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_fsm_trip_proof_declines_nonprogress_guard_bypass_unknown_seeds_and_wraparound() {
    let base = source("start", "at <= 8u64", "at + 1u64");
    let cases = [
        source("start", "at <= 8u64", "at + 0u64"),
        source("start", "at <= 8u64", "8u64 + 1u64"),
        source("start", "at <= 8u64", "at - 1u64"),
        source("start", "at >= 1u64", "at - 1u64"),
        source("8u64", "at >= 0u64", "bits/wrapping-sub(at, 1u64)"),
        source("8u64", "at > 0u64", "bits/wrapping-sub(at, 2u64)"),
        source(
            "start",
            "at <= 18446744073709551615u64",
            "bits/wrapping-add(at, 1u64)",
        ),
        base.replace("| * -> :Done(data)", "| * -> :Step(data, at + 1u64)"),
        base.replace("at + 1u64", "at"),
    ];
    for src in cases {
        for artifact in artifacts(&src) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(proof(&jit).is_err(), "must decline: {src}");
        }
    }
}

#[test]
fn numeric_fsm_trip_proof_bounds_all_complete_optimization_fixture_children() {
    let src = include_str!("../../../../../tests/fixtures/polybench/deriche.mec");
    for artifact in artifacts(src) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let m = jit.descriptor().metadata();
        let trips = m
            .plan
            .iterations()
            .iter()
            .filter_map(|control| {
                let NativeControlLoop::Tail(tail) = control else {
                    return None;
                };
                let region = m
                    .closed_tails
                    .regions
                    .iter()
                    .find(|r| r.tail.header() == tail.header())
                    .unwrap();
                assert!(region.numeric.as_ref().unwrap().exits.is_empty());
                Some(
                    fixed_state_trips(
                        &m.plan.facts(),
                        &m.buffer_frames,
                        &m.scalar_aggregates,
                        tail,
                    )
                    .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            trips.iter().map(|t| t.maximum).collect::<Vec<_>>(),
            [67, 66, 67, 66]
        );
    }
}

#[test]
fn numeric_fsm_trip_proof_declines_multiple_recurring_tags() {
    let text = r#"#Count(initial<[f64]:1,4>, start<u64>) => <[f64]:1,4>
  | :Step(data<[f64]:1,4>, at<u64>)
  | :Other(data<[f64]:1,4>, at<u64>)
  | :Done(data<[f64]:1,4>).
#Count(initial,start) -> :Step(initial,start)
  :Step(data,at)
    | at < 8u64 -> :Other(data,at + 1u64)
    | * -> :Done(data)
  :Other(data,at) -> :Step(data,at)
  :Done(data) => data.
#Count(input<[f64]:1,4>,start<u64>)
"#;
    for artifact in artifacts(text) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            proof(&jit).unwrap_err(),
            "FSM requires exactly one recurring state"
        );
    }
}

#[test]
fn numeric_fsm_trip_proof_also_covers_untagged_managed_tuple_tails() {
    let text = r#"walk(data<[f64]:1,4>, at<u64>) => (out<[f64]:1,4>) :=
  out := (at < 8u64? | true => walk(data,at + 1u64) | false => data).
walk(input<[f64]:1,4>,start<u64>)
"#;
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(text) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let trips = proof(&jit).unwrap();
        assert_eq!(trips.tag, None);
        assert_eq!(trips.maximum, 10);
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let header = ordinary.descriptor().metadata().plan.iterations()[trips.iteration as usize]
            .as_ref()
            .header() as u64;
        let memory = mode.memory_budget();
        for start in [0, 4, 8, u64::MAX] {
            let args = super::numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    (
                        "input",
                        Data::Matrix(vec![Data::F64(F64Bits::from_f64(1.0)); 4].into()),
                        &[1, 4],
                    ),
                    ("start", Data::U64(start), &[]),
                ],
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            let (report, observed) =
                observe::run(&ordinary, &args, &mut outputs, mode.execution(), &memory);
            report.unwrap();
            let actual = observed.admits.get(&header).copied().unwrap_or(0);
            assert!(
                actual > 0 && actual <= trips.maximum,
                "start {start}: {actual} > {trips:?}"
            );
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
