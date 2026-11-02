//! Exact native import admission uses the production String observation helper.
use crate::native_artifact::program::runtime::session::*;
use mech_core::{SchemaBody, SchemaDraft, SchemaTable, SchemaTableBuilder};
use std::sync::Arc;

fn fixture() -> (
    Value,
    mech_core::SchemaId,
    Arc<SchemaTable>,
    ManagedMemoryBudget,
) {
    let schema = |body| {
        SchemaDraft {
            body,
            dimension_parameters: Box::new([]),
        }
        .finalize()
        .unwrap()
    };
    let mut source = SchemaTableBuilder::new();
    let handle = source.insert(schema(SchemaBody::String)).unwrap();
    source
        .insert(schema(SchemaBody::Tuple(
            vec![SchemaBody::Bool; 2_048].into_boxed_slice(),
        )))
        .unwrap();
    let source = source.finish().unwrap();
    let source_id = source.resolve(handle).unwrap();
    let source = Arc::new(source.table);
    let raw = ValueDraft {
        schema: source_id,
        shape_values: Box::new([]),
        data: ValueDataDraft::String("éACGT".repeat(16_384)),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&source))
    .unwrap();
    let provider = ManagedMemoryBudget::new(4 * 1024 * 1024);
    let mut reservation = provider
        .reserve_capacity(
            raw.memory_budget_required_bytes().unwrap()
                + Value::memory_budget_claim_metadata_bytes(),
        )
        .unwrap();
    let raw = raw
        .into_independent_memory_budget(&mut reservation)
        .unwrap();
    drop(reservation);
    let mut target = SchemaTableBuilder::new();
    target.insert(schema(SchemaBody::Bool)).unwrap();
    let handle = target.insert(schema(SchemaBody::String)).unwrap();
    let target = target.finish().unwrap();
    let target_id = target.resolve(handle).unwrap();
    (raw, target_id, Arc::new(target.table), provider)
}

// Derive the test boundary through the independent finalized-value witness,
// rather than repeating the prospective-String formula used by production.
fn envelope(raw: &Value, target: &SchemaTable) -> u64 {
    raw.memory_budget_required_bytes().unwrap()
        + target.clone_allocation_bound_bytes().unwrap()
        + SchemaTable::clone_metadata_allocation_bound_bytes()
        - core::mem::size_of::<SchemaTable>() as u64
        + Value::memory_budget_claim_metadata_bytes()
}

#[test]
fn larger_string_exact_envelope_denial_success_and_release_preserve_provider_owner() {
    let (raw, schema, target, provider) = fixture();
    let provider_before = provider.used_bytes();
    let bound = envelope(&raw, &target);
    let memory = ManagedMemoryBudget::new(bound);
    let ballast = memory.reserve_capacity(1).unwrap();
    memory.inject_snapshot_import_failure_after(0);
    let mut work = NativeWorkMeter::new(1_000_000_000).unwrap();
    work.bind_memory_budget(&memory).unwrap();
    let import = |work: &mut NativeWorkMeter| {
        import_string_observation(
            &raw,
            schema,
            &target,
            target.clone_allocation_bound_bytes().unwrap(),
            &memory,
            work,
        )
    };
    assert!(matches!(
        import(&mut work),
        Err(NativeProgramExecutionError::Base(
            NativeExecutionError::Memory {
                error: mech_core::MemoryRuntimeError::BudgetExceeded { .. },
                ..
            }
        ))
    ));
    assert_eq!(memory.used_bytes(), 1);
    drop(ballast);
    // Capacity denial must occur before registration allocation. Its one-shot
    // failure is still pending at the exact successful capacity boundary.
    assert!(matches!(
        import(&mut work),
        Err(NativeProgramExecutionError::Base(
            NativeExecutionError::Memory {
                error: mech_core::MemoryRuntimeError::AllocationFailed { .. },
                ..
            }
        ))
    ));
    assert_eq!(memory.used_bytes(), 0);
    for _ in 0..3 {
        let before_work = work.used();
        let (owned, tail) = import(&mut work).unwrap();
        assert_eq!(work.used() - before_work, bound * 8 + 128);
        assert_eq!(memory.used_bytes(), bound);
        assert!(owned.shares_frozen_storage(&raw));
        assert!(owned.memory_budget_retained_bytes(&memory).is_some());
        assert!(Arc::ptr_eq(&owned.schemas().unwrap(), &target));
        let retained = bound - tail.capacity_bytes();
        drop(tail);
        assert_eq!(memory.used_bytes(), retained);
        assert!(retained >= owned.memory_budget_required_bytes().unwrap());
        drop(owned);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(provider.used_bytes(), provider_before);
        let ValueData::String(text) = raw.data() else {
            panic!("String");
        };
        assert_eq!(text.as_ref(), "éACGT".repeat(16_384));
    }
    drop(raw);
    assert_eq!(provider.used_bytes(), 0);
}

#[test]
fn string_import_work_boundary_and_both_allocation_failures_release_every_claim() {
    let (raw, schema, target, _) = fixture();
    let bound = envelope(&raw, &target);
    let debit = bound * 8 + 128;
    let target_bytes = target.clone_allocation_bound_bytes().unwrap();
    let memory = ManagedMemoryBudget::new(bound);
    memory.inject_snapshot_import_failure_after(0);
    let mut short = NativeWorkMeter::new(debit - 1).unwrap();
    short.bind_memory_budget(&memory).unwrap();
    assert!(matches!(
        import_string_observation(&raw, schema, &target, target_bytes, &memory, &mut short),
        Err(NativeProgramExecutionError::Base(
            NativeExecutionError::WorkLimit { .. }
        )),
    ));
    assert_eq!(memory.used_bytes(), 0);
    // The failed work admission must not consume the pending allocation fault.
    let mut exact = NativeWorkMeter::new(debit).unwrap();
    exact.bind_memory_budget(&memory).unwrap();
    assert!(matches!(
        import_string_observation(&raw, schema, &target, target_bytes, &memory, &mut exact),
        Err(NativeProgramExecutionError::Base(
            NativeExecutionError::Memory {
                error: mech_core::MemoryRuntimeError::AllocationFailed { .. },
                ..
            }
        )),
    ));
    assert_eq!(exact.used(), debit);
    assert_eq!(memory.used_bytes(), 0);
    for successful_allocations in [0, 1] {
        memory.inject_snapshot_import_failure_after(successful_allocations);
        let mut work = NativeWorkMeter::new(debit).unwrap();
        work.bind_memory_budget(&memory).unwrap();
        assert!(matches!(
            import_string_observation(&raw, schema, &target, target_bytes, &memory, &mut work),
            Err(NativeProgramExecutionError::Base(
                NativeExecutionError::Memory {
                    error: mech_core::MemoryRuntimeError::AllocationFailed { .. },
                    ..
                }
            )),
        ));
        assert_eq!(work.used(), debit);
        assert_eq!(memory.used_bytes(), 0);
    }
    let mut exact = NativeWorkMeter::new(debit).unwrap();
    exact.bind_memory_budget(&memory).unwrap();
    let (owned, tail) =
        import_string_observation(&raw, schema, &target, target_bytes, &memory, &mut exact)
            .unwrap();
    assert_eq!(exact.used(), debit);
    drop((owned, tail));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn string_import_rejects_wrong_account_and_wrong_target_without_materializing() {
    let (raw, schema, target, _) = fixture();
    let bound = envelope(&raw, &target);
    let memory = ManagedMemoryBudget::new(bound);
    let foreign = ManagedMemoryBudget::new(bound);
    let mut work = NativeWorkMeter::new(1_000_000_000).unwrap();
    work.bind_memory_budget(&foreign).unwrap();
    assert!(matches!(
        import_string_observation(
            &raw,
            schema,
            &target,
            target.clone_allocation_bound_bytes().unwrap(),
            &memory,
            &mut work
        ),
        Err(NativeProgramExecutionError::Base(
            NativeExecutionError::Descriptor { .. }
        )),
    ));
    assert_eq!(work.used(), 0);
    assert_eq!(memory.used_bytes(), 0);
    let bool_id = target
        .entries()
        .enumerate()
        .find_map(|(index, entry)| {
            (entry.schema().body() == &SchemaBody::Bool)
                .then_some(mech_core::SchemaId::new(index as u32))
        })
        .unwrap();
    let mut work = NativeWorkMeter::new(1_000_000_000).unwrap();
    work.bind_memory_budget(&memory).unwrap();
    assert!(
        import_string_observation(
            &raw,
            bool_id,
            &target,
            target.clone_allocation_bound_bytes().unwrap(),
            &memory,
            &mut work
        )
        .is_err()
    );
    assert_eq!(work.used(), 0);
    assert_eq!(memory.used_bytes(), 0);
}
