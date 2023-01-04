extern crate crossbeam_channel;
use mech_core::*;
use mech_utilities::*;
//use std::sync::mpsc::{self, Sender};
use std::thread::{self};
use crossbeam_channel::Sender;
use std::collections::HashMap;
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
  "#io/gamepad = [|id<u64> left-stick-x<f32> left-stick-y<f32> right-stick-x<f32> right-stick-y<f32> north<f32> south<f32> east<f32> west<f32>|]".to_string()
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
                  EventType::AxisChanged(Axis::RightStickX,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(4), Value::F32(F32::new(value)))]))]));}
                  EventType::AxisChanged(Axis::RightStickY,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(5), Value::F32(F32::new(value)))]))]));}
                  EventType::ButtonChanged(Button::North,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(6), Value::F32(F32::new(value)))]))]));}
                  EventType::ButtonChanged(Button::South,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(7), Value::F32(F32::new(value)))]))]));}
                  EventType::ButtonChanged(Button::East,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(8), Value::F32(F32::new(value)))]))]));}
                  EventType::ButtonChanged(Button::West,value,code) => {outgoing.send(RunLoopMessage::Transaction(vec![Change::Set((*IO_GAMEPAD,vec![(TableIndex::Index(1), TableIndex::Index(9), Value::F32(F32::new(value)))]))]));}
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