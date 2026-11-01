//! Weak identity retains allocation identity, never a canonical payload owner.
use super::*;
use crate::{SchemaDraft, SchemaTableBuilder};

fn string(text: &str, unrelated: bool) -> Value {
    let mut builder = SchemaTableBuilder::new();
    if unrelated {
        builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
    }
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
    let schemas = Arc::new(build.table);
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap()
}

#[test]
fn weak_frozen_identity_matches_clone_rebind_and_independent_import_only() {
    let raw = string("éACGT", false);
    let identity = raw.frozen_storage_identity();
    let clone = raw.clone();
    assert!(identity.matches(&raw));
    assert!(identity.matches(&clone));
    let equal = string("éACGT", false);
    assert!(!identity.matches(&equal));
    let target = string("", true);
    let target_schemas = target.schemas().unwrap();
    let rebound = raw
        .rebind_with_context(
            target.schema(),
            raw.shape(),
            &SnapshotValidationContext::with_shared_schemas(&target_schemas),
        )
        .unwrap();
    assert!(raw.shares_frozen_storage(&rebound));
    assert!(identity.matches(&rebound));
    let memory = crate::ManagedMemoryBudget::new(1_000_000);
    let mut reservation = memory
        .reserve_capacity(
            rebound.memory_budget_required_bytes().unwrap()
                + Value::memory_budget_claim_metadata_bytes(),
        )
        .unwrap();
    let imported = rebound
        .into_independent_memory_budget(&mut reservation)
        .unwrap();
    assert!(identity.matches(&imported));
    assert_eq!(format!("{identity:?}"), "FrozenSnapshotIdentity { .. }");
    drop((imported, reservation));
    assert_eq!(memory.used_bytes(), 0);
    assert!(identity.matches(&raw));
}

#[test]
fn weak_frozen_identity_releases_payload_and_budget_before_weak_shell_retirement() {
    let raw = string(&"ACGT".repeat(16_384), false);
    let identity = raw.frozen_storage_identity();
    let sibling = identity.clone();
    let old_address = identity.data.as_ptr();
    let memory = crate::ManagedMemoryBudget::new(1_000_000);
    let mut reservation = memory
        .reserve_capacity(
            raw.memory_budget_required_bytes().unwrap()
                + Value::memory_budget_claim_metadata_bytes(),
        )
        .unwrap();
    let imported = raw
        .into_independent_memory_budget(&mut reservation)
        .unwrap();
    drop(reservation);
    assert!(memory.used_bytes() >= 65_536);
    assert!(identity.matches(&imported));
    assert_eq!(identity.data.strong_count(), 1);
    drop(imported);
    assert_eq!(
        memory.used_bytes(),
        0,
        "weak identity must not retain the import claim"
    );
    assert_eq!(identity.data.strong_count(), 0);
    assert!(
        identity.data.upgrade().is_none(),
        "payload contents have been destroyed"
    );
    assert!(sibling.data.upgrade().is_none());
    // The weak allocation is still live, so a replacement allocation cannot
    // reuse its control block and produce an ABA match. Pointer inspection is
    // confined to this private test; the public identity exposes no pointer.
    for _ in 0..128 {
        let replacement = string(&"ACGT".repeat(16_384), false);
        assert_ne!(Arc::as_ptr(&replacement.root.data), old_address);
        assert!(!identity.matches(&replacement));
    }
    drop(identity);
    assert!(sibling.data.upgrade().is_none());
    drop(sibling); // Last weak owner releases the shell under Arc semantics.
}

#[test]
fn weak_frozen_identity_bound_includes_full_arc_shell_and_alignment() {
    let counters = 2 * core::mem::size_of::<usize>();
    let data_alignment = core::mem::align_of::<FrozenSnapshotData>();
    let allocation_alignment = data_alignment.max(core::mem::align_of::<usize>());
    let prefix = counters.div_ceil(data_alignment) * data_alignment;
    let expected = (prefix + core::mem::size_of::<FrozenSnapshotData>())
        .div_ceil(allocation_alignment)
        * allocation_alignment;
    let bound = Value::frozen_storage_identity_allocation_bytes();
    assert_eq!(bound, expected as u64);
    assert!(bound >= core::mem::size_of::<FrozenSnapshotData>() as u64);
    assert_eq!(bound % allocation_alignment as u64, 0);
}
