extern crate mech_program;
extern crate mech_utilities;
extern crate mech_core;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};
use mech_utilities::{Watcher, RunLoopMessage};
use mech_core::{Value, Hasher};

fn main() {
  let mut runner = ProgramRunner::new("test", 1000);
  let outgoing = runner.program.outgoing.clone();
  runner.load_program("
block
  #test1 = math/sin(degrees: 45)
  #test2 = math/cos(degrees: 60)
block
  #test = #test1 + #test2".to_string());
  let running = runner.run();
  running.send(RunLoopMessage::Table(Hasher::hash_str("test")));
  loop {
    match running.receive() {
      (Ok(ClientMessage::Table(table))) => {
          println!("{:?}", table);
      },
      message => println!("{:?}", message),
    }
  }
}