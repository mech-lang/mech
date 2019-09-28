extern crate mech_program;
extern crate rumble;




use mech_program::{ProgramRunner, RunLoop, RunLoopMessage};

fn main() {

  let mut runner = ProgramRunner::new("p1", 1000);
  runner.load_program("#time/timer = [period: 1000]".to_string());
  runner.run();

  let mut runner = ProgramRunner::new("p2", 1000);
  runner.load_program("#ball = 1324".to_string());
  runner.run();
  println!("FOO");
  //loop{}

}