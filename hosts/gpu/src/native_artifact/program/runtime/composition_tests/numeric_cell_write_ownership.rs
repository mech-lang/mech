//! Combined two-axis bounds preserve repeated writes, aliases and first change.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source(kind: &str) -> String {
    r#"#Repeat(initial<[K]:2,3>, steps<u64>, row<u64>, column<u64>, marker<K>) => <([K]:2,3,K)>
  | :Write(data<[K]:2,3>, remaining<u64>, seen<K>)
  | :Done(data<[K]:2,3>, seen<K>).
#Repeat(initial, steps, row, column, marker) -> :Write(initial, steps, marker)
  :Write(data, remaining, seen)
    | remaining > 0u64 -> {
        old := data[row,column]
        ~next := data
        next[row,column] = marker
      } -> :Write(next, remaining - 1u64, old)
    | * -> :Done(data, seen)
  :Done(data, seen) => (data, seen).
data := input<[K]:2,3>
answer := #Repeat(data,steps<u64>,row<u64>,column<u64>,marker<K>)
(answer,data)
"#
    .replace('K', kind)
}

#[test]
fn numeric_cell_bounds_keep_repeated_equal_writes_aliases_exact_errors_and_noop_owners() {
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for artifact in artifacts(&source(kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let old_object =
                crate::native_artifact::program::compiler::emit_program_object_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::ScalarLoops,
                )
                .unwrap();
            let sites = |object: &crate::native_artifact::NativeProgramObject| {
                let metadata = object.metadata();
                [
                    "compiled_readonly_numeric_loops",
                    "compiled_numeric_write_loops",
                ]
                .into_iter()
                .flat_map(|key| metadata[key].as_array().unwrap())
                .map(|r| r["typed_stores"].as_array().unwrap().len())
                .sum::<usize>()
            };
            assert!(
                sites(&object) > 0,
                "actual native cell store emission: {kind}"
            );
            assert_eq!(
                sites(&old_object),
                0,
                "ablation retains ordinary selected source operations"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                for (row, column, steps, incoming, changed, error) in [
                    (1u64, 1u64, 8u64, cells[0].clone(), false, false),
                    (1, 1, 8, marker.clone(), true, false),
                    (0, 1, 8, marker.clone(), false, true),
                    (1, 0, 8, marker.clone(), false, true),
                    (3, 1, 8, marker.clone(), false, true),
                    (1, 4, 8, marker.clone(), false, true),
                    (0, 0, 8, marker.clone(), false, true),
                    (u64::MAX, u64::MAX, 0, marker.clone(), false, false),
                    (2, 3, 8, marker.clone(), true, false),
                ] {
                    let args = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("marker", incoming.clone(), &[]),
                            ("steps", Data::U64(steps), &[]),
                            ("row", Data::U64(row), &[]),
                            ("column", Data::U64(column), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (a, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                    };
                    let (b, _) = observe::run(&old, &args, &mut cold, mode.execution(), &memory);
                    assert_eq!(
                        a.is_err(),
                        error,
                        "{kind} ({row},{column}) steps{steps}: {a:?}"
                    );
                    assert_eq!(
                        a.as_ref().map(|_| ()).map_err(|e| format!("{e:?}")),
                        b.as_ref().map(|_| ()).map_err(|e| format!("{e:?}")),
                        "exact source error parity"
                    );
                    assert_eq!(
                        fast.iter()
                            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                            .collect::<Vec<_>>(),
                        cold.iter()
                            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                            .collect::<Vec<_>>()
                    );
                    if let Ok(report) = a {
                        assert!(report.work > 0);
                        assert_eq!(
                            !counts.first_change_sites.is_empty(),
                            changed,
                            "{kind} first-change vs shared no-op"
                        );
                        assert!(
                            counts
                                .first_change_backings
                                .iter()
                                .all(|(old, new)| old != new)
                        );
                        let mut expected = cells.clone();
                        if changed {
                            expected[((row - 1) * 3 + column - 1) as usize] = incoming.clone();
                        }
                        let actual = fast
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap();
                        assert_eq!(
                            actual,
                            Data::Tuple(
                                vec![
                                    Data::Tuple(
                                        vec![Data::Matrix(expected.into()), incoming].into()
                                    ),
                                    Data::Matrix(cells.clone().into())
                                ]
                                .into()
                            )
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop((fast, cold));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

fn drafts(values: &[Option<Value>]) -> Vec<Option<Data>> {
    values
        .iter()
        .map(|value| {
            value
                .as_ref()
                .map(|value| value.canonical_data_draft().unwrap())
        })
        .collect()
}

#[test]
fn numeric_f64_homes_preserve_exact_payloads_aliases_errors_and_restored_source() {
    use mech_core::snapshot::F64Bits;
    let cells = [
        0x8000_0000_0000_0000,
        0x7ff0_0000_0000_1234,
        0x7ff8_0000_0000_5678,
        1,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
    ]
    .map(|bits| Data::F64(F64Bits::from_bits(bits)))
    .to_vec();
    for artifact in artifacts(&source("f64")) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let word = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::F64SsaHomes,
        )
        .unwrap();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let word_object = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &catalog,
            OptimizationPass::F64SsaHomes,
        )
        .unwrap();
        let homes = |object: &crate::native_artifact::NativeProgramObject| {
            let metadata = object.metadata();
            [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().unwrap())
            .map(|region| region["f64_home_slots"].as_array().unwrap().len())
            .sum::<usize>()
        };
        assert!(homes(&object) > 0, "real checked typed numeric emission");
        assert_eq!(homes(&word_object), 0);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for marker in [
                0,
                1 << 63,
                0x7ff0_0000_0000_0091,
                0x7ff8_0000_0000_0091,
                u64::MAX,
            ] {
                for (row, column, steps, error) in [
                    (1, 1, 0, false),
                    (1, 1, 1, false),
                    (1, 1, 17, false),
                    (2, 3, 17, false),
                    (0, 1, 1, true),
                    (1, 4, 17, true),
                ] {
                    let incoming = Data::F64(F64Bits::from_bits(marker));
                    let args = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("marker", incoming.clone(), &[]),
                            ("steps", Data::U64(steps), &[]),
                            ("row", Data::U64(row), &[]),
                            ("column", Data::U64(column), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    let (a, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut actual, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut actual, mode.execution(), &memory)
                    };
                    let (b, _) =
                        observe::run(&word, &args, &mut expected, mode.execution(), &memory);
                    assert_eq!(a.is_err(), error);
                    assert_eq!(
                        format!("{a:?}"),
                        format!("{b:?}"),
                        "all reports and exact source errors"
                    );
                    assert_eq!(drafts(&actual), drafts(&expected));
                    if !error {
                        let mut changed = cells.clone();
                        let seen = if steps == 0 {
                            incoming.clone()
                        } else if steps == 1 {
                            cells[((row - 1) * 3 + column - 1) as usize].clone()
                        } else {
                            incoming.clone()
                        };
                        if steps > 0 {
                            changed[((row - 1) * 3 + column - 1) as usize] = incoming.clone();
                        }
                        assert_eq!(
                            actual
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Tuple(
                                vec![
                                    Data::Tuple(vec![Data::Matrix(changed.into()), seen].into()),
                                    Data::Matrix(cells.clone().into())
                                ]
                                .into()
                            )
                        );
                        if steps > 0 && incoming != cells[((row - 1) * 3 + column - 1) as usize] {
                            assert!(!counts.first_change_sites.is_empty());
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_f64_homes_keep_arithmetic_branch_joins_finite_failures_and_atomic_retry() {
    use mech_core::snapshot::F64Bits;
    let source = r#"#Fold(initial<[f64]:2,3>, steps<u64>, seed<f64>) => <([f64]:2,3,f64)>
  | :Walk(data<[f64]:2,3>, remaining<u64>, total<f64>, flip<bool>)
  | :Done(data<[f64]:2,3>, total<f64>).
#Fold(initial, steps, seed) -> :Walk(initial, steps, seed, true)
  :Walk(data, remaining, total, flip)
    | remaining > 0u64 -> {
        old := data[1u64,1u64]
        value := (flip? | true => total * 1.0000000000000002 + old | false => total / 3.0 - old)
        ~next := data
        next[1u64,1u64] = value
      } -> :Walk(next, remaining - 1u64, value, flip == false)
    | * -> :Done(data, total)
  :Done(data, total) => (data, total).
data := input<[f64]:2,3>
answer := #Fold(data,steps<u64>,seed<f64>)
(answer,data)
"#;
    let cells = [1.5, -0.0, 2.0, -3.0, 5.0, -7.0]
        .map(|value| Data::F64(F64Bits::from_f64(value)))
        .to_vec();
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let word = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::F64SsaHomes,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = super::numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                ("steps", Data::U64(17), &[]),
                ("seed", Data::F64(F64Bits::from_f64(-0.75)), &[]),
            ],
        );
        let aliases = args.clone();
        let before = contents(&args);
        let mut prior = vec![None; artifact.outputs().len()];
        let (reference, counts) = observe::run(&jit, &args, &mut prior, mode.execution(), &memory);
        let reference = reference.unwrap();
        assert!(
            !counts.first_change_sites.is_empty(),
            "real resumable first-change boundary"
        );
        let mut ordinary = vec![None; artifact.outputs().len()];
        let old = observe::run(&word, &args, &mut ordinary, mode.execution(), &memory)
            .0
            .unwrap();
        assert_eq!(reference, old, "complete source accounting");
        assert_eq!(drafts(&prior), drafts(&ordinary));
        let mut value = -0.75f64;
        let mut cell = 1.5f64;
        for i in 0..17 {
            value = if i % 2 == 0 {
                value * 1.0000000000000002 + cell
            } else {
                value / 3.0 - cell
            };
            cell = value;
        }
        let mut changed = cells.clone();
        changed[0] = Data::F64(F64Bits::from_f64(value));
        assert_eq!(
            prior
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Tuple(
                vec![
                    Data::Tuple(
                        vec![
                            Data::Matrix(changed.into()),
                            Data::F64(F64Bits::from_f64(value))
                        ]
                        .into()
                    ),
                    Data::Matrix(cells.clone().into())
                ]
                .into()
            )
        );
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            for field in 0..5 {
                let mut policy = mode.execution();
                match field {
                    0 => policy.execution.max_work = 1,
                    1 => policy.library.max_compute_work = Some(0),
                    2 => policy.library.max_comparison_work = Some(0),
                    3 => policy.library.max_cloned_bytes = Some(0),
                    4 => policy.library.max_output_elements = Some(0),
                    _ => unreachable!(),
                }
                let mut actual = prior.clone();
                let mut expected = prior.clone();
                let a = if restored {
                    observe::run_restored(&jit, &args, &mut actual, policy, &memory).0
                } else {
                    observe::run(&jit, &args, &mut actual, policy, &memory).0
                };
                let b = observe::run(&word, &args, &mut expected, policy, &memory).0;
                assert!(a.is_err(), "finite denial {field}: {a:?}");
                assert_eq!(
                    format!("{a:?}"),
                    format!("{b:?}"),
                    "exact finite boundary {field}"
                );
                assert_eq!(drafts(&actual), drafts(&expected));
                for (now, previous) in actual.iter().zip(&prior) {
                    assert!(
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(previous.as_ref().unwrap()),
                        "failure keeps prior publication"
                    );
                }
                let recovered =
                    observe::run_restored(&jit, &args, &mut actual, mode.execution(), &memory)
                        .0
                        .unwrap();
                let recovered_word =
                    observe::run_restored(&word, &args, &mut expected, mode.execution(), &memory)
                        .0
                        .unwrap();
                assert_eq!(
                    recovered, recovered_word,
                    "identical prior publication state"
                );
                assert_eq!(drafts(&actual), drafts(&prior));
                drop((actual, expected));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
        }
        drop((ordinary, prior, args, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
