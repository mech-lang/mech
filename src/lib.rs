#![allow(dead_code)]
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;
extern crate hashbrown;
extern crate crossbeam_channel;

use hashbrown::HashMap;
use mech_core::{Table, Value, Transaction, TableId, Transformation, Register, Change};

use crossbeam_channel::Sender;

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum WebsocketMessage {
  Listening(Register),
  Control(u8),
  Code(MechCode),
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
  Listening(Register),
  GetTable(u64),
  Transaction(Transaction),
  Code((u64,MechCode)),
  EchoCode(String),
  Blocks(Vec<MiniBlock>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniBlock {
  pub transformations: Vec<(String, Vec<Transformation>)>,
  pub plan: Vec<Transformation>,
  pub strings: Vec<(u64, String)>,
  pub register_map: Vec<(u64, Register)>,
}

impl MiniBlock {
  pub fn new() -> MiniBlock { 
    MiniBlock {
      transformations: Vec::with_capacity(1),
      plan: Vec::with_capacity(1),
      strings: Vec::with_capacity(1),
      register_map: Vec::with_capacity(1),
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
    pub register: unsafe extern "C" fn(&mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>)->Vec<Change>,
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

