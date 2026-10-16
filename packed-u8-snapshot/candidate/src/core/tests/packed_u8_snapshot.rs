#![cfg(feature = "std")]

use mech_core::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, IntegerWidth, ManagedMemoryBudget, SchemaBody, SchemaDraft,
    SchemaId, SchemaTable, SchemaTableBuilder, Value, ValueData, ValueDataDraft, ValueDraft,
    snapshot::{SequenceView, SnapshotValidationContext},
};
use std::sync::Arc;

#[path = "support/allocation_probe.rs"]
mod allocation_probe;

#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

fn schema() -> (Arc<SchemaTable>, SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(SchemaDraft {
        dimension_parameters: (0..2).map(|ordinal| DimensionParameterDeclaration {
            id: DimensionParameterId::new(ordinal),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }).collect(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: (0..2).map(|ordinal| DimensionExpr::Parameter(DimensionParameterId::new(ordinal))).collect(),
        },
    }.finalize().unwrap()).unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    (Arc::new(built.table), id)
}

fn bytes(value: &Value) -> &[u8] {
    let ValueData::Matrix(matrix) = value.data() else { panic!("matrix payload") };
    let SequenceView::U8(bytes) = matrix.elements() else { panic!("packed U8 payload") };
    bytes
}

#[test]
fn all_byte_values_match_validated_drafts_and_preserve_zero_axis_identity() {
    let (schemas, id) = schema();
    let input = (0..=255u8).collect::<Vec<_>>();
    let context = SnapshotValidationContext::with_shared_schemas(&schemas);
    let packed = Value::from_u8_matrix_slice(id, Box::new([1, 256]), &input, &context).unwrap();
    let reference = ValueDraft {
        schema: id,
        shape_values: Box::new([1, 256]),
        data: ValueDataDraft::Matrix(input.iter().copied().map(ValueDataDraft::U8).collect()),
    }.finalize(&context).unwrap();
    assert_eq!(bytes(&packed), input.as_slice());
    assert_eq!(packed.schema_key(), reference.schema_key());
    assert_eq!(packed.shape(), reference.shape());
    assert_eq!(packed.resident_token(), reference.resident_token());
    assert_eq!(packed.canonical_payload_bytes(&schemas).unwrap().as_ref(), input.as_slice());
    assert_eq!(packed.canonical_snapshot_bytes(&schemas).unwrap(), reference.canonical_snapshot_bytes(&schemas).unwrap());
    assert!(Arc::ptr_eq(&packed.schemas().unwrap(), &schemas));
    let row = Value::from_u8_matrix_slice(id, Box::new([1, 0]), &[], &context).unwrap();
    let columnless = Value::from_u8_matrix_slice(id, Box::new([0, 17]), &[], &context).unwrap();
    assert!(bytes(&row).is_empty() && bytes(&columnless).is_empty());
    assert_eq!(row.shape().parameter_values(), [1, 0]);
    assert_eq!(columnless.shape().parameter_values(), [0, 17]);
    assert!(!row.snapshot_eq(&schemas, &columnless, &schemas).unwrap());
}

#[test]
fn packed_construction_and_encoding_have_bounded_actual_allocations_as_bytes_grow() {
    let (schemas, id) = schema();
    let context = SnapshotValidationContext::with_shared_schemas(&schemas);
    let mut empty = None;
    let mut empty_required = None;
    for length in [0usize, 1, 256, 65_536, 1_048_576] {
        let input = (0..length).map(|index| index as u8).collect::<Vec<_>>();
        let shape = Box::new([1, length as u64]);
        let (result, calls, allocated) = allocation_probe::measured_with_bytes(|| {
            Value::from_u8_matrix_slice(id, shape, &input, &context)
        });
        let value = result.unwrap();
        let (empty_calls, empty_bytes) = *empty.get_or_insert((calls, allocated));
        assert!(calls <= empty_calls + 2, "payload-length-dependent allocation count: N={length}, calls={calls}");
        assert!(allocated <= empty_bytes + length * 2 + 4096, "draft-sized expansion: N={length}, allocated={allocated}");
        assert_eq!(bytes(&value), input.as_slice());
        let required = value.memory_budget_required_bytes().unwrap();
        let baseline = *empty_required.get_or_insert(required);
        assert_eq!(required - baseline, length as u64, "retained capacity must grow by one byte per element");
        // Canonical encoding is still real work and allocates its output.
        // U8's canonical payload consists of exactly the original bytes.
        let (encoded, encoding_calls, encoding_bytes) = allocation_probe::measured_with_bytes(|| value.canonical_payload_bytes(&schemas));
        assert_eq!(encoded.unwrap().as_ref(), input.as_slice());
        assert!(encoding_calls <= 2);
        assert!(encoding_bytes <= length * 2 + 16);
    }
}

#[test]
fn packed_snapshot_uses_the_existing_retained_budget_and_clone_lifetime() {
    let (schemas, id) = schema();
    let context = SnapshotValidationContext::with_shared_schemas(&schemas);
    let input = (0..=255u8).cycle().take(65_536).collect::<Vec<_>>();
    let value = Value::from_u8_matrix_slice(id, Box::new([1, input.len() as u64]), &input, &context).unwrap();
    let budget = ManagedMemoryBudget::new(1_000_000);
    let required = value.memory_budget_admission_bytes(&budget).unwrap();
    let mut denied = budget.reserve_capacity(required - 1).unwrap();
    assert!(value.clone().into_memory_budget(&mut denied).is_err());
    assert_eq!(budget.used_bytes(), required - 1);
    drop(denied);
    assert_eq!(budget.used_bytes(), 0);
    let mut admitted = budget.reserve_capacity(required).unwrap();
    let value = value.into_memory_budget(&mut admitted).unwrap();
    let alias = value.clone().into_memory_budget(&mut admitted).unwrap();
    assert_eq!(admitted.capacity_bytes(), 0);
    drop(admitted);
    assert_eq!(budget.used_bytes(), required);
    let (_, allocations) = allocation_probe::measured(|| {
        for _ in 0..128 {
            let clone = value.clone();
            assert!(clone.shares_frozen_storage(&value));
            assert_eq!(bytes(&clone), input.as_slice());
        }
    });
    assert_eq!(allocations, 0);
    drop(value);
    assert_eq!(budget.used_bytes(), required);
    assert_eq!(bytes(&alias), input.as_slice());
    drop(alias);
    assert_eq!(budget.used_bytes(), 0);
}
