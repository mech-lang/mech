//! Canonical synchronous machines use existing typed loop/branch storage.
#![cfg(feature = "sequence-cli")]

use mech_engine::native_sequence::control::{Expr, Op, Program, compile};
use mech_gpu::control::{JitProgram, ResidentProgram};

const COUNTER: &str = r#"
#Count(n<u64>) => <u64>
  | :Running(n<u64>, count<u64>)
  | :Done(value<u64>).
#Count(n) -> :Running(n, 0u64)
  :Running(n, count)
    | n > 0u64 -> :Running(n - 1u64, count + 1u64)
    | n == 0u64 -> :Done(count)
  :Done(value) => value.
"#;
const ADVANCE: &str = include_str!("fixtures/native/fannkuch-advance-fsm.mec");
const IDIOMATIC_FANNKUCH: &str = include_str!("fixtures/native/fannkuch-idiomatic.mec");
const BASELINE_FANNKUCH: &str = include_str!("fixtures/native/fannkuch-control-baseline.mec");

fn program(source: &str) -> Program {
    compile(source, mech_stdlib::native_control_catalog())
        .unwrap_or_else(|e| panic!("{source}\n{e}"))
}

fn check(source: &str, args: &[u64]) {
    let program = program(source);
    ResidentProgram::lower(&program)
        .unwrap()
        .run(args)
        .unwrap_or_else(|e| panic!("resident: {source}\n{e}"));
    for simd in [false, true] {
        JitProgram::from_program(program.clone(), simd)
            .unwrap()
            .run(args)
            .unwrap_or_else(|e| panic!("native simd={simd}: {source}\n{e}"));
    }
}

fn fails(source: &str, args: &[u64]) {
    let program = program(source);
    assert!(
        ResidentProgram::lower(&program).unwrap().run(args).is_err(),
        "{source}"
    );
    for simd in [false, true] {
        assert!(
            JitProgram::from_program(program.clone(), simd)
                .unwrap()
                .run(args)
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn canonical_counter_keeps_completion_internal_and_uses_inline_loop_callbacks() {
    let source = format!(
        "{COUNTER}\nn := io/argument-u64(1)\nresult := #Count(n)\nio/write(text/format(\"{{}}\", number/bounded(result, n, n)))\n"
    );
    let p = program(&source);
    assert_eq!(p.functions.len(), 2);
    assert!(p.functions.values().all(|function| function.native_inline));
    assert!(
        p.functions
            .values()
            .all(|function| function.inputs.len() == 3)
    );
    for count in [0, 1, 19] {
        check(&source, &[count]);
    }
    // This inspects actual IR, not source spelling. The machine has neither
    // runtime-managed state objects nor a dynamic source-function invocation.
    fn structural(e: &Expr) {
        match &e.op {
            Op::Heap { .. } | Op::Arena { .. } | Op::Call(..) | Op::Invoke { .. } => {
                panic!("unexpected runtime machine representation: {:?}", e.op)
            }
            Op::Tuple(values) | Op::Primitive(_, values) => values.iter().for_each(structural),
            Op::Field(value, _) | Op::Loop { initial: value, .. } => structural(value),
            Op::Match {
                scrutinee, arms, ..
            } => {
                structural(scrutinee);
                for (condition, value) in arms {
                    structural(condition);
                    structural(value);
                }
            }
            Op::Sequence { effects, output } => {
                effects.iter().for_each(structural);
                structural(output);
            }
            _ => {}
        }
    }
    structural(&p.output);
    for function in p.functions.values() {
        structural(&function.output);
    }
}

#[test]
fn machine_guards_and_postfix_match_skip_unselected_errors() {
    let source = format!(
        "{COUNTER}\nresult := true ? | true => #Count(19u64) | false => 1u64 / 0u64.\nio/write(text/format(\"{{}}\", number/bounded(result, 19, 19)))\n"
    );
    check(&source, &[]);
    let source = "result := false ? | true => 1u64 / io/write(\"unexpected-unselected-write\") | false => 7u64.\nio/write(text/format(\"{}\", number/bounded(result, 7, 7)))\n";
    check(source, &[]);
    let source = "result := true ? | true => 7u64 | false => number/mod(1u64, 0u64).\nio/write(text/format(\"{}\", number/bounded(result, 7, 7)))\n";
    check(source, &[]);
    for expression in ["1u64 / 0u64", "number/mod(1u64, 0u64)"] {
        fails(
            &format!(
                "result := true ? | true => {expression} | false => 7u64.\nio/write(text/format(\"{{}}\", result))\n"
            ),
            &[],
        );
    }
    let source = format!(
        "{COUNTER}\nresult := false ? | true => #Count(1u64 / 0u64) | false => 7u64.\nio/write(text/format(\"{{}}\", number/bounded(result, 7, 7)))\n"
    );
    check(&source, &[]);
    let source = COUNTER
        .replace("n > 0u64", "false")
        .replace("n == 0u64", "true");
    check(
        &format!(
            "{source}\nresult := #Count(0u64)\nio/write(text/format(\"{{}}\", number/bounded(result, 0, 0)))\n"
        ),
        &[],
    );
    let partial = COUNTER.replace("n > 0u64", "n > 1u64");
    fails(
        &format!("{partial}\nresult := #Count(1u64)\nio/write(text/format(\"{{}}\", result))\n"),
        &[],
    );
}

#[test]
fn missing_simd_payload_guard_fails_without_invalid_vector_constants() {
    let source = r#"
#Gap(c<[u8]:4,1>, n<u64>) => <[u8]:4,1>
  | :Carry(c<[u8]:4,1>, n<u64>)
  | :Done(c<[u8]:4,1>).
#Gap(c, n) -> :Carry(c, n)
  :Carry(c, n)
    | n > 1u64 -> :Done(c)
    | n == 0u64 -> :Carry(c, 1u64)
  :Done(c) => c.
result := #Gap(sequence/zero-bytes(4), 1u64)
io/write(text/format("{}", result[1]))
"#;
    fails(source, &[]);
}

#[test]
fn state_transition_payloads_keep_checked_errors_in_selected_unused_locals() {
    let machine = r#"
#Checked(n<u64>) => <u64>
  | :Running(n<u64>)
  | :Done(value<u64>).
#Checked(n) -> :Running(n)
  :Running(n) -> { unused := 1u64 / (n - n) } -> :Done(n)
  :Done(value) => value.
result := #Checked(io/argument-u64(1))
io/write(text/format("{}", result))
"#;
    fails(machine, &[u64::MAX]);
}

#[test]
fn malformed_or_unsupported_machine_definitions_fail_clearly() {
    for (machine, message) in [
        (
            COUNTER.replace(":Done(count)", ":Unknown(count)"),
            "undeclared",
        ),
        (
            COUNTER.replace(":Done(count)", ":Done(false)"),
            "kind mismatch",
        ),
        (
            COUNTER.replace("n - 1u64, count + 1u64", "n - 1u64"),
            "arity mismatch",
        ),
        (
            COUNTER.replace("-> :Done(count)", "~> :Done(count)"),
            "asynchronous",
        ),
        (
            COUNTER.replace("-> :Done(count)", "-> :Running(n, count)"),
            "terminal-state transition",
        ),
        (
            COUNTER.replace(
                ":Done(value) => value.",
                ":Done(value) -> :Running(value, value).",
            ),
            "terminal state",
        ),
        (COUNTER.replace("n > 0u64", "n"), "guard must be Boolean"),
    ] {
        let error = compile(
            &format!(
                "{machine}\nresult := #Count(1u64)\nio/write(text/format(\"{{}}\", result))\n"
            ),
            mech_stdlib::native_control_catalog(),
        )
        .unwrap_err();
        assert!(error.contains(message), "{message}: {error}");
    }
}

#[test]
fn actual_advance_machine_matches_old_carry_state_without_external_done_flag() {
    let old = r#"
NeedsCarry(p<[u8]:16,1>, c<[u8]:16,1>, rank<u64>, finished<bool>) => <bool> := finished == false
Carry(p<[u8]:16,1>, c<[u8]:16,1>, rank<u64>, finished<bool>) = (q<[u8]:16,1>, counts<[u8]:16,1>, next<u64>, done<bool>) :=
  digit := sequence/get(c, rank + 1) + 1
  overflow := digit > rank
  q := sequence/rotate-prefix(p, rank + 1, 1)
  counts := sequence/set(c, rank + 1, control/select(overflow, 0, digit))
  next := control/select(overflow, rank + 1, rank)
  done := digit <= rank.
"#;
    let source = format!(
        "{ADVANCE}\n{old}\np := sequence/iota-bytes(16)\nc := sequence/zero-bytes(16)\n(q, counts, rank, done) := control/while((p, c, 1, false), NeedsCarry, Carry)\n(actual, counters) := #Advance(p, c)\nresult := number/bounded(actual[1], q[1], q[1]) + number/bounded(actual[2], q[2], q[2]) + number/bounded(counters[2], counts[2], counts[2])\nio/write(text/format(\"{{}}\", result))\n"
    );
    check(&source, &[]);
}

#[test]
fn complete_fannkuch_uses_machines_matches_and_indexing_with_same_small_outputs() {
    for helper in [
        "control/while",
        "control/if",
        "control/select",
        "sequence/get",
        "sequence/set",
    ] {
        assert!(
            !IDIOMATIC_FANNKUCH.contains(helper),
            "idiomatic fixture retained {helper}"
        );
    }
    for (n, checksum, maximum) in [(3, 2, 2), (4, 4, 4), (5, 11, 7)] {
        for source in [BASELINE_FANNKUCH, IDIOMATIC_FANNKUCH] {
            let end = source.rfind("io/write(text/format(").unwrap();
            let checked = format!(
                "{}io/write(text/format(\"{{}} {{}}\", number/bounded(checksum, {checksum}<i64>, {checksum}<i64>), number/bounded(maximum, {maximum}, {maximum})))\n",
                &source[..end]
            );
            check(&checked, &[n]);
        }
    }
}
