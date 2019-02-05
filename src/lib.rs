#![feature(alloc)]
#![feature(drain_filter)]
extern crate wasm_bindgen;
extern crate hashbrown;
//extern crate web_sys;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate serde_derive;
extern crate core;
extern crate web_sys;
extern crate mech_core;
extern crate mech_syntax;

use wasm_bindgen::prelude::*;
use hashbrown::hash_set::HashSet;
use alloc::vec::Vec;
use core::fmt;
use mech_syntax::compiler::Compiler;
use mech_core::{Transaction, Hasher, Change, Index, Value, Table};

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub struct Core {
  core: mech_core::Core,
  changes: Vec<Change>,
}

#[wasm_bindgen]
impl Core {

  pub fn new() -> Core {
    Core {
      core: mech_core::Core::new(100,100),
      changes: Vec::new(),
    }
  }

  pub fn compile_code(&mut self, code: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(code);
    self.core.register_blocks(compiler.blocks.clone());
    self.core.step();
    log!("Compiled {} blocks.", compiler.blocks.len());
  }


  pub fn clear(&mut self) {
    self.core.clear();
    log!("Core Cleared");
  }

  pub fn pause(&mut self) {
    self.core.pause();
    log!("Core Paused");
  }

  pub fn resume(&mut self) {
    self.core.resume();
    log!("Core Resumed");
  }

  pub fn display_core(&mut self) {
    log!("{:?}", self.core);
  }

  pub fn display_runtime(&mut self) {
    log!("{:?}", self.core.runtime);
  }

  pub fn queue_change(&mut self, table: String, row: u32, column: u32, value: u32) {
    let table_id = Hasher::hash_string(table);
    let change = Change::Set{table: table_id, 
                             row: Index::Index(row as u64), 
                             column: Index::Index(column as u64),
                             value: Value::from_u64(value as u64),
                            };
    self.changes.push(change);
  }

  pub fn process_transaction(&mut self) {
    let txn = Transaction::from_changeset(self.changes.clone());
    log!("{:?}", txn);
    self.core.process_transaction(&txn);
    self.changes.clear();
  }

  pub fn get_column(&mut self, table: u64, column: u64) -> Vec<u64> {
      let mut output: Vec<u64> = vec![];
      match self.core.store.get_column(table, Index::Index(column)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_u64().unwrap());
              }
          }
          _ => log!("{} not found", table),
      }
      output
  }

}

#[wasm_bindgen]
pub fn hash_string(input: String) -> u64 {
    Hasher::hash_string(input)
} 