//! Full-bin differential checks and an explicitly non-hosted kernel diagnostic.
use super::{AsciiSource, Compiled, Instant, black_box};

fn reference(source: &str, width: usize) -> Vec<u64> {
    if width == 0 || width > source.len() {
        return Vec::new();
    }
    let mut bins = vec![0; 1usize << (width * 7)];
    for word in source.as_bytes().windows(width) {
        let key = word
            .iter()
            .fold(0usize, |key, byte| (key << 7) | *byte as usize);
        bins[key] += 1;
    }
    bins
}

fn check(code: &Compiled, source: &str, width: usize, expected: &[u64]) {
    let source = AsciiSource::borrow(source).unwrap();
    let guard = 0xcafeba98_76543210;
    let mut storage = vec![0; expected.len() + 2];
    storage[0] = guard;
    storage[expected.len() + 1] = guard;
    let accepted = code.histogram_count(&source, width as u64, &mut storage[1..expected.len() + 1]);
    assert_eq!(accepted, expected.iter().sum::<u64>());
    assert_eq!(&storage[1..expected.len() + 1], expected);
    assert_eq!(storage[0], guard);
    assert_eq!(storage[expected.len() + 1], guard);
}

#[test]
fn histogram_batches_keep_every_tail_alignment_and_repeated_key_update() {
    let selected = Compiled::histogram(false);
    let scalar = Compiled::histogram(true);
    for ir in [&selected.ir, &scalar.ir] {
        assert!(!ir.contains("call ") && !ir.contains("call_indirect"));
        assert!(!ir.contains("aligned") && !ir.contains("notrap") && !ir.contains("can_move"));
    }
    for length in (0..=128).chain([255, 256, 257, 4095, 4096, 4097]) {
        for pattern in 0..3 {
            let storage = (0..length + 15)
                .map(|i| match pattern {
                    0 => 0,
                    1 => 127,
                    _ => ((i * 73 + 19) & 127) as u8,
                })
                .collect::<Vec<_>>();
            let storage = String::from_utf8(storage).unwrap();
            for alignment in 0..16 {
                let source = &storage[alignment..alignment + length];
                for width in 0..=2 {
                    let expected = reference(source, width);
                    check(&selected, source, width, &expected);
                    check(&scalar, source, width, &expected);
                }
            }
        }
    }
}

#[test]
fn histogram_batches_count_every_admissible_ascii_pair_and_dense_overlaps() {
    let selected = Compiled::histogram(false);
    let scalar = Compiled::histogram(true);
    let mut bytes = Vec::new();
    for a in 0..128u8 {
        for b in 0..128u8 {
            if (a, b) != (b'\r', b'\n') {
                bytes.extend_from_slice(&[a, b, 127]);
            }
        }
    }
    let all_pairs = String::from_utf8(bytes).unwrap();
    for source in [all_pairs, "X".repeat(65_537), "XY".repeat(32_769)] {
        for width in [1, 2] {
            let expected = reference(&source, width);
            check(&selected, &source, width, &expected);
            check(&scalar, &source, width, &expected);
        }
    }
    for source in ["\r\n", "a\r\nb", "é", "🧑‍🚀"] {
        assert!(AsciiSource::borrow(source).is_none());
    }
}

#[test]
#[ignore = "manual machine-code histogram diagnostic; NOT full secure program timings"]
fn measure_compiled_histogram_kernel_only() {
    let input_path = std::env::var("MECH_HISTOGRAM_KERNEL_INPUT").expect("explicit input required");
    let input = std::fs::read_to_string(input_path).unwrap();
    let mut selected_record = false;
    let mut source = String::new();
    for line in input.lines() {
        if line.starts_with('>') {
            selected_record = line.starts_with(">THREE");
        } else if selected_record {
            source.push_str(line);
        }
    }
    source.make_ascii_uppercase();
    assert_eq!(source.len(), 10_000_000);
    let loan = AsciiSource::borrow(&source).unwrap();
    let selected = Compiled::histogram(false);
    let scalar = Compiled::histogram(true);
    let mut samples = Vec::new();
    for width in [1usize, 2] {
        let expected = reference(&source, width);
        check(&selected, &source, width, &expected);
        check(&scalar, &source, width, &expected);
        let mut bins = vec![0; expected.len()];
        for round in 0..7 {
            let variants = if round % 2 == 0 {
                [("selected", &selected), ("scalar", &scalar)]
            } else {
                [("scalar", &scalar), ("selected", &selected)]
            };
            for (lowering, code) in variants {
                // Allocation,zeroing,source/oracle preparation and full-bin
                // comparisons are outside these kernel-only intervals.
                bins.fill(0);
                let start = Instant::now();
                let count =
                    code.histogram_count(black_box(&loan), width as u64, black_box(&mut bins));
                let elapsed_ns = start.elapsed().as_nanos();
                assert_eq!(bins, expected);
                assert_eq!(count, source.len() as u64 - width as u64 + 1);
                samples.push(serde_json::json!({"round":round,"width":width,"lowering":lowering,"count":count,"all_bins_exact":true,"elapsed_ns":elapsed_ns}));
            }
        }
    }
    eprintln!(
        "HISTOGRAM_KERNEL_DIAGNOSTIC {}",
        serde_json::json!({
            "scope":"Same executable/raw emitted kernel,NOT secured program timing or isolated capability overhead;source prep,allocation,zeroing,host,capabilities,output,accounting and commit excluded",
            "source_bases":source.len(),"samples":samples,"runtime_callbacks_in_emitted_body":0,
            "scratch_layout_changed":false,"object_opt_level":"speed","production_wired":true
        })
    );
}
