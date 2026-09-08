use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static MAX_ALLOCATION: Cell<Option<usize>> = const { Cell::new(None) };
}

pub struct ProbeAllocator;

fn observe(size: usize) {
    let _ = MAX_ALLOCATION.try_with(|maximum| {
        if let Some(current) = maximum.get() {
            maximum.set(Some(current.max(size)));
        }
    });
}

// SAFETY: requests are forwarded unchanged to the system allocator. The
// thread-local probe records only requested sizes and never touches bytes.
unsafe impl GlobalAlloc for ProbeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        observe(size);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

pub fn maximum_requested<T>(run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            MAX_ALLOCATION.with(|maximum| maximum.set(None));
        }
    }

    MAX_ALLOCATION.with(|maximum| assert!(maximum.replace(Some(0)).is_none()));
    let reset = Reset;
    let result = run();
    let maximum = MAX_ALLOCATION.with(|value| value.get().unwrap());
    drop(reset);
    (result, maximum)
}
