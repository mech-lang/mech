//! Actual shared word-extension source inside an ordinary current-argument loop.
use super::*;

fn source() -> String {
    format!("{}\n{}
sample(bytes<[u8]>, 0u64, rounds<u64>, left<u64>, right<u64>, start<u64>, limit<u64>, 0u64)\n",
        include_str!("../../../../../tests/fixtures/native/deflate-matchfinder.mec"),
        "sample(bytes<[u8]>, turn<u64>, rounds<u64>, left<u64>, right<u64>, start<u64>, limit<u64>, total<u64>) => (out<u64>) :=
  more := turn < rounds
  odd := bits/and(turn, 1u64) != 0u64
  a := (odd? | true => right | false => left)
  b := (odd? | true => left | false => right)
  found := (more? | true => extend-words(bytes, a, b, start, limit) | false => 0u64)
  out := (more? | true => sample(bytes, turn + 1u64, rounds, left, right, start, limit, total + found) | false => total).")
}

pub(super) fn four_source() -> String {
    source().replace("sample(bytes<[u8]>, turn<u64>",
        "probe(bytes<[u8]>, left<u64>, right<u64>, start<u64>, limit<u64>) => (out<u64>) :=\n  end := start + 32u64\n  out := ((end <= limit)? | true => extend-four(bytes, left, right, start, end, limit) | false => extend-words(bytes, left, right, start, limit)).\nsample(bytes<[u8]>, turn<u64>")
        .replace("true => extend-words(bytes, a, b, start, limit)", "true => probe(bytes, a, b, start, limit)")
}

pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    dimensions: [u64; 2],
    bytes: &[u8],
    coordinates: [u64; 5],
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = match name.as_str() {
                "bytes" => Data::Matrix(bytes.iter().copied().map(Data::U8).collect()),
                "left" => Data::U64(coordinates[0]),
                "right" => Data::U64(coordinates[1]),
                "start" => Data::U64(coordinates[2]),
                "limit" => Data::U64(coordinates[3]),
                "rounds" => Data::U64(coordinates[4]),
                _ => panic!("unexpected input {name}"),
            };
            let shape_values: Box<[u64]> = if name == "bytes" {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(port.schema).unwrap(),
                    &dimensions,
                )
                .unwrap()
                .parameter_values()
                .into()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: port.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

fn windows(metadata: &serde_json::Value) -> Vec<&serde_json::Value> {
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|name| metadata[name].as_array().unwrap())
    .flat_map(|row| row["guarded_loop_windows"].as_array().unwrap())
    .filter(|window| {
        window["captured_windows"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
    })
    .collect()
}

#[test]
fn captured_read_windows_select_the_real_nested_word_recurrence() {
    for artifact in artifacts(&source()) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let windows = windows(&metadata);
        assert!(
            !windows.is_empty(),
            "must qualify the unchanged shared word loop"
        );
        assert!(
            windows
                .iter()
                .all(|window| window["captured_windows"].as_array().unwrap().len() == 2)
        );
        assert!(
            metadata["compiled_readonly_numeric_loops"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| !row["proven_scalar_arithmetic"]
                    .as_array()
                    .unwrap()
                    .is_empty())
        );
        assert!(
            metadata["compiled_readonly_numeric_loops"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| !row["fully_lowered_read_packs"]
                    .as_array()
                    .unwrap()
                    .is_empty())
        );
    }
}

#[test]
fn captured_read_windows_keep_current_offsets_tails_geometry_errors_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source()) {
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            crate::native_artifact::codegen::options::OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let mut cases = Vec::new();
        for dimensions in [[1, 96], [96, 1], [8, 12], [12, 8], [1, 7], [0, 1]] {
            for mismatch in 0..=33 {
                let mut bytes = vec![17; (dimensions[0] * dimensions[1]) as usize];
                if bytes.len() > 48 + mismatch {
                    bytes[48 + mismatch] = 23;
                }
                for end in [0, 7, 8, 9, 16, 17, 32, 33] {
                    cases.push((dimensions, bytes.clone(), [0, 48, 0, end, 2], false));
                }
            }
        }
        for coordinates in [
            [0, 48, 7, 33, 3],
            [0, 48, 33, 32, 2],
            [u64::MAX, 0, 0, 16, 2],
            [0, 48, u64::MAX, 0, 0],
            [0, 48, u64::MAX, 0, 1],
            [0, 48, 0, 32, 2],
        ] {
            cases.push((
                [1, 96],
                vec![17; 96],
                coordinates,
                coordinates[2] == u64::MAX && coordinates[4] != 0,
            ));
        }
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut actual = vec![None; artifact.outputs().len()];
            let mut expected = actual.clone();
            for (dimensions, bytes, coordinates, fails) in &cases {
                let args = arguments(&artifact, &memory, *dimensions, bytes, *coordinates);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = actual.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let run = |jit: &JitProgramArtifact, output: &mut [Option<Value>]| {
                    if restored {
                        observe::run_restored(jit, &args, output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, output, mode.execution(), &memory)
                    }
                };
                let (reference, _) = run(&ordinary, &mut expected);
                let (result, _) = run(&native, &mut actual);
                if *fails {
                    assert_eq!(
                        format!("{:?}", result.unwrap_err()),
                        format!("{:?}", reference.unwrap_err())
                    );
                    read_packs::unchanged_publication(&actual, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    reference.unwrap();
                    result.unwrap();
                    assert_eq!(
                        contents(
                            &actual
                                .iter()
                                .map(|v| v.as_ref().unwrap().clone())
                                .collect::<Vec<_>>()
                        ),
                        contents(
                            &expected
                                .iter()
                                .map(|v| v.as_ref().unwrap().clone())
                                .collect::<Vec<_>>()
                        )
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop((actual, expected));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn captured_read_windows_decline_changed_capture_or_unchecked_guard() {
    let original = source();
    for changed in [
        original.replace(
            "length + 8u64 <= maximum",
            "bits/wrapping-add(length, 8u64) <= maximum",
        ),
        original.replace(
            "bits/wrapping-add(length, 8u64), maximum)",
            "bits/wrapping-add(length, 8u64), maximum - 1u64)",
        ),
    ] {
        assert_ne!(original, changed);
        for artifact in artifacts(&changed) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            assert!(windows(&object.metadata()).is_empty());
        }
    }
}
