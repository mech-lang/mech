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
extern crate mech_core;
extern crate mech_syntax;
extern crate mech_utilities;
extern crate serde_json;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use hashbrown::hash_set::HashSet;
use alloc::vec::Vec;
use core::fmt;
use mech_syntax::formatter::Formatter;
use mech_syntax::compiler::{Compiler, Node, Program, Section, Element};
use mech_core::{ErrorType, Transaction, BlockState, Hasher, Change, Index, Value, Table, Quantity, ToQuantity, QuantityMath};
use mech_utilities::WebsocketClientMessage;

#[wasm_bindgen]
pub struct Core {
  core: mech_core::Core,
  programs: Vec<Program>,
  changes: Vec<Change>,
}

#[wasm_bindgen]
impl Core {
  pub fn new(changes: usize, tables: usize) -> Core {
    Core {
      core: mech_core::Core::new(changes,tables),
      programs: Vec::new(),
      changes: Vec::new(),
    }
  }

  pub fn compile_code(&mut self, code: String) {
    let mech_code = Hasher::hash_str("mech/code");
    let changes = vec![
      Change::NewTable{id: mech_code, rows: 1, columns: 1},
      Change::Set{table: mech_code, row: mech_core::Index::Index(1), column: mech_core::Index::Index(1), value: Value::from_str(&code)},
    ];
    let mut compiler = Compiler::new();
    compiler.compile_string(code);
    self.core.register_blocks(compiler.blocks.clone());
    self.core.step();
    self.core.process_transaction(&Transaction::from_changeset(changes));
    self.programs = compiler.programs.clone();
  }

  pub fn clear(&mut self) {
    self.core.clear();
    self.programs.clear();
    self.changes.clear();
  }

  pub fn pause(&mut self) {
    self.core.pause();
  }

  pub fn resume(&mut self) {
    self.core.resume();
  }

  pub fn step_back_one(&mut self) {
    self.core.step_back_one();
  }

  pub fn step_forward_one(&mut self) {
    self.core.step_forward_one();
  }

  pub fn set_time(&mut self, time: usize) {
    self.core.set_time(time);
  }

  pub fn display_core(&self) {
  }

  pub fn display_runtime(&self) {
  }

  pub fn display_changes(&self) {

  }

  pub fn queue_change(&mut self, table: String, row: u32, column: u32, value: i32) {
    let table_id = Hasher::hash_string(table);
    let change = Change::Set{table: table_id, 
                             row: Index::Index(row as u64), 
                             column: Index::Index(column as u64),
                             value: Value::from_i64(value as i64),
                            };
    self.changes.push(change);
  }

  pub fn process_transaction(&mut self) {
    if !self.core.paused {
      let txn = Transaction::from_changeset(self.changes.clone());
      let pre_changes = self.core.store.len();
      self.core.process_transaction(&txn);
    }
    self.changes.clear();
  }

  pub fn get_mantissas(&mut self, table: String, column: u32) -> Vec<i32> {
      let table_id = Hasher::hash_string(table);
      let mut output: Vec<i32> = vec![];
      match self.core.store.get_column(table_id, Index::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().mantissa() as i32);
              }
          }
          _ => (),
      }
      output
  }

  pub fn get_ranges(&mut self, table: String, column: u32) -> Vec<i32> {
      let table_id = Hasher::hash_string(table);    
      let mut output: Vec<i32> = vec![];
      match self.core.store.get_column(table_id, Index::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().range() as i32);
              }
          }
          _ => (),
      }
      output
  }

  pub fn get_column(&mut self, table: String, column: u32) -> Vec<f32> {
      let table_id = Hasher::hash_string(table);    
      let mut output: Vec<f32> = vec![];
      match self.core.store.get_column(table_id, Index::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().to_float() as f32);
              }
          }
          _ => (),
      }
      output
  }

}
