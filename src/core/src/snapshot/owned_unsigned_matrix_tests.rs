use super::*;
use crate::snapshot::{SequenceView, ValueDataDraft};
use crate::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, SchemaDraft, SchemaTableBuilder,
};

fn schema(width: IntegerWidth) -> (SchemaId, Arc<SchemaTable>) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(width)),
                    dimensions: [
                        DimensionExpr::Constant(1),
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    ]
                    .into(),
                },
                dimension_parameters: [DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Activation,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                }]
                .into(),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    (build.resolve(handle).unwrap(), Arc::new(build.table))
}
fn shape(schema: SchemaId, table: &SchemaTable, count: usize) -> ShapeInstance {
    table
        .get(schema)
        .unwrap()
        .instantiate_shape([count as u64].into())
        .unwrap()
}
fn admit(value: Value, budget: &crate::ManagedMemoryBudget) -> Value {
    let mut reservation = budget
        .reserve_capacity(value.memory_budget_admission_bytes(budget).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}
fn u64_value(capacity: usize) -> Value {
    let (id, table) = schema(IntegerWidth::W64);
    let mut words = Vec::with_capacity(capacity);
    words.extend([0x80000000, 17, u64::MAX]);
    Value::from_unsigned_matrix_buffer(
        id,
        shape(id, &table, 3),
        UnsignedMatrixBuffer::U64(words),
        table,
    )
    .unwrap()
}
fn unavailable(result: UnsignedMatrixOwnershipTransfer) -> Value {
    match result {
        UnsignedMatrixOwnershipTransfer::Unavailable(value) => value,
        UnsignedMatrixOwnershipTransfer::Owned(_) => {
            panic!("shared or unadmitted matrix was consumed")
        }
    }
}

#[test]
fn reserved_words_preserve_capacity_and_charge_it_but_encode_only_initialized_elements() {
    macro_rules! check {
        ($variant:ident, $width:ident, $word:expr) => {{
            let (id, table) = schema(IntegerWidth::$width);
            for count in [0, 3] {
                let mut words = Vec::with_capacity(305257);
                words.resize(count, $word);
                let capacity = words.capacity();
                let pointer = words.as_ptr();
                let reserved = Value::from_unsigned_matrix_buffer(
                    id,
                    shape(id, &table, count),
                    UnsignedMatrixBuffer::$variant(words),
                    table.clone(),
                )
                .unwrap();
                let ordinary = ValueDraft {
                    schema: id,
                    shape_values: [count as u64].into(),
                    data: ValueDataDraft::Matrix(
                        vec![ValueDataDraft::$variant($word); count].into(),
                    ),
                }
                .finalize(&SnapshotValidationContext::with_shared_schemas(&table))
                .unwrap();
                assert_eq!(
                    reserved.canonical_data_draft().unwrap(),
                    ordinary.canonical_data_draft().unwrap()
                );
                assert_eq!(
                    reserved.value_hash(&table).unwrap(),
                    ordinary.value_hash(&table).unwrap()
                );
                assert_eq!(reserved.resident_token(), ordinary.resident_token());
                assert!(reserved.language_eq(&table, &ordinary, &table).unwrap());
                let footprint = reserved.retained_footprint(&table).unwrap();
                let plain = ordinary.retained_footprint(&table).unwrap();
                assert_eq!(footprint.encoded_bytes, plain.encoded_bytes);
                assert_eq!(
                    reserved.canonical_snapshot_bytes(&table).unwrap(),
                    ordinary.canonical_snapshot_bytes(&table).unwrap()
                );
                let mut retained = 0;
                crate::snapshot::visit_canonical_data_work(
                    table.get(id).unwrap().body(),
                    reserved.data(),
                    |work| {
                        retained += work.retained_bytes;
                        Ok::<_, ()>(())
                    },
                )
                .unwrap();
                let data = crate::snapshot::canonical_data_retained_footprint(
                    table.get(id).unwrap().body(),
                    reserved.data(),
                )
                .unwrap();
                assert_eq!(retained, data.retained_bytes);
                assert_eq!(
                    footprint.retained_bytes - plain.retained_bytes,
                    ((capacity - count) * core::mem::size_of_val(&$word)) as u64
                );
                let ValueData::Matrix(matrix) = reserved.data() else {
                    panic!()
                };
                let SequenceView::$variant(elements) = matrix.elements() else {
                    panic!()
                };
                assert_eq!(elements.as_ptr(), pointer);
                let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
                let reserved = admit(reserved, &budget);
                assert!(budget.used_bytes() >= (capacity * core::mem::size_of_val(&$word)) as u64);
                let identity = reserved.frozen_storage_identity();
                let used = budget.used_bytes();
                let UnsignedMatrixOwnershipTransfer::Owned(owned) =
                    reserved.try_into_owned_unsigned_matrix(&budget).unwrap()
                else {
                    panic!()
                };
                assert_eq!(
                    owned.capacity_bytes(),
                    capacity * core::mem::size_of_val(&$word)
                );
                let SequenceView::$variant(elements) = owned.elements() else {
                    panic!()
                };
                assert_eq!(elements.as_ptr(), pointer);
                assert_eq!(budget.used_bytes(), used);
                assert_eq!(identity.data.strong_count(), 0);
                assert_eq!(budget.retained_snapshot_nodes(), 0);
                drop(owned);
                assert_eq!(budget.used_bytes(), 0);
            }
        }};
    }
    check!(U8, W8, 129u8);
    check!(U16, W16, 32769u16);
    check!(U32, W32, 0x80000000u32);
    check!(U64, W64, 0x8000000000000000u64);
}

#[test]
fn path_nodes_clear_append_and_refreeze_reuse_the_actual_305257_slot_allocation() {
    let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let mut value = admit(u64_value(305257), &budget);
    let id = value.schema();
    let schemas = value.schemas().unwrap();
    let pointer = match value.data() {
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::U64(words) => words.as_ptr(),
            _ => panic!(),
        },
        _ => panic!(),
    };
    let charged = budget.used_bytes();
    for iteration in 0..3 {
        let old = value.frozen_storage_identity();
        let UnsignedMatrixOwnershipTransfer::Owned(owned) =
            value.try_into_owned_unsigned_matrix(&budget).unwrap()
        else {
            panic!()
        };
        let (buffer, mut reservation) = owned.into_parts();
        let UnsignedMatrixBuffer::U64(mut words) = buffer else {
            panic!()
        };
        assert_eq!(words.capacity(), 305257);
        words.clear();
        words.push(0x80000000);
        words.push(iteration);
        assert_eq!(words.as_ptr(), pointer);
        value = Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &schemas, 2),
            UnsignedMatrixBuffer::U64(words),
            schemas.clone(),
        )
        .unwrap()
        .into_memory_budget(&mut reservation)
        .unwrap();
        drop(reservation);
        assert!(!old.matches(&value));
        assert_eq!(budget.used_bytes(), charged);
        assert_eq!(budget.retained_snapshot_nodes(), 3);
        assert_eq!(
            value.canonical_data_draft().unwrap(),
            ValueDataDraft::Matrix(
                [
                    ValueDataDraft::U64(0x80000000),
                    ValueDataDraft::U64(iteration)
                ]
                .into()
            )
        );
    }
    drop(value);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn aliases_and_cross_account_imports_block_consumption_then_allow_recovery() {
    let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let other = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let value = admit(u64_value(305257), &budget);
    let alias = value.clone();
    let imported = admit(value.clone(), &other);
    let identity = value.frozen_storage_identity();
    let charged = budget.used_bytes();
    let value = unavailable(value.try_into_owned_unsigned_matrix(&budget).unwrap());
    assert!(identity.matches(&value));
    assert!(value.shares_frozen_storage(&alias));
    drop(alias);
    let value = unavailable(value.try_into_owned_unsigned_matrix(&budget).unwrap());
    assert!(value.shares_frozen_storage(&imported));
    assert_eq!(budget.used_bytes(), charged);
    drop(imported);
    assert_eq!(other.used_bytes(), 0);
    let UnsignedMatrixOwnershipTransfer::Owned(owned) =
        value.try_into_owned_unsigned_matrix(&budget).unwrap()
    else {
        panic!()
    };
    assert_eq!(owned.capacity_bytes(), 305257 * 8);
    drop(owned);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn wrong_account_and_failed_capacity_admission_preserve_the_previous_publication() {
    let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let wrong = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let value = u64_value(305257);
    let value = unavailable(value.try_into_owned_unsigned_matrix(&budget).unwrap());
    let mut value = admit(value, &budget);
    let old = value.clone();
    let identity = value.frozen_storage_identity();
    let charged = budget.used_bytes();
    value = unavailable(value.try_into_owned_unsigned_matrix(&wrong).unwrap());
    assert!(identity.matches(&value));
    let denied = crate::ManagedMemoryBudget::new(1);
    assert!(
        denied
            .reserve_capacity(value.memory_budget_admission_bytes(&denied).unwrap())
            .is_err()
    );
    assert_eq!(denied.used_bytes(), 0);
    assert_eq!(budget.used_bytes(), charged);
    assert!(value.shares_frozen_storage(&old));
    drop(old);
    let UnsignedMatrixOwnershipTransfer::Owned(owned) =
        value.try_into_owned_unsigned_matrix(&budget).unwrap()
    else {
        panic!()
    };
    drop(owned);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn output_bytes_reuse_the_original_input_sized_reservation_across_repetitions() {
    let input_size = 6627202usize;
    let reserve = input_size + 5 * input_size.div_ceil(5000).max(1);
    let (id, schemas) = schema(IntegerWidth::W8);
    let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let words = Vec::<u8>::with_capacity(reserve);
    let pointer = words.as_ptr();
    let mut value = admit(
        Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &schemas, 0),
            UnsignedMatrixBuffer::U8(words),
            schemas.clone(),
        )
        .unwrap(),
        &budget,
    );
    let charged = budget.used_bytes();
    for _ in 0..3 {
        let UnsignedMatrixOwnershipTransfer::Owned(owned) =
            value.try_into_owned_unsigned_matrix(&budget).unwrap()
        else {
            panic!()
        };
        let (buffer, mut charge) = owned.into_parts();
        let UnsignedMatrixBuffer::U8(mut words) = buffer else {
            panic!()
        };
        assert_eq!(words.capacity(), reserve);
        words.clear();
        words.extend([0x01, 0xff, 0x80]);
        assert_eq!(words.as_ptr(), pointer);
        value = Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &schemas, 3),
            UnsignedMatrixBuffer::U8(words),
            schemas.clone(),
        )
        .unwrap()
        .into_memory_budget(&mut charge)
        .unwrap();
        drop(charge);
        assert_eq!(budget.used_bytes(), charged);
    }
    drop(value);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn reserved_buffer_construction_rejects_wrong_element_kinds_and_logical_extents() {
    let (id, schemas) = schema(IntegerWidth::W64);
    assert!(
        Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &schemas, 0),
            UnsignedMatrixBuffer::U8(Vec::with_capacity(305257)),
            schemas.clone()
        )
        .is_err()
    );
    assert!(
        Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &schemas, 1),
            UnsignedMatrixBuffer::U64(Vec::with_capacity(305257)),
            schemas.clone()
        )
        .is_err()
    );
    let mut words = Vec::with_capacity(305257);
    words.push(9u64);
    assert!(
        Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &schemas, 0),
            UnsignedMatrixBuffer::U64(words),
            schemas.clone()
        )
        .is_err()
    );
}

#[test]
fn private_fixed_shape_updates_publish_fresh_identity_without_new_admission() {
    macro_rules! check {
        ($variant:ident, $width:ident, $count:expr, $word:expr) => {{
            let (id, table) = schema(IntegerWidth::$width);
            let count = $count;
            let mut words = Vec::with_capacity(count + 11);
            words.resize(count, $word);
            let pointer = words.as_ptr();
            let geometry = shape(id, &table, count);
            let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
            let value = admit(
                Value::from_unsigned_matrix_buffer(
                    id,
                    geometry.clone(),
                    UnsignedMatrixBuffer::$variant(words),
                    table.clone(),
                )
                .unwrap(),
                &budget,
            );
            let old_identity = value.frozen_storage_identity();
            let old_token = value.resident_token();
            let old_charge = budget.used_bytes();
            let UnsignedMatrixOwnershipTransfer::Owned(owned) =
                value.try_into_owned_unsigned_matrix(&budget).unwrap()
            else {
                panic!()
            };
            let mut private = owned.into_private_updates(geometry).unwrap();
            assert_eq!(budget.used_bytes(), old_charge);
            let UnsignedMatrixViewMut::$variant(words) = private.elements_mut() else {
                panic!()
            };
            assert_eq!(words.as_ptr(), pointer);
            words[count - 1] = $word + 1;
            // Freeze must work even when no new byte claim can be admitted and
            // the next fallible snapshot-import allocation is armed to fail.
            let rest = budget
                .reserve_capacity(budget.limit_bytes() - budget.used_bytes())
                .unwrap();
            budget.inject_snapshot_import_failure_after(0);
            let frozen = private.finish();
            assert!(!old_identity.matches(&frozen));
            assert_eq!(old_identity.data.strong_count(), 0);
            assert_ne!(frozen.resident_token(), old_token);
            let ValueData::Matrix(matrix) = frozen.data() else {
                panic!()
            };
            let SequenceView::$variant(words) = matrix.elements() else {
                panic!()
            };
            assert_eq!(words.as_ptr(), pointer);
            assert_eq!(words[count - 1], $word + 1);
            assert!(
                budget.check_snapshot_import_allocation(1, 1).is_err(),
                "finish consumed the allocation-failure sentinel"
            );
            drop(rest);
            assert_eq!(budget.used_bytes(), old_charge);
            // Subsequent ordinary imports/aliases remain immutable and retain
            // this publication; a new exclusive phase waits for their release.
            let alias = frozen.clone();
            let frozen = unavailable(frozen.try_into_owned_unsigned_matrix(&budget).unwrap());
            assert!(frozen.shares_frozen_storage(&alias));
            drop(alias);
            let UnsignedMatrixOwnershipTransfer::Owned(owned) =
                frozen.try_into_owned_unsigned_matrix(&budget).unwrap()
            else {
                panic!()
            };
            drop(
                owned
                    .into_private_updates(shape(id, &table, count))
                    .unwrap(),
            );
            assert_eq!(budget.used_bytes(), 0);
            assert_eq!(budget.retained_snapshot_nodes(), 0);
        }};
    }
    check!(U8, W8, 3, 129u8);
    check!(U16, W16, 262144, 32769u16);
    check!(U32, W32, 1500513, 0x80000000u32);
    check!(U64, W64, 3, 0x8000000000000000u64);
}

#[test]
fn private_update_preparation_rejects_geometry_and_allocation_without_leaking() {
    for wrong_geometry in [true, false] {
        let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
        let value = admit(u64_value(305257), &budget);
        let table = value.schemas().unwrap();
        let id = value.schema();
        let UnsignedMatrixOwnershipTransfer::Owned(owned) =
            value.try_into_owned_unsigned_matrix(&budget).unwrap()
        else {
            panic!()
        };
        if !wrong_geometry {
            budget.inject_snapshot_import_failure_after(0);
        }
        assert!(
            owned
                .into_private_updates(shape(id, &table, if wrong_geometry { 4 } else { 3 }))
                .is_err()
        );
        assert_eq!(budget.used_bytes(), 0);
        assert_eq!(budget.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn private_reserved_appends_keep_backing_shape_charge_and_infallible_finish() {
    macro_rules! check {
        ($variant:ident, $width:ident, $capacity:expr, $first:expr, $last:expr) => {{
            let (id, table) = schema(IntegerWidth::$width);
            let words = Vec::with_capacity($capacity);
            let pointer = words.as_ptr();
            let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
            let value = admit(
                Value::from_unsigned_matrix_buffer(
                    id,
                    shape(id, &table, 0),
                    UnsignedMatrixBuffer::$variant(words),
                    table.clone(),
                )
                .unwrap(),
                &budget,
            );
            let old_identity = value.frozen_storage_identity();
            let old_charge = budget.used_bytes();
            let preparation = value
                .prepare_private_unsigned_appends(&budget)
                .unwrap()
                .unwrap();
            assert!(old_identity.matches(&value));
            let UnsignedMatrixOwnershipTransfer::Owned(owned) =
                value.try_into_owned_unsigned_matrix(&budget).unwrap()
            else {
                panic!()
            };
            let mut private = owned
                .into_private_appends(shape(id, &table, 0), preparation)
                .unwrap();
            assert_eq!(private.capacity_elements(), $capacity);
            assert_eq!(private.elements().len(), 0);
            assert!(private.append_in_capacity($first));
            assert!(private.append_in_capacity($last));
            let SequenceView::$variant(cells) = private.elements() else {
                panic!()
            };
            assert_eq!(cells.as_ptr(), pointer);
            assert_eq!(cells.len(), 2);
            assert_eq!(cells[0] as u64, $first);
            assert_eq!(cells[1] as u64, $last);
            let rest = budget
                .reserve_capacity(budget.limit_bytes() - budget.used_bytes())
                .unwrap();
            budget.inject_snapshot_import_failure_after(0);
            let frozen = private.finish();
            assert!(!old_identity.matches(&frozen));
            assert_eq!(frozen.shape().parameter_values(), &[2]);
            frozen.validate_against(&table).unwrap();
            let ValueData::Matrix(matrix) = frozen.data() else {
                panic!()
            };
            let SequenceView::$variant(cells) = matrix.elements() else {
                panic!()
            };
            assert_eq!(cells.as_ptr(), pointer);
            assert_eq!(matrix.unsigned_buffer_capacity(), Some($capacity));
            assert!(
                budget.check_snapshot_import_allocation(1, 1).is_err(),
                "finish consumed the failure sentinel"
            );
            drop(rest);
            assert_eq!(budget.used_bytes(), old_charge);
            let alias = frozen.clone();
            let frozen = unavailable(frozen.try_into_owned_unsigned_matrix(&budget).unwrap());
            assert!(frozen.shares_frozen_storage(&alias));
            drop((frozen, alias));
            assert_eq!(
                (budget.used_bytes(), budget.retained_snapshot_nodes()),
                (0, 0)
            );
        }};
    }
    check!(U8, W8, 6633832, 129u64, 255u64);
    check!(U16, W16, 8, 32769u64, 65535u64);
    check!(U32, W32, 8, 0x80000000u64, 0xffffffffu64);
    check!(U64, W64, 305257, 2147483648u64, u64::MAX);
}

#[test]
fn append_preparation_denial_and_aborted_private_phase_preserve_publication_and_recover() {
    let budget = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    let value = admit(u64_value(305257), &budget);
    let table = value.schemas().unwrap();
    let id = value.schema();
    let alias = value.clone();
    let identity = value.frozen_storage_identity();
    let charged = budget.used_bytes();
    let other = crate::ManagedMemoryBudget::new(32 * 1024 * 1024);
    assert!(
        value
            .prepare_private_unsigned_appends(&other)
            .unwrap()
            .is_none()
    );
    let rest = budget
        .reserve_capacity(budget.limit_bytes() - charged)
        .unwrap();
    assert!(value.prepare_private_unsigned_appends(&budget).is_err());
    assert!(identity.matches(&value));
    assert!(value.shares_frozen_storage(&alias));
    drop(rest);
    drop(alias);
    let preparation = value
        .prepare_private_unsigned_appends(&budget)
        .unwrap()
        .unwrap();
    let UnsignedMatrixOwnershipTransfer::Owned(owned) =
        value.try_into_owned_unsigned_matrix(&budget).unwrap()
    else {
        panic!()
    };
    let mut private = owned
        .into_private_appends(shape(id, &table, 3), preparation)
        .unwrap();
    assert!(private.append_in_capacity(23));
    drop(private);
    assert_eq!(
        (budget.used_bytes(), budget.retained_snapshot_nodes()),
        (0, 0)
    );
    let value = admit(u64_value(305257), &budget);
    let preparation = value
        .prepare_private_unsigned_appends(&budget)
        .unwrap()
        .unwrap();
    let UnsignedMatrixOwnershipTransfer::Owned(owned) =
        value.try_into_owned_unsigned_matrix(&budget).unwrap()
    else {
        panic!()
    };
    let mut private = owned
        .into_private_appends(shape(id, &table, 3), preparation)
        .unwrap();
    assert!(private.append_in_capacity(29));
    let frozen = private.finish();
    assert_eq!(frozen.shape().parameter_values(), &[4]);
    frozen.validate_against(&table).unwrap();
    drop(frozen);
    assert_eq!(
        (budget.used_bytes(), budget.retained_snapshot_nodes()),
        (0, 0)
    );
}

#[test]
fn private_append_rejects_full_capacity_and_out_of_kind_words_without_changing_length() {
    let (id, table) = schema(IntegerWidth::W8);
    let budget = crate::ManagedMemoryBudget::new(1_000_000);
    let mut words = Vec::with_capacity(2);
    words.push(7u8);
    let value = admit(
        Value::from_unsigned_matrix_buffer(
            id,
            shape(id, &table, 1),
            UnsignedMatrixBuffer::U8(words),
            table.clone(),
        )
        .unwrap(),
        &budget,
    );
    let preparation = value
        .prepare_private_unsigned_appends(&budget)
        .unwrap()
        .unwrap();
    let UnsignedMatrixOwnershipTransfer::Owned(owned) =
        value.try_into_owned_unsigned_matrix(&budget).unwrap()
    else {
        panic!()
    };
    let mut private = owned
        .into_private_appends(shape(id, &table, 1), preparation)
        .unwrap();
    assert!(!private.append_in_capacity(256));
    assert_eq!(private.elements().len(), 1);
    assert!(private.append_in_capacity(255));
    assert!(!private.append_in_capacity(1));
    let frozen = private.finish();
    assert_eq!(
        frozen.canonical_data_draft().unwrap(),
        ValueDataDraft::Matrix([ValueDataDraft::U8(7), ValueDataDraft::U8(255)].into())
    );
    assert_eq!(frozen.shape().parameter_values(), &[2]);
    drop(frozen);
    assert_eq!(
        (budget.used_bytes(), budget.retained_snapshot_nodes()),
        (0, 0)
    );
}
