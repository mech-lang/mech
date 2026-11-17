//! Check the all-thread allocator window and report empty-window overhead.
mod measurement;
use std::alloc::System;
use std::hint::black_box;
use std::mem::MaybeUninit;
use std::sync::{Arc, Barrier};

#[global_allocator]
static ALLOCATOR: measurement::Tracked<System> = measurement::Tracked(System);

#[repr(C, align(16))]
struct TestBlock {
    marker: u8,
    _spare: [MaybeUninit<u8>; 1023],
}

const _: () = assert!(std::mem::size_of::<TestBlock>() == 1024);
const _: () = assert!(std::mem::align_of::<TestBlock>() == 16);

fn main() {
    let ready = Arc::new(Barrier::new(5));
    let allocated = Arc::new(Barrier::new(5));
    let release = Arc::new(Barrier::new(5));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (ready, allocated, release) = (ready.clone(), allocated.clone(), release.clone());
            std::thread::spawn(move || {
                ready.wait();
                let blocks: [_; 3] = std::array::from_fn(|_| {
                    black_box(Box::new(TestBlock {
                        marker: 7,
                        _spare: [MaybeUninit::uninit(); 1023],
                    }))
                });
                allocated.wait();
                release.wait();
                for block in blocks {
                    assert_eq!(black_box(&*block).marker, 7);
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
