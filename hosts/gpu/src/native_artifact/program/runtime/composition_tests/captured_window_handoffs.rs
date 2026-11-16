//! Fresh child admission after a selected parent handoff.
use super::captured_windows::{arguments, four_source};
use super::*;

#[test]
fn captured_read_windows_readmit_current_children_after_a_versioned_handoff() {
    let mode = NativeProgramLimitMode::NoLimits;
    let source = four_source().replace(
        "probe(bytes, a, b, start, limit)",
        "probe(bytes, a, b, start, ((odd? | true => limit - 1u64 | false => limit)))",
    );
    for artifact in artifacts(&source) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        assert!(
            [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops"
            ]
            .into_iter()
            .flat_map(|name| metadata[name].as_array().unwrap())
            .flat_map(|row| row["version_entry_gates"].as_array().unwrap())
            .any(|gate| gate[2]
                .as_u64()
                .is_some_and(|parent| Some(parent) != gate[1].as_u64())),
            "a selected caller must freshly gate its child, rather than bypass its version"
        );
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for dimensions in [[1, 512], [512, 1], [16, 32]] {
                for mismatch in (0..8).chain(24..40).chain([64, 127, 255, 257, usize::MAX]) {
                    let mut bytes = vec![17; 512];
                    if mismatch != usize::MAX {
                        bytes[48 + mismatch] = 23;
                    }
                    for start in [0, 3] {
                        let [rows, columns] = dimensions;
                        let byte = |at: u64| bytes[((at % rows) * columns + at / rows) as usize];
                        let expected = [258, 257]
                            .into_iter()
                            .map(|end| {
                                (start..end)
                                    .find(|&at| byte(128 + at) != byte(48 + at))
                                    .unwrap_or(end)
                            })
                            .sum::<u64>();
                        let args = arguments(
                            &artifact,
                            &memory,
                            dimensions,
                            &bytes,
                            [128, 48, start, 258, 2],
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let (result, counts) = if restored {
                            observe::run_restored(
                                &native,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(&native, &args, &mut output, mode.execution(), &memory)
                        };
                        result.unwrap();
                        assert!(counts.phases > 0);
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected)
                        );
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                }
            }
            drop(output);
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
