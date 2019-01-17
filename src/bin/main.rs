extern crate mech_program;
use mech_program::{ProgramRunner, RunLoop, RunLoopMessage};

fn main() {

  let mut runner = ProgramRunner::new("p1", 1000);
  runner.load_program("#system/timer = [resolution: 1000]".to_string());
  runner.run();

  let mut runner = ProgramRunner::new("p2", 1000);
  runner.load_program("#ball = 1324".to_string());
  runner.run();
  loop{}

}   