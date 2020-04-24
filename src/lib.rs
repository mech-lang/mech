#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;

use mech_core::{Transaction, Interner, TableId, Core};

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum WebsocketClientMessage {
  Listening(Vec<TableId>),
  Control(u8),
  Code(String),
  Table(usize),
  RemoveBlock(usize),
  Transaction(Transaction),
}

// Run loop messages are sent to the run loop from the client

#[derive(Debug, Clone)]
pub enum RunLoopMessage {
  Stop,
  StepBack,
  StepForward,
  Pause,
  Resume,
  Clear,
  PrintCore(Option<u64>),
  PrintRuntime,
  Listening(Vec<TableId>),
  Table(u64),
  Transaction(Transaction),
  Code((u64,String)),
  EchoCode(String),
  //Core(Core),
}

// ## Watchers

pub trait Watcher {
  fn get_name(& self) -> String;
}
