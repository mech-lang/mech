// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Tom Kaitchuck
// contributed by Ryohei Machida

extern crate libc;
extern crate pcre2_sys;
extern crate rayon;

use crate::pcre2::Regex;
use rayon::prelude::*;
use std::cmp;
use std::io::{self, Read};
use std::mem;
use std::sync::mpsc;

mod pcre2 {
    use pcre2_sys::*;
    use std::ffi::c_void;
    use std::ptr;

    struct MatchData {
        match_context: *mut pcre2_match_context_8,
        match_data: *mut pcre2_match_data_8,
        jit_stack: *mut pcre2_jit_stack_8,
        ovector_ptr: *const usize,
    }

    impl MatchData {
        fn new(code: *mut pcre2_code_8) -> Self {
            let match_context =
                unsafe { pcre2_match_context_create_8(ptr::null_mut()) };
            assert!(
                !match_context.is_null(),
                "failed to allocate match context"
            );

            let match_data = unsafe {
                pcre2_match_data_create_from_pattern_8(code, ptr::null_mut())
            };
            assert!(
                !match_data.is_null(),
                "failed to allocate match data block"
            );

            let jit_stack = unsafe {
                pcre2_jit_stack_create_8(16384, 16384, ptr::null_mut())
            };
            assert!(!jit_stack.is_null(), "failed to allocate JIT stack");

            unsafe {
                pcre2_jit_stack_assign_8(
                    match_context,
                    None,
                    jit_stack as *mut c_void,
                )
            };

            let ovector_ptr =
                unsafe { pcre2_get_ovector_pointer_8(match_data) };
            assert!(!ovector_ptr.is_null(), "got NULL ovector pointer");

            MatchData { match_context, match_data, jit_stack, ovector_ptr }
        }
    }

    impl Drop for MatchData {
        fn drop(&mut self) {
            unsafe {
                pcre2_jit_stack_free_8(self.jit_stack);
                pcre2_match_data_free_8(self.match_data);
                pcre2_match_context_free_8(self.match_context);
            }
        }
    }

    pub struct Regex {
        pattern: &'static str,
        ctx: *mut pcre2_compile_context_8,
        code: *mut pcre2_code_8,
        match_data: MatchData,
    }

    impl Regex {
        pub fn new(pattern: &'static str) -> Regex {
            let ctx =
                unsafe { pcre2_compile_context_create_8(ptr::null_mut()) };
            assert!(!ctx.is_null(), "could not allocate compile context");

            // compile and generate ast
            let (mut error_code, mut error_offset) = (0, 0);
            let code = unsafe {
                pcre2_compile_8(
                    pattern.as_ptr(),
                    pattern.len(),
                    0,
                    &mut error_code,
                    &mut error_offset,
                    ctx,
                )
            };
            assert!(!code.is_null(), "Failed to compile pattern");

            // JIT compile
            let error_code =
                unsafe { pcre2_jit_compile_8(code, PCRE2_JIT_COMPLETE) };
            assert_eq!(
                error_code, 0,
                "Failed to JIT compile (error code: {:?})",
                error_code
            );

            Regex { pattern, ctx, code, match_data: MatchData::new(code) }
        }

        pub fn pattern(&self) -> &str {
            self.pattern
        }

        pub fn find_at<'s>(
            &self,
            subject: &'s [u8],
            start: usize,
        ) -> Option<(usize, usize)> {
            assert!(start <= subject.len());

            // pcre2_jit_match is 10-20% faster than pcre2_jit_match, but it
            // skips many sanity-checks and dangerous.
            // See https://github.com/BurntSushi/rust-pcre2/pull/17 for details.
            unsafe {
                let rc = pcre2_jit_match_8(
                    self.code,
                    subject.as_ptr(),
                    subject.len(),
                    start,
                    0,
                    self.match_data.match_data,
                    self.match_data.match_context,
                );

                if rc > 0 {
                    Some((
                        *self.match_data.ovector_ptr,
                        *self.match_data.ovector_ptr.add(1),
                    ))
                } else {
                    assert!(rc == -1, "matching error (error code: {:?})", rc);
                    None
                }
            }
        }

        pub fn count<'s>(&self, subject: &'s [u8]) -> usize {
            let mut count = 0;
            let mut last_match = 0;

            while let Some((_, e)) = self.find_at(subject, last_match) {
                count += 1;
                last_match = e;
            }

            count
        }

        pub fn replace<'s, 'a, 'o>(
            &self,
            subject: &'s [u8],
            alt: &'a [u8],
            out: &'o mut Vec<u8>,
        ) {
            let mut last_match = 0;

            while let Some((s, e)) = self.find_at(subject, last_match) {
                out.extend_from_slice(&subject[last_match..s]);
                out.extend_from_slice(alt);
                last_match = e;
            }

            out.extend_from_slice(&subject[last_match..]);
        }

        pub fn replace_inplace<'s, 'a>(
            &self,
            subject: &'s mut Vec<u8>,
            alt: &'a [u8],
        ) {
            let mut last_match = 0;
            let mut last_write = 0;

            while let Some((s, e)) = self.find_at(subject, last_match) {
                assert!(e - s >= alt.len());
                subject.copy_within(last_match..s, last_write);
                last_write += s - last_match;
                subject[last_write..last_write + alt.len()]
                    .copy_from_slice(alt);
                last_write += alt.len();
                last_match = e;
            }

            subject.copy_within(last_match.., last_write);
            subject.truncate(last_write + (subject.len() - last_match));
        }
    }

    impl Drop for Regex {
        fn drop(&mut self) {
            unsafe {
                pcre2_code_free_8(self.code);
                pcre2_compile_context_free_8(self.ctx);
            }
        }
    }

    // Regex matching causes mutation of match_data, so this Regex doesn't
    // implement Sync.
    unsafe impl Send for Regex {}
}

/// Get the number of bytes in the stdin socket
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
))]
#[inline]
fn stdin_size_hint() -> Option<usize> {
    use libc::{ioctl, FIONREAD, STDIN_FILENO};

    let mut len: libc::c_int = 0;
    if unsafe { ioctl(STDIN_FILENO, FIONREAD, &mut len as *mut _) } != -1 {
        Some(len as usize)
    } else {
        None
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
)))]
#[inline]
fn stdin_size_hint() -> Option<usize> {
    None
}

fn count_reverse_complements(sequence: mpsc::Receiver<Vec<u8>>) -> Vec<String> {
    // Search for occurrences of the following patterns:
    let variants = vec![
        Regex::new("agggtaaa|tttaccct"),
        Regex::new("[cgt]gggtaaa|tttaccc[acg]"),
        Regex::new("a[act]ggtaaa|tttacc[agt]t"),
        Regex::new("ag[act]gtaaa|tttac[agt]ct"),
        Regex::new("agg[act]taaa|ttta[agt]cct"),
        Regex::new("aggg[acg]aaa|ttt[cgt]ccct"),
        Regex::new("agggt[cgt]aa|tt[acg]accct"),
        Regex::new("agggta[cgt]a|t[acg]taccct"),
        Regex::new("agggtaa[cgt]|[acg]ttaccct"),
    ];
    let sequence = sequence.recv().unwrap();

    variants
        .into_par_iter()
        .map(|variant| {
            let count = variant.count(&*sequence);
            format!("{} {}", variant.pattern(), count)
        })
        .collect()
}

fn find_replaced_sequence_length(sequence: mpsc::Receiver<Vec<u8>>) -> usize {
    // Replace the following patterns, one at a time:
    let substs = vec![
        (Regex::new("tHa[Nt]"), &b"<4>"[..]),
        (Regex::new("aND|caN|Ha[DS]|WaS"), &b"<3>"[..]),
        (Regex::new("a[NSt]|BY"), &b"<2>"[..]),
        (Regex::new("<[^>]*>"), &b"|"[..]),
        (Regex::new("\\|[^|][^|]*\\|"), &b"-"[..]),
    ];

    let mut buf = sequence.recv().unwrap();

    substs[0].0.replace_inplace(&mut buf, substs[0].1);
    substs[1].0.replace_inplace(&mut buf, substs[1].1);

    {
        // the length of tmp will be at most 1.5 * buf.len() bytes because
        // substs[2] replaces two characters with triple characters.
        let mut tmp = Vec::with_capacity(buf.len() * 3 / 2);
        substs[2].0.replace(&buf, substs[2].1, &mut tmp);
        mem::swap(&mut buf, &mut tmp);
    }

    substs[3].0.replace_inplace(&mut buf, substs[3].1);
    substs[4].0.replace_inplace(&mut buf, substs[4].1);

    buf.len()
}

// allocate at least 2 pages
const MALLOC_OVERHEAD: usize = 16;
const MIN_ALLOC_SIZE: usize = 4096 * 2 - MALLOC_OVERHEAD;

fn main() {
    let mut input_len = 0;
    let mut sequence_len = 0;
    let mut result = 0;
    let mut counts = Vec::new();

    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();

    rayon::scope(|s| {
        let input_len = &mut input_len;
        let sequence_len = &mut sequence_len;
        let result = &mut result;
        let counts = &mut counts;

        s.spawn(move |_| {
            let mut capacity =
                stdin_size_hint().map_or(MIN_ALLOC_SIZE, |s| s + 1);
            capacity = cmp::max(capacity, MIN_ALLOC_SIZE);

            let mut input = Vec::with_capacity(capacity);
            io::stdin().read_to_end(&mut input).unwrap();
            *input_len = input.len();

            Regex::new(">[^\n]*\n|\n").replace_inplace(&mut input, b"");

            *sequence_len = input.len();

            tx1.send(input.clone()).unwrap();
            tx2.send(input).unwrap();
        });

        s.spawn(move |_| {
            *result = find_replaced_sequence_length(rx1);
        });

        s.spawn(move |_| {
            *counts = count_reverse_complements(rx2);
        })
    });

    for variant in counts {
        println!("{}", variant)
    }
    println!("\n{}\n{}\n{:?}", input_len, sequence_len, result);
}
