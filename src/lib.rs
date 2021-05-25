#![allow(dead_code)]
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;
extern crate hashbrown;
extern crate crossbeam_channel;

use hashbrown::HashMap;
use mech_core::{Table, Value, Error, Transaction, TableId, Transformation, Register, Change, NumberLiteral};

use crossbeam_channel::Sender;

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum SocketMessage {
  Ping,
  Pong,
  RemoteCoreConnect(String),
  RemoteCoreDisconnect(String),
  Listening(Register),
  Producing(Register),
  Code(MechCode),
  RemoveBlock(usize),
  Transaction(Transaction),
}

// Run loop messages are sent to the run loop from the client

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunLoopMessage {
  Ping,
  Pong,
  Stop,
  StepBack,
  StepForward,
  Pause,
  Resume,
  Clear,
  String((String, u32)),
  Exit(i32),
  PrintCore(Option<u64>),
  PrintRuntime,
  Listening((u64,Register)),
  GetTable(u64),
  Transaction(Transaction),
  Code((u64,MechCode)),
  EchoCode(String),
  Blocks(Vec<MiniBlock>),
  RemoteCoreConnect(String),
  RemoteCoreDisconnect(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniBlock {
  pub id: u64,
  pub transformations: Vec<(String, Vec<Transformation>)>,
  pub plan: Vec<Transformation>,
  pub strings: Vec<(u64, String)>,
  pub errors: Vec<Error>,
  pub number_literals: Vec<(u64, NumberLiteral)>,
}

impl MiniBlock {
  pub fn new() -> MiniBlock { 
    MiniBlock {
      id: 0,
      transformations: Vec::with_capacity(1),
      plan: Vec::with_capacity(1),
      strings: Vec::with_capacity(1),
      errors: Vec::with_capacity(1),
      number_literals: Vec::with_capacity(1),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechCode {
  String(String),
  MiniBlocks(Vec<MiniBlock>),
}

pub trait Machine {
  fn name(&self) -> String;
  fn id(&self) -> u64;
  fn on_change(&mut self, table: &Table) -> Result<(), String>;
}

#[derive(Copy, Clone)]
pub struct MachineDeclaration {
  pub register: unsafe extern "C" fn(&mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>)->String,
}

pub trait MachineRegistrar {
  fn register_machine(&mut self, machine: Box<dyn Machine>);
}

#[macro_export]
macro_rules! export_machine {
  ($name:ident, $register:expr) => {
    #[doc(hidden)]
    #[no_mangle]
    pub static $name: $crate::MachineDeclaration =
      $crate::MachineDeclaration {
        register: $register,
      };
  };
}

