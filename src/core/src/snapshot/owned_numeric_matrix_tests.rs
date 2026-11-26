use super::*;
use crate::snapshot::{F64Bits, SequenceView};
use crate::{DimensionExpr, ManagedMemoryBudget, SchemaDraft, SchemaTableBuilder};

fn cases() -> [(SchemaBody, ValueDataDraft, u64); 3] {
    [
        (SchemaBody::Bool, ValueDataDraft::Bool(false), 1),
        (
            SchemaBody::SignedInteger(IntegerWidth::W64),
            ValueDataDraft::I64(i64::MIN),
            i64::MAX as u64,
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(F64Bits::from_bits(1 << 63)),
            0x7ff8_0000_0000_0001,
        ),
    ]
}

fn matrix(element: SchemaBody, cell: ValueDataDraft) -> Value {
    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: element.into(),
                    dimensions: [DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into(),
                },
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = schemas.finish().unwrap();
    ValueDraft {
        schema: built.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(vec![cell; 6].into()),
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .unwrap()
}

fn admit(value: Value, memory: &ManagedMemoryBudget) -> Value {
    let mut claim = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut claim).unwrap()
}

fn pointer(value: SequenceView<'_>) -> usize {
    match value {
        SequenceView::Bool(v) => v.as_ptr() as usize,
        SequenceView::I64(v) => v.as_ptr() as usize,
        SequenceView::F64(v) => v.as_ptr() as usize,
        _ => panic!("boxed native scalar kind"),
    }
}

fn unavailable(result: NumericMatrixOwnershipTransfer) -> Value {
    match result {
        NumericMatrixOwnershipTransfer::Unavailable(value) => value,
        NumericMatrixOwnershipTransfer::Owned(_) => panic!("shared matrix was consumed"),
    }
}

#[test]
fn numeric_handoff_reuses_exact_backing_charge_and_retires_old_identity() {
    for (element, cell, replacement) in cases() {
        let memory = ManagedMemoryBudget::new(1 << 20);
        let value = admit(matrix(element, cell), &memory);
        let schemas = value.schemas().unwrap();
        let constructor = value
            .numeric_matrix_replacement_constructor(schemas.clone())
            .unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!()
        };
        let backing = pointer(matrix.elements());
        let old = value.frozen_storage_identity();
        let bytes = memory.used_bytes();
        let NumericMatrixOwnershipTransfer::Owned(owned) =
            value.try_into_owned_numeric_matrix(&memory).unwrap()
        else {
            panic!()
        };
        let (mut buffer, mut claim) = owned.into_parts();
        assert_eq!(pointer(buffer.elements()), backing);
        assert_eq!(old.data.strong_count(), 0);
        assert_eq!(memory.used_bytes(), bytes);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
        assert_eq!(buffer.replace_word(3, replacement), Some(()));
        assert_eq!(buffer.replace_word(6, replacement), None);
        let value = constructor
            .finish(buffer)
            .unwrap()
            .into_memory_budget(&mut claim)
            .unwrap();
        drop(claim);
        assert!(!old.matches(&value));
        let ValueData::Matrix(matrix) = value.data() else {
            panic!()
        };
        assert_eq!(pointer(matrix.elements()), backing);
        let actual = matrix.elements().value_at(3).unwrap();
        assert_eq!(
            match actual {
                ValueData::Bool(v) => u64::from(v),
                ValueData::I64(v) => v as u64,
                ValueData::F64(v) => v.bits(),
                _ => panic!(),
            },
            replacement
        );
        value.validate_against(&schemas).unwrap();
        assert_eq!(memory.used_bytes(), bytes);
        drop(value);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_handoff_keeps_aliases_imports_and_unadmitted_owners_frozen() {
    for (element, cell, _) in cases() {
        let memory = ManagedMemoryBudget::new(1 << 20);
        let other = ManagedMemoryBudget::new(1 << 20);
        let value = matrix(element, cell);
        let value = unavailable(value.try_into_owned_numeric_matrix(&memory).unwrap());
        let value = admit(value, &memory);
        let expected = value.canonical_data_draft().unwrap();
        let identity = value.frozen_storage_identity();
        let value = unavailable(value.try_into_owned_numeric_matrix(&other).unwrap());
        let alias = value.clone();
        let imported = admit(value.clone(), &other);
        let value = unavailable(value.try_into_owned_numeric_matrix(&memory).unwrap());
        assert!(identity.matches(&value));
        drop(alias);
        let value = unavailable(value.try_into_owned_numeric_matrix(&memory).unwrap());
        assert_eq!(value.canonical_data_draft().unwrap(), expected);
        assert_eq!(imported.canonical_data_draft().unwrap(), expected);
        drop(imported);
        assert_eq!(other.used_bytes(), 0);
        let NumericMatrixOwnershipTransfer::Owned(owned) =
            value.try_into_owned_numeric_matrix(&memory).unwrap()
        else {
            panic!()
        };
        drop(owned);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_reconstruction_checks_kind_cardinality_and_boolean_words() {
    for (element, cell, _) in cases() {
        let value = matrix(element, cell);
        let schemas = value.schemas().unwrap();
        for count in [0, 5, 7] {
            let constructor = value
                .numeric_matrix_replacement_constructor(schemas.clone())
                .unwrap();
            assert!(
                constructor
                    .finish(NumericMatrixBuffer::Bool(vec![false; count].into()))
                    .is_err()
            );
        }
        let foreign = matrix(SchemaBody::String, ValueDataDraft::String("x".into()));
        assert!(
            value
                .numeric_matrix_replacement_constructor(foreign.schemas().unwrap())
                .is_err()
        );
        assert!(
            foreign
                .numeric_matrix_replacement_constructor(foreign.schemas().unwrap())
                .is_err()
        );
    }
    let mut booleans = NumericMatrixBuffer::Bool([false, true].into());
    assert_eq!(booleans.replace_word(0, 2), None);
    assert!(matches!(
        booleans.elements(),
        SequenceView::Bool([false, true])
    ));
    let mut narrow = NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U8(vec![0]));
    assert_eq!(narrow.replace_word(0, 256), None);
    assert!(matches!(narrow.elements(), SequenceView::U8([0])));
}

#[test]
fn numeric_private_phase_keeps_exact_cells_and_publication_admission_without_a_second_copy() {
    for (element, cell, replacement) in cases() {
        let memory = ManagedMemoryBudget::new(1 << 20);
        let value = matrix(element, cell);
        let schemas = value.schemas().unwrap();
        let ValueData::Matrix(source) = value.data() else {
            panic!()
        };
        let address = pointer(source.elements());
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let mut private = value.into_private_numeric_matrix(&mut reservation).unwrap();
        drop(reservation);
        assert_eq!(pointer(private.elements()), address);
        assert_eq!(private.memory_budget(), memory);
        let used = memory.used_bytes();
        let nodes = memory.retained_snapshot_nodes();
        assert!(nodes > 0);
        match private.elements_mut() {
            NumericMatrixViewMut::Bool(v) => v[3] = replacement != 0,
            NumericMatrixViewMut::I64(v) => v[3] = replacement as i64,
            NumericMatrixViewMut::F64(v) => v[3] = F64Bits::from_bits(replacement),
            _ => panic!("boxed scalar case"),
        }
        let value = private.finish();
        assert_eq!(memory.used_bytes(), used);
        assert_eq!(memory.retained_snapshot_nodes(), nodes);
        value.validate_against(&schemas).unwrap();
        let ValueData::Matrix(result) = value.data() else {
            panic!()
        };
        assert_eq!(pointer(result.elements()), address);
        assert_eq!(
            match result.elements().value_at(3).unwrap() {
                ValueData::Bool(v) => u64::from(v),
                ValueData::I64(v) => v as u64,
                ValueData::F64(v) => v.bits(),
                _ => panic!(),
            },
            replacement
        );
        drop(value);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_private_phase_rejects_aliases_identities_imports_and_short_admission() {
    for (element, cell, _) in cases() {
        let memory = ManagedMemoryBudget::new(1 << 20);
        let fresh = || matrix(element.clone(), cell.clone());
        let reserve = |v: &Value| {
            memory
                .reserve_capacity(v.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap()
        };
        let value = fresh();
        let alias = value.clone();
        let mut reservation = reserve(&value);
        assert!(value.into_private_numeric_matrix(&mut reservation).is_err());
        alias.validate_against(&alias.schemas().unwrap()).unwrap();
        drop((alias, reservation));
        let value = fresh();
        let identity = value.frozen_storage_identity();
        let mut reservation = reserve(&value);
        assert!(value.into_private_numeric_matrix(&mut reservation).is_err());
        drop((identity, reservation));
        let value = fresh();
        let required = value.memory_budget_admission_bytes(&memory).unwrap();
        let mut reservation = memory.reserve_capacity(required - 1).unwrap();
        assert!(value.into_private_numeric_matrix(&mut reservation).is_err());
        drop(reservation);
        let value = admit(fresh(), &memory);
        let other = ManagedMemoryBudget::new(1 << 20);
        let mut reservation = other.reserve_capacity(1 << 16).unwrap();
        assert!(value.into_private_numeric_matrix(&mut reservation).is_err());
        drop(reservation);
        let value = fresh();
        let mut reservation = reserve(&value);
        assert!(
            value
                .into_private_unsigned_matrix(&mut reservation)
                .is_err()
        );
        drop(reservation);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
        assert_eq!(
            (other.used_bytes(), other.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
