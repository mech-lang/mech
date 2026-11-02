#![cfg(all(feature = "functions", feature = "string"))]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use mech_core::{CanonicalCellBacking, CurrentMemoryFootprint, Ref, ValueCell};

thread_local! {
    static LARGEST_ALLOCATION: Cell<Option<usize>> = const { Cell::new(None) };
}

struct AllocationProbe;

fn record_allocation(bytes: usize) {
    let _ = LARGEST_ALLOCATION.try_with(|largest| {
        if let Some(previous) = largest.get() {
            largest.set(Some(previous.max(bytes)));
        }
    });
}

// SAFETY: each allocation is forwarded unchanged to System. The thread-local
// probe observes request sizes only and never reads allocated memory.
unsafe impl GlobalAlloc for AllocationProbe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation(size);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: AllocationProbe = AllocationProbe;

fn measured<T>(run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            LARGEST_ALLOCATION.with(|largest| largest.set(None));
        }
    }
    LARGEST_ALLOCATION.with(|largest| assert!(largest.replace(Some(0)).is_none()));
    let reset = Reset;
    let result = run();
    let largest = LARGEST_ALLOCATION.with(|largest| largest.get().unwrap());
    drop(reset);
    (result, largest)
}

fn pinned<T: CanonicalCellBacking>(value: T) -> (Ref<T>, ValueCell, CurrentMemoryFootprint) {
    // Owned indirect values use immutable canonical storage. This independent
    // reference measurement and pinned ingress validation occur before probing.
    let canonical = ValueCell::from_exact(value.clone()).unwrap();
    let expected = canonical.current_memory_footprint().unwrap();
    let reference = Ref::new(value);
    let pinned = ValueCell::from_ref(
        reference.clone(),
        canonical.schema(),
        canonical.shape().clone(),
        canonical.retained_schema_table(),
    )
    .unwrap();
    (reference, pinned, expected)
}

#[test]
fn pinned_scalar_string_footprint_borrows_unicode_payload() {
    let text = "é🧬\n".repeat(16_384);
    let payload_length = text.len();
    let (_reference, cell, expected) = pinned(text);
    let (actual, largest) = measured(|| cell.current_memory_footprint());
    assert_eq!(actual.unwrap(), expected);
    assert!(
        largest < payload_length,
        "footprint inspection allocated a complete String payload: {largest}"
    );
}

#[cfg(feature = "matrixd")]
#[test]
fn pinned_matrix_string_footprint_borrows_each_payload_and_preserves_geometry() {
    let text = "aé🧬\n".repeat(16_384);
    let payload_length = text.len();
    let (_reference, cell, expected) = pinned(nalgebra::DMatrix::from_element(2, 3, text));
    let (actual, largest) = measured(|| cell.current_memory_footprint());
    let actual = actual.unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.logical_elements, 6);
    assert!(
        largest < payload_length,
        "footprint inspection reconstructed a matrix String payload: {largest}"
    );
}

#[cfg(feature = "matrixd")]
#[test]
fn pinned_empty_string_matrices_keep_both_axes_and_exact_footprint() {
    for (rows, columns) in [(1, 0), (0, 3)] {
        let (_reference, cell, expected) = pinned(nalgebra::DMatrix::<String>::from_element(
            rows,
            columns,
            String::new(),
        ));
        let (actual, _) = measured(|| cell.current_memory_footprint());
        assert_eq!(actual.unwrap(), expected);
        assert_eq!(expected.logical_elements, 0);
        assert_eq!(
            cell.resolved_descriptor()
                .unwrap()
                .current_extents()
                .unwrap()
                .as_ref(),
            &[rows as u64, columns as u64],
        );
    }
}

#[test]
fn pinned_string_footprint_retains_structured_borrow_conflict() {
    let (reference, cell, _) = pinned("Unicode: é🧬\n".to_string());
    let guard = reference.borrow_mut();
    let snapshot_error = cell.snapshot().unwrap_err();
    let (measured_error, _) = measured(|| cell.current_memory_footprint());
    assert_eq!(snapshot_error.kind_name(), "ValueCellBorrowConflict");
    assert_eq!(
        measured_error.unwrap_err().kind_name(),
        snapshot_error.kind_name()
    );
    drop(guard);
    assert!(cell.current_memory_footprint().is_ok());
}

#[cfg(feature = "matrixd")]
#[test]
fn pinned_string_footprint_rejects_same_element_count_reshape_like_snapshot() {
    let (reference, cell, _) = pinned(nalgebra::DMatrix::from_element(2, 3, "é🧬".to_string()));
    *reference.borrow_mut() = nalgebra::DMatrix::from_element(3, 2, "é🧬".to_string());
    let snapshot_error = cell.snapshot().unwrap_err();
    let (measured_error, _) = measured(|| cell.current_memory_footprint());
    assert_eq!(snapshot_error.kind_name(), "ValueCellShapeMismatch");
    assert_eq!(
        measured_error.unwrap_err().kind_name(),
        snapshot_error.kind_name()
    );
}
