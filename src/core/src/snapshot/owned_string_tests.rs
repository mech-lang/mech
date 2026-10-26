use super::*;
use crate::{ManagedMemoryBudget, SchemaDraft, SchemaTableBuilder};

fn string(text: &str) -> Value {
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
    let built = builder.finish().unwrap();
    let schema = built.resolve(handle).unwrap();
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text.into()),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&Arc::new(
        built.table,
    )))
    .unwrap()
}

fn admit(value: Value, budget: &ManagedMemoryBudget) -> Value {
    let mut capacity = budget
        .reserve_capacity(value.memory_budget_admission_bytes(budget).unwrap())
        .unwrap();
    let value = value.into_memory_budget(&mut capacity).unwrap();
    drop(capacity);
    value
}

fn unavailable(result: StringOwnershipTransfer) -> Value {
    match result {
        StringOwnershipTransfer::Unavailable(value) => value,
        StringOwnershipTransfer::Owned(_) => panic!("shared/unadmitted String was consumed"),
    }
}

#[test]
fn exclusive_strings_transfer_without_payload_copy_or_new_budget_charge() {
    for text in ["", "acgt", "éA\0🧑‍🚀", &"aCGt".repeat(16_384)] {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let value = admit(string(text), &budget);
        let pointer = match value.data() {
            ValueData::String(text) => text.as_ptr(),
            _ => unreachable!(),
        };
        let schema = value.schema();
        let key = value.schema_key();
        let identity = value.frozen_storage_identity();
        let charged = budget.used_bytes();
        let StringOwnershipTransfer::Owned(owned) = value.try_into_owned_string(&budget).unwrap()
        else {
            panic!("exclusive String could not transfer");
        };
        assert_eq!(owned.as_str(), text);
        assert_eq!(owned.as_str().as_ptr(), pointer);
        assert_eq!(owned.schema(), schema);
        assert_eq!(owned.schema_key(), key);
        assert_eq!(owned.memory_budget(), budget);
        assert_eq!(owned.retained_capacity_bytes(), charged);
        assert_eq!(budget.used_bytes(), charged);
        assert_eq!(identity.data.strong_count(), 0);
        drop(owned);
        assert_eq!(budget.used_bytes(), 0);
    }
}

#[test]
fn refreezing_reused_string_backing_cannot_revive_old_frozen_identity() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let original = admit(string("acgt"), &budget);
    let schema = original.schema();
    let schemas = original.schemas().unwrap();
    let identity = original.frozen_storage_identity();
    let pointer = match original.data() {
        ValueData::String(text) => text.as_ptr(),
        _ => unreachable!(),
    };
    let charged = budget.used_bytes();
    let StringOwnershipTransfer::Owned(owned) = original.try_into_owned_string(&budget).unwrap()
    else {
        panic!()
    };
    let (mut text, mut reservation) = owned.into_parts();
    text.make_ascii_uppercase();
    assert_eq!(text.as_ptr(), pointer);
    let output = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap()
    .into_memory_budget(&mut reservation)
    .unwrap();
    assert!(!identity.matches(&output));
    assert!(
        matches!(output.data(),ValueData::String(text) if text.as_ref()=="ACGT" && text.as_ptr()==pointer)
    );
    assert_eq!(budget.used_bytes(), charged);
    drop((output, reservation));
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn shared_wrapper_rejection_preserves_owner_identity_admission_and_import_cache() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let value = admit(string("untouched"), &budget);
    let alias = value.clone();
    let identity = value.frozen_storage_identity();
    let charged = budget.used_bytes();
    let value = unavailable(value.try_into_owned_string(&budget).unwrap());
    assert!(identity.matches(&value));
    assert!(value.shares_frozen_storage(&alias));
    assert!(value.memory_budget_retained_bytes(&budget).is_some());
    assert_eq!(budget.used_bytes(), charged);
    let mut none = budget.reserve_capacity(0).unwrap();
    let retained = value.clone().into_memory_budget(&mut none).unwrap();
    assert!(Arc::ptr_eq(&value.root, &retained.root));
    drop((value, alias, retained, none));
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn foreign_import_blocks_data_transfer_until_that_owner_retires() {
    let first = ManagedMemoryBudget::new(1_000_000);
    let second = ManagedMemoryBudget::new(1_000_000);
    let value = admit(string("shared data"), &first);
    let imported = admit(value.clone(), &second);
    assert!(!Arc::ptr_eq(&value.root, &imported.root));
    assert!(value.shares_frozen_storage(&imported));
    let identity = value.frozen_storage_identity();
    let charged = first.used_bytes();
    let value = unavailable(value.try_into_owned_string(&first).unwrap());
    assert!(identity.matches(&value));
    assert!(identity.matches(&imported));
    assert_eq!(first.used_bytes(), charged);
    assert!(second.used_bytes() > 0);
    drop(imported);
    assert_eq!(second.used_bytes(), 0);
    let StringOwnershipTransfer::Owned(owned) = value.try_into_owned_string(&first).unwrap() else {
        panic!()
    };
    assert_eq!(owned.as_str(), "shared data");
    assert_eq!(first.used_bytes(), charged);
    assert_eq!(identity.data.strong_count(), 0);
    drop(owned);
    assert_eq!(first.used_bytes(), 0);
}

#[test]
fn unadmitted_and_wrong_account_transfers_do_not_rehome_capacity() {
    let first = ManagedMemoryBudget::new(1_000_000);
    let other = ManagedMemoryBudget::new(1_000_000);
    let raw = string("unadmitted");
    let identity = raw.frozen_storage_identity();
    let raw = unavailable(raw.try_into_owned_string(&first).unwrap());
    assert!(identity.matches(&raw));
    assert_eq!(first.used_bytes(), 0);
    let value = admit(raw, &first);
    let charged = first.used_bytes();
    let value = unavailable(value.try_into_owned_string(&other).unwrap());
    assert!(identity.matches(&value));
    assert!(value.memory_budget_retained_bytes(&first).is_some());
    assert!(value.memory_budget_retained_bytes(&other).is_none());
    assert_eq!(first.used_bytes(), charged);
    assert_eq!(other.used_bytes(), 0);
    drop(value);
    assert_eq!(first.used_bytes(), 0);
}

#[test]
fn mutable_bytes_keep_the_original_charge_and_require_utf8_revalidation() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let value = admit(string("ascii"), &budget);
    let identity = value.frozen_storage_identity();
    let key = value.schema_key();
    let pointer = match value.data() {
        ValueData::String(text) => text.as_ptr(),
        _ => unreachable!(),
    };
    let charged = budget.used_bytes();
    let StringOwnershipTransfer::Owned(owned) = value.try_into_owned_string(&budget).unwrap()
    else {
        panic!()
    };
    let mut bytes = owned.into_bytes();
    assert_eq!(bytes.as_bytes().as_ptr(), pointer);
    assert_eq!(bytes.schema_key(), key);
    assert_eq!(bytes.memory_budget(), budget);
    assert_eq!(bytes.retained_capacity_bytes(), charged);
    assert_eq!(identity.data.strong_count(), 0);
    bytes.as_mut_bytes()[0] = 0xff;
    assert!(core::str::from_utf8(bytes.as_bytes()).is_err());
    assert_eq!(budget.used_bytes(), charged);
    let (bytes, reservation) = bytes.into_parts();
    let invalid = String::from_utf8(bytes).unwrap_err();
    assert_eq!(budget.used_bytes(), charged);
    drop(invalid); // Failed validation retires backing before its capacity claim.
    drop(reservation);
    assert_eq!(budget.used_bytes(), 0);
}

#[cfg(all(feature = "functions", feature = "string"))]
#[test]
fn an_exclusive_physical_payload_ticket_is_not_consumed() {
    let domain = crate::MemoryDomain::new().unwrap();
    let cell = crate::ValueCell::from_exact_in(&domain, "physical owner".to_owned()).unwrap();
    let value = cell.snapshot().unwrap();
    drop(cell);
    domain.close().unwrap();
    drop(domain);
    let budget = ManagedMemoryBudget::new(1_000_000);
    let value = admit(value, &budget);
    assert!(value.has_retained_payload_ticket());
    assert_eq!(Arc::strong_count(&value.root), 1);
    assert_eq!(Arc::strong_count(&value.root.data), 1);
    let identity = value.frozen_storage_identity();
    let charged = budget.used_bytes();
    let value = unavailable(value.try_into_owned_string(&budget).unwrap());
    assert!(identity.matches(&value));
    assert!(value.has_retained_payload_ticket());
    assert!(matches!(value.data(), ValueData::String(text) if text.as_ref() == "physical owner"));
    assert_eq!(budget.used_bytes(), charged);
    drop(value);
    assert_eq!(budget.used_bytes(), 0);
}
