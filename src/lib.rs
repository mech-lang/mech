#![allow(warnings)]
#![allow(dead_code)]
#![feature(get_mut_unchecked)]
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;
extern crate hashbrown;
extern crate crossbeam_channel;
extern crate core as rust_core;

use rust_core::fmt;
use std::sync::Arc;
use hashbrown::HashMap;
use mech_core::*;
use crossbeam_channel::Sender;

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum SocketMessage {
  Ping,
  Pong,
  RemoteCoreConnect(String),
  RemoteCoreDisconnect(u64),
  Listening((TableId,RegisterIndex,RegisterIndex)),
  Producing((TableId,RegisterIndex,RegisterIndex)),
  Code(MechCode),
  RemoveBlock(usize),
  Transaction(Transaction),
}

// Run loop messages are sent to the run loop from the client

// This is dumb that I need to put this on every line :(
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "web")]
extern crate websocket;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "web")]
pub enum MechSocket {
  UdpSocket(String),
  WebSocket(websocket::sync::Client<std::net::TcpStream>),
  WebSocketSender(websocket::sender::Writer<std::net::TcpStream>),
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "web")]
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
#[cfg(feature = "web")]
#[repr(C)]
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
  PrintDebug,
  //Table(Table),
  String((String, Option<u32>)),
  Exit(i32),
  PrintCore(Option<u64>),
  DumpCore(u64),
  PrintTable(u64),
  PrintRuntime,
  Listening((u64,(TableId,RegisterIndex,RegisterIndex))),
  GetTable(u64),
  GetValue((u64,TableIndex,TableIndex)),
  Transaction(Transaction),
  Code(MechCode),
  Blocks(Vec<MiniBlock>),
  RemoteCoreConnect(MechSocket),
  RemoteCoreDisconnect(u64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiniBlock {
  pub id: u64,
  pub ast: nodes::AstNode,
  pub transformations: Vec<Transformation>,
  pub strings: Vec<(u64, String)>,
  pub number_literals: Vec<(u64, NumberLiteral)>,
}

impl MiniBlock {
  pub fn new() -> MiniBlock { 
    MiniBlock {
      id: 0,
      ast: nodes::AstNode::Null,
      transformations: Vec::with_capacity(1),
      strings: Vec::with_capacity(1),
      number_literals: Vec::with_capacity(1),
    }
  }

  pub fn maximize_block(miniblock: &MiniBlock) -> Block {
    let mut block = Block::new();
    for tfms in &miniblock.transformations {
      block.add_tfm(tfms.clone());
    }
    block.ast = miniblock.ast.clone();
    block
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MechCode {
  String(String),
  MiniBlocks(Vec<Vec<MiniBlock>>),
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "web")]
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

#[derive(Copy, Clone)]
pub struct MechFunctionDeclaration {
  pub register: unsafe extern "C" fn(&mut dyn MechFunctionRegistrar),
}

pub trait MechFunctionRegistrar {
  fn register_mech_function(&mut self, function_id: u64, mech_function_compiler: Box<dyn MechFunctionCompiler>);
}

#[macro_export]
macro_rules! export_mech_function {
  ($name:ident, $register:expr) => {
    #[doc(hidden)]
    #[no_mangle]
    pub static $name: $crate::MechFunctionDeclaration =
      $crate::MechFunctionDeclaration {
        register: $register,
      };
  };
}