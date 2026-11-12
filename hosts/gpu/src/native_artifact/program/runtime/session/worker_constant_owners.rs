//! Worker startup follows the complete source CFG's immutable capture proof.
use super::call_cache_tests::artifacts;
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

fn source() -> String {
    let table = (0..64)
        .map(|n| format!("{n}u64"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{}
+> number
+> text
+> sequence
render(s<string>, n<u64>, d<u64>) => (out<string>) :=
  h := #Count(s, 1u64, n, empty)
  q := 100u64 / d
  out := text/join([number/format(h{{key}} + q, 0u64) | key <- ["a", "b"]], ",").
serial-table := [{table}]
marker := sequence/length(serial-table)
s := seed<string>
d := divisor<u64>
text/join([render(s, n, d) | n <- [4u64 8u64]], "|") + ":" + number/format(marker, 0u64)
"#,
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT
    )
}

#[test]
fn worker_constant_owners_only_import_uncaptured_reads_in_original_decoded_and_restored_setup() {
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        for descriptor in [jit.descriptor(), &restored] {
            let metadata = descriptor.metadata();
            let region = metadata.parallel.regions.first().expect("outlined worker");
            let constants = metadata.plan.constants();
            let table = constants
                .iter()
                .position(|c| matches!(c.constant().data(), ValueData::Matrix(v) if v.elements().len()==64))
                .expect("serial-only table");
            assert!(metadata.constant_bytes[table] > 0);
            assert!(!region.captures.contains(&constants[table].value().get()));
            assert!(!region.owned_constants.contains(&(table as u32)));
            assert!(
                constants
                    .iter()
                    .any(|c| region.captures.contains(&c.value().get())),
                "live constants must still arrive through captures"
            );
            let memory = mode.memory_budget();
            let parent = Prepared::new(descriptor, mode.execution(), &memory).unwrap();
            let worker =
                Prepared::new_with_budget(descriptor, mode.execution(), &memory, None, Some(0))
                    .unwrap();
            for (index, constant) in constants.iter().enumerate() {
                for (prepared, live) in [
                    (&parent, metadata.constant_bytes[index] != 0),
                    (
                        &worker,
                        region.owned_constants.contains(&(index as u32))
                            && metadata.constant_bytes[index] != 0,
                    ),
                ] {
                    let state = prepared.state.as_ref().unwrap();
                    assert_eq!(
                        state
                            .arena
                            .native_entry_preview(constant.value().get(), &state.control.work)
                            .unwrap()
                            .is_some(),
                        live
                    );
                }
            }
            drop((parent, worker));
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
fn worker_constant_owners_images_rederive_imports_captures_and_all_sites() {
    let artifact = super::super::tests::artifact_source(&source());
    let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let image = jit.descriptor().encode_build_owned_image().unwrap();
    let original: serde_json::Value = serde_json::from_slice(&image).unwrap();
    let table = jit
        .descriptor()
        .metadata()
        .plan
        .constants()
        .iter()
        .position(|c| matches!(c.constant().data(),ValueData::Matrix(v)if v.elements().len()==64))
        .unwrap();
    for replacement in [
        serde_json::json!([table]),
        serde_json::json!([table, table]),
        serde_json::json!([u32::MAX]),
        serde_json::Value::Null,
    ] {
        let mut changed = original.clone();
        changed["descriptor"]["parallel"]["regions"][0]["owned_constants"] = replacement;
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&changed).unwrap()
        ));
    }
    for field in ["captures", "sites"] {
        let mut changed = original.clone();
        changed["descriptor"]["parallel"]["regions"][0][field]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(super::super::tests::rejects_malformed_helper_image(
            &serde_json::to_vec(&changed).unwrap()
        ));
    }
}

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    divisor: u64,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = match name.as_str() {
                "seed" => ValueDataDraft::String("ab:ab:ab".into()),
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
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

#[test]
fn worker_constant_owners_keep_parallel_results_cold_errors_failed_publication_and_retry() {
    use crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS;
    struct Reset(Option<usize>);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_WORKERS.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(TEST_WORKERS.with(|v| v.replace(Some(1))));
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        for descriptor in [jit.descriptor(), &restored] {
            let memory = mode.memory_budget();
            let mut serial = vec![None; artifact.outputs().len()];
            let mut parallel = vec![None; artifact.outputs().len()];
            for (divisor, deny) in [(1, false), (0, false), (2, true), (2, false)] {
                let args = arguments(&artifact, divisor, &memory);
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let previous = parallel.clone();
                let held = (
                    memory.used_bytes(),
                    memory.retained_nodes(),
                    memory.retained_snapshot_nodes(),
                );
                let mut limits = super::super::tests::limits();
                limits.library.max_retained_nodes = Some(1_048_576);
                if deny {
                    limits.execution.max_work = 0;
                }
                TEST_WORKERS.with(|v| v.set(Some(1)));
                let left = execute(
                    descriptor,
                    |f| unsafe { jit.test_checked_entry()(f) },
                    &args,
                    &mut serial,
                    limits,
                    &memory,
                );
                TEST_WORKERS.with(|v| v.set(Some(4)));
                let right = execute(
                    descriptor,
                    |f| unsafe { jit.test_checked_entry()(f) },
                    &args,
                    &mut parallel,
                    limits,
                    &memory,
                );
                if divisor == 0 || deny {
                    let le = format!("{:?}", left.unwrap_err());
                    let re = format!("{:?}", right.unwrap_err());
                    assert!(
                        if deny {
                            le.contains("WorkLimit") && re.contains("WorkLimit")
                        } else {
                            le.contains("Arithmetic") && re.contains("Arithmetic")
                        },
                        "{le} / {re}"
                    );
                    assert!(parallel.iter().zip(&previous).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (
                            memory.used_bytes(),
                            memory.retained_nodes(),
                            memory.retained_snapshot_nodes()
                        ),
                        held
                    );
                } else {
                    let left = left.unwrap();
                    let right = right.unwrap();
                    assert_eq!(left.parallel_tasks, 0);
                    assert_eq!(right.parallel_tasks, 2);
                    assert_eq!(
                        serial
                            .iter()
                            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        parallel
                            .iter()
                            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                            .collect::<Vec<_>>()
                    );
                }
                assert_eq!(
                    aliases
                        .iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
            }
            drop((serial, parallel));
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
fn numeric_related_workers_use_physical_prefixes_with_full_admission_and_checked_handles() {
    let related =
        include_str!("../../../../../../../benchmarks/related-post-gen/mech/related_optimized.mec");
    for artifact in artifacts(related) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        assert_eq!(
            jit.descriptor().metadata().worker_value_slots,
            restored.metadata().worker_value_slots
        );
        for descriptor in [jit.descriptor(), &restored] {
            let metadata = descriptor.metadata();
            assert_eq!(
                metadata.parallel.regions.len(),
                1,
                "actual related source must outline its worker"
            );
            let slots = metadata.worker_value_slots[0].as_ref().unwrap();
            let physical = slots.physical_slots();
            let prefix = slots.logical_prefix();
            let full = metadata.types.len();
            assert!(
                physical > 0 && physical < prefix && prefix < full,
                "actual related source must positively select compact owner backing"
            );
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let worker =
                Prepared::new_with_budget(descriptor, mode.execution(), &memory, None, Some(0))
                    .unwrap();
            assert_eq!(
                worker.state.as_ref().unwrap().arena.test_physical_slots(),
                (physical, physical, prefix)
            );
            drop(worker);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            let types = &metadata.types;
            let bytes = NativeTypedStorage::<Option<Value>>::admission_bytes(full).unwrap()
                + NativeWordStorage::admission_bytes(full).unwrap();
            let setup = full as u64 * 24 + 256;
            let raw_bytes = (full * (core::mem::size_of::<Option<Value>>() + 8)) as u64;
            for count in [0, physical, full] {
                let exact = memory.with_limits(bytes, 0);
                let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
                let arena =
                    NativeValueArena::new_with_prefix(types, count, raw_bytes, &exact, &mut work)
                        .unwrap();
                assert_eq!(arena.test_physical_slots(), (count, count, count));
                assert_eq!(work.used(), setup);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (bytes, 0));
                drop(arena);
                let short = memory.with_limits(bytes - 1, 0);
                let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
                assert!(
                    NativeValueArena::new_with_prefix(types, count, raw_bytes, &short, &mut work)
                        .is_err()
                );
                assert_eq!(work.used(), setup);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                let mut work = NativeWorkMeter::new(setup - 1).unwrap();
                assert!(
                    NativeValueArena::new_with_prefix(types, count, raw_bytes, &exact, &mut work)
                        .is_err()
                );
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
                assert!(
                    matches!(NativeValueArena::new_with_prefix(types,count,raw_bytes-1,&exact,&mut work),Err(NativeValueError::Base(NativeExecutionError::TransportLimit {required,..})) if required==raw_bytes)
                );
                assert_eq!(work.used(), 0);
            }
            let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
            assert!(matches!(
                NativeValueArena::new_with_prefix(types, full + 1, raw_bytes, &memory, &mut work),
                Err(NativeValueError::InvalidSlot)
            ));
            assert_eq!((memory.used_bytes(), work.used()), (0, 0));
            let mut arena =
                NativeValueArena::new_with_prefix(types, physical, raw_bytes, &memory, &mut work)
                    .unwrap();
            let slot = metadata
                .plan
                .values()
                .iter()
                .take(physical)
                .position(|ty| {
                    ty.scalar_kind() == Some(mech_engine::native_artifact::NativeScalarKind::U64)
                })
                .unwrap() as u32;
            let raw = ValueDraft {
                schema: metadata.plan.values()[slot as usize].schema(),
                shape_values: Box::new([]),
                data: ValueDataDraft::U64(17),
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(
                &metadata.schemas,
            ))
            .unwrap();
            let value = raw.clone().admit_into_memory_budget(&memory).unwrap();
            let alias = value.clone();
            let first = arena.put(slot, &value, &mut work).unwrap();
            assert!(arena.put(slot, &raw, &mut work).is_err());
            assert_eq!(
                arena
                    .get(first, &mut work)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::U64(17)
            );
            let second = arena.put(slot, &alias, &mut work).unwrap();
            assert!(matches!(
                arena.get(first, &mut work),
                Err(NativeValueError::StaleHandle)
            ));
            assert!(matches!(
                arena.put(physical as u32, &alias, &mut work),
                Err(NativeValueError::InvalidSlot)
            ));
            assert!(
                arena
                    .get(second, &mut work)
                    .unwrap()
                    .shares_frozen_storage(&alias)
            );
            arena.retire_current(slot, &mut work).unwrap();
            assert!(matches!(
                arena.get(second, &mut work),
                Err(NativeValueError::StaleHandle)
            ));
            let retry = arena.put(slot, &alias, &mut work).unwrap();
            assert!(
                arena
                    .get(retry, &mut work)
                    .unwrap()
                    .shares_frozen_storage(&alias)
            );
            drop((arena, value, alias, raw));
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
fn unsupported_worker_projection_categories_keep_full_arena_and_no_image_permission() {
    for artifact in artifacts(&source()) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let restored = super::super::tests::restore_admitted_helper_descriptor(&jit);
        for descriptor in [jit.descriptor(), &restored] {
            let metadata = descriptor.metadata();
            assert!(!metadata.parallel.regions.is_empty());
            assert!(metadata.worker_value_slots.iter().all(Option::is_none));
            let image = descriptor.encode_build_owned_image().unwrap();
            let mut wire: serde_json::Value = serde_json::from_slice(&image).unwrap();
            assert!(wire["descriptor"].get("worker_value_slots").is_none());
            wire["descriptor"]["worker_value_slots"] = serde_json::json!([0]);
            assert!(super::super::tests::rejects_malformed_helper_image(
                &serde_json::to_vec(&wire).unwrap()
            ));
        }
    }
}
