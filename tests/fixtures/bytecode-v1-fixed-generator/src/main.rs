use std::collections::BTreeSet;
use std::env;
use std::fs;

use mech_core::{BytecodeInstruction, ParsedProgram};
use mech_runtime::RuntimeBuilder;

fn main() {
    let output = env::args_os()
        .nth(1)
        .expect("usage: bytecode-v1-fixed-generator <output.mecb> <functions.json> <source>");
    let functions_output = env::args_os()
        .nth(2)
        .expect("usage: bytecode-v1-fixed-generator <output.mecb> <functions.json> <source>");
    let source = env::args_os()
        .nth(3)
        .expect("usage: bytecode-v1-fixed-generator <output.mecb> <functions.json> <source>");
    let source = source
        .into_string()
        .expect("authoritative source argument must be UTF-8");
    let bytecode = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .expect("fixed-matrix source compiler construction failed")
        .compile_source_frozen_v1(&source)
        .expect("fixed-matrix bytecode compilation failed")
        .into_parts()
        .1;
    let parsed = ParsedProgram::from_bytes(&bytecode).expect("fixed bytecode must parse");
    let catalog = mech_stdlib::runtime_catalog();
    let functions = parsed
        .instructions
        .iter()
        .filter_map(BytecodeInstruction::runtime_function)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|id| {
            catalog
                .runtime_entries()
                .find(|entry| entry.id.raw() == id)
                .unwrap_or_else(|| panic!("fixed source emitted unknown runtime ID {id:016x}"))
                .name
                .clone()
        })
        .collect::<Vec<_>>();
    fs::write(output, bytecode).expect("failed to write fixed-matrix bytecode");
    fs::write(
        functions_output,
        serde_json::to_vec(&functions).expect("fixed runtime names must serialize"),
    )
    .expect("failed to write fixed-matrix runtime names");
}
