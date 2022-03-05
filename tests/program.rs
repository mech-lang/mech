extern crate mech_program;
extern crate mech_utilities;
extern crate mech_core;
use mech_program::*;
use mech_utilities::*;
use mech_core::*;

#[test]
fn program_test() {
  let mut runner = ProgramRunner::new("test", 1000);
  let running = runner.run().unwrap();
  running.send(RunLoopMessage::Code(MechCode::String("#data = [1 2 3 4 5]".to_string())));
  running.send(RunLoopMessage::Stop);

}

#[test]
fn load_module_with_program() {
  let mut runner = ProgramRunner::new("test", 1000);
  let running = runner.run().unwrap();
  running.send(RunLoopMessage::Code(MechCode::String("#test = math/sin(angle: 0)".to_string())));
  running.send(RunLoopMessage::GetValue((hash_str("test"),TableIndex::Index(1),TableIndex::Index(1))));
  loop {
    match running.receive() {
      Ok(ClientMessage::Value(value)) => {
          assert_eq!(value, Value::F32(F32::new(0.0)));
          break;
      },
      message => (),
    }
  }
}