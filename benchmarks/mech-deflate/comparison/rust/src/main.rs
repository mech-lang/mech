//! Raw DEFLATE library measurements, with fresh compressor state per call.
mod pinned_allocator;

use miniz_oxide::deflate::core::{
    CompressorOxide, TDEFLFlush, TDEFLStatus, compress, create_comp_flags_from_zip_params,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[global_allocator]
static ALLOCATOR: pinned_allocator::PinnedMimalloc = pinned_allocator::PinnedMimalloc;

fn compress_into(backend: &str, input: &[u8], output: &mut [u8]) -> usize {
    match backend {
        "miniz-oxide" => {
            // Level10 is this implementation's highest supported setting.
            // No zlib wrapper; compressor construction and destruction are timed.
            let flags = create_comp_flags_from_zip_params(10, 0, 0);
            let mut compressor = CompressorOxide::new(flags);
            let (status, consumed, produced) =
                compress(&mut compressor, input, output, TDEFLFlush::Finish);
            assert_eq!(status, TDEFLStatus::Done);
            assert_eq!(consumed, input.len());
            produced
        }
        "zlib-rs" => {
            // Negative window_bits selects raw DEFLATE, without a zlib header.
            let config = zlib_rs::DeflateConfig {
                level: 9,
                window_bits: -15,
                ..zlib_rs::DeflateConfig::default()
            };
            let (written, status) = zlib_rs::compress_slice(output, input, config);
            assert_eq!(status, zlib_rs::ReturnCode::Ok);
            written.len()
        }
        name => panic!("unknown backend {name}"),
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        7,
        "backend input prefix repetitions output reference"
    );
    let backend = &args[1];
    let input = std::fs::read(&args[2]).unwrap();
    let prefix = std::fs::read(&args[3]).unwrap();
    let repetitions: usize = args[4].parse().unwrap();
    assert!(repetitions > 0);
    let reference = std::fs::read(&args[6]).unwrap();
    // A conservative common bound for the two Rust APIs. Allocate and initialize
    // once before warmup; every compression writes into this same backing.
    let mut output = vec![0u8; zlib_rs::compress_bound(input.len()).max(64) + prefix.len()];
    output[..prefix.len()].copy_from_slice(&prefix);
    let pointer = output.as_ptr();
    let warm_started = Instant::now();
    let first = compress_into(backend, black_box(&input), &mut output[prefix.len()..]);
    let expected = output[..prefix.len() + first].to_vec();
    assert_eq!(
        miniz_oxide::inflate::decompress_to_vec(&expected[prefix.len()..]).unwrap(),
        input
    );
    while warm_started.elapsed() < Duration::from_millis(200) {
        let produced = compress_into(backend, black_box(&input), &mut output[prefix.len()..]);
        assert_eq!(produced, first);
        assert_eq!(&output[..prefix.len() + produced], expected);
    }
    for rep in 0..repetitions {
        let start = Instant::now();
        let produced = compress_into(backend, black_box(&input), &mut output[prefix.len()..]);
        let ns = start.elapsed().as_nanos();
        assert_eq!(produced, first);
        assert_eq!(&output[..prefix.len() + produced], expected);
        assert_eq!(output.as_ptr(), pointer);
        println!("rep\t{backend}\t{rep}\t{ns}\t{produced}");
    }
    std::fs::write(&args[5], &expected).unwrap();
    println!(
        "verified\t{backend}\t{}\t{}\t{}\t{}",
        input.len(),
        first,
        expected == reference,
        prefix.len()
    );
}
