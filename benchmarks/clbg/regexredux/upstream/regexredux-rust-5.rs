// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Tom Kaitchuck
// contributed by Andre Bogus

extern crate crossbeam_utils;
extern crate regex;

use crossbeam_utils::thread::scope;
use regex::bytes::Regex;
use std::{
    borrow::Cow,
    io::{self, Read},
};

fn regex(s: &str) -> Regex {
    Regex::new(s).unwrap()
}

fn count_reverse_complements(sequence: &[u8]) -> String {
    // Search for occurrences of the following patterns:
    static VARIANTS: &[&str] = &[
        "agggtaaa|tttaccct",
        "[cgt]gggtaaa|tttaccc[acg]",
        "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct",
        "agg[act]taaa|ttta[agt]cct",
        "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct",
        "agggta[cgt]a|t[acg]taccct",
        "agggtaa[cgt]|[acg]ttaccct",
    ];
    VARIANTS
        .iter()
        .map(|variant| {
            format!(
                "{} {}\n",
                variant,
                regex(variant).find_iter(sequence).count()
            )
        })
        .collect()
}

fn find_replaced_sequence_length(sequence: &[u8]) -> usize {
    // Replace the following patterns, one at a time:
    static SUBSTS: &[(&str, &[u8])] = &[
        ("tHa[Nt]", b"<4>"),
        ("aND|caN|Ha[DS]|WaS", b"<3>"),
        ("a[NSt]|BY", b"<2>"),
        ("<[^>]*>", b"|"),
        ("\\|[^|][^|]*\\|", b"-"),
    ];
    let mut seq = Cow::Borrowed(sequence);
    // Perform the replacements in sequence:
    for (re, replacement) in SUBSTS.iter().cloned() {
        seq = Cow::Owned(regex(re).replace_all(&seq, replacement).into_owned());
    }
    seq.len()
}

fn main() {
    let mut input = Vec::with_capacity(51 * (1 << 20));
    io::stdin().read_to_end(&mut input).unwrap();
    let sequence = regex(">[^\n]*\n|\n").replace_all(&input, &b""[..]).into_owned();
    scope(|s| {
        let result = s.spawn(|_| find_replaced_sequence_length(&sequence));

        println!(
            "{}\n{}\n{}\n{}",
            count_reverse_complements(&sequence[..]),
            input.len(),
            sequence.len(),
            result.join().unwrap()
        );
    })
    .unwrap();
}
