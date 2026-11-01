use super::*;

const CATALOG: &[&str] = &[
    "io/write",
    "io/argument-u64",
    "parallel/reduce-ranges",
    "stats/sum",
];

fn program(body: &str, fields: &str) -> Result<Program, String> {
    compile(
        &format!("{body}\nio/write(text/format(\"{{}}\", {fields}))\n"),
        CATALOG,
    )
}

#[test]
fn annotated_signed_literals_keep_their_exact_kind_and_limits() {
    for (literal, expected) in [
        ("0<i64>", 0),
        ("-1<i64>", -1),
        ("9223372036854775807<i64>", i64::MAX),
        ("-9223372036854775808<i64>", i64::MIN),
    ] {
        let result = program(&format!("result := {literal}"), "result").unwrap();
        let Op::Tuple(fields) = result.output.op else {
            panic!("expected output tuple");
        };
        assert_eq!(fields[0].kind, ControlKind::SignedInteger);
        assert_eq!(fields[0].op, Op::SignedInteger(expected));
    }
    for literal in ["9223372036854775808<i64>", "-9223372036854775809<i64>"] {
        assert!(program(&format!("result := {literal}"), "result").is_err());
    }
}

#[test]
fn kind_annotations_lower_to_checked_conversions() {
    let result = program("raw := io/argument-u64(1u64)\nresult<i64> := raw", "result").unwrap();
    let Op::Tuple(fields) = result.output.op else {
        panic!("expected output tuple");
    };
    assert_eq!(fields[0].kind, ControlKind::SignedInteger);
    let Op::Convert(value) = &fields[0].op else {
        panic!("annotation must retain a checked conversion");
    };
    assert_eq!(value.kind, ControlKind::Integer);
}

#[test]
fn signed_match_patterns_and_comprehension_sums_retain_signed_values() {
    program(
        "value := -1<i64>\nresult := value?\n  | -1<i64> => 7u64\n  | * => 0u64.",
        "result",
    )
    .unwrap();
    let result = program(
        "result := stats/sum([-1<i64> | index <- 0u64..3u64])",
        "result",
    )
    .unwrap();
    assert_eq!(
        result.output.kind,
        ControlKind::Tuple(vec![ControlKind::SignedInteger])
    );
}

#[test]
fn range_reductions_accept_mixed_signed_and_unsigned_output_lanes() {
    compile(
        "Chunk(start<u64>, end<u64>, value<i64>) = (checksum<i64>, maximum<u64>) :=\n  checksum := value\n  maximum := end.\n(checksum, maximum) := parallel/reduce-ranges(0u64, 4u64, 1u64, Chunk, -1<i64>, \"sum,max\")\nio/write(text/format(\"{} {}\", checksum, maximum))\n",
        CATALOG,
    )
    .unwrap();
}

#[test]
fn mixed_integer_arithmetic_is_not_silently_narrowed_to_i64() {
    assert!(program("result := 1u64 + 1<i64>", "result").is_err());
    assert!(program("result := -true", "result").is_err());
    assert!(program("result := -0u64", "result").is_err());
    assert!(program("result := -1u64", "result").is_err());
}

#[test]
fn multiple_generators_share_one_fold_without_reassociating_terms() {
    let result = program(
        "result := stats/sum([-1<i64> | i <- 0u64..2u64, j <- 0u64..2u64])",
        "result",
    )
    .unwrap();
    let Op::Tuple(fields) = result.output.op else {
        panic!("expected output tuple");
    };
    let Op::Fold { fold, body } = &fields[0].op else {
        panic!("expected an owned fold");
    };
    let Op::Reduce {
        fold: outer, body, ..
    } = &body.op
    else {
        panic!("expected outer generator");
    };
    assert_eq!(fold, outer);
    let Op::Reduce {
        fold: inner, body, ..
    } = &body.op
    else {
        panic!("expected inner generator");
    };
    assert_eq!(fold, inner);
    assert!(matches!(&body.op, Op::Accumulate { fold: leaf, .. } if leaf == fold));
}

#[test]
fn definition_and_filter_only_comprehensions_keep_an_owned_fold_scope() {
    for body in [
        "result := stats/sum([value | value := -1<i64>, true])",
        "result := stats/sum([value | value := -1<i64>, false])",
    ] {
        let result = program(body, "result").unwrap();
        let Op::Tuple(fields) = result.output.op else {
            panic!("expected output tuple");
        };
        assert!(matches!(fields[0].op, Op::Fold { .. }));
    }
}
