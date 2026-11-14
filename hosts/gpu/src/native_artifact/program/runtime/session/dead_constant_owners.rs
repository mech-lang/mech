//! All-path unread constants keep compiler identity but no invocation owner.
use super::call_cache_tests::artifacts;
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/dead-constant-owners.mec");

#[test]
fn dead_constant_owners_omit_only_unread_values_in_original_and_restored_setup() {
    for source in ["[1u8,2u8,3u8]\n", "[1u32,2u32,3u32]\n", SOURCE] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
            for descriptor in [jit.descriptor(), &restored] {
                let metadata = descriptor.metadata();
                let facts = metadata.plan.facts();
                let reads = facts.semantic_use_counts().unwrap();
                let constants = metadata.plan.constants();
                assert!(
                    constants
                        .iter()
                        .any(|c| reads[c.value().get() as usize] == 0)
                );
                assert!(
                    constants
                        .iter()
                        .any(|c| reads[c.value().get() as usize] != 0)
                );
                if source != SOURCE {
                    assert_eq!(
                        constants.len(),
                        4,
                        "immutable table and its three compile-time literals stay in the plan"
                    );
                }
                assert_eq!(
                    metadata.constant_admission_bytes,
                    metadata.constant_bytes.iter().sum::<u64>()
                );
                let empty = ManagedMemoryBudget::new(0);
                assert!(Prepared::new(descriptor, mode.execution(), &empty).is_err());
                assert_eq!(
                    (empty.used_bytes(), empty.retained_snapshot_nodes()),
                    (0, 0)
                );
                let memory = mode.memory_budget();
                let prepared = Prepared::new(descriptor, mode.execution(), &memory).unwrap();
                let state = prepared.state.as_ref().unwrap();
                for (c, &bound) in constants.iter().zip(&metadata.constant_bytes) {
                    let live = reads[c.value().get() as usize] != 0;
                    assert_eq!(
                        bound != 0,
                        live,
                        "only proved unread constants may omit ownership"
                    );
                    assert_eq!(
                        state
                            .arena
                            .native_entry_preview(c.value().get(), &state.control.work)
                            .unwrap()
                            .is_some(),
                        live
                    );
                }
                drop(prepared);
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
}

#[test]
fn dead_constant_owners_images_rederive_the_live_set_and_reject_changed_bounds() {
    for artifact in artifacts("[1u8,2u8,3u8]\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let metadata = jit.descriptor().metadata();
        let facts = metadata.plan.facts();
        let reads = facts.semantic_use_counts().unwrap();
        let constants = metadata.plan.constants();
        let live = constants
            .iter()
            .position(|c| reads[c.value().get() as usize] != 0)
            .unwrap();
        let dead = constants
            .iter()
            .position(|c| reads[c.value().get() as usize] == 0)
            .unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for (index, bound) in [
            (live, 0),
            (dead, 1),
            (live, metadata.constant_bytes[live] - 1),
        ] {
            let mut malformed: serde_json::Value = serde_json::from_slice(&image).unwrap();
            malformed["descriptor"]["constant_bytes"][index] = bound.into();
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&malformed).unwrap()
                ),
                "serialized constant bounds cannot change the derived ownership decision"
            );
        }
    }
}

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    flag: bool,
    divisor: u64,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = match name.as_str() {
                "flag" => ValueDataDraft::Bool(flag),
                "divisor" => ValueDataDraft::U64(divisor),
                _ => panic!("{name}"),
            };
            let value = ValueDraft {
                schema: port.schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut claim = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut claim).unwrap()
        })
        .collect()
}

#[test]
fn dead_constant_owners_keep_unselected_reads_finite_failure_publication_and_retry() {
    let mut expected_error = None;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let mut output = vec![None; artifact.outputs().len()];
        for (flag, divisor, deny) in [
            (true, 0, false),
            (false, 0, false),
            (false, 1, false),
            (true, 1, true),
            (true, 2, false),
        ] {
            let args = arguments(&artifact, flag, divisor, &memory);
            let aliases = args.clone();
            let before = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let previous = output.clone();
            let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut policy = mode.execution();
            if deny {
                policy.execution.max_work = 0;
            }
            let result = jit.evaluate_into(&args, &mut output, policy, &memory);
            if deny || (!flag && divisor == 0) {
                let error = format!("{:?}", result.unwrap_err());
                if !deny {
                    assert!(error.contains("Arithmetic"), "{error}");
                    assert_eq!(error, *expected_error.get_or_insert(error.clone()));
                }
                assert!(output.iter().zip(&previous).all(|(a, b)| {
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
                let expected = if flag { [1, 2, 3] } else { [4, 5, 6] };
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    ValueDataDraft::Matrix(expected.map(ValueDataDraft::U8).into())
                );
            }
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
            assert_eq!(
                aliases
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
