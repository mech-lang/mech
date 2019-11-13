extern crate mech_program;
extern crate mech_utilities;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};
use mech_utilities::Watcher;

#[test]
fn program_test() {
  let mut runner = ProgramRunner::new("test", 1000);
  runner.load_program("
  #test = 1 + 1
    ".to_string());
  let running = runner.run();
}

#[test]
fn load_module_with_program() {
  let mut runner = ProgramRunner::new("test", 1000);
  runner.load_program("#test = math/floor(column: 1.5)".to_string());
  let running = runner.run();
}