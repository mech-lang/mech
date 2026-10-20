use super::*;
use cranelift_module::default_libcall_names;
use std::{hint::black_box, time::Instant};

type Entry = unsafe extern "C" fn(*const u8, u64, *const u8, u64, u64) -> u64;
struct Compiled {
    entry: Entry,
    _module: cranelift_jit::JITModule,
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
        let catalog = mech_stdlib::source_catalog();
        let mut compiler = mech_runtime::RuntimeBuilder::new()
            .function_catalog(catalog.clone())
            .build_compiler()
            .unwrap();
        let product = compiler
            .compile_canonical_source(include_str!("../../tests/fixtures/native/window-query.mec"))
            .unwrap();
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            product.artifact(),
            &catalog,
        )
        .unwrap();
        let regions = plan.window_query_regions();
        assert_eq!(regions.len(), 1);
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("preserve_frame_pointers", "true")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let (id, ir, _, _) = define(&mut module, &regions[0]).unwrap();
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
                source.as_ptr(),
                source.len() as u64,
                query.as_ptr(),
                query.len() as u64,
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
    }
    let mut rows = Vec::new();
    for round in 0..7 {
        for (index, query) in queries.iter().enumerate() {
            let start = Instant::now();
            let count = compiled.count(
                black_box(&source_loan),
                query.len() as u64,
                black_box(query),
            );
            let elapsed_ns = start.elapsed().as_nanos();
            assert_eq!(count, expected[index]);
            rows.push(serde_json::json!({"round":round,"query":query,"count":count,"elapsed_ns":elapsed_ns}));
        }
    }
    eprintln!(
        "WINDOW_KERNEL_DIAGNOSTIC {}",
        serde_json::json!({
        "scope":"isolated shared-emitter kernel; ASCII loan validated once outside samples; NOT secure end-to-end performance or recovered-original-body qualification",
        "source_bases":source.len(),"object_opt_level":"speed","samples":rows,
        "runtime_callbacks_in_emitted_body":0,"production_wired":true})
    );
}
