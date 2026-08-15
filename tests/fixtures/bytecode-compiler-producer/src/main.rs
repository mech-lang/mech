use mech_runtime::RuntimeBuilder;
use std::env;
use std::fs;

fn main() {
    let output = env::args_os()
        .nth(1)
        .expect("usage: bytecode-compiler-producer <output.mecb>");
    let bytecode = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .expect("source compiler construction failed")
        .compile_source("1.0 + 2.0")
        .expect("bytecode compilation failed")
        .into_parts()
        .1;
    fs::write(output, bytecode).expect("failed to write bytecode");
}
