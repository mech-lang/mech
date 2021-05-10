extern crate crossbeam_channel;
use mech_core::{hash_string, TableIndex, Table, Value, ValueType, ValueMethods, Transaction, Change, TableId, Register};
use mech_utilities::{Machine, MachineRegistrar, RunLoopMessage};
//use std::sync::mpsc::{self, Sender};
use std::thread::{self};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;

lazy_static! {
  static ref IO_OUT: u64 = hash_string("io/out");
  static ref TEXT: u64 = hash_string("text");
  static ref COLOR: u64 = hash_string("color");
}

export_machine!(io_out, io_out_reg);

extern "C" fn io_out_reg(registrar: &mut dyn MachineRegistrar, outgoing: Sender<RunLoopMessage>) -> String {
  registrar.register_machine(Box::new(Out{outgoing}));
  "#io/out = [|text color|]".to_string()
}

#[derive(Debug)]
pub struct Out {
  outgoing: Sender<RunLoopMessage>,
}

impl Machine for Out {

  fn name(&self) -> String {
    "io/out".to_string()
  }

  fn id(&self) -> u64 {
    Register{table_id: TableId::Global(*IO_OUT), row: TableIndex::All, column: TableIndex::All}.hash()
  }

  fn on_change(&mut self, table: &Table) -> Result<(), String> {
    for i in 1..=table.rows {
      let value = table.get(&TableIndex::Index(i),&TableIndex::Alias(*TEXT));
      let number_literal = table.get_number_literal(&TableIndex::Index(i),&TableIndex::Alias(*COLOR));
      match (value,number_literal) {
        (Some((value,_)),Some((number_literal,_))) => {
          match (value.value_type(), number_literal.as_u32()) {
            (ValueType::String, color) => {
              let out_string = format!("{}",table.get_string_from_hash(value).unwrap());
              self.outgoing.send(RunLoopMessage::String((out_string,color)));
            }
            (ValueType::Quantity, color) => {
              let out_string = format!("{}",value.as_f64().unwrap());
              self.outgoing.send(RunLoopMessage::String((out_string,color)));
            }
            (ValueType::Boolean, color) => {
              let out_string = format!("{}",value.as_bool().unwrap());
              self.outgoing.send(RunLoopMessage::String((out_string,color)));
            }
            (ValueType::NumberLiteral, color) => {
              // TODO print number literals
            }
            _ => (), // No output representation for other value types
          }
        }
        _ => (), // TODO Warn error
      }
    }
    Ok(())
  }
}

