// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Renat Galimov


use std::sync::Arc;
use std::thread;
use std::collections::HashMap;
use std::vec::Vec;
use std::string::String;


fn count_nucleotides(input: &Vec<u8>, length: usize) -> HashMap<Vec<u8>, usize> {
    let mut nucleotides = HashMap::<Vec<u8>, usize>::new();

    if input.len() < length {
        return nucleotides;
    }

    let mut position = 0;
    let end = input.len() - length;

    while position <= end {
        *nucleotides.entry(input[position .. position + length].to_vec()).or_insert(0) += 1;
        position += 1;
    }

    nucleotides
}


fn read_input() -> Vec<u8> {
    let stdin = std::io::stdin();

    let mut input = String::new();

    while stdin.read_line(&mut input).unwrap() > 0 {
        if input.contains("THREE") {
            input.clear();
            break;
        }
        input.clear();
    }

    let mut data = Vec::<u8>::new();

    loop {
        let mut input = String::new();
        let bytes_read = stdin.read_line(&mut input).unwrap();

        if bytes_read == 0 {
            break
        }

        let mut input_bytes = input.to_uppercase().into_bytes();

        loop {
            match input_bytes.last() {
                Some(&b'\r') | Some(&b'\n') => { input_bytes.pop(); },
                _ => break
            }
        }

        data.append(&mut input_bytes);
    }

    data
}


fn print_percents(mut items: HashMap<Vec<u8>, usize>) {
    let mut sorted_items = Vec::<(Vec<u8>, usize)>::new();

    let mut total_count = 0;

    for (key, value) in items.drain() {
        total_count += value;
        sorted_items.push((key, value));
    }

    sorted_items.sort_by_key(|item| item.1);

    for item in sorted_items.iter().rev() {
        let count = (item.1 * 100) as f64 / total_count as f64;
        println!("{} {:.3}", std::str::from_utf8(&item.0).unwrap(), count);
    }
}


fn print_count(pattern: &str, items: HashMap<Vec<u8>, usize>) {
    let count = match items.get(pattern.as_bytes()) {
        Some(count) => *count,
        _ => 0
    };

    println!("{}\t{}", count, pattern);
}


fn main() {

    let data = Arc::new(read_input());

    let mut threads = HashMap::<usize, thread::JoinHandle<HashMap<Vec<u8>, usize>>>::new();

    for k in vec![1, 2, 3, 4, 6, 12, 18] {
        let data_ptr = data.clone();

        let handle = thread::spawn(move || {
            count_nucleotides(&data_ptr, k)
        });

        threads.insert(k, handle);
    }

    let mut results = HashMap::<usize, HashMap<Vec<u8>, usize>>::new();

    for (key, thread) in threads.drain() {
        thread.join().map(|value| {
            results.insert(key, value)
        });
    }

    print_percents(results.remove(&1).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
    println!("");
    print_percents(results.remove(&2).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
    println!("");

    print_count("GGT", results.remove(&3).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
    print_count("GGTA", results.remove(&4).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
    print_count("GGTATT", results.remove(&6).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
    print_count("GGTATTTTAATT", results.remove(&12).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
    print_count("GGTATTTTAATTTATAGT", results.remove(&18).unwrap_or_else(|| HashMap::<Vec<u8>, usize>::new()));
}
