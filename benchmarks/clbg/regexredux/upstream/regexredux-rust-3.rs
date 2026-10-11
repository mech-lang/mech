// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Francois Green

extern crate rayon;
extern crate regex;

use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{self, Read};
use std::thread;

macro_rules! regex { ($re:expr) => { ::regex::bytes::Regex::new($re).unwrap() } }

fn main() {
    let mut input = Vec::with_capacity(51 * (1 << 20));

    io::stdin().read_to_end(&mut input).unwrap();

    let sequence = regex!(">[^\n]*\n|\n").replace_all(&input, &b""[..]).into_owned();

    let sequence_c = sequence.clone();

    let result = thread::spawn(move|| {
        vec![
            ("tHa[Nt]", &b"<4>"[..]),
            ("aND|caN|Ha[DS]|WaS", &b"<3>"[..]),
            ("a[NSt]|BY", &b"<2>"[..]),
            ("<[^>]*>", &b"|"[..]),
            ("\\|[^|][^|]*\\|", &b"-"[..]),
        ].iter()
         .fold(sequence_c, |mut buffer, &(pattern, replacement)| {
             regex!(pattern).replace_all(&mut buffer, replacement).into_owned()
         })
    });

    let variants = vec![
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
    
    let results: HashMap<&str, usize> = variants.par_iter()
        .map(|v| (&**v, regex!(v).find_iter(&sequence).count()))
        .collect();

    for v in variants.iter() {
        println!("{} {}", v, results.get::<str>(v).unwrap());
    }

    println!("\n{}\n{}\n{:?}", input.len(), sequence.len(), result.join().unwrap().len());
}
