extern crate mech_program;
extern crate mech_utilities;
extern crate mech_core;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};
use mech_utilities::{RunLoopMessage, MechCode};
use mech_core::{Hasher, Index, Value};

#[test]
fn program_test() {
  let mut runner = ProgramRunner::new("test", 1000);
  let running = runner.run();
  running.send(RunLoopMessage::Code((0,MechCode::String("#data = [1 2 3 4 5]".to_string()))));
  running.send(RunLoopMessage::Stop);

}

#[test]
fn load_module_with_program() {
  let mut runner = ProgramRunner::new("test", 1000);
  let running = runner.run();
  running.send(RunLoopMessage::Code((0,MechCode::String("#test = math/sin(degrees: 0)".to_string()))));
  running.send(RunLoopMessage::GetTable(Hasher::hash_str("test")));
  loop {
    match running.receive() {
      (Ok(ClientMessage::Table(table))) => {
          assert_eq!(table.unwrap().index(&Index::Index(1),&Index::Index(1)),
                     Some(&Value::from_u64(0)));
          break;
      },
      message => (),
    }
  }
}