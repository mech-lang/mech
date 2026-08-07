use mech_interpreter as _;
use mech_engine::{MechProgram, MechProgramConfig};
use std::env;
use std::fs;

fn main() {
    let output = env::args_os()
        .nth(1)
        .expect("usage: bytecode-compiler-producer <output.mecb>");
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.load_full_stdlib();
    program
        .run_string("1.0 + 2.0")
        .expect("source execution failed");
    let bytecode = program
        .compile_bytecode()
        .expect("bytecode compilation failed");
    fs::write(output, bytecode).expect("failed to write bytecode");
}
