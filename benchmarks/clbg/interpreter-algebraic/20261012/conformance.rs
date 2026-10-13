// Included only by the copied resident module's unscored test configuration.
// Tests call the actual ScalarTape/ScalarTile and scalar_row implementations.
use super::{Kind, ScalarTape, ScalarTile, SCALAR_TILE, scalar_row};

fn constant_tape(coefficient: f64) -> ScalarTape {
    ScalarTape { cells: vec![coefficient.to_bits()], parameters: vec![],
        instructions: vec![], output: 0, arity: 2 }
}

fn constant_tile(coefficient: f64) -> ScalarTile {
    ScalarTile { cells: vec![[coefficient.to_bits(); SCALAR_TILE]], parameters: vec![],
        instructions: vec![], output: 0, varying: 1 }
}

fn classify(value: f64) -> &'static str {
    if value.is_nan() { "nan" }
    else if value.is_infinite() { "infinite" }
    else if value == 0.0 { if value.is_sign_negative() { "negative-zero" } else { "positive-zero" } }
    else if value.is_subnormal() { "subnormal" }
    else { "normal" }
}

fn ordered(bits: u64) -> u64 {
    if bits >> 63 != 0 { !bits } else { bits | (1_u64 << 63) }
}

fn varying_tape() -> ScalarTape {
    // An arbitrary checked source-like callable: to_f64(2 * row + column + 1).
    // This exercises source-graph instructions/parameters, not a workload kernel.
    ScalarTape { cells: vec![0, 0, 1, 2, 0, 0, 0, 0],
        parameters: vec![(0, Kind::Integer, 0), (1, Kind::Integer, 1)],
        instructions: vec![(20, Kind::Integer, 0, 3, 4), (22, Kind::Integer, 4, 1, 5),
            (22, Kind::Integer, 5, 2, 6), (19, Kind::Float, 6, 6, 7)], output: 7, arity: 2 }
}

#[test]
fn algebraic_experiment_axes_tails_and_integer_errors() {
    for length in [0_usize, 1, 7, 31, 32, 33, 65] {
        let input = (0..length).map(|index| if index % 3 == 0 { -2.0 } else { 1.0 }).collect::<Vec<_>>();
        for row in 0..3 {
            for transpose in [false, true] {
                let maxima = if transpose { [length.saturating_sub(1) as u64, 2] }
                    else { [2, length.saturating_sub(1) as u64] };
                for divide in [false, true] {
                    let expected = input.iter().enumerate().fold(0.0, |sum, (column, &x)| {
                        let coefficient = (if transpose { 2 * column + row + 1 } else { 2 * row + column + 1 }) as f64;
                        sum + if divide { x / coefficient } else { x * coefficient }
                    });
                    for tiled in [false, true] {
                        let mut tape = varying_tape();
                        let mut tile = if tiled { ScalarTile::load(&tape, &maxima, usize::from(!transpose)) } else { None };
                        assert_eq!(tile.is_some(), tiled);
                        let actual = scalar_row(&mut tape, &mut tile, row, &input, transpose, divide).unwrap();
                        if MODE == "strict" { assert_eq!(actual.to_bits(), expected.to_bits()); }
                        println!("FP_AXES|{MODE}|{length}|{row}|{transpose}|{divide}|{tiled}|{:016x}|{:016x}",
                            expected.to_bits(), actual.to_bits());
                    }
                }
            }
        }
    }
    let mut tape = varying_tape();
    let mut tile = ScalarTile::load(&tape, &[u64::MAX, 0], 1);
    assert!(tile.is_none());
    let error = scalar_row(&mut tape, &mut tile, usize::MAX, &[1.0], false, false).unwrap_err();
    assert_eq!(error, "unsigned arithmetic overflow");
}

#[test]
fn algebraic_experiment_conformance() {
    let special = [
        ("positive", vec![1.0, 2.0, 3.0, 4.0]),
        ("mixed-sign", vec![1.0, -2.0, 3.0, -4.0]),
        ("cancellation", vec![1.0e16, 1.0, -1.0e16, 1.0]),
        ("signed-zero", vec![-0.0, -0.0, 0.0, -0.0]),
        ("subnormal", vec![f64::from_bits(1), -f64::from_bits(1), f64::MIN_POSITIVE, -f64::MIN_POSITIVE]),
        ("overflow", vec![f64::MAX, f64::MAX, -f64::MAX, -f64::MAX]),
        ("nan", vec![f64::NAN, 1.0, -1.0]),
        ("infinity", vec![f64::INFINITY, f64::NEG_INFINITY, 1.0]),
    ];
    for (name, pattern) in special {
        for length in [0, 1, 2, 7, 31, 32, 33, 65] {
            let input = (0..length).map(|index| pattern[index % pattern.len()]).collect::<Vec<_>>();
            for coefficient in [1.0, 0.0, -0.0, 3.0, f64::MIN_POSITIVE, f64::INFINITY, f64::NAN] {
                for divide in [false, true] {
                    let expected = input.iter().fold(0.0, |sum, &x| {
                        sum + if divide { x / coefficient } else { x * coefficient }
                    });
                    for tiled in [false, true] {
                        let mut tape = constant_tape(coefficient);
                        let mut tile = tiled.then(|| constant_tile(coefficient));
                        let actual = scalar_row(&mut tape, &mut tile, 3, &input, true, divide).unwrap();
                        if MODE == "strict" {
                            if expected.is_nan() { assert!(actual.is_nan()); }
                            else { assert_eq!(actual.to_bits(), expected.to_bits()); }
                        }
                        let ulp = if actual.is_finite() && expected.is_finite() {
                            ordered(actual.to_bits()).abs_diff(ordered(expected.to_bits())).to_string()
                        } else { "nonfinite".to_owned() };
                        println!("FP_CONFORMANCE|{MODE}|{name}|{length}|{:016x}|{divide}|{tiled}|{:016x}|{:016x}|{}|{ulp}",
                            coefficient.to_bits(), expected.to_bits(), actual.to_bits(), classify(actual));
                    }
                }
            }
        }
    }
}
