// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by the Rust Project Developers
// contributed by TeXitoi
// modified by Tung Duong

#![allow(non_snake_case)]
use std::iter::repeat;
use std::thread;

#[inline(never)]
fn div_and_add(top0 : f64, top1 : f64, bot0 : f64, bot1 : f64, sum : &mut [f64; 2]){	
	sum[0] += top0/bot0;
	sum[1] += top1/bot1;
}

fn main() {
    let n = std::env::args_os().nth(1)
        .and_then(|s| s.into_string().ok())
        .and_then(|n| n.parse().ok())
        .unwrap_or(100);
    let answer = spectralnorm(n);
    println!("{:.9}", answer);
}

fn spectralnorm(n: usize) -> f64 {
    assert!(n % 2 == 0, "only even lengths are accepted");
    let mut u = repeat(1.0).take(n).collect::<Vec<_>>();
    let mut v = u.clone();
    let mut tmp = v.clone();
    for _ in 0..10 {
        mult_AtAv(&u, &mut v, &mut tmp);
        mult_AtAv(&v, &mut u, &mut tmp);
    }
    (dot(&u, &v) / dot(&v, &v)).sqrt()
}

fn mult_AtAv(v: &[f64], out: &mut [f64], tmp: &mut [f64]) {
    mult_Av(v, tmp);
    mult_Atv(tmp, out);
}

fn mult_Av(v: &[f64], out: &mut [f64]) {
    parallel(out, |start, out| mult(v, out, start, |i, j| A(i, j)));
}

fn mult_Atv(v: &[f64], out: &mut [f64]) {
    parallel(out, |start, out| mult(v, out, start, |i, j| A(j, i)));
}

fn mult<F>(v: &[f64], out: &mut [f64], start: usize, a: F)
           where F: Fn(usize, usize) -> f64 {
    for (i, slot) in out.iter_mut().enumerate().map(|(i, s)| (i + start, s)) {
        let mut sum : [f64;2] = [0.0,0.0];
        for (j, chunk) in v.chunks(2).enumerate().map(|(j, s)| (2 * j, s)) {
            div_and_add(chunk[0], chunk[1], a(i, j), a(i, j + 1), &mut sum); 
        }        
        *slot = sum[0]+sum[1];
    }
}

fn A(i: usize, j: usize) -> f64 {
    ((i + j) * (i + j + 1) / 2 + i + 1) as f64
}

fn dot(v: &[f64], u: &[f64]) -> f64 {
    v.iter().zip(u.iter()).map(|(a, b)| *a * *b).fold(0., |acc, i| acc + i)
}

struct Racy<T>(T);
unsafe impl<T: 'static> Send for Racy<T> {}

// Executes a closure in parallel over the given mutable slice. The closure `f`
// is run in parallel and yielded the starting index within `v` as well as a
// sub-slice of `v`.
fn parallel<'a, T, F>(v: &mut [T], ref f: F)
    where T: 'static + Send + Sync,
          F: Fn(usize, &mut [T]) + Sync {
    let size = v.len() / 4 + 1;
    let jhs = v.chunks_mut(size).enumerate().map(|(i, chunk)| {
        // Need to convert `f` and `chunk` to something that can cross the task
        // boundary.
        let f = Racy(f as *const F as *const usize);
        let raw = Racy((&mut chunk[0] as *mut T, chunk.len()));
        thread::spawn(move|| {
            let f = f.0 as *const F;
            let raw = raw.0;
            unsafe { (*f)(i * size, std::slice::from_raw_parts_mut(raw.0, raw.1)) }
        })
    }).collect::<Vec<_>>();
    for jh in jhs { jh.join().unwrap(); }
}
// end of main program
