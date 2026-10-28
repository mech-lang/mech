//! Allocation evidence for the generic storage primitive, not a source benchmark.
use mech_core::{
    DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, IntegerWidth, ManagedMemoryBudget, SchemaBody, SchemaDraft,
    SchemaTableBuilder, Value,
    snapshot::{SequenceView, UnsignedMatrixBuffer, UnsignedMatrixOwnershipTransfer},
};
use std::sync::Arc;

#[path = "support/allocation_probe.rs"]
pub mod allocation_probe;
#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

fn roundtrips(width: IntegerWidth, capacity: usize) -> Vec<(usize, usize)> {
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
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    let schemas = Arc::new(built.table);
    let shape = |count| {
        schemas
            .get(id)
            .unwrap()
            .instantiate_shape([count].into())
            .unwrap()
    };
    let buffer = match width {
        IntegerWidth::W8 => UnsignedMatrixBuffer::U8(Vec::with_capacity(capacity)),
        IntegerWidth::W64 => UnsignedMatrixBuffer::U64(Vec::with_capacity(capacity)),
        _ => panic!("measured path/output kinds"),
    };
    let original_pointer = match buffer.elements() {
        SequenceView::U8(words) => words.as_ptr() as usize,
        SequenceView::U64(words) => words.as_ptr() as usize,
        _ => panic!(),
    };
    let value = Value::from_unsigned_matrix_buffer(id, shape(0), buffer, schemas.clone()).unwrap();
    let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
    let mut admission = memory
        .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
        .unwrap();
    let mut value = value.into_memory_budget(&mut admission).unwrap();
    drop(admission);
    let charged = memory.used_bytes();
    let mut costs = Vec::new();
    for _ in 0..3 {
        let (next, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
            let UnsignedMatrixOwnershipTransfer::Owned(owned) =
                value.try_into_owned_unsigned_matrix(&memory).unwrap()
            else {
                panic!()
            };
            let (mut buffer, mut admission) = owned.into_parts();
            match &mut buffer {
                UnsignedMatrixBuffer::U8(words) => {
                    words.clear();
                    words.push(0x81);
                }
                UnsignedMatrixBuffer::U64(words) => {
                    words.clear();
                    words.push(0x80000000);
                }
                _ => panic!(),
            }
            assert_eq!(
                match buffer.elements() {
                    SequenceView::U8(words) => words.as_ptr() as usize,
                    SequenceView::U64(words) => words.as_ptr() as usize,
                    _ => panic!(),
                },
                original_pointer
            );
            assert_eq!(
                buffer.capacity_bytes(),
                capacity * if width == IntegerWidth::W8 { 1 } else { 8 }
            );
            Value::from_unsigned_matrix_buffer(id, shape(1), buffer, schemas.clone())
                .unwrap()
                .into_memory_budget(&mut admission)
                .unwrap()
        });
        value = next;
        assert_eq!(memory.used_bytes(), charged);
        costs.push((allocations, bytes));
    }
    drop(value);
    assert_eq!(memory.used_bytes(), 0);
    costs
}

#[test]
fn retained_capacity_roundtrips_allocate_only_size_independent_snapshot_metadata() {
    for (width, capacity) in [
        (IntegerWidth::W64, 305257),
        (IntegerWidth::W8, 6627202 + 5 * 6627202usize.div_ceil(5000)),
    ] {
        let small = roundtrips(width, 16);
        let real = roundtrips(width, capacity);
        assert_eq!(
            real, small,
            "payload capacity must not add any allocation per repetition"
        );
        println!(
            "width={width:?} capacity={capacity} rounds=3 allocation_requests_and_bytes={real:?} payload_pointer_unchanged=true"
        );
    }
}
