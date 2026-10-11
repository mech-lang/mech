// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Ryohei Machida

// Compile with these flags:
// -C target-cpu=native -C panic=abort

extern crate fxhash;
extern crate num_cpus;
extern crate num_traits;
extern crate scoped_threadpool;

use num_traits::FromPrimitive;
use scoped_threadpool::Pool;
use std::cmp::Eq;
use std::hash::Hash;
use std::sync::mpsc;

type Map<T> = fxhash::FxHashMap<T, u32>;

trait ShlXorMsk<T> {
    fn sh(a: T, x: u8, m: T) -> T;
    fn mask(len: usize) -> T;
}

macro_rules! impl_shl_xor_msk {
    ($($prim:ty)*) => {
        $(
            impl ShlXorMsk<$prim> for $prim {
                fn sh(a: $prim, x: u8, m: $prim) -> $prim {
                    m & (a << 2) | (x as $prim)
                }

                fn mask(len: usize) -> $prim {
                    ((1u64 << 2 * len) - 1) as $prim
                }
            }
        )*
    }
}

impl_shl_xor_msk!(u8 u16 u32 u64);

fn match_key(k: u8) -> char {
    match k {
        0b00 => 'A',
        0b01 => 'C',
        0b10 => 'T',
        0b11 => 'G',
        _ => '_',
    }
}

fn print_stat(h: Map<u8>, seq_len: usize) {
    let total = h.values().sum::<u32>();

    let mut vec = h.into_iter().collect::<Vec<_>>();
    vec.sort_unstable_by(|&(ref a, x), &(ref b, y)| Ord::cmp(&(y, b), &(x, a)));

    for (k, v) in vec {
        if seq_len == 1 {
            println!("{} {:.3}", match_key(k), (100 * v) as f32 / total as f32);
        } else {
            println!(
                "{}{} {:.3}",
                match_key(k >> 2),
                match_key(0b11 & k),
                (100 * v) as f32 / total as f32
            );
        };
    }
    println!();
}

fn print<T: FromPrimitive + Default + Hash + Eq + ShlXorMsk<T> + Copy>(
    h: Map<T>,
    seq: &str,
) {
    let mask = T::from_u64((1u64 << (2 * seq.len() as u32)) - 1).unwrap();
    let k = seq
        .to_ascii_lowercase()
        .as_bytes()
        .iter()
        .map(|x| 0b11u8 & x >> 1)
        .fold(T::default(), |acc, x| T::sh(acc, x, mask));
    println!("{}\t{}", h.get(&k).unwrap_or(&0), seq);
}

fn freq<T: Default + Hash + Eq + ShlXorMsk<T> + Copy>(
    s_vec: &[u8],
    len: usize,
) -> Map<T> {
    let mut h = Map::default();
    let mask = T::mask(len);
    let mut it = s_vec.iter();
    let mut a = it
        .by_ref()
        .take(len - 1)
        .fold(T::default(), |acc, &x| T::sh(acc, x, mask));
    for &x in it {
        a = T::sh(a, x, mask);
        *h.entry(a).or_insert(0) += 1;
    }
    h
}

fn freq_par<T: Default + Hash + Eq + ShlXorMsk<T> + Copy + Send>(
    pool: &mut Pool,
    s_vec: &[u8],
    len: usize,
) -> Map<T> {
    if s_vec.len() < 1000 {
        return freq(s_vec, len);
    }

    let num_partitions = pool.thread_count() as usize;
    let (tx, rx) = mpsc::channel();

    pool.scoped(|scope| {
        for i in 0..num_partitions {
            // split s_vec into partitions
            let start = s_vec.len() * i / num_partitions;
            let end = if i != num_partitions - 1 {
                s_vec.len() * (i + 1) / num_partitions + len - 1
            } else {
                s_vec.len()
            };
            let sub_vec = &s_vec[start..end];
            let tx = tx.clone();

            scope.execute(move || {
                tx.send(freq::<T>(sub_vec, len)).unwrap();
            });
        }
    });

    {
        let mut merged = rx.recv().unwrap();

        // merge results
        for _ in 1..num_partitions {
            let map = rx.recv().unwrap();
            for (k, v) in map {
                *merged.entry(k).or_insert(0) += v;
            }
        }

        merged
    }
}

fn get_seq<R: std::io::BufRead>(mut r: R, key: &[u8]) -> Vec<u8> {
    let mut res = Vec::with_capacity(65536);
    let mut line = Vec::with_capacity(64);

    loop {
        match r.read_until(b'\n', &mut line) {
            Ok(b) if b > 0 => {
                if line.starts_with(key) {
                    break;
                }
            }
            _ => break,
        }
        line.clear();
    }

    loop {
        line.clear();
        match r.read_until(b'\n', &mut line) {
            Ok(b) if b > 0 => res
                .extend(line[..line.len() - 1].iter().map(|&x| 0b11 & x >> 1)),
            _ => break,
        }
    }

    res
}

pub fn calc<R: std::io::BufRead>(r: R) {
    let s_vec = get_seq(r, b">THREE");
    let mut pool = Pool::new(num_cpus::get() as u32);

    let f1 = freq_par(&mut pool, &s_vec, 1);
    print_stat(f1, 1);

    let f2 = freq_par(&mut pool, &s_vec, 2);
    print_stat(f2, 2);

    let f3 = freq_par::<u8>(&mut pool, &s_vec, 3);
    print(f3, "GGT");

    let f4 = freq_par::<u8>(&mut pool, &s_vec, 4);
    print(f4, "GGTA");

    let f5 = freq_par::<u16>(&mut pool, &s_vec, 6);
    print(f5, "GGTATT");

    let f6 = freq_par::<u32>(&mut pool, &s_vec, 12);
    print(f6, "GGTATTTTAATT");

    let f7 = freq_par::<u64>(&mut pool, &s_vec, 18);
    print(f7, "GGTATTTTAATTTATAGT");
}

fn main() {
    let stdin = std::io::stdin();
    calc(stdin.lock());
}
