use super::*;
use mech_core::{
    DimensionExpr, FloatWidth, IntegerWidth, ManagedMemoryBudget, SchemaBody, SchemaDraft,
    SchemaTableBuilder, ValueDataDraft,
};
use std::sync::Arc;

fn boxed_scalar_cases() -> [(SchemaBody, Vec<ValueDataDraft>, usize); 3] {
    [
        (
            SchemaBody::Bool,
            [false, true, false, true, true]
                .map(ValueDataDraft::Bool)
                .into(),
            1,
        ),
        (
            SchemaBody::SignedInteger(IntegerWidth::W64),
            [i64::MIN, -1, 0, 1, i64::MAX]
                .map(ValueDataDraft::I64)
                .into(),
            8,
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            [
                0,
                0x8000_0000_0000_0000,
                0x7ff8_0000_0000_0001,
                0x7ff0_0000_0000_0000,
                0xfff0_0000_0000_0000,
            ]
            .map(|bits| ValueDataDraft::F64(F64Bits::from_bits(bits)))
            .into(),
            8,
        ),
    ]
}

fn matrix_schema(
    element: SchemaBody,
    len: usize,
) -> (
    SchemaBody,
    mech_core::SchemaId,
    mech_core::ShapeInstance,
    Arc<mech_core::SchemaTable>,
) {
    let body = SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: Box::new([
            DimensionExpr::Constant(1),
            DimensionExpr::Constant(len as u64),
        ]),
    };
    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas
        .insert(
            SchemaDraft {
                body: body.clone(),
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let schemas = schemas.finish().unwrap();
    let schema = schemas.resolve(handle).unwrap();
    let schemas = Arc::new(schemas.table);
    let shape = schemas
        .get(schema)
        .unwrap()
        .instantiate_shape(Box::new([]))
        .unwrap();
    (body, schema, shape, schemas)
}

#[test]
fn boxed_scalar_completion_preserves_exact_bits_kind_and_bounded_publication() {
    for (element, cells, width) in boxed_scalar_cases() {
        for len in [0usize, 1, 3, 5] {
            let (body, schema, shape, schemas) = matrix_schema(element.clone(), len);
            let mut values = CollectedValues::new(&body);
            let capacity = if len == 0 { 0 } else { len.next_power_of_two() };
            values.reserve(capacity).unwrap();
            for cell in cells[..len].iter().cloned() {
                values.push(cell).unwrap();
            }
            let payload = (len * width) as u64;
            assert_eq!(values.packed_width(), Some(width as u64));
            assert_eq!(
                values.backing_bytes(capacity, payload).unwrap(),
                (capacity * width) as u64
            );
            assert_eq!(values.retained_nodes().unwrap(), 2);
            assert_eq!(
                values.finalization_cloned_bytes(payload),
                if capacity == len { 0 } else { payload }
            );
            assert_eq!(
                values
                    .output_bytes(payload, len as u64, capacity, 0)
                    .unwrap(),
                payload
                    + (core::mem::size_of::<Value>() + core::mem::size_of::<ValueData>()) as u64
            );
            let value = values.finalize(schema, shape, schemas.clone()).unwrap();
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Matrix(cells[..len].to_vec().into_boxed_slice())
            );
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("matrix")
            };
            match (&element, matrix.elements()) {
                (SchemaBody::Bool, SequenceView::Bool(_)) => (),
                (SchemaBody::SignedInteger(IntegerWidth::W64), SequenceView::I64(_)) => (),
                (SchemaBody::FloatingPoint(FloatWidth::W64), SequenceView::F64(_)) => (),
                _ => panic!("collector must publish its exact canonical packed kind"),
            }
            value.validate_against(&schemas).unwrap();
            let memory = ManagedMemoryBudget::new(1024 * 1024);
            let bytes = value.memory_budget_admission_bytes(&memory).unwrap();
            let mut short = memory.reserve_capacity(bytes - 1).unwrap();
            assert!(value.clone().into_memory_budget(&mut short).is_err());
            drop(short);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
            let mut reservation = memory.reserve_capacity(bytes).unwrap();
            let value = value.into_memory_budget(&mut reservation).unwrap();
            drop((reservation, value));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn boxed_scalar_collectors_reject_unsigned_loans_and_wrong_cardinality_or_kind() {
    for (element, cells, _) in boxed_scalar_cases() {
        let (body, schema, shape, schemas) = matrix_schema(element, 3);
        let mut empty = CollectedValues::new(&body);
        assert!(empty.unsigned_pointer().is_err());
        assert!(empty.unsigned_capacity_pointer().is_err());
        // SAFETY: these zero-length calls claim no initialized cells. The
        // unsigned-only APIs must reject the kind before exposing a raw bank.
        assert!(unsafe { empty.initialize_unsigned(0) }.is_err());
        assert!(unsafe { empty.set_initialized_unsigned_len(0) }.is_err());
        assert!(empty.fill_unsigned(0, NativeScalarKind::U64, 0).is_err());
        for len in [2, 4] {
            let mut values = CollectedValues::new(&body);
            values.reserve(len).unwrap();
            for cell in cells[..len].iter().cloned() {
                values.push(cell).unwrap();
            }
            assert!(
                values
                    .finalize(schema, shape.clone(), schemas.clone())
                    .is_err()
            );
        }
        let mut values = CollectedValues::new(&body);
        values.reserve(1).unwrap();
        assert!(values.push(ValueDataDraft::U64(1)).is_err());
        assert_eq!(values.len(), 0);
    }
}

#[test]
fn unsigned_completion_transfers_the_same_allocation_with_spare_capacity() {
    for (width, bytes) in [
        (IntegerWidth::W8, 1),
        (IntegerWidth::W16, 2),
        (IntegerWidth::W32, 4),
        (IntegerWidth::W64, 8),
    ] {
        for len in [3usize, 5] {
            let body = SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(width)),
                dimensions: Box::new([
                    DimensionExpr::Constant(1),
                    DimensionExpr::Constant(len as u64),
                ]),
            };
            let mut schemas = SchemaTableBuilder::new();
            let handle = schemas
                .insert(
                    SchemaDraft {
                        body: body.clone(),
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let schemas = schemas.finish().unwrap();
            let schema = schemas.resolve(handle).unwrap();
            let schemas = Arc::new(schemas.table);
            let shape = schemas
                .get(schema)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap();
            let mut values = CollectedValues::new(&body);
            let capacity = len.next_power_of_two();
            values.reserve(capacity).unwrap();
            for i in 0..len {
                values
                    .push(match width {
                        IntegerWidth::W8 => ValueDataDraft::U8(i as u8),
                        IntegerWidth::W16 => ValueDataDraft::U16(i as u16),
                        IntegerWidth::W32 => ValueDataDraft::U32(i as u32),
                        IntegerWidth::W64 => ValueDataDraft::U64(i as u64),
                        _ => unreachable!(),
                    })
                    .unwrap();
            }
            let pointer = match &values {
                CollectedValues::U8(v) => v.as_ptr() as usize,
                CollectedValues::U16(v) => v.as_ptr() as usize,
                CollectedValues::U32(v) => v.as_ptr() as usize,
                CollectedValues::U64(v) => v.as_ptr() as usize,
                _ => unreachable!(),
            };
            assert_eq!(
                values
                    .output_bytes((len * bytes) as u64, len as u64, capacity, 0)
                    .unwrap(),
                (capacity * bytes
                    + core::mem::size_of::<Value>()
                    + core::mem::size_of::<ValueData>()) as u64
            );
            let value = values.finalize(schema, shape, schemas.clone()).unwrap();
            let memory = ManagedMemoryBudget::new(1024 * 1024);
            let admission_bytes = value.memory_budget_admission_bytes(&memory).unwrap();
            let mut short = memory.reserve_capacity(admission_bytes - 1).unwrap();
            assert!(value.clone().into_memory_budget(&mut short).is_err());
            drop(short);
            assert_eq!(memory.used_bytes(), 0);
            let mut admission = memory.reserve_capacity(admission_bytes).unwrap();
            let value = value.into_memory_budget(&mut admission).unwrap();
            drop(admission);
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity));
            let actual_pointer = match matrix.elements() {
                SequenceView::U8(v) => v.as_ptr() as usize,
                SequenceView::U16(v) => v.as_ptr() as usize,
                SequenceView::U32(v) => v.as_ptr() as usize,
                SequenceView::U64(v) => v.as_ptr() as usize,
                _ => unreachable!(),
            };
            assert_eq!(actual_pointer, pointer, "{width:?}/{len}");
            drop(value);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}
