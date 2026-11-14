//! The current shape of a captured buffer is independent of a trip-count proof.
use super::*;

fn source() -> String {
    let ordinary = guarded_read_packs::source(8, false, false);
    let start = ordinary.find("walk(").unwrap();
    format!(
        "{}\n{}\n{}",
        &ordinary[..start],
        "#ReadPhases(xs<[u8]>, start<u64>, seed<u64>) => <u64>
  | :Start
  | :First(at<u64>, visits<u64>, total<u64>)
  | :Second(at<u64>, visits<u64>, total<u64>)
  | :Done(total<u64>).
#ReadPhases(xs, start, seed) -> :Start
  :Start -> :First(start, 0u64, seed)
  :First(at, visits, total)
    | visits < 2u64 -> {
        word := guarded(xs, at)
        other := guarded(xs, at + 8u64)
      } -> :Second(at + 1u64, visits + 1u64, bits/xor(total, bits/xor(word, other)))
    | * -> :Done(total)
  :Second(at, visits, total)
    | visits < 2u64 -> {
        word := guarded(xs, at)
        other := guarded(xs, at + 8u64)
      } -> :First(at + 1u64, visits + 1u64, bits/xor(total, bits/xor(word, other)))
    | * -> :Done(total)
  :Done(total) => total.",
        ordinary[start..]
            .replace(
                "true => guarded(xs, at)",
                "true => #ReadPhases(xs, at, total)"
            )
            .replace("bits/xor(total, word)", "word")
    )
}

#[test]
fn multistate_captured_packs_hoist_only_current_geometry() {
    for artifact in artifacts(&source()) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let geometry_scopes = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|name| metadata[name].as_array().unwrap())
        .flat_map(|region| region["guarded_loop_windows"].as_array().unwrap())
        .filter(|window| !window["geometry_views"].as_array().unwrap().is_empty())
        .count();
        assert_eq!(geometry_scopes, 1);
        let descriptor = object.descriptor().metadata();
        let child = descriptor.plan.iterations()[1].as_ref();
        assert!(
            descriptor
                .plan
                .blocks()
                .iter()
                .filter(|block| { block.iteration() == Some(1) })
                .count()
                > 256,
            "the real comparison needs geometry even when value analysis declines"
        );
        assert!(child.parent().is_some());
        let packs = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|name| metadata[name].as_array().unwrap())
        .flat_map(|region| region["fully_lowered_read_packs"].as_array().unwrap())
        .count();
        // The second read uses a separately checked at+8. The existing window
        // recognizer declines that expression; geometry must not invent its
        // missing index proof. Only each state's first pack is fully lowered.
        assert_eq!(packs, 2, "both source-authorized packs must be lowered");
    }
}

#[test]
fn multistate_packs_keep_geometry_current_errors_publication_and_retry() {
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
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            let mut expected = published.clone();
            for (dimensions, at, steps) in [
                ([1, 40], 0, 3),
                ([5, 8], 0, 3),
                ([40, 1], 2, 2),
                ([8, 5], 2, 2),
                ([1, 7], 0, 3),
                ([0, 8], 0, 2),
                ([1, 40], u64::MAX, 0),
                ([1, 40], u64::MAX, 1),
                ([1, 40], 0, 3),
                ([5, 8], u64::MAX, 1),
                ([40, 1], 0, 3),
            ] {
                let args = read_packs::arguments(&artifact, &memory, dimensions, at, steps, 41);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let run = |jit: &JitProgramArtifact, output: &mut [Option<Value>]| {
                    if restored {
                        observe::run_restored(jit, &args, output, mode.execution(), &memory).0
                    } else {
                        observe::run(jit, &args, output, mode.execution(), &memory).0
                    }
                };
                let reference = run(&ordinary, &mut expected);
                let actual = run(&native, &mut published);
                if at == u64::MAX && steps != 0 {
                    let error = format!("{:?}", actual.unwrap_err());
                    assert!(error.contains("Arithmetic"), "{error}");
                    assert_eq!(error, format!("{:?}", reference.unwrap_err()));
                    read_packs::unchanged_publication(&published, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
                    reference.unwrap();
                    actual.unwrap();
                    assert_eq!(
                        contents(&published.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&expected.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                    let [rows, columns] = dimensions;
                    let mut total = 41u64;
                    for turn in 0..steps {
                        for visit in 0..2 {
                            for offset in [0, 8] {
                                let index = at + turn + visit + offset;
                                if rows * columns >= 8 && index <= rows * columns - 8 {
                                    let mut word = 0u64;
                                    for byte in 0..8 {
                                        let ordinal = index + byte;
                                        let physical = (ordinal % rows) * columns + ordinal / rows;
                                        let value =
                                            (physical as u8).wrapping_mul(37).wrapping_add(41);
                                        word |= u64::from(value) << (8 * byte);
                                    }
                                    total ^= word;
                                }
                            }
                        }
                    }
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(total)
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop((published, expected));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn multistate_packs_keep_finite_boundaries() {
    resources::finite_boundaries(&source(), |artifact, memory| {
        read_packs::arguments(artifact, memory, [1, 40], 2, 3, 41)
    });
}
