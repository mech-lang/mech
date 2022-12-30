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
use hashbrown::{HashMap, HashSet};
use mech_core::*;
use crossbeam_channel::Sender;
use std::rc::Rc;
use std::cell::RefCell;

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

type CoreIndex = u64;

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
  Reset(CoreIndex),
  PrintDebug,
  //Table(Table),
  String((String, Option<u32>)),
  Exit(i32),
  PrintCore(Option<CoreIndex>),
  DumpCore(CoreIndex),
  NewCore,
  PrintTable(u64),
  PrintInfo,
  PrintRuntime,
  Listening((u64,(TableId,RegisterIndex,RegisterIndex))),
  GetTable(u64),
  GetValue((u64,TableIndex,TableIndex)),
  Transaction(Transaction),
  Code((CoreIndex,MechCode)),
  Blocks(Vec<MiniBlock>),
  RemoteCoreConnect(MechSocket),
  RemoteCoreDisconnect(CoreIndex),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiniTable {
  pub id: u64,     
  pub dynamic: bool,                      
  pub rows: usize,                       
  pub cols: usize,                       
  pub col_kinds: Vec<ValueKind>,                 
  pub col_map: (u64,Vec<Alias>,Vec<(Alias,TableIx)>),  
  pub row_map: (u64,Vec<Alias>,Vec<(Alias,TableIx)>),
  pub data: Vec<Vec<Value>>,
  pub dictionary: Vec<(u64,String)>,
}

impl MiniTable {

  fn minify_table(table: &Table) -> MiniTable {
    MiniTable {
      id: table.id,
      dynamic: table.dynamic,
      rows: table.rows,
      cols: table.cols,
      col_kinds: table.col_kinds.clone(),
      col_map: (0,vec![],vec![]),
      row_map: (0,vec![],vec![]),
      data: vec![],
      dictionary: vec![],
    }
  }

  fn maximize_table(minitable: &MiniTable) -> Table {
    Table {
      id: minitable.id,
      dynamic: minitable.dynamic,
      rows: minitable.rows,
      cols: minitable.cols,                     
      col_kinds: Vec::with_capacity(minitable.cols),
      col_map: AliasMap::new(minitable.cols),
      row_map: AliasMap::new(minitable.rows),
      data: Vec::with_capacity(minitable.cols),
      dictionary: Rc::new(RefCell::new(HashMap::new())),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiniCore {
  //pub sections: Vec<HashMap<BlockId,Rc<RefCell<Block>>>>,
  pub blocks: Vec<MiniBlock>,
  pub unsatisfied_blocks: Vec<MiniBlock>,
  pub database: Vec<MiniTable>,
  //pub functions: Rc<RefCell<Functions>>,
  //pub user_functions: Rc<RefCell<HashMap<u64,UserFunction>>>,
  pub required_functions: Vec<u64>,
  pub errors: Vec<(MechErrorKind,Vec<BlockId>)>,
  pub input: Vec<(TableId,RegisterIndex,RegisterIndex)>,
  pub output: Vec<(TableId,RegisterIndex,RegisterIndex)>,
  pub defined_tables: Vec<(TableId,RegisterIndex,RegisterIndex)>,
  //pub schedule: Schedule,
  pub dictionary: Vec<(u64,String)>,
}

impl MiniCore {

  fn minify_core(core: &Core) -> MiniCore {

    let blocks: Vec<MiniBlock> = core.blocks.iter().map(|(block_id,block_ref)| MiniBlock::minify_block(&block_ref.borrow()) ).collect::<Vec<MiniBlock>>();
    let unsatisfied_blocks: Vec<MiniBlock> = core.unsatisfied_blocks.iter().map(|(block_id,block_ref)| MiniBlock::minify_block(&block_ref.borrow()) ).collect::<Vec<MiniBlock>>();
    let database: Vec<MiniTable> = core.database.borrow().tables.iter().map(|(table_id,table)| MiniTable::minify_table(&table.borrow())).collect::<Vec<MiniTable>>();
    let required_functions: Vec<u64> = core.required_functions.iter().map(|fxn_id| *fxn_id).collect::<Vec<u64>>();
    let errors: Vec<(MechErrorKind,Vec<BlockId>)> = core.errors.iter().map(|(kind,blocks)| (kind.clone(),blocks.iter().map(|b| b.borrow().id ).collect::<Vec<BlockId>>()) ).collect::<Vec<(MechErrorKind,Vec<BlockId>)>>();
    let input: Vec<(TableId,RegisterIndex,RegisterIndex)> = core.input.iter().map(|register| *register).collect::<Vec<(TableId,RegisterIndex,RegisterIndex)>>();
    let output: Vec<(TableId,RegisterIndex,RegisterIndex)> = core.output.iter().map(|register| *register).collect::<Vec<(TableId,RegisterIndex,RegisterIndex)>>();
    let defined_tables: Vec<(TableId,RegisterIndex,RegisterIndex)> = core.defined_tables.iter().map(|register| *register).collect::<Vec<(TableId,RegisterIndex,RegisterIndex)>>();
    let dictionary: Vec<(u64,String)> = core.dictionary.borrow().iter().map(|(k,s)| (*k,s.to_string())).collect::<Vec<(u64,String)>>();

    MiniCore {
      blocks,
      unsatisfied_blocks,
      database,
      required_functions,
      errors,
      input,
      output,
      defined_tables,
      dictionary,
    }
  }

  fn maximize_core(minicore: &MiniCore) -> Core {
    let mut core = Core::new();
    let blocks: Vec<Block> = minicore.blocks.iter().map(|b| MiniBlock::maximize_block(b)).collect();
    core
  }
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

  pub fn minify_block(block: &Block) -> MiniBlock {
    let mut miniblock = MiniBlock::new();
    miniblock.transformations = block.transformations.clone();
    match &block.unsatisfied_transformation {
      Some((_,tfm)) => miniblock.transformations.push(tfm.clone()),
      _ => (),
    }
    miniblock.transformations.append(&mut block.pending_transformations.clone());
    /*for (k,v) in block.store.number_literals.iter() {
      miniblock.number_literals.push((k.clone(), v.clone()));
    }
    for error in &block.errors {
      miniblock.errors.push(error.clone());
    }*/
    miniblock.id = block.id;
    miniblock.ast = block.ast.clone();
    miniblock
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