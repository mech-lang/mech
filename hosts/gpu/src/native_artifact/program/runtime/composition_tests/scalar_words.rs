//! Scalar-only aggregates must not split an otherwise proved numeric piece.
use super::*;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use std::collections::BTreeSet;

const SOURCE: &str = r#"
+> sequence
<answer> := <{number<u64> flag<bool>}>
child(number<u64>, left<u64>, flag<bool>, allowed<bool>, divisor<u64>) => (out<answer>) :=
  allowed! := allowed
  out := ((left > 0u64)? | true => child(number + 1u64, left - 1u64, flag == false, allowed, divisor) | false => {number: number / divisor, flag: flag}).
parent(words<[u64]>, left<u64>, number<u64>, flag<bool>, repeats<u64>, allowed<bool>, divisor<u64>) => (out<u64>) :=
  pair := (number, flag, words[1u64], left)
  (current-number, current-flag, word, current-left) := pair
  more := current-left > 0u64
  result := (more? | true => child(current-number, repeats, current-flag, allowed, divisor) | false => {number: current-number, flag: current-flag})
  marked := (result.flag? | true => result.number | false => result.number + 100u64)
  out := (more? | true => parent(words, current-left - 1u64, marked + word, result.flag == false, repeats, allowed, divisor) | false => number).
parent(words<[u64]>, steps<u64>, start<u64>, flag<bool>, repeats<u64>, allowed<bool>, divisor<u64>)
"#;

fn expected(steps: u64, repeats: u64, mut number: u64, mut flag: bool, divisor: u64) -> u64 {
    for _ in 0..steps {
        number = (number + repeats) / divisor;
        flag ^= repeats % 2 != 0;
        number += if flag { 7 } else { 107 };
        flag = !flag;
    }
    number
}

#[test]
fn scalar_aggregate_actions_stay_in_parent_ssa_with_current_words_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let interior =
            metadata
                .compositions
                .parents
                .iter()
                .flat_map(|parent| &parent.segments)
                .filter(|piece| metadata.compositions.entry(piece.entry as usize).is_some())
                .flat_map(|piece| {
                    piece.numeric.as_ref().unwrap().actions.iter().filter_map(
                        |(site, operation)| {
                            (matches!(operation, Operation::Aggregate)
                                && *site != piece.entry as usize)
                                .then_some(*site as u64)
                        },
                    )
                })
                .collect::<BTreeSet<_>>();
        assert!(
            !interior.is_empty(),
            "scalar aggregate still splits the native parent piece"
        );
        assert!(
            metadata
                .compositions
                .parents
                .iter()
                .flat_map(|parent| &parent.segments)
                .any(|piece| {
                    metadata.compositions.entry(piece.entry as usize).is_some()
                        && piece.exits.iter().any(|&(_, to)| {
                            metadata
                                .map_admission
                                .group(to as usize)
                                .is_some_and(|group| {
                                    group.owner.is_none()
                                        && piece.sites.contains(&group.start())
                                        && group
                                            .sites
                                            .iter()
                                            .any(|site| !piece.sites.contains(site))
                                })
                        })
                }),
            "fixture must exercise an unfunded ordinary scalar suffix after native exit"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (steps, repeats, flag, allowed, divisor, empty, rejection) in [
                (3, 2, true, true, 1, false, None),
                (2, 1, false, true, 2, false, None),
                (3, 2, false, false, 0, false, Some("Integrity")),
                (3, 2, false, true, 0, false, Some("Arithmetic")),
                (0, 2, false, false, 0, false, None),
                (3, 2, true, true, 1, true, Some("IndexOutOfRange")),
                (3, 2, true, false, 0, true, Some("IndexOutOfRange")),
                (3, 2, false, true, 1, false, None),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "words",
                            Data::Matrix(if empty { vec![] } else { vec![Data::U64(7)] }.into()),
                        ),
                        ("steps", Data::U64(steps)),
                        ("start", Data::U64(2)),
                        ("flag", Data::Bool(flag)),
                        ("repeats", Data::U64(repeats)),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let prior = outputs.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut outputs, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut outputs, mode.execution(), &memory)
                };
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    for (actual, old) in outputs.iter().zip(&prior) {
                        let (actual, old) = (actual.as_ref().unwrap(), old.as_ref().unwrap());
                        assert!(actual.shares_frozen_storage(old));
                        assert_eq!(
                            actual.canonical_data_draft().unwrap(),
                            old.canonical_data_draft().unwrap()
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    let report = result.unwrap();
                    let expected = if steps == 0 {
                        2
                    } else {
                        expected(steps, repeats, 2, flag, divisor)
                    };
                    assert_eq!(
                        outputs
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(expected)
                    );
                    assert!(report.work > 0);
                }
                assert!(counts.selected > 0);
                assert!(
                    counts.admits.keys().all(|site| !interior.contains(site)),
                    "{counts:?}"
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
