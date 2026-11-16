use mech_gpu::native_artifact::compact_runtime::{Heap, NativeStatus, OwnerStorage, View};
#[path = "pinned_allocator.rs"]
mod pinned_allocator;
#[global_allocator]
static ALLOCATOR: pinned_allocator::PinnedMimalloc = pinned_allocator::PinnedMimalloc;
use serde::Deserialize;
use std::{
    io::{BufRead, Write},
    time::Instant,
};
unsafe extern "C" {
    fn mech_compact_closed_v2(
        context: *mut std::ffi::c_void,
        input: *const u64,
        output: *mut u64,
    ) -> NativeStatus;
}
#[derive(Deserialize)]
struct Case {
    name: String,
    input: String,
    prefix: String,
    expected: String,
    capacity: usize,
}
fn run(case: &Case, samples: usize, metadata: &serde_json::Value, reader: &mut impl BufRead) {
    let input = std::fs::read(&case.input).unwrap();
    let prefix = std::fs::read(&case.prefix).unwrap();
    let expected = std::fs::read(&case.expected).unwrap();
    assert_eq!(&expected[..prefix.len()], prefix);
    assert_eq!(
        miniz_oxide::inflate::decompress_to_vec(&expected[prefix.len()..]).unwrap(),
        input
    );
    assert_eq!(metadata["output_words"], 6);
    let mut current = Vec::with_capacity(case.capacity);
    current.extend_from_slice(&prefix);
    assert_eq!(current.capacity(), case.capacity);
    let mut arguments = vec![0; metadata["input_words"].as_u64().unwrap() as usize];
    let mut output: View = [0; 6];
    println!("CASE_READY\t{}", case.name);
    std::io::stdout().flush().unwrap();
    {
    let mut execute=|sample:usize| {
        // Match the caller's excluded clear/prefix/argument/output-slot setup.
        // Keep the borrowed prepared buffer alive throughout complete execution.
        current.clear();
        current.extend_from_slice(&prefix);
        assert_eq!(current.capacity(), case.capacity);
        output.fill(0);
        for port in metadata["inputs"].as_array().unwrap() {
            let name =
                mech_engine::decode_source_input_name(port["name"].as_str().unwrap()).unwrap();
            let at = port["input_offset"].as_u64().unwrap() as usize;
            let view = match name.as_str() {
                "input" => [
                    input.as_ptr() as u64,
                    input.len() as u64,
                    input.capacity() as u64,
                    1,
                    input.len() as u64,
                    0,
                ],
                "output" => [
                    current.as_ptr() as u64,
                    current.len() as u64,
                    current.capacity() as u64,
                    1,
                    current.len() as u64,
                    0,
                ],
                _ => panic!("unexpected source input {name}"),
            };
            arguments[at..at + 6].copy_from_slice(&view);
        }
        let input_alias = &input[..];
        let prior_pointer = current.as_ptr();
        let start = Instant::now();
        let mut storage = OwnerStorage::default();
        let mut heap = Heap::with_owner_storage(&mut storage);
        // SAFETY: all source ports are validated immutable initialized U8 views;
        // private growth/owners use the live local heap. The linked full Plan
        // checks its source arithmetic/assertions/bounds and returns six words.
        let status = unsafe {
            mech_compact_closed_v2(
                (&mut heap as *mut Heap).cast(),
                arguments.as_ptr(),
                output.as_mut_ptr(),
            )
        };
        let returned = if status.status == 0 {
            Some(unsafe { heap.take_u8(output) }.expect("sole final output ownership"))
        } else {
            None
        };
        let payload_requests = heap.counters.payload_requests;
        let requested_bytes = heap.counters.requested_payload_bytes;
        let copied = heap.counters.initialized_bytes_copied;
        let zeroed = heap.counters.zeroed_bytes;
        let peak = heap.counters.peak_payload_bytes;
        let residual = heap.counters.live_payload_bytes;
        drop(heap);
        let seconds = start.elapsed().as_secs_f64();
        assert_eq!(
            status.status, 0,
            "{}: compact status {} at origin {}",
            case.name, status.status, status.origin
        );
        assert_eq!(
            residual, 0,
            "all nonreturned source owners must retire before completion"
        );
        let returned = returned.unwrap();
        assert_eq!(returned, expected, "{}", case.name);
        assert_eq!(returned.capacity(), case.capacity);
        assert_eq!(
            miniz_oxide::inflate::decompress_to_vec(&returned[prefix.len()..]).unwrap(),
            input_alias
        );
        assert_eq!(current, prefix);
        assert_ne!(
            returned.as_ptr(),
            prior_pointer,
            "the live borrowed argument cannot be consumed"
        );
        current = returned;
        serde_json::json!({"case":case.name,"sample":sample,"warmup":sample==0,"profile":sample==samples+1,"seconds":seconds,"input_bytes":input.len(),"prefix_bytes":prefix.len(),"raw_bytes":expected.len()-prefix.len(),"capacity":case.capacity,"exact_reference":true,"independent_inflation":true,"input_alias_unchanged":true,"borrowed_output_alias_unchanged":true,"payload_allocation_requests":payload_requests,"requested_payload_bytes":requested_bytes,"initialized_bytes_copied":copied,"zeroed_bytes":zeroed,"peak_private_payload_bytes":peak,"nonreturned_private_bytes_at_return":residual,"boundary":"complete compact call plus actual heap construction, final allocation transfer and private cleanup; preparation/validation/output slots/drop excluded","contract":"Diagnostic. Ordinary exact resource admission/accounting/publication recovery omitted; not NoLimits or production entry."})
    };

        let mut sample=0usize;
        while sample<samples+2 {
            let mut command=[0u8;1];reader.read_exact(&mut command).unwrap();
            if command==*b"W" {
                let warm_start=Instant::now();let mut calls=0u64;
                while warm_start.elapsed()<std::time::Duration::from_millis(200) {
                    let value=execute(usize::MAX);drop(value);calls+=1;
                }
                println!("WARMUP_DONE\t{}\t{calls}\t{}",case.name,warm_start.elapsed().as_secs_f64());
            } else {
                assert_eq!(command,*b"N");let value=execute(sample);println!("{value}");sample+=1;
            }
            std::io::stdout().flush().unwrap();
        }
    }
    drop((input, prefix, expected, current, arguments));
    println!("CASE_DONE\t{}", case.name);
    std::io::stdout().flush().unwrap();
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 2);
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&args[0]).unwrap()).unwrap();
    assert_eq!(metadata["symbol"], "mech_compact_closed_v2");
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    println!("SESSION_READY");
    std::io::stdout().flush().unwrap();
    let mut reader = std::io::BufReader::new(std::io::stdin());
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        if line == "Q\n" {
            break;
        }
        let fields = line.trim_end().split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "C");
        let index: usize = fields[1].parse().unwrap();
        let samples: usize = fields[2].parse().unwrap();
        run(&cases[index], samples, &metadata, &mut reader);
    }
    eprintln!("CLEANUP private_payload_bytes=0 live_owner_records=0");
}
