use mech_gpu::native_artifact::compact_runtime::{Heap, NativeStatus, OwnerStorage, View};
#[path = "pinned_allocator.rs"]
mod pinned_allocator;
#[global_allocator]
static ALLOCATOR: pinned_allocator::PinnedMimalloc = pinned_allocator::PinnedMimalloc;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::VecDeque, ffi::c_void, time::{Duration, Instant}};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Vm {
    faults: u64,
    pageins: u64,
    cow_faults: u64,
    unix_calls: u64,
    mach_calls: u64,
}

impl Vm {
    fn read() -> Self {
        let mut value = Self::default();
        unsafe { probe_vm_read(&mut value) };
        value
    }

    fn delta(self, before: Self) -> Value {
        json!({"faults": self.faults.wrapping_sub(before.faults) & 0xffffffff,
            "pageins": self.pageins.wrapping_sub(before.pageins) & 0xffffffff,
            "cow_faults": self.cow_faults.wrapping_sub(before.cow_faults) & 0xffffffff,
            "unix_calls": self.unix_calls.wrapping_sub(before.unix_calls) & 0xffffffff,
            "mach_calls": self.mach_calls.wrapping_sub(before.mach_calls) & 0xffffffff})
    }
}

#[repr(C)]
#[derive(Default)]
struct CAlloc { requests: u64, requested_bytes: u64, releases: u64 }

unsafe extern "C" {
    fn mech_compact_closed_v2(context: *mut c_void, input: *const u64, output: *mut u64) -> NativeStatus;
    fn probe_vm_read(out: *mut Vm);
    fn probe_page_bytes() -> u64;
    fn probe_alloc_reset();
    fn probe_alloc_read(out: *mut CAlloc);
    fn probe_c_malloc(bytes: usize) -> *mut c_void;
    fn probe_c_free(pointer: *mut c_void);
    fn libdeflate_set_memory_allocator(alloc: unsafe extern "C" fn(usize) -> *mut c_void,
                                      free: unsafe extern "C" fn(*mut c_void));
    fn libdeflate_alloc_compressor(level: i32) -> *mut c_void;
    fn libdeflate_free_compressor(compressor: *mut c_void);
    fn libdeflate_deflate_compress_bound(compressor: *mut c_void, bytes: usize) -> usize;
    fn libdeflate_deflate_compress(compressor: *mut c_void, input: *const u8, bytes: usize,
                                  output: *mut u8, capacity: usize) -> usize;
}

#[derive(Deserialize)]
struct Case { name: String, input: String, prefix: String, expected: String, capacity: usize }

fn ns(later: Instant, earlier: Instant) -> u64 {
    later.duration_since(earlier).as_nanos().try_into().unwrap()
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 6);
    let actor = &args[0];
    let metadata: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    let case = cases.iter().find(|c| c.name == args[3]).unwrap();
    let samples: usize = args[4].parse().unwrap();
    assert!(samples >= 1);
    let input = std::fs::read(&case.input).unwrap();
    let original_input = input.clone();
    let prefix = std::fs::read(&case.prefix).unwrap();
    let expected = std::fs::read(&case.expected).unwrap();
    let mut borrowed = Vec::with_capacity(case.capacity);
    borrowed.extend_from_slice(&prefix);
    let mut arguments = vec![0; metadata["input_words"].as_u64().unwrap() as usize];
    assert_eq!(metadata["output_words"], 6);
    for port in metadata["inputs"].as_array().unwrap() {
        let name = mech_engine::decode_source_input_name(port["name"].as_str().unwrap()).unwrap();
        let at = port["input_offset"].as_u64().unwrap() as usize;
        let data = match name.as_str() { "input" => &input, "output" => &borrowed, _ => panic!() };
        arguments[at..at+6].copy_from_slice(&[data.as_ptr() as u64, data.len() as u64,
            data.capacity() as u64, 1, data.len() as u64, 0]);
    }
    unsafe { libdeflate_set_memory_allocator(probe_c_malloc, probe_c_free) };
    let calibration: Vec<_> = (0..512).map(|_| {
        let start = Instant::now();
        start.elapsed().as_nanos() as u64
    }).collect();
    println!("{}", json!({"calibration_empty_clock_ns": calibration,
        "page_bytes": unsafe { probe_page_bytes() }, "actor": actor, "case": case.name}));
    // Retain the final ten warmup records, then every measured record. Every
    // warmup also constructs and retires a fresh context; no context is reused.
    let mut first_output: Option<Vec<u8>> = None;
    let warmup_start = Instant::now();
    let mut warmup_records = VecDeque::with_capacity(10);
    let (mut sample, mut measured, mut warming) = (0usize, 0usize, true);
    while warming || measured < samples {
        let is_warmup = warming;
        let mut output: View = [0; 6];
        #[cfg(setup_probe)]
        let mut probe = mech_gpu::native_artifact::compact_runtime::setup_probe::Probe::default();
        #[cfg(setup_probe)]
        probe.reserve();
        unsafe { probe_alloc_reset() };
        let before = Vm::read();
        let (returned, phases, counts, events, after) = if actor == "libdeflate" {
            let start = Instant::now();
            let compressor = unsafe { libdeflate_alloc_compressor(12) };
            assert!(!compressor.is_null());
            let context_ready = Instant::now();
            let bound = unsafe { libdeflate_deflate_compress_bound(compressor, input.len()) };
            let capacity = case.capacity.max(prefix.len() + bound);
            let mut result = Vec::<u8>::with_capacity(capacity);
            result.extend_from_slice(&prefix);
            let prepared = Instant::now();
            let size = unsafe { libdeflate_deflate_compress(compressor, input.as_ptr(), input.len(),
                result.as_mut_ptr().add(prefix.len()), result.capacity()-prefix.len()) };
            assert!(size > 0);
            unsafe { result.set_len(prefix.len()+size) };
            let encoded = Instant::now();
            unsafe { libdeflate_free_compressor(compressor) };
            let cleaned = Instant::now();
            let after = Vm::read();
            let mut allocations = CAlloc::default();
            unsafe { probe_alloc_read(&mut allocations) };
            let phases = json!({"context_construction": ns(context_ready, start),
                "output_preparation": ns(prepared, context_ready), "source_call": ns(encoded, prepared),
                "context_cleanup": ns(cleaned, encoded), "total": ns(cleaned, start)});
            let counts = json!({"context_allocation_requests": allocations.requests,
                "context_requested_bytes": allocations.requested_bytes, "context_releases": allocations.releases,
                "output_allocation_requests": 1, "output_requested_bytes": result.capacity()});
            (result, phases, counts, Vec::<Value>::new(), after)
        } else {
            let start = Instant::now();
            let mut storage = OwnerStorage::default();
            let mut heap = Heap::with_owner_storage(&mut storage);
            #[cfg(setup_probe)]
            { heap.setup_probe = probe; }
            let constructed = Instant::now();
            let status = unsafe { mech_compact_closed_v2((&mut heap as *mut Heap).cast(),
                arguments.as_ptr(), output.as_mut_ptr()) };
            let encoded = Instant::now();
            assert_eq!(status.status, 0, "{}: status {} origin {}", case.name, status.status, status.origin);
            let result = unsafe { heap.take_u8(output) }.unwrap();
            let transferred = Instant::now();
            let raw_counts = [heap.counters.payload_requests,
                heap.counters.requested_payload_bytes, heap.counters.zeroed_bytes,
                heap.counters.initialized_bytes_copied, heap.counters.peak_payload_bytes,
                heap.counters.live_payload_bytes];
            assert_eq!(heap.counters.live_payload_bytes, 0);
            #[cfg(setup_probe)]
            let raw_events = std::mem::take(&mut heap.setup_probe.events);
            drop(heap);
            let cleaned = Instant::now();
            let after = Vm::read();
            let counts = json!({"payload_allocation_requests": raw_counts[0],
                "requested_payload_bytes": raw_counts[1], "requested_zero_initialized_bytes": raw_counts[2],
                "initialized_bytes_copied": raw_counts[3], "peak_payload_bytes": raw_counts[4],
                "nonreturned_private_bytes_at_return": raw_counts[5]});
            #[cfg(setup_probe)]
            let events: Vec<_> = raw_events.into_iter().map(|e| json!({"op":e.op,
                "pointer":e.pointer,"bytes":e.bytes,"stride":e.stride,"zeroed":e.zeroed,"mapped":e.mapped,
                "total_ns":e.total_ns,"payload_ns":e.payload_ns})).collect();
            #[cfg(not(setup_probe))]
            let events = Vec::<Value>::new();
            let phases = json!({"heap_construction": ns(constructed, start),
                "source_call": ns(encoded, constructed), "final_transfer": ns(transferred, encoded),
                "heap_cleanup_and_read_counters": ns(cleaned, transferred), "total": ns(cleaned, start)});
            (result, phases, counts, events, after)
        };
        assert_eq!(input, original_input);
        assert_eq!(borrowed, prefix);
        assert_ne!(returned.as_ptr(), borrowed.as_ptr());
        assert_eq!(&returned[..prefix.len()], prefix);
        assert_eq!(miniz_oxide::inflate::decompress_to_vec(&returned[prefix.len()..]).unwrap(), input);
        if actor != "libdeflate" {
            assert_eq!(returned, expected);
            assert_eq!(returned.capacity(), case.capacity);
        }
        if let Some(first) = &first_output { assert_eq!(&returned, first); }
        else {
            std::fs::write(&args[5], &returned).unwrap();
            first_output = Some(returned.clone());
        }
        let value = json!({"actor":actor,"case":case.name,"sample":sample,"warmup":is_warmup,
            "phases_ns":phases,"counts":counts,"vm_delta":after.delta(before),"events":events,
            "exact_stable_bytes":true,"independent_inflation":true,"borrowed_aliases_unchanged":true,
            "returned_capacity":returned.capacity(),"raw_bytes":returned.len()-prefix.len()});
        if is_warmup {
            if warmup_records.len() == 10 { warmup_records.pop_front(); }
            warmup_records.push_back(value);
            if sample >= 9 && warmup_start.elapsed() >= Duration::from_millis(200) {
                warming = false;
                for record in warmup_records.drain(..) { println!("{record}"); }
            }
        } else {
            println!("{value}");
            measured += 1;
        }
        drop(returned);
        sample += 1;
    }
}
