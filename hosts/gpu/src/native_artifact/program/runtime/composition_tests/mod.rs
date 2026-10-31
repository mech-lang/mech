//! Actual parent/child admissions, current roots, finite funding and recovery.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_core::ValueDataDraft as Data;
use mech_engine::ProgramArtifact;
mod observe;
mod pack;
mod record_lookup;
mod resources;
mod scalar_child;

#[test]
fn huffman_parent_cleanup_selects_funded_retirement_batches() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = huffman_inputs(&artifact, 3, vec![0, 1, 1, 2], &memory);
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let mut expected_work = None;
        for _ in 0..2 {
            output.fill(None);
            super::retirement::take_observation();
            super::retirement::take_child_observation();
            let report = jit
                .evaluate_into(&args, &mut output, mode.execution(), &memory)
                .unwrap();
            let batches = super::retirement::take_observation();
            let omitted = super::retirement::take_child_observation();
            assert_eq!(contents(&args), before);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([3, 1, 3, 2].map(Data::U8).into()),
            );
            assert_eq!(report.work, *expected_work.get_or_insert(report.work));
            assert_eq!(
                report.work, 91_452_918,
                "original cleanup prices must remain"
            );
            println!("huffman-cleanup work={} batches={batches}", report.work);
            assert!(
                batches > 0,
                "original Huffman parent must use funded cleanup"
            );
            assert!(
                omitted > 0,
                "completed Huffman children must avoid repeated physical cleanup"
            );
        }
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let original = super::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, Data)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = named.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            let shape_values: Box<[u64]> = if let Data::Matrix(cells) = &data {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, cells.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .into()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reserve = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reserve).unwrap()
        })
        .collect()
}
fn huffman_inputs(
    artifact: &ProgramArtifact,
    width: u64,
    counts: Vec<u64>,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
            ("lens", Data::Matrix([9, 9, 9, 9].map(Data::U8).into())),
            (
                "counts",
                Data::Matrix(counts.into_iter().map(Data::U64).collect()),
            ),
            ("width", Data::U64(width)),
            ("start", Data::U64(0)),
        ],
        memory,
    )
}
fn contents(values: &[Value]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.canonical_data_draft().unwrap())
        .collect()
}

#[test]
fn composition_counts_real_child_initializations_and_completed_empty_calls() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let proof = &jit.descriptor().metadata().compositions.parents[0];
        let mut output = vec![None; artifact.outputs().len()];
        let memory = mode.memory_budget();
        for (width, counts, expected_calls, expected) in [
            (0, vec![], 0, vec![9, 9, 9, 9]),
            (3, vec![0, 0, 0, 0], 3, vec![9, 9, 9, 9]),
            (3, vec![0, 1, 1, 2], 3, vec![3, 1, 3, 2]),
        ] {
            let args = huffman_inputs(&artifact, width, counts, &memory);
            let before = contents(&args);
            let (result, actual) =
                observe::run(&jit, &args, &mut output, mode.execution(), &memory);
            result.unwrap();
            assert!(actual.selected > 0);
            assert_eq!(
                actual
                    .admits
                    .get(&u64::from(proof.child_initialize))
                    .copied()
                    .unwrap_or(0),
                expected_calls
            );
            assert_eq!(
                actual
                    .admits
                    .get(&u64::from(proof.child_exit))
                    .copied()
                    .unwrap_or(0),
                expected_calls
            );
            if width == 0 || expected == vec![9, 9, 9, 9] {
                let writes = jit
                    .descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .find(|r| r.iteration == proof.child)
                    .unwrap()
                    .numeric
                    .as_ref()
                    .unwrap();
                for &(site, _) in writes.actions.iter().filter(|(_, op)| matches!(op, crate::native_artifact::program::prepare::readonly_numeric::Operation::Write { .. })) {
                    assert!(
                        actual.helpers.keys().all(|&(s, _)| s != site as u64),
                        "empty child performed a first write: {actual:?}"
                    );
                }
            }
            assert_eq!(contents(&args), before);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(expected.into_iter().map(Data::U8).collect())
            );
            println!(
                "{}",
                serde_json::json!({"witness":"actual-child-boundaries", "width":width, "child_calls":expected_calls, "selected_parent_fragments":actual.selected})
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn composition_native_parent_resumes_after_an_ordinary_empty_child() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = inputs(
            &artifact,
            &[
                ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
                ("lens", Data::Matrix(vec![Data::U8(9); 8].into())),
                ("counts", Data::Matrix([0, 0, 0, 0].map(Data::U64).into())),
                ("width", Data::U64(3)),
                ("start", Data::U64(0)),
            ],
            &memory,
        );
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = mode.execution();
        // The child preview must decline its possible eight-element write.
        // Its actual empty path constructs only the existing two-field result.
        // Parent frame packs have at most five fields and remain eligible.
        policy.library.max_output_elements = Some(5);
        let (result, counts) = observe::run(&jit, &args, &mut output, policy, &memory);
        result.unwrap();
        assert!(counts.selected > 0);
        let proof = &jit.descriptor().metadata().compositions.parents[0];
        let mech_engine::native_artifact::NativeControlLoop::Tail(child) =
            &jit.descriptor().plan().iterations()[proof.child as usize]
        else {
            panic!()
        };
        for site in [proof.child_initialize, child.body(), proof.child_exit] {
            assert_eq!(
                counts.admits.get(&u64::from(site)).copied().unwrap_or(0),
                3,
                "ordinary child was replayed or skipped: {counts:?}"
            );
        }
        assert_eq!(contents(&args), before);
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Matrix(vec![Data::U8(9); 8].into())
        );
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
