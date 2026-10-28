//! Checked ordinary calls retain the argument kinds of their unchecked forms.
use super::{Data, check_cases};
use mech_core::snapshot::F64Bits;

#[test]
fn checked_tuple_argument_used_preserves_checks_and_retry() {
    let source = "checked(pair<([u16],u64)>, ok<bool>) => (out<u16>) :=\n  (words, at) := pair\n  allowed! := ok\n  out := words[at].\nchecked((input<[u16]>, at<u64>), ok<bool>)\n";
    check_cases(
        source,
        &[
            (
                vec![
                    (
                        "input",
                        Data::Matrix(vec![Data::U16(11), Data::U16(22)].into()),
                    ),
                    ("at", Data::U64(2)),
                    ("ok", Data::Bool(true)),
                ],
                Some(Data::U16(22)),
            ),
            (
                vec![
                    ("input", Data::Matrix(vec![].into())),
                    ("at", Data::U64(0)),
                    ("ok", Data::Bool(false)),
                ],
                None,
            ),
            (
                vec![
                    ("input", Data::Matrix(vec![Data::U16(33)].into())),
                    ("at", Data::U64(1)),
                    ("ok", Data::Bool(true)),
                ],
                Some(Data::U16(33)),
            ),
        ],
    );
}

#[test]
fn checked_tuple_argument_unused_preserves_checks_and_retry() {
    let source = "checked(pair<([u16],u64)>, ok<bool>) => (out<u64>) :=\n  allowed! := ok\n  out := 9u64.\nchecked((input<[u16]>, at<u64>), ok<bool>)\n";
    check_cases(
        source,
        &[
            (
                vec![
                    ("input", Data::Matrix(vec![Data::U16(7)].into())),
                    ("at", Data::U64(1)),
                    ("ok", Data::Bool(true)),
                ],
                Some(Data::U64(9)),
            ),
            (
                vec![
                    ("input", Data::Matrix(vec![].into())),
                    ("at", Data::U64(0)),
                    ("ok", Data::Bool(false)),
                ],
                None,
            ),
            (
                vec![
                    (
                        "input",
                        Data::Matrix(vec![Data::U16(8), Data::U16(9)].into()),
                    ),
                    ("at", Data::U64(2)),
                    ("ok", Data::Bool(true)),
                ],
                Some(Data::U64(9)),
            ),
        ],
    );
}

fn floats(values: &[f64]) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|value| Data::F64(F64Bits::from_f64(*value)))
            .collect(),
    )
}

#[test]
fn unused_float_buffer_keeps_argument_order_and_assertion_recovery() {
    let source = "checked(xs<[f64]>, ok<bool>, divisor<u64>) => (out<u64>) :=\n  allowed! := ok\n  out := 12u64 / divisor.\nchecked(input<[f64]>, ok<bool>, divisor<u64>)\n";
    check_cases(
        source,
        &[
            (
                vec![
                    ("input", floats(&[1.25, -0.0])),
                    ("ok", Data::Bool(true)),
                    ("divisor", Data::U64(3)),
                ],
                Some(Data::U64(4)),
            ),
            (
                vec![
                    ("input", floats(&[])),
                    ("ok", Data::Bool(false)),
                    ("divisor", Data::U64(0)),
                ],
                None,
            ),
            (
                vec![
                    ("input", floats(&[9.5])),
                    ("ok", Data::Bool(true)),
                    ("divisor", Data::U64(2)),
                ],
                Some(Data::U64(6)),
            ),
        ],
    );
}

#[test]
fn float_buffer_capture_keeps_direct_and_expression_caller_checks_before_reads() {
    let checked = "checked(xs<[f64]>, ok<bool>, at<u64>) => (out<f64>) :=\n  allowed! := ok\n  out := xs[at].\n";
    for caller in [
        "checked(input<[f64]>, ok<bool>, at<u64>)\n",
        "outer(xs<[f64]>, ok<bool>, at<u64>) => <f64> := checked(xs, ok, at).\nouter(input<[f64]>, ok<bool>, at<u64>)\n",
    ] {
        check_cases(
            &format!("{checked}{caller}"),
            &[
                (
                    vec![
                        ("input", floats(&[9.25, -0.0])),
                        ("ok", Data::Bool(true)),
                        ("at", Data::U64(1)),
                    ],
                    Some(Data::F64(F64Bits::from_f64(9.25))),
                ),
                (
                    vec![
                        ("input", floats(&[])),
                        ("ok", Data::Bool(false)),
                        ("at", Data::U64(0)),
                    ],
                    None,
                ),
                (
                    vec![
                        ("input", floats(&[2.5])),
                        ("ok", Data::Bool(true)),
                        ("at", Data::U64(1)),
                    ],
                    Some(Data::F64(F64Bits::from_f64(2.5))),
                ),
            ],
        );
    }
}

#[test]
fn unused_argument_check_precedes_closed_caller_arithmetic() {
    let source = "argument(xs<[f64]>, n<u64>) => (out<[f64]>) :=\n  positive! := n > 0u64\n  out := xs.\ncaller(xs<[f64]>, ok<bool>) => (out<u64>) :=\n  allowed! := ok\n  out := 1u64 / 0u64.\ncaller(argument(input<[f64]>, n<u64>), ok<bool>)\n";
    check_cases(
        source,
        &[(
            vec![
                ("input", floats(&[7.5])),
                ("n", Data::U64(0)),
                ("ok", Data::Bool(true)),
            ],
            None,
        )],
    );
}
