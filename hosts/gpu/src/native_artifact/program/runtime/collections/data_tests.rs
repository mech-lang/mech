use super::*;
use mech_core::{
    DimensionExpr, IntegerWidth, ManagedMemoryBudget, SchemaBody, SchemaDraft, SchemaTableBuilder,
    ValueDataDraft,
};
use std::sync::Arc;

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
