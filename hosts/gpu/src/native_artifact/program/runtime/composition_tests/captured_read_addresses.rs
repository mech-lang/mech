//! The shared extension loop must stage invariant address parts per invocation.
use super::*;

#[test]
fn captured_read_addresses_use_current_native_bases() {
    for artifact in artifacts(&captured_windows::four_source()) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let staged = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|name| metadata[name].as_array().unwrap())
        .filter_map(|region| region.get("hoisted_pack_addresses"))
        .flat_map(|addresses| addresses.as_array().unwrap())
        .count();
        assert!(
            staged >= 2,
            "both actual packed reads must use staged bases"
        );
    }
}

#[test]
fn captured_read_addresses_refresh_between_calls_and_shape_changes() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&captured_windows::four_source()) {
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
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            let mut expected = output.clone();
            for (dimensions, coordinates, mismatch, rejects) in [
                ([1, 96], [0, 48, 0, 33, 3], 33, false),
                ([8, 12], [1, 49, 0, 32, 4], 7, false),
                ([96, 1], [3, 51, 0, 32, 2], 17, false),
                ([1, 96], [0, 48, u64::MAX, 0, 1], 33, true),
                ([12, 8], [0, 48, 0, 33, 3], 0, false),
                ([1, 96], [0, 48, u64::MAX, 0, 0], 33, false),
                ([96, 1], [0, 48, 0, 33, 3], 33, false),
            ] {
                let mut bytes = vec![17; 96];
                bytes[48 + mismatch] = 23;
                let args = captured_windows::arguments(
                    &artifact,
                    &memory,
                    dimensions,
                    &bytes,
                    coordinates,
                );
                let aliases = args.clone();
                let input = contents(&args);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let run = |jit: &JitProgramArtifact, result: &mut [Option<Value>]| {
                    if restored {
                        observe::run_restored(jit, &args, result, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, result, mode.execution(), &memory)
                    }
                };
                let (reference, _) = run(&ordinary, &mut expected);
                let (result, _) = run(&native, &mut output);
                if rejects {
                    assert_eq!(
                        format!("{:?}", result.unwrap_err()),
                        format!("{:?}", reference.unwrap_err())
                    );
                    read_packs::unchanged_publication(&output, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    reference.unwrap();
                    result.unwrap();
                    assert_eq!(
                        contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&expected.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                }
                assert_eq!(contents(&args), input);
                assert_eq!(contents(&aliases), input);
            }
            drop((output, expected));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
