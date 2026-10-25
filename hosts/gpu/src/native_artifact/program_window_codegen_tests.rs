use super::*;
use cranelift_module::default_libcall_names;
use std::{hint::black_box, time::Instant};

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;

#[path = "program_window_histogram_reference.rs"]
mod histogram_reference;
#[path = "program_window_histogram_tests.rs"]
mod histogram_tests;
struct Compiled {
    entry: Entry,
    _module: cranelift_jit::JITModule,
    ir: String,
}
struct AsciiSource<'a>(&'a str);
impl<'a> AsciiSource<'a> {
    fn borrow(source: &'a str) -> Option<Self> {
        (source.is_ascii() && !source.as_bytes().windows(2).any(|pair| pair == b"\r\n"))
            .then_some(Self(source))
    }
}
impl Compiled {
    fn new() -> Self {
        Self::build(false)
    }
    fn scalar() -> Self {
        Self::build(true)
    }
    fn build(force_scalar: bool) -> Self {
        Self::build_region(force_scalar, false)
    }
    fn histogram(reference: bool) -> Self {
        Self::build_region(reference, true)
    }
    fn build_region(reference: bool, histogram: bool) -> Self {
        let catalog = mech_stdlib::source_catalog();
        let mut compiler = mech_runtime::RuntimeBuilder::new()
            .function_catalog(catalog.clone())
            .build_compiler()
            .unwrap();
        let source = if histogram {
            include_str!("../../tests/fixtures/native/window-histogram.mec")
        } else {
            include_str!("../../tests/fixtures/native/window-query.mec")
        };
        let product = compiler.compile_canonical_source(source).unwrap();
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            product.artifact(),
            &catalog,
        )
        .unwrap();
        let regions = if histogram {
            plan.window_histogram_regions()
        } else {
            plan.window_query_regions()
        };
        assert_eq!(regions.len(), 1);
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("preserve_frame_pointers", "true")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let vectorized =
            module.isa().name() == "aarch64" && module.isa().endianness() == Endianness::Little;
        let (id, ir, _, _) = if histogram && reference {
            define_with(&mut module, &regions[0], histogram_reference::emit)
        } else if reference {
            define_impl(&mut module, &regions[0], false)
        } else {
            define_impl(&mut module, &regions[0], vectorized)
        }
        .unwrap();
        assert!(
            !ir.contains("call ") && !ir.contains("call_indirect"),
            "{ir}"
        );
        module.finalize_definitions().unwrap();
        // SAFETY: define emits precisely this five-word/read-only signature;
        // this owner retains the generated code through every call.
        let entry =
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        Self {
            entry,
            _module: module,
            ir,
        }
    }
    fn count(&self, source: &AsciiSource<'_>, width: u64, query: &str) -> u64 {
        let source = source.0;
        // SAFETY: both immutable String borrows outlive the synchronous
        // call. The source representation guard proves byte/grapheme identity.
        // Generated bounds check width and each source/query load; no pointer
        // or allocation escapes. This wrapper is test-only, NOT a runtime route.
        unsafe {
            (self.entry)(
                source.as_ptr() as u64,
                source.len() as u64,
                query.as_ptr() as u64,
                query.len() as u64,
                width,
            )
        }
    }

    fn histogram_count(&self, source: &AsciiSource<'_>, width: u64, bins: &mut [u64]) -> u64 {
        assert!(width <= 2);
        let source = source.0;
        let active = width > 0 && width <= source.len() as u64;
        let cells = if active { 1usize << (width * 7) } else { 0 };
        assert!(
            bins.len() == cells || (matches!(width, 1 | 2) && active && bins.len() == 4 * cells)
        );
        // SAFETY: this is the exact emitted five-word signature. The code
        // owner and immutable ASCII/no-CRLF String outlive the call. The
        // actual initialized mutable bins slice is disjoint from that String
        // and has every possible base-128 cell, optionally in four disjoint
        // banks for width one/two. Its actual extent selects the emitted variant.
        // Source/load bounds are proved
        // by active starts; neither loan escapes. Callers zero bins outside
        // timing. This test wrapper is not a runtime route or admission permit.
        unsafe {
            (self.entry)(
                source.as_ptr() as u64,
                source.len() as u64,
                bins.as_mut_ptr() as u64,
                bins.len() as u64,
                width,
            )
        }
    }
}
fn reference(source: &str, width: u64, query: &str) -> u64 {
    if width == 0 || width > source.len() as u64 {
        return 0;
    }
    source
        .as_bytes()
        .windows(width as usize)
        .filter(|word| *word == query.as_bytes())
        .count() as u64
}

#[test]
fn compiled_window_query_loop_keeps_overlaps_and_has_no_runtime_callbacks() {
    let compiled = Compiled::new();
    for source in ["", "A", "AAAAA", "GGTAGGTATTGGT", "a\0a\0a", "\rA\n\r"] {
        let loan = AsciiSource::borrow(source).unwrap();
        for width in [0, 1, 2, 3, 4, 6, 12, 18, u64::MAX] {
            for query in ["", "A", "AA", "GGT", "GGTATT", "a\0", "é"] {
                assert_eq!(
                    compiled.count(&loan, width, query),
                    reference(source, width, query),
                    "source={source:?} width={width} query={query:?}"
                );
            }
        }
    }
    for source in ["\r\n", "é", "🧑‍🚀", "A\r\nB"] {
        assert!(
            AsciiSource::borrow(source).is_none(),
            "not a byte/grapheme-equivalent representation"
        );
    }
}

#[test]
fn compiled_query_words_cover_short_tails_wide_prefixes_and_unaligned_bounds() {
    let compiled = Compiled::new();
    for len in [
        0, 1, 2, 3, 6, 7, 8, 9, 15, 16, 17, 18, 19, 31, 32, 33, 63, 64, 65, 4095, 4096, 4097,
    ] {
        for byte in [0, b'A', 127] {
            let storage = String::from_utf8(vec![byte; len + 7]).unwrap();
            for alignment in 0..8 {
                let source = &storage[alignment..alignment + len];
                let loan = AsciiSource::borrow(source).unwrap();
                for width in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16, 18, 19, 32, 33, 65] {
                    let query = String::from_utf8(vec![byte; width]).unwrap();
                    assert_eq!(
                        compiled.count(&loan, width as u64, &query),
                        reference(source, width as u64, &query),
                        "len={len} alignment={alignment} byte={byte} width={width}"
                    );
                    if width > 0 {
                        let mut miss = query.into_bytes();
                        // Match the entire word prefix but differ at the last
                        // byte: the wide path must still compare its remainder.
                        miss[width - 1] ^= 1;
                        let miss = String::from_utf8(miss).unwrap();
                        assert_eq!(
                            compiled.count(&loan, width as u64, &miss),
                            reference(source, width as u64, &miss)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn compiled_query_word_scan_matches_arbitrary_ascii_and_query_lengths() {
    let compiled = Compiled::new();
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for case in 0..512usize {
        let len = case % 131;
        let bytes = (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let byte = (state & 127) as u8;
                // Only CRLF pairs, not arbitrary ASCII, invalidate the loan.
                if byte == b'\r' { b'\n' } else { byte }
            })
            .collect::<Vec<_>>();
        let source = String::from_utf8(bytes).unwrap();
        let loan = AsciiSource::borrow(&source).unwrap();
        for width in 1..=35usize {
            let start = if width <= len {
                case % (len - width + 1)
            } else {
                0
            };
            let query = if width <= len {
                &source[start..start + width]
            } else {
                ""
            };
            for supplied_width in [0, width as u64, width as u64 + 1, u64::MAX] {
                assert_eq!(
                    compiled.count(&loan, supplied_width, query),
                    reference(&source, supplied_width, query),
                    "case={case} width={supplied_width} query={query:?}"
                );
            }
            // A non-ASCII key must not become an ASCII match merely because
            // its byte length equals the native width.
            assert_eq!(compiled.count(&loan, 2, "é"), 0);
        }
    }
}

#[test]
fn vector_queries_match_scalar_and_reference_at_every_batch_and_word_boundary() {
    let compiled = Compiled::new();
    let scalar = Compiled::scalar();
    assert!(!scalar.ir.contains("shuffle"));
    assert!(!scalar.ir.contains("swizzle"));
    if cfg!(all(target_arch = "aarch64", target_endian = "little")) {
        assert!(compiled.ir.contains("swizzle") && compiled.ir.contains("i64x2"));
        assert!(!compiled.ir.contains("shuffle"));
        assert!(compiled.ir.contains("load.i8x16"));
    }
    for ir in [&compiled.ir, &scalar.ir] {
        assert!(!ir.contains("call ") && !ir.contains("call_indirect"));
        assert!(!ir.contains("notrap") && !ir.contains("aligned") && !ir.contains("can_move"));
    }
    for len in 0..=96usize {
        let storage = (0..len + 7)
            .map(|i| [0, b'A', b'Z', 127, b'A'][i % 5])
            .collect::<Vec<_>>();
        let storage = String::from_utf8(storage).unwrap();
        for alignment in 0..8 {
            let source = &storage[alignment..alignment + len];
            let loan = AsciiSource::borrow(source).unwrap();
            for width in 0..=40usize {
                let query = if width > 0 && width <= len {
                    &source[len - width..]
                } else {
                    ""
                };
                let expected = reference(source, width as u64, query);
                assert_eq!(
                    compiled.count(&loan, width as u64, query),
                    expected,
                    "vector len={len} alignment={alignment} width={width}"
                );
                assert_eq!(scalar.count(&loan, width as u64, query), expected);
            }
        }
    }
}

#[test]
fn vector_wide_prefix_hits_keep_all_lanes_dense_overlaps_and_late_mismatches() {
    let compiled = Compiled::new();
    let scalar = Compiled::scalar();
    for width in [8usize, 9, 12, 15, 16, 17, 18, 24, 32, 65, 140] {
        let source = "A".repeat(512);
        let query = "A".repeat(width);
        let loan = AsciiSource::borrow(&source).unwrap();
        let expected = (source.len() - width + 1) as u64;
        assert_eq!(compiled.count(&loan, width as u64, &query), expected);
        assert_eq!(scalar.count(&loan, width as u64, &query), expected);
        for lane in 0..8 {
            let mut source = source.as_bytes().to_vec();
            source[128 + lane + width - 1] = b'B';
            let source = String::from_utf8(source).unwrap();
            let mut query = query.as_bytes().to_vec();
            query[width - 1] = b'B';
            let query = String::from_utf8(query).unwrap();
            let loan = AsciiSource::borrow(&source).unwrap();
            assert_eq!(reference(&source, width as u64, &query), 1);
            assert_eq!(
                compiled.count(&loan, width as u64, &query),
                1,
                "width={width} hit_lane={lane}"
            );
            assert_eq!(scalar.count(&loan, width as u64, &query), 1);
        }
    }
}

#[test]
#[ignore = "manual machine-code kernel diagnostic; NOT full secure program timings"]
fn measure_compiled_window_query_kernel_only() {
    let input_path =
        std::env::var("MECH_WINDOW_KERNEL_INPUT").expect("explicit diagnostic input required");
    let oracle_path =
        std::env::var("MECH_WINDOW_KERNEL_ORACLE").expect("explicit exact oracle required");
    let input = std::fs::read_to_string(input_path).unwrap();
    let oracle = std::fs::read_to_string(oracle_path).unwrap();
    // Diagnostic setup ONLY. This is deliberately outside every timed call;
    // it is NOT claimed to measure FASTA selection/case conversion, hosting,
    // accounting, snapshot ownership, output or round/commit overhead.
    let mut selected = false;
    let mut source = String::new();
    for line in input.lines() {
        if line.starts_with('>') {
            selected = line.starts_with(">THREE");
        } else if selected {
            source.push_str(line);
        }
    }
    source.make_ascii_uppercase();
    assert_eq!(source.len(), 10_000_000);
    let compiled = Compiled::new();
    let scalar = Compiled::scalar();
    let source_loan = AsciiSource::borrow(&source).unwrap();
    let queries = [
        "GGT",
        "GGTA",
        "GGTATT",
        "GGTATTTTAATT",
        "GGTATTTTAATTTATAGT",
    ];
    let expected = queries.map(|query| {
        oracle
            .lines()
            .find_map(|line| {
                let (count, label) = line.split_once('\t')?;
                (label == query).then(|| count.parse::<u64>().unwrap())
            })
            .unwrap()
    });
    for (query, wanted) in queries.iter().zip(expected) {
        assert_eq!(
            compiled.count(&source_loan, query.len() as u64, query),
            wanted
        );
        assert_eq!(
            scalar.count(&source_loan, query.len() as u64, query),
            wanted
        );
    }
    let mut rows = Vec::new();
    for round in 0..7 {
        for (index, query) in queries.iter().enumerate() {
            let variants = if round % 2 == 0 {
                [("selected", &compiled), ("scalar", &scalar)]
            } else {
                [("scalar", &scalar), ("selected", &compiled)]
            };
            for (lowering, code) in variants {
                let start = Instant::now();
                let count = code.count(
                    black_box(&source_loan),
                    query.len() as u64,
                    black_box(query),
                );
                let elapsed_ns = start.elapsed().as_nanos();
                assert_eq!(count, expected[index]);
                rows.push(serde_json::json!({"round":round,"query":query,"lowering":lowering,"count":count,"elapsed_ns":elapsed_ns}));
            }
        }
    }
    eprintln!(
        "WINDOW_KERNEL_DIAGNOSTIC {}",
        serde_json::json!({
        "scope":"isolated shared-emitter kernel; ASCII loan validated once outside samples; NOT secure end-to-end performance or recovered-original-body qualification",
        "source_bases":source.len(),"object_opt_level":"speed","samples":rows,
        "comparison":"same executable and input; target-selected lowering versus scalar; alternating order; excludes host/capabilities and is NOT full-program timing",
        "runtime_callbacks_in_emitted_body":0,"production_wired":true})
    );
}
