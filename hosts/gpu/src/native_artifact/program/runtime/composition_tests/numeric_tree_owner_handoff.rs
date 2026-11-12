//! A selected numeric tree can retire a dead canonical pin at its cold write.
//! A real earlier result still owns its original backing and forces a copy.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source(keep_old: bool) -> String {
    let result = if keep_old {
        "(fresh, answer)"
    } else {
        "answer"
    };
    format!(
        r#"+> sequence
owned-store(data<[u64]>, at<u64>, word<u64>) => (out<[u64]>) :=
  ~next := data
  next[at] = word
  out := next.
owned-leaf(data<[u64]>, at<u64>, end<u64>, marker<u64>, deny-at<u64>, divisor<u64>) => (out<[u64]>) :=
  more := at < end
  divide := (at == deny-at? | true => divisor | false => 1u64)
  next := (more? | true => owned-store(data,at,marker / divide) | false => data)
  out := (more? | true => owned-leaf(next,at + 1u64,end,marker,deny-at,divisor) | false => next).
owned-parent(data<[u64]>, ends<[u64]>, round<u64>, rounds<u64>, start<u64>, marker<u64>, deny-at<u64>, divisor<u64>) => (out<[u64]>) :=
  more := round < rounds
  end := (more? | true => ends[round + 1u64] | false => 0u64)
  next := (more? | true => owned-leaf(data,start,end,marker,deny-at,divisor) | false => data)
  out := (more? | true => owned-parent(next,ends,round + 1u64,rounds,start,marker,deny-at,divisor) | false => next).
size := count<u64>
fresh := ((size == 0u64)? | true => sequence/with-capacity(0u64,0u64) | false => [0u64 | n <- 1u64..=size])
answer := owned-parent(fresh,ends<[u64]>,0u64,rounds<u64>,start<u64>,marker<u64>,deny-at<u64>,divisor<u64>)
{result}
"#
    )
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    rounds: u64,
    start: u64,
    end: u64,
    deny: u64,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("count", Data::U64(4)),
            ("rounds", Data::U64(rounds)),
            ("start", Data::U64(start)),
            ("ends", Data::Matrix(vec![Data::U64(end); 2].into())),
            ("marker", Data::U64(17)),
            ("deny-at", Data::U64(deny)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}
fn expected(keep_old: bool, changed: bool) -> Data {
    let fresh = Data::Matrix(vec![Data::U64(0); 4].into());
    let answer = Data::Matrix(vec![Data::U64(if changed { 17 } else { 0 }); 4].into());
    if keep_old {
        Data::Tuple(vec![fresh, answer].into())
    } else {
        answer
    }
}
#[test]
fn numeric_tree_owned_handoff_reuses_dead_pins_and_preserves_live_aliases_errors_and_retry() {
    for keep_old in [false, true] {
        for artifact in artifacts(&source(keep_old)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let control = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::NumericTreeOwnerHandoff,
            )
            .unwrap();
            for (jit, enabled) in [(&native, true), (&control, false)] {
                assert!(
                    jit.descriptor()
                        .metadata()
                        .closed_tails
                        .regions
                        .iter()
                        .any(|r| r.numeric_tree.is_some()
                            && r.numeric.as_ref().is_some_and(|p| p.phase.is_none())),
                    "exercise the selected tree rather than a phase"
                );
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut output = vec![None; artifact.outputs().len()];
                    for (rounds, start, end, deny, divisor, error) in [
                        (2, 1, 5, u64::MAX, 1, None),
                        (0, 1, 5, u64::MAX, 1, None),
                        (2, 1, 5, 2, 0, Some("Arithmetic")),
                        (1, 1, 6, u64::MAX, 1, Some("IndexOutOfRange")),
                        (1, 0, 1, u64::MAX, 1, Some("InvalidInput")),
                        (
                            1,
                            u32::MAX as u64 + 1,
                            u32::MAX as u64 + 2,
                            u64::MAX,
                            1,
                            Some("InvalidInput"),
                        ),
                        (2, 1, 5, u64::MAX, 1, None),
                    ] {
                        let args = arguments(&artifact, &memory, rounds, start, end, deny, divisor);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (result, counts) = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(jit, &args, &mut output, mode.execution(), &memory)
                        };
                        if let Some(error) = error {
                            let actual = format!("{:?}", result.unwrap_err());
                            assert!(actual.contains(error), "{error}: {actual}");
                            assert!(output.iter().zip(&prior).all(|(a, b)| {
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(b.as_ref().unwrap())
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        } else {
                            result.unwrap();
                            assert_eq!(last(&output), expected(keep_old, rounds > 0));
                            assert_eq!(counts.first_change_backings.len(), usize::from(rounds > 0));
                            for (old, new) in counts.first_change_backings {
                                assert_eq!(
                                    old == new,
                                    enabled && !keep_old,
                                    "actual cold write backing: enabled={enabled}, keep_old={keep_old}"
                                );
                            }
                        }
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}
fn last(output: &[Option<Value>]) -> Data {
    output
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}

#[test]
fn numeric_tree_owned_handoff_keeps_copy_denial_and_recovers() {
    for keep_old in [false, true] {
        for artifact in artifacts(&source(keep_old)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for deny in [false, true, false] {
                    let args = arguments(&artifact, &memory, 2, 1, 5, u64::MAX, 1);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if deny {
                        observe::run_with_first_change_copy_limit(
                            &jit,
                            &args,
                            &mut output,
                            mode.execution(),
                            &memory,
                            restored,
                            0,
                        )
                    } else if restored {
                        observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                    };
                    if keep_old && deny {
                        assert!(format!("{:?}", result.unwrap_err()).contains("ClonedBytes"));
                        assert!(counts.first_change_backings.is_empty());
                        assert!(output.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(last(&output), expected(keep_old, true));
                        assert_eq!(counts.first_change_backings.len(), 1);
                        assert_eq!(
                            counts.first_change_backings[0].0 == counts.first_change_backings[0].1,
                            !keep_old
                        );
                    }
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
#[test]
fn numeric_tree_owned_handoff_preserves_finite_admission_and_atomic_retry() {
    resources::finite_boundaries(&source(false), |artifact, memory| {
        arguments(artifact, memory, 2, 1, 5, u64::MAX, 1)
    });
}
#[test]
fn numeric_tree_owned_handoff_is_a_bound_emission_request() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(&source(false)) {
        let native = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let control = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericTreeOwnerHandoff,
        )
        .unwrap();
        let count = |m: serde_json::Value| {
            [
                "compiled_numeric_write_loops",
                "compiled_readonly_numeric_loops",
            ]
            .into_iter()
            .flat_map(|k| m[k].as_array().unwrap().iter())
            .map(|r| r["owned_handoff_boundaries"].as_array().unwrap().len())
            .sum::<usize>()
        };
        assert!(count(native.metadata()) > 0);
        assert_eq!(count(control.metadata()), 0);
        let encoded = native.identity();
        for (ir, work) in [(true, false), (false, true), (true, true)] {
            let mut old = *encoded.bytes();
            if ir {
                old[16..24].copy_from_slice(&307u64.to_le_bytes());
            }
            if work {
                old[32..40].copy_from_slice(&124u64.to_le_bytes());
            }
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact, &catalog, &old
                )
                .is_err()
            );
        }
    }
}
