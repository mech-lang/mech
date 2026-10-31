use super::super::{artifacts, inputs, reference_result};
use super::{WEIGHTED, witness_limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

fn body() -> &'static str {
    WEIGHTED.rsplit_once("\nweighted(").unwrap().0
}
fn matrix(values: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(values.into_iter().collect())
}
fn result(outputs: &[Option<mech_core::Value>]) -> Data {
    outputs
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn selected(source: &str, expected: bool, parent: bool) {
    let artifact = artifacts(source).0.into_iter().next().unwrap();
    let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let metadata = object.metadata();
    let regions = metadata["compiled_readonly_numeric_loops"]
        .as_array()
        .unwrap();
    assert_eq!(!regions.is_empty(), expected, "{metadata}");
    if expected {
        assert!(
            regions.iter().any(|r| r["parent"].is_number() == parent),
            "{metadata}"
        );
        for region in regions {
            assert_eq!(region["successful_backedge_helpers"], 0);
        }
    }
}

#[test]
fn readonly_numeric_widths_short_inputs_and_high_bits_preserve_aliases() {
    for width in [8, 16, 32, 64] {
        let source = WEIGHTED.replace("u32", &format!("u{width}"));
        selected(&source, true, false);
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (length, coefficient_length, steps, value, seed) in [
                (0, 0, 8, 0, 17),
                (1, 0, 8, 7, 19),
                (1, 1, 1, u64::MAX >> (64 - width), 0),
                (3, 1, 8, 9, 23),
                (2, 2, 0, 11, 29),
            ] {
                let data = match width {
                    8 => Data::U8(value as u8),
                    16 => Data::U16(value as u16),
                    32 => Data::U32(value as u32),
                    64 => Data::U64(value),
                    _ => unreachable!(),
                };
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix(std::iter::repeat_n(data, length))),
                        (
                            "coefficients",
                            matrix(std::iter::repeat_n(Data::U8(1), coefficient_length)),
                        ),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(seed)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
                    .unwrap();
                assert_eq!(
                    result(&output),
                    Data::U64(seed + value * steps.min(length.min(coefficient_length) as u64))
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn readonly_numeric_two_fields_may_share_the_same_read_only_backing() {
    let source = WEIGHTED.replace("u32", "u8");
    selected(&source, true, false);
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut args = inputs(
            &artifact,
            &[
                ("data", matrix([Data::U8(3), Data::U8(7)])),
                ("coefficients", matrix([Data::U8(0), Data::U8(0)])),
                ("steps", Data::U64(2)),
                ("seed", Data::U64(17)),
            ],
            &memory,
        );
        let at = |name: &str| {
            artifact
                .inputs()
                .iter()
                .position(|input| {
                    mech_engine::decode_source_input_name(&input.name).unwrap() == name
                })
                .unwrap()
        };
        let (data, coefficients) = (at("data"), at("coefficients"));
        args[coefficients] = args[data].clone();
        assert!(args[data].shares_frozen_storage(&args[coefficients]));
        let aliases = args.clone();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
            .unwrap();
        assert_eq!(result(&output), Data::U64(75));
        assert!(
            args.iter()
                .zip(&aliases)
                .all(|(a, b)| a.shares_frozen_storage(b))
        );
        drop((args, aliases, output));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn readonly_numeric_child_refreshes_equal_and_changing_shapes_and_scalar_seeds() {
    let source = format!(
        "{}\nouter(first<[u32]>, second<[u32]>, coefficients<[u8]>, left<u64>, flip<bool>, end<u64>, seed<u64>) => (out<u64>) :=\n  chosen := (flip? | true => second | false => first)\n  partial := weighted(chosen, coefficients, 0u64, end, seed)\n  out := ((left > 0u64)? | true => outer(first, second, coefficients, left - 1u64, flip == false, end, partial + 1u64) | false => partial).\nouter(first<[u32]>, second<[u32]>, coefficients<[u8]>, 2u64, false, steps<u64>, 10u64)\n",
        body()
    );
    selected(&source, true, true);
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for second in [vec![7, 11], vec![7], vec![13, 17], vec![]] {
            let mut observations = Vec::new();
            for steps in [0, 1, 2, 8, 64, 256] {
                let args = inputs(
                    &artifact,
                    &[
                        ("first", matrix([Data::U32(3), Data::U32(5)])),
                        ("second", matrix(second.iter().copied().map(Data::U32))),
                        ("coefficients", matrix([Data::U8(2), Data::U8(3)])),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
                    .unwrap();
                let (run, requests, bytes) =
                    super::super::allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
                    });
                observations.push((requests, bytes, run.unwrap().visits));
                let first_cost = [3, 5]
                    .into_iter()
                    .zip([2, 3])
                    .take(steps as usize)
                    .map(|(a, b)| a * b)
                    .sum::<u64>();
                let second_cost = second
                    .iter()
                    .zip([2, 3])
                    .take(steps as usize)
                    .map(|(a, b)| u64::from(*a) * b)
                    .sum::<u64>();
                let expected = 12 + 2 * first_cost + second_cost;
                assert_eq!(result(&output), Data::U64(expected));
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            assert!(
                observations.iter().all(|row| *row == observations[0]),
                "parented inner iterations must have no transport slope: {observations:?}"
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn readonly_numeric_rectangular_reads_keep_column_major_source_order() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDraft};
    for artifact in artifacts(WEIGHTED).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut args = inputs(
            &artifact,
            &[
                ("data", matrix((1..=6).map(Data::U32))),
                ("coefficients", matrix([1, 2, 1, 2, 1, 2].map(Data::U8))),
                ("steps", Data::U64(6)),
                ("seed", Data::U64(0)),
            ],
            &memory,
        );
        for (at, input) in artifact.inputs().iter().enumerate() {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            if matches!(name.as_str(), "data" | "coefficients") {
                let schema = artifact.schemas().get(input.schema).unwrap();
                let shape = mech_core::shape_for_resolved_extents(schema, &[2, 3]).unwrap();
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                    data: args[at].canonical_data_draft().unwrap(),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                args[at] = value.into_memory_budget(&mut reservation).unwrap();
            }
        }
        let expected = reference_result(&artifact, &args).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, witness_limits(), &memory)
            .unwrap();
        super::super::equal(&artifact, &output, &expected);
        assert_eq!(result(&output), Data::U64(33));
        drop((args, output, expected));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn readonly_numeric_changed_array_edges_and_escaped_frames_keep_the_maintained_path() {
    for source in [
        "+> sequence\nwalk(a<[u8]>, b<[u8]>, left<u64>, total<u64>) => (out<u64>) :=\n  word<u64> := a[1u64]\n  out := ((left > 0u64)? | true => walk(b, a, left - 1u64, total + word) | false => total).\nwalk([3u8], [7u8], 2u64, 0u64)\n",
        "+> sequence\nwalk(a<[u8]>, b<[u8]>, left<u64>, total<u64>) => (out<u64>) :=\n  chosen := ((left > 1u64)? | true => b | false => a)\n  word<u64> := a[1u64]\n  out := ((left > 0u64)? | true => walk(chosen, b, left - 1u64, total + word) | false => total).\nwalk([3u8], [7u8], 2u64, 0u64)\n",
        "+> sequence\nwalk(a<[u8]>, left<u64>) => (out<u64>) :=\n  ~next := a\n  next[1u64] = 1u8\n  word<u64> := next[1u64]\n  out := ((left > 0u64)? | true => walk(next, left - 1u64) | false => word).\nwalk([9u8], 2u64)\n",
        "+> sequence\nwalk(a<[u32]>, left<u64>) => (out<([u32],u64)>) :=\n  size := sequence/length(a)\n  out := ((left > 0u64)? | true => walk(a, left - 1u64) | false => (a, size)).\nwalk([3u32 7u32], 2u64)\n",
    ] {
        selected(source, false, false);
        for artifact in artifacts(source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&[], &mut output, witness_limits(), &memory)
                .unwrap();
            let expected = reference_result(&artifact, &[]).unwrap();
            super::super::equal(&artifact, &output, &expected);
            drop((output, expected));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn readonly_numeric_checked_visits_keep_error_order_publication_and_retry() {
    for arithmetic_first in [false, true] {
        let earlier = if arithmetic_first {
            "  earlier := 12u64 / divisor\n"
        } else {
            ""
        };
        let source = format!(
            "walk(words<[u32]>, left<u64>, allowed<bool>, next<bool>, divisor<u64>) => (out<u64>) :=\n{earlier}  enabled! := allowed\n  value<u64> := words[1u64]\n  out := ((left > 0u64)? | true => walk(words, left - 1u64, next, next, divisor) | false => value / divisor).\nwalk(words<[u32]>, 2u64, true, allowed<bool>, divisor<u64>)\n"
        );
        selected(&source, true, false);
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (allowed, divisor, empty) in [
                (true, 3, false),
                (false, 0, false),
                (true, 3, false),
                (true, 3, true),
                (true, 0, false),
                (false, 3, false),
                (true, 2, false),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "words",
                            matrix(if empty { vec![] } else { vec![Data::U32(12)] }),
                        ),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let native = jit.evaluate_into(&args, &mut output, witness_limits(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && !empty {
                    native.unwrap();
                    super::super::equal(&artifact, &output, &resident.unwrap());
                    assert_eq!(result(&output), Data::U64(12 / divisor));
                } else {
                    let native = format!("{:?}", native.unwrap_err());
                    let resident = resident.unwrap_err();
                    let assertion = !allowed && !(arithmetic_first && divisor == 0);
                    assert_eq!(native.contains("Integrity"), assertion, "{native}");
                    assert_eq!(
                        resident.contains("LexicalIntegrity"),
                        assertion,
                        "{resident}"
                    );
                    assert_eq!(
                        before,
                        (memory.used_bytes(), memory.retained_snapshot_nodes())
                    );
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn numeric_static_vector_and_rectangular_reads_keep_bounds_aliases_and_retry() {
    for (rows, columns) in [(1u64, 6u64), (6, 1), (2, 3), (1, 0)] {
        let source = format!(
            "<words> := <[u32]:{rows},{columns}>\nwalk(xs<words>, i<u64>, end<u64>, total<u64>) => (out<u64>) :=\n  more := i < end\n  word<u64> := (more? | true => xs[i + 1u64] | false => 0u32)\n  next := total + word\n  out := (more? | true => walk(xs, i + 1u64, end, next) | false => total).\nwalk(data<words>, 0u64, steps<u64>, seed<u64>)\n"
        );
        selected(&source, true, false);
        let mut work = Vec::new();
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut published = vec![None; artifact.outputs().len()];
            let count = rows * columns;
            let mut current_work = Vec::new();
            // A real bounds failure occurs after a valid prefix. Recovery then
            // uses fresh input owners and scalar arguments in the same image.
            for (steps, seed, bump) in [
                (0, 17, 0),
                (count.min(4), 0, 10),
                (count + 1, 0, 0),
                (count, 3, 20),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "data",
                            matrix((0..count).map(|n| Data::U32((n + 1 + bump) as u32))),
                        ),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(seed)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let contents = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let old = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let expected = reference_result(&artifact, &args);
                let actual = jit.evaluate_into(&args, &mut published, witness_limits(), &memory);
                if steps <= count {
                    let report = actual.unwrap();
                    current_work.push(report.work);
                    super::super::equal(&artifact, &published, &expected.unwrap());
                    let total = seed
                        + (0..steps)
                            .map(|logical| {
                                let physical = (logical % rows) * columns + logical / rows;
                                physical + 1 + bump
                            })
                            .sum::<u64>();
                    assert_eq!(result(&published), Data::U64(total));
                } else {
                    assert!(expected.is_err());
                    let error = format!("{:?}", actual.unwrap_err());
                    assert!(
                        !error.contains("Arithmetic"),
                        "original bounds error must win: {error}"
                    );
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert!(published.iter().zip(&old).all(|(now, old)| {
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    contents
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(now, old)| now.shares_frozen_storage(old))
                );
            }
            work.push(current_work);
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
        assert_eq!(
            work[0], work[1],
            "decoded geometry must retain the same finite work"
        );
    }
}
