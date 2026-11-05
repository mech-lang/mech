//! Source window checks remain lexical while the packed read reuses their fact.
use super::*;

pub(super) fn source(width: usize, false_first: bool, checked_short: bool) -> String {
    let ordinary = super::read_packs::source(width, 1);
    let split = ordinary.find("walk(").unwrap();
    let branches = if false_first {
        "false => 0u64 | true => pack(xs, at)"
    } else {
        "true => pack(xs, at) | false => 0u64"
    };
    let window = format!("((at <= available - {width}u64)? | {branches})");
    let result = if checked_short {
        window
    } else {
        format!("((available >= {width}u64)? | true => {window} | false => 0u64)")
    };
    format!(
        "+> sequence\n{}guarded(xs<[u8]>, at<u64>) => (out<u64>) :=\n  available := sequence/length(xs)\n  out := {result}.\n{}",
        &ordinary[..split],
        ordinary[split..].replace("pack(xs, at) | false", "guarded(xs, at) | false")
    )
}

fn expected(width: usize, dimensions: [u64; 2], at: u64, steps: u64, nonce: u8) -> u64 {
    let [rows, columns] = dimensions;
    let length = rows * columns;
    let mut result = u64::from(nonce);
    for visit in 0..steps {
        let at = at + visit;
        if length < width as u64 || at > length - width as u64 {
            continue;
        }
        let mut word = 0;
        for byte in 0..width {
            let ordinal = at + byte as u64;
            let physical = (ordinal % rows) * columns + ordinal / rows;
            let value = (physical as u8).wrapping_mul(37).wrapping_add(nonce);
            word |= u64::from(value) << (8 * byte);
        }
        result ^= word;
    }
    result
}

#[test]
fn source_window_guard_reaches_a_packed_read_without_another_window_test() {
    for width in [2, 4, 8] {
        for false_first in [false, true] {
            for artifact in artifacts(&source(width, false_first, false)) {
                let object = crate::native_artifact::emit_program_object_with_limits(
                    &artifact,
                    &mech_stdlib::source_catalog(),
                    NativeProgramLimitMode::NoLimits.compilation(),
                )
                .unwrap();
                let metadata = object.metadata();
                let guarded = ["compiled_readonly_numeric_loops", "compiled_numeric_write_loops"]
                    .into_iter()
                    .flat_map(|name| metadata[name].as_array().unwrap())
                    .flat_map(|region| {
                        region.get("source_guarded_read_packs").and_then(|v| v.as_array())
                            .into_iter().flatten()
                    })
                    .count();
                assert_eq!(guarded, 1, "source window must authorize its own pack: {metadata}");
            }
        }
    }
}

#[test]
fn source_guarded_packs_keep_current_windows_geometry_and_unselected_paths() {
    for width in [2, 4, 8] {
        for false_first in [false, true] {
            for artifact in artifacts(&source(width, false_first, false)) {
                let mode = NativeProgramLimitMode::NoLimits;
                let native = JitProgramArtifact::compile_with_limits(
                    &artifact, &mech_stdlib::source_catalog(), mode.compilation(),
                ).unwrap();
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut published = vec![None; artifact.outputs().len()];
                    for (dimensions, at, steps) in [
                        ([1, 40], 0, 4), ([40, 1], 5, 3),
                        ([5, 8], 3, 4), ([8, 5], 8, 3),
                        ([1, width as u64], 0, 2),
                        ([1, width as u64 - 1], 0, 3),
                        ([0, 8], 0, 2), ([8, 0], 0, 2),
                        ([1, 40], 40 - width as u64, 3),
                        ([1, 40], u64::MAX, 0), ([1, 40], 41, 3),
                        ([40, 1], 1, 4),
                    ] {
                        let nonce = 53;
                        let args = super::read_packs::arguments(&artifact, &memory, dimensions, at, steps, nonce);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let result = if restored {
                            observe::run_restored(&native, &args, &mut published, mode.execution(), &memory).0
                        } else {
                            observe::run(&native, &args, &mut published, mode.execution(), &memory).0
                        };
                        result.unwrap();
                        assert_eq!(published.last().unwrap().as_ref().unwrap().canonical_data_draft().unwrap(),
                                   Data::U64(expected(width, dimensions, at, steps, nonce)));
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                    drop(published);
                    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
                }
            }
        }
    }
}
