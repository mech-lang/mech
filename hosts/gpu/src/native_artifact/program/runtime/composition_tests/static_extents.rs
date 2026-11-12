//! Fixed extents are properties of an admitted typed view, including at errors.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const TEMPLATE: &str = r#"table := TABLE
read-key(key<KEY>) => (out<u64>) :=
  wide<u64> := key
  loaded<u64> := table[wide + 1u64]
  out := loaded.
sum-keys(keys<[KEY]>, at<u64>, end<u64>, total<u64>) => (out<u64>) :=
  more := at < end
  next := (more? | true => total + read-key(keys[at + 1u64]) | false => total)
  out := (more? | true => sum-keys(keys, at + 1u64, end, next) | false => total).
sum-keys(input<[KEY]>, 0u64, count<u64>, seed<u64>)
"#;

fn source(table: &str, byte: bool) -> String {
    TEMPLATE
        .replace("TABLE", table)
        .replace("KEY", if byte { "u8" } else { "u64" })
}

fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    keys: &[u64],
    count: u64,
    seed: u64,
    byte: bool,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "input",
                Data::Matrix(
                    keys.iter()
                        .map(|&v| {
                            if byte {
                                Data::U8(u8::try_from(v).unwrap())
                            } else {
                                Data::U64(v)
                            }
                        })
                        .collect(),
                ),
            ),
            ("count", Data::U64(count)),
            ("seed", Data::U64(seed)),
        ],
        memory,
    )
}

fn selected(object: &crate::native_artifact::NativeProgramObject) -> usize {
    let metadata = object.metadata();
    [
        "compiled_numeric_write_loops",
        "compiled_readonly_numeric_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap())
    .map(|r| r["static_array_extents"].as_array().unwrap().len())
    .sum()
}

fn exercise(text: &str, values: &[u64], byte: bool) {
    let mode = NativeProgramLimitMode::NoLimits;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(text) {
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let checked = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::StaticNumericExtents,
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            mode.compilation(),
        )
        .unwrap();
        let old = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &catalog,
            OptimizationPass::StaticNumericExtents,
        )
        .unwrap();
        assert!(
            selected(&object) > 0,
            "exercise the actual fixed-view native path"
        );
        assert_eq!(selected(&old), 0);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs: [_; 3] = std::array::from_fn(|_| vec![None; artifact.outputs().len()]);
            let last = values.len() as u64 - 1;
            let mut cases = vec![
                (vec![0, last], 2, 0),
                (vec![], 1, 0),
                (vec![0], 2, 0),
                (vec![1], 1, u64::MAX),
                (vec![last, 0], 2, 7),
            ];
            if !byte {
                cases.extend([
                    (vec![values.len() as u64], 1, u64::MAX),
                    (vec![u64::MAX], 1, 0),
                ]);
            }
            for (keys, rounds, seed) in cases {
                let input = args(&artifact, &memory, &keys, rounds, seed, byte);
                let aliases = input.clone();
                let before = contents(&input);
                let expected = (rounds as usize <= keys.len()).then_some(()).and_then(|_| {
                    keys.iter()
                        .take(rounds as usize)
                        .try_fold(seed, |sum, &key| {
                            let value = *values.get(usize::try_from(key).ok()?)?;
                            sum.checked_add(value)
                        })
                });
                let mut outcomes = Vec::new();
                for (jit, output) in [&native, &checked, &ordinary].into_iter().zip(&mut outputs) {
                    let previous = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = if restored {
                        observe::run_restored(jit, &input, output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &input, output, mode.execution(), &memory)
                    };
                    assert_eq!(result.is_ok(), expected.is_some(), "{result:?}");
                    if let Some(value) = expected {
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(value)
                        );
                    } else {
                        assert!(output.iter().zip(previous).all(|(a, b)| match (a, b) {
                            (Some(a), Some(b)) => a.shares_frozen_storage(&b),
                            (None, None) => true,
                            _ => false,
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                    outcomes.push((
                        result.as_ref().copied().map_err(|e| format!("{e:?}")),
                        contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                    ));
                }
                assert_eq!(
                    outcomes[0], outcomes[1],
                    "same source plan and all native charges"
                );
                assert!(
                    outcomes.windows(2).all(|p| p[0].0.as_ref().map(|_| ())
                        == p[1].0.as_ref().map(|_| ())
                        && p[0].1 == p[1].1),
                    "{outcomes:?}"
                );
                assert_eq!(contents(&input), before);
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

#[test]
fn static_numeric_extents_preserve_errors_aliases_and_atomic_publication() {
    for (table, values) in [
        ("[2u64,3u64,5u64,7u64]", vec![2, 3, 5, 7]),
        ("[2u32,3u32,5u32,7u32]", vec![2, 3, 5, 7]),
        ("[2u64 3u64;5u64 7u64]", vec![2, 5, 3, 7]),
    ] {
        exercise(&source(table, false), &values, false);
    }
}

#[test]
fn static_numeric_extents_admit_the_full_byte_index_range() {
    let values = (0..256).collect::<Vec<u64>>();
    let table = format!(
        "[{}]",
        values
            .iter()
            .map(|x| format!("{x}u64"))
            .collect::<Vec<_>>()
            .join(",")
    );
    exercise(&source(&table, true), &values, true);
}

#[test]
fn static_numeric_extents_preserve_finite_funding_and_dynamic_views() {
    let text = source("[2u64,3u64,5u64,7u64]", false);
    resources::finite_boundaries(&text, |artifact, memory| {
        args(artifact, memory, &[0, 3], 2, 0, false)
    });
    let dynamic = r#"walk(a<[u64]>, i<u64>, end<u64>, sum<u64>) => (out<u64>) :=
  more := i < end
  next := (more? | true => sum + a[i + 1u64] | false => sum)
  out := (more? | true => walk(a, i + 1u64, end, next) | false => sum).
walk(input<[u64]>,0u64,count<u64>,0u64)
"#;
    for artifact in artifacts(dynamic) {
        let object =
            crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        assert_eq!(
            selected(&object),
            0,
            "a caller's dynamic matrix keeps its current length"
        );
    }
    super::numeric_tree::check_numeric_tree_finite_fuel(Some(
        OptimizationPass::StaticNumericExtents,
    ));
}
