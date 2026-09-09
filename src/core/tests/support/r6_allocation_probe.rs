use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static COUNT: Cell<Option<usize>> = const { Cell::new(None) };
    static BYTES: Cell<Option<usize>> = const { Cell::new(None) };
}

pub struct ProbeAllocator;

fn count(bytes: usize) {
    let _ = COUNT.try_with(|count| {
        if let Some(current) = count.get() {
            count.set(Some(current + 1));
        }
    });
    let _ = BYTES.try_with(|total| {
        if let Some(current) = total.get() {
            total.set(Some(current.saturating_add(bytes)));
        }
    });
}

// SAFETY: every request is forwarded unchanged to System. Thread-local
// counters observe allocation calls and never access the allocation's bytes.
unsafe impl GlobalAlloc for ProbeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

pub fn measured<T>(run: impl FnOnce() -> T) -> (T, usize) {
    let (result, count, _) = measured_with_bytes(run);
    (result, count)
}

pub fn measured_with_bytes<T>(run: impl FnOnce() -> T) -> (T, usize, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNT.with(|count| count.set(None));
            BYTES.with(|bytes| bytes.set(None));
        }
    }
    COUNT.with(|count| assert!(count.replace(Some(0)).is_none()));
    BYTES.with(|bytes| assert!(bytes.replace(Some(0)).is_none()));
    let reset = Reset;
    let result = run();
    let count = COUNT.with(|count| count.get().unwrap());
    let bytes = BYTES.with(|bytes| bytes.get().unwrap());
    drop(reset);
    (result, count, bytes)
}
