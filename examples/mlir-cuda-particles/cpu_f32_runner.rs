mod native_numeric {
    include!(env!("MECH_NUMERIC_SOURCE"));
}

use std::{hint::black_box, time::Instant};

fn hash(values: &[f32]) -> u64 {
    values.iter().fold(0xcbf29ce484222325, |hash, value| {
        (hash ^ u64::from(value.to_bits())).wrapping_mul(0x100000001b3)
    })
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let turns = arguments
        .next()
        .expect("missing turns")
        .parse::<u64>()
        .expect("turns must be an integer");
    let lanes = arguments
        .next()
        .expect("missing particle lanes")
        .parse::<usize>()
        .expect("particle lanes must be an integer");
    assert!(turns > 0 && lanes > 0);
    assert_eq!(native_numeric::STATE_LEN, lanes * 6);

    let inputs = vec![0.0; native_numeric::INPUT_LEN];
    let mut state = vec![0.0; native_numeric::STATE_LEN];
    native_numeric::initialize(&mut state);
    let started = Instant::now();
    for _ in 0..turns {
        native_numeric::turn_in_place(&inputs, &mut state);
    }
    let elapsed = started.elapsed().as_secs_f64();
    let throughput = turns as f64 * lanes as f64 / elapsed / 1.0e6;
    println!("CPU f32 resident dispatch: {:.3} ms", elapsed * 1_000.0);
    println!("CPU f32 throughput: {throughput:.3} million particle-turns/s");
    println!("state checksum: {}", black_box(hash(&state)));
    println!("benchmark_csv,cpu,f32,{turns},{elapsed:.9},{throughput:.3}");
}
