use super::*;
use mech_core::{
    ManagedMemoryBudget, SchemaBody, SchemaDraft, SchemaTableBuilder, ValueDataDraft, ValueDraft,
    snapshot::SnapshotValidationContext,
};

fn source(text: &str, memory: &ManagedMemoryBudget) -> Value {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::String,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::new(&build.table))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}
fn unlimited(_: u64, _: u64) -> Result<(), ResidentKernelError> {
    Ok(())
}

#[test]
fn weak_index_does_not_retain_source_payload_and_releases_all_index_backing() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
    let ledger = memory.used_bytes();
    let value = source(&"ACGT".repeat(16_384), &memory);
    let payload = memory.used_bytes() - ledger;
    assert!(payload >= 65_536);
    let slot = Slot::default();
    let index = slot.prepare(&value, &work, unlimited).unwrap();
    assert_eq!(index.len(), 65_536);
    assert!(matches!(
        &index.index.index,
        PreparedGraphemeIndex::UnitBoundaries { .. }
    ));
    assert_eq!(memory.used_bytes(), ledger + payload + slot.bytes());
    let held = slot.bytes();
    assert!(
        held < 4096,
        "unit-boundary metadata is independent of source length"
    );
    drop(index);
    drop(value);
    assert_eq!(
        memory.used_bytes(),
        ledger + held,
        "weak identity retained source payload or its budget claim"
    );
    drop(slot);
    assert_eq!(memory.used_bytes(), ledger);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn source_replacement_uses_exact_identity_and_never_overlaps_live_index_users() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
    let first = source("éA", &memory);
    let second = source("Aé", &memory);
    let equal_but_distinct = source("Aé", &memory);
    let baseline = memory.used_bytes();
    let slot = Slot::default();
    let old = slot.prepare(&first, &work, unlimited).unwrap();
    assert_eq!(old.byte_range(0), Some(0..2));
    let held = memory.used_bytes();
    assert!(slot.prepare(&second, &work, unlimited).is_err());
    assert_eq!(memory.used_bytes(), held);
    assert!(
        slot.entry
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .matches(&first)
    );
    drop(old);
    let current = slot.prepare(&second, &work, unlimited).unwrap();
    assert_eq!(current.byte_range(0), Some(0..1));
    assert_eq!(current.byte_range(1), Some(1..3));
    let hit = slot.prepare(&second, &work, unlimited).unwrap();
    assert!(Rc::ptr_eq(&hit.index, &current.index));
    drop((hit, current));
    let replaced = slot.prepare(&equal_but_distinct, &work, unlimited).unwrap();
    assert!(
        slot.entry
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .matches(&equal_but_distinct)
    );
    assert!(
        !slot
            .entry
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .matches(&second)
    );
    drop((replaced, slot));
    assert_eq!(memory.used_bytes(), baseline);
    drop((first, second, equal_but_distinct, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn index_work_and_offset_capacity_denials_are_prepaid_atomic_and_recover() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let large = source(&"A".repeat(65_536), &memory);
    let small = source("ACGT", &memory);
    let unicode = source("e\u{301}🧑‍🚀\r\n🇺🇸\0é", &memory);
    let sources = memory.used_bytes();
    let lookup = (core::mem::size_of::<Slot>() as u64) * 8 + 256;
    let header = Value::frozen_storage_identity_allocation_bytes()
        + (core::mem::size_of::<CachedIndex>() + 2 * core::mem::size_of::<usize>()) as u64;
    let work =
        NativeLibraryWorkBudget::new(lookup + header * 8 + 4096 + 65_536, 1_000_000, &memory)
            .unwrap();
    let baseline = memory.used_bytes();
    let slot = Slot::default();
    // The scan needs 65,537 units; its debit fails BEFORE segmentation.
    assert!(slot.prepare(&large, &work, unlimited).is_err());
    assert_eq!(slot.bytes(), 0);
    assert_eq!(slot.nodes(), 0);
    assert!(!slot.building.get());
    assert_eq!(memory.used_bytes(), baseline);
    let index = slot.prepare(&small, &work, unlimited).unwrap();
    assert_eq!(index.len(), 4);
    drop((index, slot, work));
    assert_eq!(memory.used_bytes(), sources);
    let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
    let baseline = memory.used_bytes();
    let slot = Slot::default();
    let mut calls = 0;
    assert!(
        slot.prepare(&unicode, &work, |bytes, _| {
            calls += 1;
            if bytes > header {
                Err(ResidentKernelError::InvalidShape)
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    assert_eq!(
        calls, 2,
        "header admitted, exact offsets denied before allocating them"
    );
    assert_eq!(slot.bytes(), 0);
    assert!(!slot.building.get());
    assert_eq!(memory.used_bytes(), baseline);
    let index = slot.prepare(&unicode, &work, unlimited).unwrap();
    let ValueData::String(raw) = unicode.data() else {
        panic!("String")
    };
    for (ordinal, expected) in ["e\u{301}", "🧑‍🚀", "\r\n", "🇺🇸", "\0", "é"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(&raw[index.byte_range(ordinal).unwrap()], expected);
    }
    assert_eq!(slot.nodes(), 3);
    drop((index, slot, large, small, unicode, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn unicode_offset_growth_unwind_retires_partial_backing_and_same_slot_recovers() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
    let value = source("éA\r\n", &memory);
    let baseline = memory.used_bytes();
    let slot = Slot::default();
    let header = Value::frozen_storage_identity_allocation_bytes()
        + (core::mem::size_of::<CachedIndex>() + 2 * core::mem::size_of::<usize>()) as u64;
    let mut calls = 0;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        slot.prepare(&value, &work, |bytes, _| {
            calls += 1;
            if bytes > header {
                panic!("controlled offset growth unwind");
            }
            Ok(())
        })
    }));
    assert!(result.is_err());
    assert_eq!(calls, 2);
    assert_eq!(slot.bytes(), 0);
    assert_eq!(slot.nodes(), 0);
    assert!(!slot.building.get());
    assert!(slot.entry.borrow().is_none());
    assert_eq!(memory.used_bytes(), baseline);
    let index = slot.prepare(&value, &work, unlimited).unwrap();
    assert_eq!(index.len(), 3);
    assert_eq!(index.byte_range(2), Some(3..5));
    drop((index, slot, value, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn synthetic_allocator_spare_capacity_is_rejected_before_index_publication() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
    let value = source("éA", &memory);
    let baseline = memory.used_bytes();
    let slot = Slot::default();
    let header = Value::frozen_storage_identity_allocation_bytes()
        + (core::mem::size_of::<CachedIndex>() + 2 * core::mem::size_of::<usize>()) as u64;
    slot.building.set(true);
    let mut pending = Pending {
        slot: &slot,
        reservation: Some(memory.reserve_capacity(0).unwrap()),
        nodes: Some(memory.reserve_nodes(0).unwrap()),
        complete: false,
    };
    let memory_error = |_| ResidentKernelError::InvalidShape;
    pending
        .grow(header, 2, &mut unlimited, &memory_error)
        .unwrap();
    let before = work.used();
    let result = PreparedGraphemeIndex::build_compact_with(
        "éA",
        |scan| work.charge(scan as u64, 0),
        |requested| {
            let offsets = requested as u64 * core::mem::size_of::<usize>() as u64;
            work.charge(offsets * 8 + 256, 0)?;
            pending.grow(header + offsets, 3, &mut unlimited, &memory_error)?;
            // Deterministic stand-in for an allocator returning spare capacity;
            // this does not depend on a particular platform allocator rounding.
            let data = Vec::<usize>::with_capacity(requested + 1);
            assert!(data.capacity() > requested);
            admitted_offsets(data, requested)
        },
        || ResidentKernelError::InvalidShape,
    );
    assert!(matches!(result, Err(ResidentKernelError::InvalidShape)));
    let charged = work.used();
    assert!(charged.0 > before.0);
    assert!(slot.entry.borrow().is_none());
    drop(pending);
    assert_eq!(slot.bytes(), 0);
    assert_eq!(slot.nodes(), 0);
    assert!(!slot.building.get());
    assert_eq!(memory.used_bytes(), baseline);
    assert_eq!(
        work.used(),
        charged,
        "cleanup cannot refund paid scan/allocation work"
    );
    let index = slot.prepare(&value, &work, unlimited).unwrap();
    assert_eq!(index.byte_range(0), Some(0..2));
    assert_eq!(index.byte_range(1), Some(2..3));
    assert!(
        slot.entry
            .borrow()
            .as_ref()
            .unwrap()
            .identity
            .matches(&value)
    );
    drop((index, slot, value, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn offset_guard_accepts_only_the_actual_admitted_reported_capacity() {
    let data = Vec::<usize>::with_capacity(4);
    let actual = data.capacity();
    let data = admitted_offsets(data, actual).unwrap();
    assert!(data.is_empty());
    assert_eq!(data.capacity(), actual);
    assert!(admitted_offsets(Vec::new(), 1).is_err());
}

#[test]
fn snapshot_index_nodes_share_one_ceiling_and_follow_outstanding_index_loans() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let value = source("ACGT", &memory);
    let baseline = memory.retained_nodes();
    let capped = memory.with_limits(memory.limit_bytes(), baseline + 2);
    let first_work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &capped).unwrap();
    let second_work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &capped).unwrap();
    let first = Slot::default();
    let loan = first.prepare(&value, &first_work, unlimited).unwrap();
    assert_eq!(first.nodes(), 2);
    assert_eq!(memory.retained_nodes(), baseline + 2);
    let second = Slot::default();
    let bytes = memory.used_bytes();
    assert!(second.prepare(&value, &second_work, unlimited).is_err());
    assert_eq!((second.bytes(), second.nodes()), (0, 0));
    assert_eq!(memory.used_bytes(), bytes);
    assert_eq!(memory.retained_nodes(), baseline + 2);
    drop(first);
    assert_eq!(
        memory.retained_nodes(),
        baseline + 2,
        "live index loan owns its claim"
    );
    assert!(second.prepare(&value, &second_work, unlimited).is_err());
    drop(loan);
    assert_eq!(memory.retained_nodes(), baseline);
    let loan = second.prepare(&value, &second_work, unlimited).unwrap();
    assert_eq!(memory.retained_nodes(), baseline + 2);
    drop((loan, second, value, first_work, second_work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn snapshot_offset_node_growth_denial_retires_pending_header_claim() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let unicode = source("éA", &memory);
    let baseline = memory.retained_nodes();
    let capped = memory.with_limits(memory.limit_bytes(), baseline + 2);
    let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &capped).unwrap();
    let bytes = memory.used_bytes();
    let slot = Slot::default();
    let mut calls = 0;
    assert!(
        slot.prepare(&unicode, &work, |_, nodes| {
            calls += 1;
            assert!(nodes == 2 || nodes == 3);
            Ok(())
        })
        .is_err()
    );
    assert_eq!(
        calls, 2,
        "header succeeds, shared offset-node admission fails"
    );
    assert_eq!((slot.bytes(), slot.nodes()), (0, 0));
    assert!(!slot.building.get());
    assert!(slot.entry.borrow().is_none());
    assert_eq!(
        (memory.used_bytes(), memory.retained_nodes()),
        (bytes, baseline)
    );
    drop((slot, unicode, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}
