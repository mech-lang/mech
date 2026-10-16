//! Counted application uses the public source frontend and shared native kernels.
#![cfg(feature = "sequence-cli")]

use mech_gpu::sequence::{Context, JitProgram, compile};

fn check(source: &str, expected: &[u8], iterations: usize) {
    let program = compile(source, 256).unwrap_or_else(|e| panic!("{source}\n{e}"));
    assert_eq!(
        program.nodes.iter().filter(|n| n.op == 39).count(),
        iterations
    );
    assert!(!program.object(256).unwrap().is_empty());
    let mut resident = Context::new(&program.constants, None, false);
    resident.run_resident(&program).unwrap();
    assert_eq!(resident.output, expected, "resident: {source}");
    for simd in [false, true] {
        let jit = JitProgram::compile(&program, 256, simd).unwrap();
        let mut context = Context::new(&program.constants, None, false);
        jit.run(&mut context).unwrap();
        assert_eq!(context.output, expected, "native simd={simd}: {source}");
    }
}

#[test]
fn zero_one_and_nineteen_applications_match_explicit_iteration() {
    for (count, result) in [(0, "2"), (1, "5"), (19, "59")] {
        for expression in [
            format!("2.0 |{count}> step"),
            format!("sequence/iterate(2.0, {count}, step)"),
        ] {
            let source = format!(
                "amount := 3.0\nstep(x<f64>) => <f64> := x + amount\nr := {expression}\nio/write(number/format(r, 0))\n"
            );
            check(&source, result.as_bytes(), 1);
        }
    }
}

#[test]
fn vector_seed_and_mixed_chains_keep_order_and_captures() {
    let source = "amount := 2.0\nstep(x<[f64]>) => <[f64]> := x + amount\ntwice(x<[f64]>) => <[f64]> := x * 2.0\nr := (sequence/fill(2, 1.0) |19> step) |> twice |1> step |0> twice\nio/write(text/join(number/format(r, 0), \",\"))\n";
    check(source, b"80,80", 3);
}

#[test]
fn unsigned_counters_use_existing_checked_integer_callbacks() {
    for (count, result) in [(0, "2"), (1, "5"), (19, "59")] {
        let source = format!(
            "amount := 3\nstep(x<u64>) => <u64> := x + amount\nr := 2 |{count}> step\nio/write(number/format(r, 0))\n"
        );
        check(&source, result.as_bytes(), 1);
    }
    let source = "step(x<u64>) => <u64> := x + 1\nr := 18446744073709551615 |1> step\nio/write(number/format(r, 0))\n";
    let program = compile(source, 256).unwrap();
    let mut context = Context::new(&program.constants, None, false);
    assert!(context.run_resident(&program).is_err());
    let jit = JitProgram::compile(&program, 256, false).unwrap();
    let mut context = Context::new(&program.constants, None, false);
    assert!(jit.run(&mut context).is_err());
}

#[test]
fn runtime_seed_input_is_lowered_once_before_repetition() {
    let source = "step(x<u64>) => <u64> := x + 1\nr := text/length(io/read()) |19> step\nio/write(number/format(r, 0))\n";
    let program = compile(source, 256).unwrap();
    assert_eq!(program.nodes.iter().filter(|n| n.op == 1).count(), 1);
    assert_eq!(program.nodes.iter().filter(|n| n.op == 39).count(), 1);
    let mut resident = Context::new(&program.constants, Some(b"seed".to_vec()), false);
    resident.run_resident(&program).unwrap();
    assert_eq!(resident.output, b"23");
    let jit = JitProgram::compile(&program, 256, false).unwrap();
    let mut context = Context::new(&program.constants, Some(b"seed".to_vec()), false);
    jit.run(&mut context).unwrap();
    assert_eq!(context.output, b"23");
}

#[test]
fn zero_still_checks_seed_callback_and_capture_dependencies() {
    for source in [
        "r := 2.0 |0> missing\n",
        "step(x<u64>) => <u64> := x\nr := 2.0 |0> step\n",
        "step(x<f64>) => <f64> := x + missing\nr := 2.0 |0> step\n",
        "step(x<f64>) => <f64> := io/read()\nr := 2.0 |0> step\n",
    ] {
        assert!(compile(source, 256).is_err(), "{source}");
    }
    let source = "step(x<u64>) => <u64> := x\nr := (18446744073709551615 + 1) |0> step\n";
    let program = compile(source, 256).unwrap();
    let mut resident = Context::new(&program.constants, None, false);
    assert!(resident.run_resident(&program).is_err());
    let jit = JitProgram::compile(&program, 256, false).unwrap();
    let mut context = Context::new(&program.constants, None, false);
    assert!(jit.run(&mut context).is_err());
}
