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
    static TOTAL: Cell<usize> = const { Cell::new(0) };
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}
// SAFETY: This test-only observer delegates every allocation operation to System
// with the original pointer and layout. Counters never inspect or retain pointers.
unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK.try_with(Cell::get).unwrap_or(false) {
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
