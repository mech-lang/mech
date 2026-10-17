//! Native Control consumes the same evaluated selection bases as canonical syntax.
#![cfg(feature = "sequence-cli")]
use mech_engine::native_sequence::control::compile;
use mech_gpu::control::{JitProgram, ResidentProgram};

fn check(source: &str) {
    let program = compile(source, mech_stdlib::native_control_catalog())
        .unwrap_or_else(|e| panic!("{source}\n{e}"));
    ResidentProgram::lower(&program).unwrap().run(&[]).unwrap();
    for simd in [false, true] {
        JitProgram::from_program(program.clone(), simd)
            .unwrap()
            .run(&[])
            .unwrap();
    }
}
#[test]
fn returned_byte_columns_records_and_parenthesized_values_use_typed_selection() {
    check(
        "Bytes() => (out<[u8]:4,1>) := out := sequence/iota-bytes(4u64).\nresult := Bytes()[3u64]\nio/write(text/format(\"{}\", number/bounded(result, 2u64, 2u64)))\n",
    );
    check(
        "Record() => (out<{value<u64>}>) := out := {value: 7u64}.\nresult := Record().value\nio/write(text/format(\"{}\", number/bounded(result, 7u64, 7u64)))\n",
    );
    check(
        "result := (sequence/iota-bytes(4u64))[3u64]\nio/write(text/format(\"{}\", number/bounded(result, 2u64, 2u64)))\n",
    );
}
#[test]
fn unsupported_text_selection_and_expression_target_writes_reject() {
    let text =
        "Text() => (out<string>) := out := \"éZ\".\nresult := Text()[1u64]\nio/write(result)\n";
    assert!(compile(text, mech_stdlib::native_control_catalog()).is_err());
    assert!(
        mech_engine::native_sequence::compile(text, 256, mech_stdlib::native_sequence_catalog())
            .is_err()
    );
    let write =
        "Bytes() => (out<[u8]:4,1>) := out := sequence/iota-bytes(4u64).\nBytes()[1u64] = 9u64\n";
    assert!(compile(write, mech_stdlib::native_control_catalog()).is_err());
}
