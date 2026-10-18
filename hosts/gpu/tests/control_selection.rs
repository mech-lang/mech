//! Canonical indexing and value-copy updates through the public control paths.
#![cfg(feature = "sequence-cli")]

use mech_engine::native_sequence::control::{Program, compile};
use mech_gpu::control::{JitProgram, ResidentProgram};

fn program(source: &str) -> Program {
    compile(source, mech_stdlib::native_control_catalog())
        .unwrap_or_else(|error| panic!("{source}\n{error}"))
}

fn checked_source(body: &str, expected: u64) -> String {
    // A wrong result fails inside each backend instead of relying on capturing
    // process stdout. Both bounds are exact, so successful execution checks the
    // value as well as admitting the source and executing its selected path.
    format!(
        "{body}\nio/write(text/format(\"{{}}\", number/bounded(result, {expected}, {expected})))\n"
    )
}

fn check(source: &str, args: &[u64]) {
    let program = program(source);
    ResidentProgram::lower(&program)
        .unwrap()
        .run(args)
        .unwrap_or_else(|error| panic!("resident: {source}\n{error}"));
    for simd in [false, true] {
        JitProgram::from_program(program.clone(), simd)
            .unwrap()
            .run(args)
            .unwrap_or_else(|error| panic!("JIT simd={simd}: {source}\n{error}"));
    }
}

fn rejects_execution(source: &str, args: &[u64]) {
    let program = program(source);
    assert!(
        ResidentProgram::lower(&program).unwrap().run(args).is_err(),
        "resident accepted invalid access: {source}"
    );
    for simd in [false, true] {
        assert!(
            JitProgram::from_program(program.clone(), simd)
                .unwrap()
                .run(args)
                .is_err(),
            "JIT simd={simd} accepted invalid access: {source}"
        );
    }
}

#[test]
fn scalar_indices_preserve_record_fields_and_one_based_sequence_reads() {
    let fixed = checked_source(
        "At(c<[u8]:4,1>, index<u64>) => <u64> := c[index]\nc := sequence/iota-bytes(4)\nresult := At(c, io/argument-u64(1))",
        3,
    );
    check(&fixed, &[4]);
    let bytes = checked_source("data := bytes/from-text(\"AZ\")\nresult := data[2]", 90);
    check(&bytes, &[]);
    let floats = checked_source(
        "data := sequence/f64-values((0.5, 1.5))\nresult := control/select(data[2] == 1.5, 1, 0)",
        1,
    );
    check(&floats, &[]);
    let record = checked_source(
        "state := {inner: {count: 7}, bytes: sequence/iota-bytes(4)}\nresult := state.inner.count + state.bytes[4]",
        10,
    );
    check(&record, &[]);
}

#[test]
fn zero_and_out_of_bounds_indices_keep_shared_native_and_resident_guards() {
    let source = checked_source(
        "At(c<[u8]:4,1>, index<u64>) => <u64> := c[index]\nc := sequence/iota-bytes(4)\nresult := At(c, io/argument-u64(1))",
        0,
    );
    for index in [0, 5, u64::MAX] {
        rejects_execution(&source, &[index]);
    }
    for source in [
        "data := bytes/from-text(\"AZ\")\nresult := data[0]",
        "data := bytes/from-text(\"AZ\")\nresult := data[3]",
        "data := sequence/f64-values((0.5, 1.5))\nresult := control/select(data[0] == 0.5, 1, 0)",
        "data := sequence/f64-values((0.5, 1.5))\nresult := control/select(data[3] == 0.5, 1, 0)",
    ] {
        rejects_execution(&checked_source(source, 0), &[]);
    }
}

#[test]
fn unsupported_selectors_and_non_integer_indices_are_rejected() {
    for selector in [
        "c[1, 2]", "c[1..=2]", "c[:]", "c[[1 2]]", "c[true]", "c[1.0]", "c{1}",
    ] {
        let source = checked_source(
            &format!("c := sequence/iota-bytes(4)\nresult := {selector}"),
            0,
        );
        assert!(
            compile(&source, mech_stdlib::native_control_catalog()).is_err(),
            "unsupported selection was accepted: {source}"
        );
    }
}

#[test]
fn ordinary_function_local_writes_preserve_the_input_value() {
    let source = checked_source(
        "Copy(c<[u8]:4,1>) = (out<[u8]:4,1>) :=\n  ~counts := c\n  counts[1u64] = 9u64\n  counts[4u64] = counts[1u64] + 1u64\n  out := counts.\noriginal := sequence/iota-bytes(4u64)\nupdated := Copy(original)\nresult := updated[1u64] + 10u64 * updated[4u64] + 100u64 * original[1u64] + 1000u64 * original[4u64]",
        3109,
    );
    // Deliberately exercise the closed-function path, where the global native
    // function flag is false and local writes must still remain observable.
    assert!(!program(&source).native_functions);
    check(&source, &[]);
}

#[test]
fn ordinary_function_unused_writes_still_enforce_index_and_value_guards() {
    for invalid_write in [
        "counts[0u64] = 1u64",
        "counts[5u64] = 1u64",
        "counts[1u64] = 256u64",
    ] {
        let source = checked_source(
            &format!(
                "Discard(c<[u8]:4,1>) = (out<u64>) :=\n  ~counts := c\n  {invalid_write}\n  counts[1u64] = 2u64\n  out := 7u64.\nc := sequence/zero-bytes(4u64)\nresult := Discard(c)"
            ),
            7,
        );
        assert!(!program(&source).native_functions);
        rejects_execution(&source, &[]);
    }
}

const UPDATE: &str = r#"
#Update(c<[u8]:4,1>) => <[u8]:4,1>
  | :Carry(c<[u8]:4,1>, rank<u64>)
  | :Done(c<[u8]:4,1>).
#Update(c) -> :Carry(c, 1)
  :Carry(c, rank)
    | rank <= 4 -> {
        ~counts := c
        counts[rank] = counts[rank] + rank
        ~next := rank
        next = next + 1
      } -> :Carry(counts, next)
    | rank > 4 -> :Done(c)
  :Done(c) => c.
original := sequence/zero-bytes(4)
updated := #Update(original)
result := updated[1] + 10 * updated[2] + 100 * updated[3] + 1000 * updated[4] + 10000 * (original[1] + original[2] + original[3] + original[4])
"#;

#[test]
fn fsm_branch_updates_are_ordered_value_copies_with_local_mutability() {
    check(&checked_source(UPDATE, 4321), &[]);
}

fn one_update(body: &str, output: &str) -> String {
    format!(
        "#Update(c<[u8]:4,1>, index<u64>) => <[u8]:4,1>\n  | :Carry(c<[u8]:4,1>, index<u64>)\n  | :Done(c<[u8]:4,1>).\n#Update(c, index) -> :Carry(c, index)\n  :Carry(c, index) -> {{\n{body}\n  }} -> :Done({output})\n  :Done(c) => c.\noriginal := sequence/zero-bytes(4)\nupdated := #Update(original, io/argument-u64(1))\nresult := updated[1]\n"
    )
}

#[test]
fn branch_writes_require_local_mutability_and_preserve_kinds_and_selectors() {
    for (body, output, message) in [
        ("c[index] = 1", "c", "branch-local mutable binding"),
        (
            "counts := c\ncounts[index] = 1",
            "counts",
            "branch-local mutable binding",
        ),
        (
            "~counts := c\n~counts := c",
            "counts",
            "duplicate branch definition",
        ),
        (
            "~counts := c\ncounts[index] = false",
            "counts",
            "incompatible native kinds",
        ),
        (
            "~counts := c\ncounts = 1",
            "counts",
            "preserve the binding kind",
        ),
        (
            "~counts := c\ncounts[1, 2] = 1",
            "counts",
            "one scalar formula index",
        ),
        (
            "~counts := c\ncounts[1..=2] = 1",
            "counts",
            "one scalar formula index",
        ),
        (
            "~counts := c\ncounts[true] = 1",
            "counts",
            "index must be u64",
        ),
    ] {
        let source = checked_source(&one_update(body, output), 0);
        let error = compile(&source, mech_stdlib::native_control_catalog()).unwrap_err();
        assert!(error.contains(message), "{source}\n{error}");
    }
    let source = checked_source(
        &one_update("~counts := c\ncounts[index] = 1", "counts")
            .replace(":Done(c) => c.", ":Done(c) => counts."),
        0,
    );
    assert!(compile(&source, mech_stdlib::native_control_catalog()).is_err());
}

#[test]
fn indexed_writes_keep_index_and_byte_value_guards() {
    let source = checked_source(&one_update("~counts := c\ncounts[index] = 1", "counts"), 1);
    check(&source, &[1]);
    for index in [0, 5, u64::MAX] {
        rejects_execution(&source, &[index]);
    }
    let source = checked_source(
        &one_update("~counts := c\ncounts[index] = 256", "counts"),
        0,
    );
    rejects_execution(&source, &[1]);
}
