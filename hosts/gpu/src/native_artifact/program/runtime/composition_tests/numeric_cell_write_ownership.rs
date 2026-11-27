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
