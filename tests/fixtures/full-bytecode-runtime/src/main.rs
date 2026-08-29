use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder, RuntimeHostInputValue};
use std::env;
use std::fs;

fn main() {
    let input = env::args_os()
        .nth(1)
        .expect("usage: full-bytecode-runtime <input.mecb>");
    let bytecode = fs::read(input).expect("failed to read bytecode");
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::runtime_catalog())
        .build()
        .expect("resident runtime construction failed");
    let result = runtime
        .load_bytecode_program(&bytecode, ResidentDurabilityPolicy::Volatile)
        .expect("bytecode execution failed")
        .initial_value
        .into_value();
    let value = RuntimeHostInputValue::from_numeric_value(&result)
        .expect("expected canonical numeric output");
    assert_eq!(value, RuntimeHostInputValue::F64(3.0));
}
