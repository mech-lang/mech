//! Genuine scalar cardinality lowering, independent of matrix element storage.
use super::call_cache_tests::artifacts;
use super::*;
use crate::native_artifact::program::prepare::call_routes::LibraryCallRoute as Route;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_core::ResourceDemand;

const SOURCE: &str = "+> sequence\nsequence/length(input<[string]>)\n";

fn matrix(artifact: &mech_engine::ProgramArtifact, rows: u64, columns: u64) -> Value {
    let words = ["", "é", "🛰", "a\nb", "\0", "ordinary"];
    ValueDraft {
        schema: artifact.inputs()[0].schema,
        shape_values: vec![rows, columns].into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            (0..rows * columns)
                .map(|i| ValueDataDraft::String(words[i as usize % words.len()].into()))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap()
}

fn admit(raw: &Value, memory: &ManagedMemoryBudget) -> Value {
    let mut reservation = memory
        .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    raw.clone().into_memory_budget(&mut reservation).unwrap()
}

fn restored(jit: &JitProgramArtifact) -> NativeProgramDescriptor {
    super::super::tests::restore_admitted_helper_descriptor(jit)
}

#[test]
fn matrix_length_lowering_keeps_rectangles_empty_unicode_and_canonical_ablation() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let image = restored(&jit);
        let mut canonical = restored(&jit);
        let selected = canonical
            .metadata
            .call_routes
            .iter()
            .filter(|&&r| r == Route::MatrixLength)
            .count();
        assert!(selected > 0);
        for route in &mut canonical.metadata.call_routes {
            if *route == Route::MatrixLength {
                *route = Route::Canonical;
            }
        }
        // Strategy is independently derived, not a serialized authority bit.
        assert_eq!(
            canonical.encode_build_owned_image().unwrap(),
            image.encode_build_owned_image().unwrap()
        );
        for (rows, columns) in [(0, 0), (0, 7), (7, 0), (1, 1), (2, 3), (3, 2), (1, 1024)] {
            let memory = mode.memory_budget();
            let raw = matrix(&artifact, rows, columns);
            let input = admit(&raw, &memory);
            let alias = input.clone();
            let mut values = Vec::new();
            for descriptor in [jit.descriptor(), &image, &canonical] {
                let mut outputs = vec![None; artifact.outputs().len()];
                crate::native_artifact::program::runtime::execute(
                    descriptor,
                    jit.test_generated_entry(),
                    &[input.clone()],
                    &mut outputs,
                    mode.execution(),
                    &memory,
                )
                .unwrap();
                let value = outputs.last().unwrap().as_ref().unwrap();
                assert_eq!(
                    value.canonical_data_draft().unwrap(),
                    ValueDataDraft::U64(rows * columns)
                );
                values.push(value.canonical_data_draft().unwrap());
            }
            assert!(values.windows(2).all(|v| v[0] == v[1]));
            assert!(input.shares_frozen_storage(&alias));
            assert_eq!(
                input.canonical_data_draft().unwrap(),
                raw.canonical_data_draft().unwrap()
            );
            drop((input, alias));
            assert_eq!(
                (
                    memory.used_bytes(),
                    memory.retained_nodes(),
                    memory.retained_snapshot_nodes()
                ),
                (0, 0, 0)
            );
        }
    }
}

#[test]
fn matrix_length_lowering_kernel_retains_finite_caps_schema_and_same_account_checks() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for descriptor in [jit.descriptor(), &restored(&jit)] {
            let call = descriptor
                .metadata()
                .calls
                .iter()
                .flatten()
                .find(|c| c.supports_matrix_length())
                .unwrap();
            assert!(!call.supports_unsigned_matrix_length());
            let memory = mode.memory_budget();
            let mut meter = NativeWorkMeter::new(u64::MAX).unwrap();
            let work = meter
                .shared_library_budget(NativeExecutionPhase::Setup, u64::MAX, &memory)
                .unwrap();
            let raw = matrix(&artifact, 2, 3);
            let limits = mode.execution().library;
            assert!(
                format!(
                    "{:?}",
                    call.matrix_length(&raw, limits, &work, ResourceDemand::default())
                        .unwrap_err()
                )
                .contains("Unadmitted")
            );
            let owned = admit(&raw, &memory);
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            assert_eq!(
                call.matrix_length(&owned, limits, &work, ResourceDemand::default())
                    .unwrap(),
                6
            );
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            let foreign_memory = mode.memory_budget();
            let foreign = admit(&raw, &foreign_memory);
            assert!(
                format!(
                    "{:?}",
                    call.matrix_length(&foreign, limits, &work, ResourceDemand::default())
                        .unwrap_err()
                )
                .contains("Unadmitted")
            );
            let [other_artifact, _] = artifacts("+> sequence\nsequence/length(input<[u64]>)\n");
            let other_raw = ValueDraft {
                schema: other_artifact.inputs()[0].schema,
                shape_values: vec![2, 3].into_boxed_slice(),
                data: ValueDataDraft::Matrix(vec![ValueDataDraft::U64(0); 6].into_boxed_slice()),
            }
            .finalize(&SnapshotValidationContext::new(other_artifact.schemas()))
            .unwrap();
            let other = admit(&other_raw, &memory);
            assert!(
                format!(
                    "{:?}",
                    call.matrix_length(&other, limits, &work, ResourceDemand::default())
                        .unwrap_err()
                )
                .contains("InvalidInput")
            );
            drop(other);
            for which in 0..7 {
                let mut missing = limits;
                match which {
                    0 => missing.max_output_elements = None,
                    1 => missing.max_output_bytes = None,
                    2 => missing.max_temporary_bytes = None,
                    3 => missing.max_cloned_bytes = None,
                    4 => missing.max_retained_nodes = None,
                    5 => missing.max_compute_work = None,
                    _ => missing.max_comparison_work = None,
                }
                assert!(
                    call.matrix_length(&owned, missing, &work, ResourceDemand::default())
                        .is_err()
                );
            }
            for which in 0..4 {
                let mut tight = limits;
                match which {
                    0 => tight.max_output_elements = Some(0),
                    1 => tight.max_output_bytes = Some(7),
                    2 => tight.max_compute_work = Some(0),
                    _ => tight.max_comparison_work = Some(0),
                }
                assert!(
                    call.matrix_length(&owned, tight, &work, ResourceDemand::default())
                        .is_err()
                );
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
            }
            assert!(
                call.matrix_length(
                    &owned,
                    limits,
                    &work,
                    ResourceDemand {
                        output_elements: u64::MAX,
                        ..ResourceDemand::default()
                    }
                )
                .is_err()
            );
            let mut output_denied = limits;
            output_denied.max_output_elements = Some(0);
            assert!(
                format!(
                    "{:?}",
                    call.matrix_length(&raw, output_denied, &work, ResourceDemand::default())
                        .unwrap_err()
                )
                .contains("TargetLimitExceeded"),
                "original output denial precedes unadmitted input access"
            );
            assert_eq!(
                call.matrix_length(&owned, limits, &work, ResourceDemand::default())
                    .unwrap(),
                6
            );
            drop((owned, foreign, work, meter));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
            assert_eq!(
                (
                    foreign_memory.used_bytes(),
                    foreign_memory.retained_snapshot_nodes()
                ),
                (0, 0)
            );
        }
    }
}

#[test]
fn matrix_length_lowering_failures_preserve_publication_aliases_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let image = restored(&jit);
        for descriptor in [jit.descriptor(), &image] {
            let memory = mode.memory_budget();
            let mut outputs = vec![None; artifact.outputs().len()];
            for (rows, columns, denial) in [
                (2, 3, 0),
                (1, 9, 1),
                (4, 1, 2),
                (3, 3, 3),
                (0, 8, 0),
                (2, 2, 0),
            ] {
                let raw = matrix(&artifact, rows, columns);
                let input = admit(&raw, &memory);
                let alias = input.clone();
                let previous = outputs.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut limits = mode.execution();
                match denial {
                    1 => limits.execution.max_work = 0,
                    2 => limits.library.max_output_elements = Some(0),
                    3 => limits.library.max_comparison_work = Some(0),
                    _ => {}
                }
                let result = crate::native_artifact::program::runtime::execute(
                    descriptor,
                    jit.test_generated_entry(),
                    &[input.clone()],
                    &mut outputs,
                    limits,
                    &memory,
                );
                if denial != 0 {
                    assert!(result.is_err());
                    assert!(outputs.iter().zip(&previous).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
                    result.unwrap();
                    assert_eq!(
                        outputs
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        ValueDataDraft::U64(rows * columns)
                    );
                }
                assert!(input.shares_frozen_storage(&alias));
                assert_eq!(
                    input.canonical_data_draft().unwrap(),
                    raw.canonical_data_draft().unwrap()
                );
            }
            drop(outputs);
            assert_eq!(
                (
                    memory.used_bytes(),
                    memory.retained_nodes(),
                    memory.retained_snapshot_nodes()
                ),
                (0, 0, 0)
            );
        }
    }
}
