extern crate crossbeam_channel;
use mech_core::*;
use mech_utilities::*;
//use std::sync::mpsc::{self, Sender};
use std::thread::{self};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use gilrs::{Gilrs, Button, Event};
use gilrs::ev::{EventType, Axis};

lazy_static! {
  static ref IO_GAMEPAD: u64 = hash_str("io/gamepad");
  static ref TEXT: u64 = hash_str("text");
  static ref COLOR: u64 = hash_str("color");
}

export_machine!(io_gamepad, io_gamepad_reg);

extern "C" fn io_gamepad_reg(registrar: &mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>) -> String {
  let mut gilrs = Gilrs::new().unwrap();
  registrar.register_machine(Box::new(Gamepad{outgoing,gamepads: HashMap::new(), gilrs}));
  "#io/gamepad = [|id<u64> left-stick-x<f32> left-stick-y<f32>|]".to_string()
}

#[derive(Debug)]
pub struct Gamepad {
  outgoing: Sender<RunLoopMessage>,
  gamepads: HashMap<usize, std::thread::JoinHandle<()>>,
  gilrs: Gilrs,
}

impl Machine for Gamepad {

  fn name(&self) -> String {
    "io/gamepad".to_string()
  }

  fn id(&self) -> u64 {
    hash_str(&self.name())
  }

  fn on_change(&mut self, table: &Table) -> Result<(), MechError> {
    for i in 1..=table.rows {
      match self.gamepads.get(&i) {
        Some(timer) => {

        }
        None => {
          let mut gilrs = Gilrs::new().unwrap();
          let outgoing = self.outgoing.clone();
          let gamepad_handle = thread::spawn(move || {
            loop {
              // Examine new events
              while let Some(Event { id, event, time }) = gilrs.next_event() {
                match event {
                  EventType::AxisChanged(Axis::LeftStickX,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(F32::new(value)))]))]));}
                  EventType::AxisChanged(Axis::LeftStickY,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(3), Value::F32(F32::new(value)))]))]));}
                  x => (),
                }
              }
            }
          });
          self.gamepads.insert(i,gamepad_handle);                  
        }
      }
    }
    Ok(())
  }
}

/*

let timer_handle = thread::spawn(move || {
  let duration = Duration::from_millis((period.unwrap() * 1000.0) as u64);
  let mut counter = 0;
  loop {
    thread::sleep(duration);
    counter = counter + 1;
    outgoing.send(RunLoopMessage::Transaction(vec![
      Change::Set((*TIME_TIMER,vec![(timer_row, TableIndex::Alias(*TICKS), Value::U64(U64::new(counter)))]))
    ]));
  }
});

*/