#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;

use mech_core::{Transaction, Interner, TableId, Core, Constraint};

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum WebsocketClientMessage {
  Listening(Vec<TableId>),
  Control(u8),
  Code(MechCode),
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
  Code((u64,MechCode)),
  EchoCode(String),
  Blocks(Vec<MiniBlock>),
  //Core(Core),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniBlock {
  pub constraints: Vec<(String, Vec<Constraint>)>,
}

impl MiniBlock {
  
  pub fn new() -> MiniBlock { 
    MiniBlock {
      constraints: Vec::with_capacity(1),
    }
  }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechCode {
  String(String),
  MiniBlock(String),
}