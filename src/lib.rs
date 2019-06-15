#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate mech_core;

use mech_core::Transaction;

// ## Client Message

#[derive(Serialize, Deserialize, Debug)]
pub enum WebsocketClientMessage {
  Listening(Vec<u64>),
  Control(u8),
  Code(String),
  Table(usize),
  RemoveBlock(usize),
  Transaction(Transaction),
}