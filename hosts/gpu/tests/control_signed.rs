//! Signed values and checked integer operations share all native executors.
#![cfg(feature = "sequence-cli")]

use mech_engine::native_sequence::control::compile;
use mech_gpu::control::{JitProgram, ResidentProgram};

fn run(source: &str, arguments: &[u64], succeeds: bool) {
    let program = compile(source, mech_stdlib::native_control_catalog())
        .unwrap_or_else(|error| panic!("{source}\n{error}"));
    let result = ResidentProgram::lower(&program).unwrap().run(arguments);
    assert_eq!(result.is_ok(), succeeds, "resident: {source}\n{result:?}");
    for simd in [false, true] {
        let result = JitProgram::from_program(program.clone(), simd)
            .unwrap_or_else(|error| panic!("codegen: {source}\n{error}"))
            .run(arguments);
        assert_eq!(
            result.is_ok(),
            succeeds,
            "native simd={simd}: {source}\n{result:?}"
        );
    }
}

fn signed(expression: &str, expected: i64) {
    run(
        &format!(
            "result := number/bounded({expression}, {expected}<i64>, {expected}<i64>)\nio/write(text/format(\"{{}}\\n\", result))\n"
        ),
        &[],
        true,
    );
}

#[test]
fn signed_arithmetic_comparison_bounds_and_conversion_keep_their_kind() {
    for (expression, expected) in [
        ("-7<i64> + 3<i64>", -4),
        ("-7<i64> - 3<i64>", -10),
        ("-7<i64> * 3<i64>", -21),
        ("-7<i64> / 3<i64>", -2),
        ("number/mod(-7<i64>, 3<i64>)", -1),
        ("number/mod(7<i64>, -3<i64>)", 1),
        ("-7<i64> % 3<i64>", -1),
        ("number/max(-7<i64>, -3<i64>)", -3),
        ("-(-7<i64>)", 7),
    ] {
        signed(expression, expected);
    }
    for comparison in [
        "-1<i64> < 0<i64>",
        "-1<i64> <= 0<i64>",
        "0<i64> > -1<i64>",
        "0<i64> >= -1<i64>",
    ] {
        run(
            &format!(
                "result := ({comparison})?\n  | true => 1u64\n  | false => 0u64.\nio/write(text/format(\"{{}}\", number/bounded(result, 1u64, 1u64)))\n"
            ),
            &[],
            true,
        );
    }
    run(
        "x<i64> := io/argument-u64(1)\ny<u64> := x\nz<f64> := -7<i64>\nresult := (z + 7.0 == 0.0)?\n  | true => y\n  | false => 0u64.\nio/write(text/format(\"{}\", number/bounded(result, 9223372036854775807u64, 9223372036854775807u64)))\n",
        &[i64::MAX as u64],
        true,
    );
    run(
        "x<f64> := io/argument-u64(1)\nresult := (x == 18446744073709551616.0)?\n  | true => 1u64\n  | false => 0u64.\nio/write(text/format(\"{}\", number/bounded(result, 1u64, 1u64)))\n",
        &[u64::MAX],
        true,
    );
    run(
        "x := -(-7.0)\nresult := (x == 7.0)?\n  | true => 1u64\n  | false => 0u64.\nio/write(text/format(\"{}\", number/bounded(result, 1u64, 1u64)))\n",
        &[],
        true,
    );
    run(
        "result := 7u64 % 3u64\nio/write(text/format(\"{}\", number/bounded(result, 1u64, 1u64)))\n",
        &[],
        true,
    );
}

#[test]
fn signed_and_unsigned_overflow_and_invalid_conversions_fail_at_execution() {
    for expression in ["-0u64", "-1u64"] {
        let source = format!("result := {expression}\nio/write(text/format(\"{{}}\", result))\n");
        assert!(
            compile(&source, mech_stdlib::native_control_catalog()).is_err(),
            "{source}"
        );
    }
    for expression in [
        "18446744073709551615u64 + 1u64",
        "0u64 - 1u64",
        "18446744073709551615u64 * 2u64",
        "9223372036854775807<i64> + 1<i64>",
        "-9223372036854775808<i64> - 1<i64>",
        "9223372036854775807<i64> * 2<i64>",
        "-(-9223372036854775808<i64>)",
        "-9223372036854775808<i64> / -1<i64>",
        "number/mod(-9223372036854775808<i64>, -1<i64>)",
        "1<i64> / 0<i64>",
        "number/mod(1<i64>, 0<i64>)",
    ] {
        run(
            &format!("result := {expression}\nio/write(text/format(\"{{}}\", result))\n"),
            &[],
            false,
        );
    }
    run(
        "x<i64> := io/argument-u64(1)\nio/write(text/format(\"{}\", x))\n",
        &[i64::MAX as u64 + 1],
        false,
    );
    run(
        "x<u64> := -1<i64>\nio/write(text/format(\"{}\", x))\n",
        &[],
        false,
    );
}

#[test]
fn lazy_matches_do_not_evaluate_unselected_checked_arithmetic() {
    run(
        "result := -1<i64>?\n  | -1<i64> => 1u64\n  | * => 0u64.\nio/write(text/format(\"{}\", number/bounded(result, 1u64, 1u64)))\n",
        &[],
        true,
    );
    for expression in [
        "1<i64> / 0<i64>",
        "number/mod(1<i64>, 0<i64>)",
        "9223372036854775807<i64> + 1<i64>",
        "-(-9223372036854775808<i64>)",
    ] {
        run(
            &format!(
                "result := true?\n  | true => -4<i64>\n  | false => {expression}.\nio/write(text/format(\"{{}}\", number/bounded(result, -4<i64>, -4<i64>)))\n"
            ),
            &[],
            true,
        );
    }
    run(
        "Bad() => (out<u64>) :=\n  out := -1<i64>.\nresult := false?\n  | true => Bad()\n  | false => 7u64.\nio/write(text/format(\"{}\", number/bounded(result, 7u64, 7u64)))\n",
        &[],
        true,
    );
}

#[test]
fn signed_comprehension_sums_keep_source_order_and_typed_filter_zero() {
    run(
        "Term(i<u64>) => <i64> := i?\n  | 0u64 => -9223372036854775808<i64>\n  | 1u64 => 9223372036854775807<i64>\n  | * => 1<i64>.\nresult := stats/sum([Term(i) | i <- 0u64..3u64])\nio/write(text/format(\"{}\", number/bounded(result, 0<i64>, 0<i64>)))\n",
        &[],
        true,
    );
    run(
        "Term(i<u64>) => <i64> := i?\n  | 0u64 => 9223372036854775807<i64>\n  | 1u64 => 1<i64>\n  | * => -1<i64>.\nresult := stats/sum([Term(i) | i <- 0u64..3u64])\nio/write(text/format(\"{}\", result))\n",
        &[],
        false,
    );
    run(
        "result := stats/sum([-1<i64> | i <- 0u64..4u64, i < 2u64])\nio/write(text/format(\"{}\", number/bounded(result, -2<i64>, -2<i64>)))\n",
        &[],
        true,
    );
}

#[test]
fn mixed_parallel_reducers_keep_signedness_and_range_order() {
    let worker = "Term(i<u64>) => <i64> := i?\n  | 0u64 => -9223372036854775808<i64>\n  | 1u64 => 9223372036854775807<i64>\n  | * => 1<i64>.\nChunk(a<u64>, b<u64>, captured<i64>) => (sum<i64>, peak<i64>, count<u64>) :=\n  sum := Term(a)\n  peak := captured\n  count := 1u64.\n";
    run(
        &format!(
            "{worker}(sum, peak, count) := parallel/reduce-ranges(0u64, 3u64, 1u64, Chunk, -1<i64>, \"sum,max,sum\")\na := number/bounded(sum, 0<i64>, 0<i64>)\nb := number/bounded(peak, -1<i64>, -1<i64>)\nio/write(text/format(\"{{}} {{}} {{}}\", a, b, number/bounded(count, 3u64, 3u64)))\n"
        ),
        &[],
        true,
    );
    let worker = worker
        .replace("-9223372036854775808<i64>", "9223372036854775807<i64>")
        .replace("1u64 => 9223372036854775807<i64>", "1u64 => 1<i64>")
        .replace("* => 1<i64>", "* => -1<i64>");
    run(
        &format!(
            "{worker}(sum, peak, count) := parallel/reduce-ranges(0u64, 3u64, 1u64, Chunk, -1<i64>, \"sum,max,sum\")\nio/write(text/format(\"{{}}\", sum))\n"
        ),
        &[],
        false,
    );
    let count = rayon::current_num_threads().saturating_mul(4) + 3;
    run(
        &format!(
            "Chunk(a<u64>, b<u64>, captured<i64>) => (sum<i64>, peak<i64>, count<u64>) :=\n  sum := captured\n  peak := captured\n  count := 1u64.\n(sum, peak, count) := parallel/reduce-ranges(0u64, io/argument-u64(1), 1u64, Chunk, -1<i64>, \"sum,max,sum\")\na := number/bounded(sum, -{count}<i64>, -{count}<i64>)\nb := number/bounded(peak, -1<i64>, -1<i64>)\nio/write(text/format(\"{{}} {{}} {{}}\", a, b, number/bounded(count, {count}u64, {count}u64)))\n"
        ),
        &[count as u64],
        true,
    );
}

#[test]
fn multiple_generators_share_one_fold_but_explicit_nested_sums_do_not() {
    let terms = "Term(i<u64>, j<u64>) => <i64> := (i * 2u64 + j)?\n  | 0u64 => 9223372036854775807<i64>\n  | 1u64 => 0<i64>\n  | 2u64 => 1<i64>\n  | * => -1<i64>.\n";
    run(
        &format!(
            "{terms}result := stats/sum([Term(i, j) | i <- 0u64..2u64, j <- 0u64..2u64])\nio/write(text/format(\"{{}}\", result))\n"
        ),
        &[],
        false,
    );
    run(
        &format!(
            "{terms}result := stats/sum([stats/sum([Term(i, j) | j <- 0u64..2u64]) | i <- 0u64..2u64])\nio/write(text/format(\"{{}}\", number/bounded(result, 9223372036854775807<i64>, 9223372036854775807<i64>)))\n"
        ),
        &[],
        true,
    );
    run(
        &format!(
            "{terms}result := stats/sum([value | i <- 0u64..2u64, j <- 0u64..2u64, value := Term(i, j), i == 0u64])\nio/write(text/format(\"{{}}\", number/bounded(result, 9223372036854775807<i64>, 9223372036854775807<i64>)))\n"
        ),
        &[],
        true,
    );
    run(
        "result := stats/sum([-1<i64> | i <- 0u64..3u64, j <- 0u64..i])\nio/write(text/format(\"{}\", number/bounded(result, -3<i64>, -3<i64>)))\n",
        &[],
        true,
    );
}

#[test]
fn recursive_function_calls_own_their_fold_instances() {
    run(
        "Sum(n<u64>) => <i64> := n?\n  | 0u64 => 1<i64>\n  | * => stats/sum([Sum(n - 1u64) | i <- 0u64..2u64]).\nresult := Sum(3u64)\nio/write(text/format(\"{}\", number/bounded(result, 8<i64>, 8<i64>)))\n",
        &[],
        true,
    );
}

#[test]
fn entry_print_uses_typed_signed_fields_with_the_native_host_abi() {
    run("io/print(\"{} {}\\n\", -7<i64>, 7u64)\n", &[], true);
}

#[test]
fn definition_qualified_comprehensions_own_a_fold_and_preserve_filter_order() {
    for qualifiers in ["value := -7<i64>", "value := -7<i64>, value < 0<i64>"] {
        run(
            &format!(
                "result := stats/sum([value | {qualifiers}])\nio/write(text/format(\"{{}}\", number/bounded(result, -7<i64>, -7<i64>)))\n"
            ),
            &[],
            true,
        );
    }
    let filter_only =
        "result := stats/sum([1u64 | false])\nio/write(text/format(\"{}\", result))\n";
    assert!(compile(filter_only, mech_stdlib::native_control_catalog()).is_err());
    run(
        "result := stats/sum([1u64 / divisor | divisor := io/argument-u64(1), false])\nio/write(text/format(\"{}\", number/bounded(result, 0u64, 0u64)))\n",
        &[0],
        true,
    );
    run(
        "result := stats/sum([value | value := 18446744073709551615u64 + 1u64, false])\nio/write(text/format(\"{}\", result))\n",
        &[],
        false,
    );
    run(
        "result := stats/sum([value | false, value := 18446744073709551615u64 + 1u64])\nio/write(text/format(\"{}\", number/bounded(result, 0u64, 0u64)))\n",
        &[],
        true,
    );
}
