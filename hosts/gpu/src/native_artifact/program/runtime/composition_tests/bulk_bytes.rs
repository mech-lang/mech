//! Selected bulk byte append must remain inside its ordinary numeric recurrence.
use super::*;

const SOURCE: &str = "+> sequence\n+> bits\nemit-bytes(bytes<[u8]>, word<u64>, count<u8>, i<u64>, end<u64>) => (out<[u8]>) :=\n  more := i < end\n  next := (more? | true => sequence/append-le-bytes(bytes, bits/wrapping-add(word, i), count) | false => bytes)\n  out := (more? | true => emit-bytes(next, word, count, i + 1u64, end) | false => bytes).\nemit-bytes(sequence/with-capacity(0u8, capacity<u64>), word<u64>, count<u8>, 0u64, steps<u64>)\n";

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    word: u64,
    count: u8,
    steps: u64,
    capacity: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("word", Data::U64(word)),
            ("count", Data::U8(count)),
            ("steps", Data::U64(steps)),
            ("capacity", Data::U64(capacity)),
        ],
        memory,
    )
}

fn expected(word: u64, count: u8, steps: u64) -> Data {
    Data::Matrix(
        (0..steps)
            .flat_map(|i| {
                word.wrapping_add(i).to_le_bytes()[..usize::from(count)]
                    .iter()
                    .copied()
                    .map(Data::U8)
                    .collect::<Vec<_>>()
            })
            .collect(),
    )
}

#[test]
fn bulk_byte_append_selects_native_loop_without_per_visit_boundaries() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| {
                    r.numeric
                        .as_ref()
                        .is_some_and(|p| p.arrays.iter().any(|a| a.appends))
                }),
            "bulk append must retain its recurrence in the existing numeric emitter"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for count in 0..=8 {
                let mut boundary_count = None;
                for steps in [1, 8, 64] {
                    let args = arguments(&artifact, &memory, u64::MAX - 3, count, steps, 2048);
                    let (result, observed) = if restored {
                        observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                    };
                    result.unwrap();
                    assert_eq!(
                        contents(&[output.last().unwrap().as_ref().unwrap().clone()]),
                        [expected(u64::MAX - 3, count, steps)]
                    );
                    let count = (
                        observed.admits.values().sum::<u64>(),
                        observed.helpers.values().sum::<u64>(),
                    );
                    assert_eq!(count, *boundary_count.get_or_insert(count));
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

#[test]
fn bulk_byte_append_keeps_current_counts_publication_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (word, count, steps, capacity) in [
                (0x0102_0304_0506_0708, 2, 16, 128),
                (u64::MAX, 9, 1, 128),
                (0, 255, 0, 0),
                (0x0807_0605_0403_0201, 8, 8, 64),
                (7, 255, 2, 64),
                (99, 0, 64, 0),
                (u64::MAX - 3, 1, 16, 0),
                (0x0102_0304_0506_0708, 3, 2, 5),
            ] {
                let args = arguments(&artifact, &memory, word, count, steps, capacity);
                let aliases = args.clone();
                let before = contents(&args);
                let previous = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, _) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if count > 8 && steps > 0 {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Library"), "{error}");
                    for (current, old) in output.iter().zip(&previous) {
                        assert!(
                            current
                                .as_ref()
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
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected(word, count.min(8), steps)
                    );
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

#[test]
fn bulk_byte_append_keeps_finite_funding_and_cleanup() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 0x0102_0304_0506_0708, 3, 8, 128)
    });
}
