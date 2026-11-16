//! A partial phase must not strand a provable narrow-array child on callbacks.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"scan(ids<[u64]>, counts<[u32]>, at<u64>, end<u64>) => (out<bool>) :=
  more := at < end
  found := (more? | true => counts[ids[at + 1u64] + 1u64] >= 6u32 | false => false)
  out := (more? | true => (found? | true => true | false => scan(ids, counts, at + 1u64, end)) | false => false).
rows(offsets<[u64]>, ids<[u64]>, counts<[u32]>, post<u64>, end<u64>) => (out<bool>) :=
  more := post < end
  found := (more? | true => scan(ids, counts, offsets[post + 1u64], offsets[post + 2u64]) | false => true)
  out := (more? | true => (found? | true => rows(offsets, ids, counts, post + 1u64, end) | false => false) | false => true).
rows(offsets<[u64]>, ids<[u64]>, counts<[u32]>, 0u64, count<u64>)
"#;

fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    offsets: &[u64],
    ids: &[u64],
    counts: &[u32],
    count: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "offsets",
                Data::Matrix(offsets.iter().copied().map(Data::U64).collect()),
            ),
            (
                "ids",
                Data::Matrix(ids.iter().copied().map(Data::U64).collect()),
            ),
            (
                "counts",
                Data::Matrix(counts.iter().copied().map(Data::U32).collect()),
            ),
            ("count", Data::U64(count)),
        ],
        memory,
    )
}

fn regions(artifact: &ProgramArtifact) -> Vec<serde_json::Value> {
    let object = crate::native_artifact::emit_program_object_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = object.metadata();
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap().clone())
    .collect()
}

#[test]
fn complete_narrow_scan_is_selected_before_a_partial_phase_and_preserves_failures() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(SOURCE) {
        assert!(
            regions(&artifact).iter().any(|r| {
                r["tree_nodes"].as_array().unwrap().len() == 2
                    && r["cold_exits"].as_array().unwrap().is_empty()
                    && r["typed_loads"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v[1] == 4)
            }),
            "the narrow child must execute inside its complete native tree"
        );
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::NumericTrees,
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs =
                std::array::from_fn::<_, 2, _>(|_| vec![None; artifact.outputs().len()]);
            for (offsets, ids, counts, count, expected) in [
                (
                    &[0, 2, 3][..],
                    &[0, 1, 2][..],
                    &[6, 0, 7][..],
                    2,
                    Some(true),
                ),
                (
                    &[0, 2, 3][..],
                    &[0, 1, 2][..],
                    &[5, 0, 7][..],
                    2,
                    Some(false),
                ),
                (&[0, 0, 1][..], &[0][..], &[u32::MAX][..], 2, Some(false)),
                (&[0, 1][..], &[u64::MAX][..], &[6][..], 1, None),
                (&[0, 1][..], &[99][..], &[6][..], 1, None),
                (&[0, 1][..], &[0][..], &[6][..], 2, None),
                (&[0, 1][..], &[0][..], &[u32::MAX][..], 1, Some(true)),
            ] {
                let args = args(&artifact, &memory, offsets, ids, counts, count);
                let aliases = args.clone();
                let before = contents(&args);
                let mut outcomes = Vec::new();
                for (jit, output) in [&native, &ordinary].into_iter().zip(&mut outputs) {
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = if restored {
                        observe::run_restored(jit, &args, output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, output, mode.execution(), &memory)
                    };
                    assert_eq!(result.is_ok(), expected.is_some(), "{result:?}");
                    if let Some(expected) = expected {
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Bool(expected)
                        );
                    } else {
                        assert!(output.iter().zip(prior).all(|(now, old)| match (now, old) {
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
                        result.as_ref().map(|_| ()).map_err(|e| format!("{e:?}")),
                        contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                    ));
                }
                assert_eq!(outcomes[0], outcomes[1]);
                assert_eq!(contents(&args), before);
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
fn complete_narrow_scan_keeps_finite_admission_and_publication() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        args(artifact, memory, &[0, 2, 3], &[0, 1, 2], &[6, 0, 7], 2)
    });
}

#[test]
fn a_collection_child_retains_the_partial_fallback_when_no_complete_tree_is_proved() {
    let source = SOURCE
        .replacen(
            "  more := at < end\n",
            "  copy-of-counts := [entry | entry <- counts]\n  more := at < end\n",
            1,
        )
        .replace(
            "counts[ids[at + 1u64] + 1u64]",
            "copy-of-counts[ids[at + 1u64] + 1u64]",
        );
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source) {
        assert!(
            regions(&artifact)
                .iter()
                .any(|r| !r["cold_exits"].as_array().unwrap().is_empty()),
            "unsupported child keeps its independently proved partial route"
        );
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let input = args(&artifact, &memory, &[0, 2, 3], &[0, 1, 2], &[6, 0, 7], 2);
        let mut output = vec![None; artifact.outputs().len()];
        for restored in [false, true] {
            let (result, _) = if restored {
                observe::run_restored(&native, &input, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&native, &input, &mut output, mode.execution(), &memory)
            };
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Bool(true)
            );
        }
        drop(output);
        drop(input);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
