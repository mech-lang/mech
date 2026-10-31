use mech_core::snapshot::{CompositeSnapshotConstructor, SnapshotValidationContext, ValueData};
use mech_core::{
    SchemaBody, SchemaDraft, SchemaField, SchemaTableBuilder, ValueDataDraft, ValueDraft,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

struct ObservedAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static REQUESTS: Cell<usize> = const { Cell::new(0) };
    static TOTAL: Cell<usize> = const { Cell::new(0) };
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}
// SAFETY: This test-only observer delegates every allocation operation to System
// with the original pointer and layout. Counters never inspect or retain pointers.
unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK.try_with(Cell::get).unwrap_or(false) {
            REQUESTS.with(|value| value.set(value.get() + 1));
            TOTAL.with(|value| value.set(value.get() + layout.size()));
            LARGEST.with(|value| value.set(value.get().max(layout.size())));
        }
        // SAFETY: The caller supplies the GlobalAlloc allocation layout unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The pointer and its original allocation layout are forwarded unchanged.
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if TRACK.try_with(Cell::get).unwrap_or(false) {
            REQUESTS.with(|value| value.set(value.get() + 1));
            TOTAL.with(|value| value.set(value.get() + size));
            LARGEST.with(|value| value.set(value.get().max(size)));
        }
        // SAFETY: The caller's live allocation, original layout and requested size are forwarded.
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

#[test]
fn long_record_labels_are_not_allocated_again_during_a_bound_construction() {
    let mut observations = Vec::new();
    for name in ["x".to_owned(), "x".repeat(65_536)] {
        let mut builder = SchemaTableBuilder::new();
        let boolean = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Bool,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let record = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Record(
                        vec![SchemaField {
                            name,
                            schema: SchemaBody::Bool,
                        }]
                        .into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let boolean = built.resolve(boolean).unwrap();
        let record = built.resolve(record).unwrap();
        let schemas = Arc::new(built.into_parts().0);
        let boolean_shape = schemas
            .get(boolean)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let record_shape = schemas
            .get(record)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let constructor = CompositeSnapshotConstructor::bind(
            record,
            record_shape,
            &[(boolean, boolean_shape)],
            Arc::clone(&schemas),
        )
        .unwrap();
        let child = ValueDraft {
            schema: boolean,
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(true),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        let children = vec![child].into_boxed_slice();
        TOTAL.with(|value| value.set(0));
        LARGEST.with(|value| value.set(0));
        TRACK.with(|value| value.set(true));
        let result = constructor.construct(children, None);
        TRACK.with(|value| value.set(false));
        let observation = (TOTAL.with(Cell::get), LARGEST.with(Cell::get));
        let result = result.unwrap();
        assert!(
            matches!(result.data(),ValueData::Record(record) if matches!(record.fields(),[ValueData::Bool(true)]))
        );
        assert!(
            observation.0 < 4096,
            "construction exceeds bounded 4 KiB allowance: {observation:?}"
        );
        observations.push(observation);
    }
    assert_eq!(
        observations[0], observations[1],
        "schema label length must not affect turn allocations"
    );
}

fn measured_requests<T>(run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TRACK.with(|value| value.set(false));
        }
    }
    REQUESTS.with(|value| value.set(0));
    TRACK.with(|value| assert!(!value.replace(true)));
    let reset = Reset;
    let value = run();
    let requests = REQUESTS.with(Cell::get);
    drop(reset);
    (value, requests)
}

#[test]
fn numeric_matrix_validation_allocates_only_the_three_retained_owners() {
    use mech_core::{
        DimensionExpr as D, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, IntegerWidth, Value,
    };
    for fixed in [false, true] {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W32)),
                        dimensions: [
                            D::Constant(1),
                            if fixed {
                                D::Constant(3)
                            } else {
                                D::Parameter(DimensionParameterId::new(0))
                            },
                        ]
                        .into(),
                    },
                    dimension_parameters: if fixed {
                        Box::new([])
                    } else {
                        [DimensionParameterDeclaration {
                            id: DimensionParameterId::new(0),
                            origin: DimensionParameterOrigin::Explicit,
                            lifetime: DimensionLifetime::Activation,
                            lower_bound: D::Constant(1),
                            upper_bound: Some(D::Constant(8)),
                        }]
                        .into()
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let id = built.resolve(handle).unwrap();
        let schemas = Arc::new(built.table);
        for seed in [7u32, 19, 31] {
            let shape = schemas
                .get(id)
                .unwrap()
                .instantiate_shape(if fixed { Box::new([]) } else { [3].into() })
                .unwrap();
            let words: Box<[u32]> = [seed, seed + 1, seed + 2].into();
            let pointer = words.as_ptr();
            let (result, requests) = measured_requests(|| {
                Value::from_u32_matrix(id, shape, words, Arc::clone(&schemas))
            });
            let value = result.unwrap();
            assert_eq!(
                requests, 3,
                "validation must borrow shape/schema; only retained owners allocate"
            );
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("matrix")
            };
            let mech_core::snapshot::SequenceView::U32(words) = matrix.elements() else {
                panic!("u32")
            };
            assert_eq!(words.as_ptr(), pointer);
            assert_eq!(words, [seed, seed + 1, seed + 2]);
            value.validate_against(&schemas).unwrap();
        }
    }
}
