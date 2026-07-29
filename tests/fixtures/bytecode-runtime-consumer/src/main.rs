use mech_interpreter as _;
use mech_program::{MechProgram, MechProgramConfig};
use std::env;
use std::fs;

fn main() {
  let input = env::args_os()
    .nth(1)
    .expect("usage: bytecode-runtime-consumer <input.mecb>");
  let bytecode = fs::read(input).expect("failed to read bytecode");
  let mut program = MechProgram::new(MechProgramConfig::default());
  program.load_full_stdlib();
  let result = program.run_bytecode(&bytecode).expect("bytecode execution failed");
  let value = result.as_f64().expect("expected f64 output");
  assert_eq!(*value.borrow(), 3.0);
}
