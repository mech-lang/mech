#![allow(dead_code)]
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;
extern crate hashbrown;
extern crate crossbeam_channel;
extern crate core as rust_core;

use rust_core::fmt;
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

// This is dumb that I need to put this on every line :(
#[cfg(not(target_arch = "wasm32"))]
extern crate websocket;

#[cfg(not(target_arch = "wasm32"))]
pub enum MechSocket {
  UdpSocket(String),
  WebSocket(websocket::sync::Client<std::net::TcpStream>),
  WebSocketSender(websocket::sender::Writer<std::net::TcpStream>),
}

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Debug for MechSocket {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &MechSocket::UdpSocket(ref address) => write!(f, "MechSocket::UdpSocket({})", address),
      &MechSocket::WebSocket(ref ws) => write!(f, "MechSocket::WebSocket({})", ws.peer_addr().unwrap()),
      &MechSocket::WebSocketSender(_) => write!(f, "MechSocket::WebSocketSender()"),
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
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
  RemoteCoreConnect(MechSocket),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniProgram {
  pub blocks: Vec<MiniBlock>,
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

#[cfg(not(target_arch = "wasm32"))]
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

