// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Mrjillhace

extern crate fnv;

use std::thread;
use std::sync::Arc;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use fnv::FnvHasher;

const SEQ_LENS: [usize; 7] = [1, 2, 3, 4, 6, 12, 18];
const LOOKUPS: [&'static str; 5] = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"];

type Table = HashMap<u64, usize, BuildHasherDefault<FnvHasher>>;

fn encode(c: char) -> u8 {
    match c {
        'a' | 'A' => 0,
        'c' | 'C' => 1,
        'g' | 'G' => 2,
        't' | 'T' => 3,
        _ => panic!("wrong character"),
    }
}

fn encode_str(s: &str) -> u64 {
    s.chars().fold(0, |acc, c| 4 * acc + encode(c) as u64)
}

//require char length to decode, otherwise "AC" == "C"
fn decode(mut v: u64, len: usize) -> String {
    let mut s = String::new();
    for _ in 0..len {
        let digit = v % 4;
        match digit {
            0 => s.push('A'),
            1 => s.push('C'),
            2 => s.push('G'),
            3 => s.push('T'),
            _ => {}
        };
        v /= 4;
    }
    s.chars().rev().collect()
}

struct Buffer{
    value: u64,
    size: usize,
}

impl Buffer {
    fn push(&mut self, c: u8) {
        self.value = (self.value * (1 << 2)) % (1 << (2 * self.size)) + (c as u64);
    }
}

fn parse(mut input: &[u8], len: usize) -> Table {
    let fnv = BuildHasherDefault::<FnvHasher>::default();
    let mut table = Table::with_hasher(fnv);
    let mut buffer = Buffer{ value: 0, size: len };
    if input.len() < len { return table; }
    for _ in 0..len - 1 {
        buffer.push(input[0]);
        input = &input[1..];
    }
    while input.len() != 0 {
        buffer.push(input[0]);
        input = &input[1..];
        let counter = table.entry(buffer.value).or_insert(0);
        *counter += 1;
    }
    table
}

fn read_input<R: std::io::BufRead>(source: R, key: &str) -> Vec<u8> {
    let mut vec = Vec::new();
    for l in source.lines().map(|l| l.ok().unwrap())
        .skip_while(|l| key != &l[..key.len()]).skip(1)
    {
        vec.extend(l.trim().chars().map(|b| encode(b)));
    }
    vec
}

fn report(table: &(usize, Table)) {
    let mut vec = Vec::new();
    let len = table.0;

    for entry in table.1.iter() {
        vec.push((decode(*entry.0, len), *entry.1));
    }
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let sum = vec.iter().fold(0, |acc, i| acc + i.1);
    for seq in vec {
        println!("{} {:.3}", seq.0, (seq.1 * 100) as f32 / sum as f32);
    }
    println!("");
}

fn main() {
    let stdin = std::io::stdin();
    let input: Vec<u8> = read_input(stdin.lock(), ">THREE");
    let input = Arc::new(input);
    //separate hashmap for each sequence length
    let tables_handle: Vec<_> = SEQ_LENS.iter().map(|&i| {
        let input = input.clone();
        (i, thread::spawn(move || parse(&input, i)))
    }).collect();

    let mut tables = Vec::new();
    for (i, handle) in tables_handle {
        tables.push((i, handle.join().unwrap()));
    }


    report(&tables[0]);
    report(&tables[1]);
    for &seq in &LOOKUPS {
        let index = SEQ_LENS.iter().position(|&x| x == seq.len()).unwrap();
        let num = encode_str(seq);
        println!("{}\t{}",tables[index].1.get(&num).unwrap_or(&0), seq);
    }
}
