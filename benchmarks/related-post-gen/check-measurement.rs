//! Check the all-thread allocator window and report empty-window overhead.
mod measurement;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::{Arc, Barrier};

#[global_allocator]
static ALLOCATOR: measurement::Tracked<System> = measurement::Tracked(System);

fn main() {
    let ready = Arc::new(Barrier::new(5));
    let allocated = Arc::new(Barrier::new(5));
    let release = Arc::new(Barrier::new(5));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (ready, allocated, release) = (ready.clone(), allocated.clone(), release.clone());
            std::thread::spawn(move || {
                ready.wait();
                let layout = Layout::from_size_align(1024, 16).unwrap();
                let pointers: [_; 3] = std::array::from_fn(|_| {
                    // SAFETY: each nonnull block has the matching size/alignment;
                    // workers retain the blocks until the release barrier.
                    let pointer = unsafe { ALLOCATOR.alloc(layout) };
                    assert!(!pointer.is_null());
                    unsafe { pointer.write_volatile(7) };
                    pointer
                });
                allocated.wait();
                release.wait();
                for pointer in pointers {
                    assert_eq!(unsafe { pointer.read_volatile() }, 7);
                    unsafe { ALLOCATOR.dealloc(pointer, layout) };
                }
            })
        })
        .collect();
    let window = measurement::Window::begin();
    ready.wait();
    allocated.wait();
    release.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    let measured = serde_json::to_value(window.finish()).unwrap();
    assert!(measured["allocation_requests"].as_u64().unwrap() >= 12);
    assert!(measured["requested_allocation_bytes"].as_u64().unwrap() >= 12 * 1024);
    assert!(
        measured["peak_additional_live_requested_heap_bytes"]
            .as_u64()
            .unwrap()
            >= 12 * 1024
    );
    assert_eq!(measured["allocator_lifetime_accounting_underflows"], 0);
    #[cfg(target_os = "macos")]
    assert!(
        measured["hardware"]["instructions_retired"]
            .as_u64()
            .unwrap()
            > 0
    );
    let overhead: Vec<_> = (0..7)
        .map(|_| measurement::Window::begin().finish())
        .collect();
    println!(
        "{}",
        serde_json::json!({"all_thread_allocation_check":measured,"empty_window_overhead":overhead,"status":"passed"})
    );
}
