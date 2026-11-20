use mech_gpu::native_artifact::compact_runtime::{Heap, NativeStatus, OwnerStorage};
#[path = "pinned_allocator.rs"]
mod pinned_allocator;
#[global_allocator]
static ALLOCATOR: pinned_allocator::PinnedMimalloc = pinned_allocator::PinnedMimalloc;
use serde::Deserialize;
use serde_json::json;
use std::{ffi::c_void, io::{BufRead, Read, Write}, time::{Duration, Instant}};

unsafe extern "C" {
    fn mech_compact_closed_v2(context: *mut c_void, input: *const u64, output: *mut u64) -> NativeStatus;
    fn replay_c_native(n: u32, freqs: *const u32, output: *mut u32) -> u32;
    fn replay_c_mech(n: u32, freqs: *const u32, output: *mut u32) -> u32;
    fn replay_reset_work();
    fn replay_read_work(output: *mut Work);
}

#[repr(C)]
#[derive(Default)]
struct Work { frequency_reads: u64, counter_updates: u64, key_comparisons: u64, output_writes: u64 }

#[derive(Deserialize)]
struct Vector { freqs: Vec<u32>, expected: Vec<u32> }
#[derive(Deserialize)]
struct Case { name: String, vectors: Vec<Vector> }

const CANARY: u32 = 0xc0defade;
#[derive(Clone)]
struct Result { words: [u32; 288], used: usize, allocations: u64 }
impl Default for Result {
    fn default() -> Self { Self { words: [CANARY; 288], used: usize::MAX, allocations: 0 } }
}

fn sort(actor: &str, vector: &Vector, result: &mut Result) {
    if actor.starts_with("c-") {
        result.used = unsafe {
            match actor {
                "c-native" => replay_c_native(vector.freqs.len() as u32, vector.freqs.as_ptr(), result.words.as_mut_ptr()),
                "c-mech" => replay_c_mech(vector.freqs.len() as u32, vector.freqs.as_ptr(), result.words.as_mut_ptr()),
                _ => panic!(),
            }
        } as usize;
        return;
    }
    assert!(matches!(actor, "mech-shell" | "mech-bucket"));
    let mut storage = OwnerStorage::default();
    let mut heap = Heap::with_owner_storage(&mut storage);
    let args = [vector.freqs.as_ptr() as u64, vector.freqs.len() as u64,
        vector.freqs.capacity() as u64, 1, vector.freqs.len() as u64, 0];
    let mut output = [0u64; 7];
    let status = unsafe { mech_compact_closed_v2((&mut heap as *mut Heap).cast(), args.as_ptr(), output.as_mut_ptr()) };
    assert_eq!(status.status, 0, "origin {}", status.origin);
    assert_eq!(output[1], vector.freqs.len() as u64);
    result.used = output[6].try_into().unwrap();
    assert!(result.used <= vector.freqs.len());
    unsafe { std::ptr::copy_nonoverlapping(output[0] as *const u32, result.words.as_mut_ptr(), result.used) };
    result.allocations = heap.counters.payload_requests;
    // Results are copied to caller storage before the private heap retires.
    // This is the standalone phase's complete context-to-cleanup boundary.
    drop(heap);
}

fn validate(vector: &Vector, result: &Result) {
    assert_eq!(result.used, vector.expected.len());
    assert_eq!(&result.words[..result.used], vector.expected);
    assert!(result.words[result.used..].iter().all(|&v| v == CANARY));
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 3);
    let metadata: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[0]).unwrap()).unwrap();
    assert_eq!(metadata["input_words"], 6);
    assert_eq!(metadata["output_words"], 7);
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let actor = &args[2];
    for case in &cases {
        for vector in &case.vectors {
            assert!((2..=288).contains(&vector.freqs.len()));
            assert!(vector.freqs.iter().map(|&x| x as u64).sum::<u64>() < 1<<22);
            let mut expected: Vec<_> = vector.freqs.iter().enumerate().filter(|(_, f)| **f > 0)
                .map(|(i, &f)| (f << 10) | i as u32).collect();
            expected.sort_unstable();
            assert_eq!(vector.expected, expected);
        }
    }
    println!("SESSION_READY");
    std::io::stdout().flush().unwrap();
    let mut reader = std::io::BufReader::new(std::io::stdin());
    loop {
        let mut command = String::new();
        assert!(reader.read_line(&mut command).unwrap() > 0);
        if command == "Q\n" { break; }
        let fields: Vec<_> = command.trim_end().split('\t').collect();
        assert_eq!(fields[0], "C");
        assert_eq!(fields.len(), 4);
        let case = &cases[fields[1].parse::<usize>().unwrap()];
        let repeats: usize = fields[2].parse().unwrap();
        let samples: usize = fields[3].parse().unwrap();
        let mut results = vec![Result::default(); repeats * case.vectors.len()];
        // Every vector is verified before timing and each measured result is
        // verified afterwards. Output storage preparation is excluded equally.
        for vector in &case.vectors {
            let mut result = Result::default();
            sort(actor, vector, &mut result);
            validate(vector, &result);
        }
        println!("CASE_READY\t{}", case.name);
        std::io::stdout().flush().unwrap();
        for sample in 0..samples {
            let mut action = [0u8; 1];
            reader.read_exact(&mut action).unwrap();
            if action == *b"T" {
                assert!(actor.starts_with("c-"));
                let mut records = Vec::new();
                for (index, vector) in case.vectors.iter().enumerate() {
                    unsafe { replay_reset_work() };
                    let mut result = Result::default();
                    sort(actor, vector, &mut result);
                    validate(vector, &result);
                    let mut work = Work::default();
                    unsafe { replay_read_work(&mut work) };
                    records.push(json!({"vector":index,"frequency_reads":work.frequency_reads,
                        "counter_updates":work.counter_updates,"key_comparisons":work.key_comparisons,
                        "output_writes":work.output_writes}));
                }
                println!("{}", json!({"case":case.name,"actor":actor,"work":records}));
            } else if action == *b"W" {
                let start = Instant::now();
                let mut calls = 0usize;
                while start.elapsed() < Duration::from_millis(200) {
                    for vector in &case.vectors {
                        let mut result = Result::default();
                        sort(actor, vector, &mut result);
                        validate(vector, &result);
                        calls += 1;
                    }
                }
                println!("{}", json!({"case":case.name,"actor":actor,"warmup_calls":calls,
                    "warmup_seconds":start.elapsed().as_secs_f64()}));
            } else {
                assert_eq!(action, *b"N");
                results.fill(Result::default());
                let start = Instant::now();
                for (repeat, block) in results.chunks_mut(case.vectors.len()).enumerate() {
                    // Rotate vector order so one vector does not always see a
                    // particular immediately preceding allocator state.
                    for offset in 0..case.vectors.len() {
                        let index = (repeat + offset) % case.vectors.len();
                        sort(actor, &case.vectors[index], &mut block[index]);
                    }
                }
                let nanoseconds = start.elapsed().as_nanos() as u64;
                for block in results.chunks(case.vectors.len()) {
                    for (vector, result) in case.vectors.iter().zip(block) { validate(vector, result); }
                }
                println!("{}", json!({"case":case.name,"actor":actor,"sample":sample,
                    "batch_nanoseconds":nanoseconds,"repeats":repeats,"vectors":case.vectors.len(),
                    "verified_calls":results.len(),"exact_sorted_keys":true,"canaries_unchanged":true,
                    "mech_payload_requests":results.iter().map(|r|r.allocations).sum::<u64>()}));
            }
            std::io::stdout().flush().unwrap();
        }
        println!("CASE_DONE\t{}", case.name);
        std::io::stdout().flush().unwrap();
    }
}
