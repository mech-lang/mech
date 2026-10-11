// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Tom Kaitchuck

extern crate regex;

use std::io::{self, Read};
use std::thread;

macro_rules! regex { ($re:expr) => {
    ::regex::bytes::Regex::new($re).unwrap() 
} }

fn count_reverse_complements(sequence : Vec<u8>) -> Vec<String> {
    // Search for occurrences of the following patterns:
    let variants = vec![
        regex!("agggtaaa|tttaccct"),
        regex!("[cgt]gggtaaa|tttaccc[acg]"),
        regex!("a[act]ggtaaa|tttacc[agt]t"),
        regex!("ag[act]gtaaa|tttac[agt]ct"),
        regex!("agg[act]taaa|ttta[agt]cct"),
        regex!("aggg[acg]aaa|ttt[cgt]ccct"),
        regex!("agggt[cgt]aa|tt[acg]accct"),
        regex!("agggta[cgt]a|t[acg]taccct"),
        regex!("agggtaa[cgt]|[acg]ttaccct"),
    ];
    variants.iter()
	    .map(|ref variant| 
		format!("{} {}", 
			variant.to_string(), 
			variant.find_iter(&sequence).count()) ) 
            .collect()
}

fn find_replaced_sequence_length(sequence : Vec<u8>) -> usize {
    // Replace the following patterns, one at a time:
    let substs = vec![
        (regex!("tHa[Nt]"), &b"<4>"[..]),
        (regex!("aND|caN|Ha[DS]|WaS"), &b"<3>"[..]),
        (regex!("a[NSt]|BY"), &b"<2>"[..]),
        (regex!("<[^>]*>"), &b"|"[..]),
        (regex!("\\|[^|][^|]*\\|"), &b"-"[..]),
    ];
    let mut seq = sequence;
    // Perform the replacements in sequence:
    for (re, replacement) in substs {
        seq = re.replace_all(&seq, replacement).into_owned();
    }
    seq.len()
}

fn main() {
    let mut input = Vec::with_capacity(51 * (1 << 20));
    io::stdin().read_to_end(&mut input).unwrap();
    let input_len = input.len();
    let sequence = regex!(">[^\n]*\n|\n")
			.replace_all(&input, &b""[..]).into_owned();
    let clen = sequence.len();
    let seq_clone = sequence.clone();
    let result = thread::spawn(|| find_replaced_sequence_length(seq_clone) );
    let counts = thread::spawn(|| count_reverse_complements(sequence) );

    for variant in counts.join().unwrap() {
	println!("{}", variant)
    }
    println!("\n{}\n{}\n{:?}", input_len, clen, result.join().unwrap());
}
