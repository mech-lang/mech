extern crate mech_program;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};

#[test]
fn program_test() {
  let mut runner = ProgramRunner::new("test", 1000);
  let outgoing = runner.program.outgoing.clone();
  runner.load_program("
  #test = 1 + 1
    ".to_string());
  let running = runner.run();

}