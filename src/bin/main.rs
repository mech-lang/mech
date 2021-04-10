extern crate mech_program;
extern crate mech_utilities;
extern crate mech_core;
extern crate crossbeam_channel;
extern crate mech_syntax;
use mech_program::{ProgramRunner, RunLoop, ClientMessage};
use mech_utilities::{RunLoopMessage, MechCode};
use mech_core::{hash_string, TableIndex, Value, ValueMethods};

use hashbrown::HashMap;

use libloading::Library;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

use std::rc::Rc;

fn main() {
  let mut runner = ProgramRunner::new("test", 1000);
  let running = runner.run();
  running.send(RunLoopMessage::Code((0,MechCode::String("#test = math/sin(angle: 0)".to_string()))));
  running.send(RunLoopMessage::GetTable(hash_string("test")));
  loop {
    match running.receive() {
      (Ok(ClientMessage::Table(table))) => {
          let value = table.unwrap().get(&TableIndex::Index(1),&TableIndex::Index(1)).unwrap();
          assert_eq!(value, Value::from_f64(0.0));
          break;
      },
      message => (),
    }
  }
}