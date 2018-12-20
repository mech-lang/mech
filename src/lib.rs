// # Mech

/*
Mech is a programming language especially suited for developing reactive 
systems. 
*/

// ## Prelude

#![cfg_attr(target_os = "none", no_std)]
#![feature(alloc)]
#![feature(nll)]

extern crate rlibc;
#[macro_use]
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate core;
extern crate hashmap_core;
extern crate rand;
#[macro_use]
extern crate serde_derive;

use alloc::vec::Vec;
use core::fmt;

// ## Modules

mod database;
mod runtime;
mod table;
mod indexes;
mod operations;

// ## Exported Modules

pub use self::database::{Transaction, Change, Interner};
pub use self::table::{Value, Index, TableId, Table};
pub use self::indexes::{TableIndex, Hasher};
pub use self::operations::{Function, Comparator, Logic, Parameter};
pub use self::runtime::{Runtime, Block, Constraint, Register};

// ## Core

pub struct Core {
  pub id: u64,
  pub epoch: usize,
  pub offset: usize, // this is an offset from now. 0 means now, 1 means 1 txn ago, etc.
  pub round: usize,
  pub changes: usize,
  pub store: Interner,
  pub runtime: Runtime,
  pub change_capacity: usize,
  pub table_capacity: usize,
  transaction_boundaries: Vec<usize>,
}

impl Core {

  pub fn new(change_capacity: usize, table_capacity: usize) -> Core {
    Core {
      id: 0,
      offset: 0,
      epoch: 0,
      round: 0,
      changes: 0,
      change_capacity,
      table_capacity,
      store: Interner::new(change_capacity, table_capacity),
      runtime: Runtime::new(),
      transaction_boundaries: Vec::new(),
    }
  }

  pub fn clear(&mut self) {
    self.epoch = 0;
    self.round = 0;
    self.runtime.clear();
    self.store.clear();
  }

  pub fn register_blocks(&mut self, blocks: Vec<Block>) {
    self.runtime.register_blocks(blocks, &mut self.store);
  }

  pub fn last_transaction(&self) -> usize {
    if self.transaction_boundaries.len() <= 1 {
      0
    } else {
      self.transaction_boundaries[self.transaction_boundaries.len() - 2]
    }
    
  }

  pub fn this_transaction(&self) -> usize {
    if self.transaction_boundaries.len() == 0 {
      0
    } else {
      self.transaction_boundaries[self.transaction_boundaries.len() - 1]
    }
    
  }

  pub fn step(&mut self) {
    self.runtime.run_network(&mut self.store);
    self.transaction_boundaries.push(self.store.change_pointer);
  }

  pub fn index(&mut self, table: u64, row: &Index, column: &Index) -> Option<&Value> {
    match self.store.tables.get(table) {
      Some(table_ref) => {
        match table_ref.index(row, column) {
          Some(cell_data) => Some(cell_data),
          None => None,
        }
      },
      None => None,
    }
  }

  pub fn step_backward(&mut self, steps: usize) {
    for _ in 0..steps {
      self.step_back_one();
    }
  }

  pub fn step_back_one(&mut self) {
    let time = self.store.offset;
    self.store.offset += 1;
    let transactions = self.transaction_boundaries.len();
    // We can only step back if there is at least one transaction, 
    // and we aren't at the beginning of time
    if transactions > 0  {
      let now_ix = if transactions <= time {
        0
      } else {
        self.transaction_boundaries[transactions - time - 1]
      };
      let prev_ix = if transactions <= time + 1 {
        0 
      } else {
        self.transaction_boundaries[transactions - time - 2]
      };

      // Now process the transactions in reverse order
      for ix in (prev_ix..now_ix).rev() {
        match &self.store.changes[ix] {
          Change::Set{table, row, column, value} => {
            self.store.process_transaction(&Transaction::from_change(
              Change::Remove{table: *table, row: row.clone(), column: column.clone(), value: value.clone()}
            ));
          },
          Change::Remove{table, row, column, value} => {
            self.store.process_transaction(&Transaction::from_change(
              Change::Set{table: *table, row: row.clone(), column: column.clone(), value: value.clone()}
            ));
          },
          Change::NewTable{id, rows, columns} => {
            self.store.process_transaction(&Transaction::from_change(
              Change::RemoveTable{id: *id, rows: *rows, columns: *columns}
            ));
          },
          _ => (),
        };
      }

      /*for (ix, change) in self.store.changes.iter().enumerate() {
        let foo = if ix < now_ix && ix >= prev_ix {
          "->"
        } else {
          "  "
        };
        println!("{} {:?}",foo, change);
      }*/

    }
    self.offset = self.store.offset;
  }

  pub fn step_forward(&mut self, steps: usize) {
    for i in 0..steps {
      self.step_forward_one();
    }
  }

  pub fn step_forward_one(&mut self) {
    let time = self.store.offset;
    let transactions = self.transaction_boundaries.len();
    // We can only step forward if there is at least one transaction and we are
    // rewound from "now"
    if time > 0 && transactions > 0 {
      let now_ix = if transactions <= time {
        0
      } else {
        self.transaction_boundaries[transactions - time - 1]
      };
      let next_ix = if transactions <= time - 1 {
        0 
      } else {
        self.transaction_boundaries[transactions - time]
      };
      for ix in now_ix..next_ix {
        self.store.process_transaction(&Transaction::from_change(self.store.changes[ix].clone()));
      }
      /*for (ix, change) in self.store.changes.iter().enumerate() {
        let foo = if ix >= now_ix && ix < next_ix {
          "->"
        } else {
          "  "
        };
        println!("{} {:?}",foo, change);
      }*/
      self.store.offset -= 1;
    }
    self.offset = self.store.offset;
  }

  pub fn resume(&mut self) {
    for _ in 0..self.offset {
      self.step_forward_one();
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) {
    self.store.process_transaction(txn);
    self.runtime.run_network(&mut self.store);

    self.transaction_boundaries.push(self.store.change_pointer);
    self.epoch = self.store.rollover;
  }

  pub fn capacity(&self) -> f64 {
    100.0 * (self.store.changes.len() as f64 / self.store.changes.capacity() as f64)
  }
}

impl fmt::Debug for Core {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "┌────────────────────┐\n").unwrap();
    write!(f, "│ Mech Core #{:0x}\n", self.id).unwrap();
    write!(f, "├────────────────────┤\n").unwrap();
    write!(f, "│ Time Offset: {:?}\n", self.offset).unwrap();
    write!(f, "│ Epoch: {:?}\n", self.epoch).unwrap();
    write!(f, "│ Changes: {:?}\n", self.store.changes_count).unwrap();
    write!(f, "│ Capacity: {:0.2}%\n", 100.0 * (self.store.changes.len() as f64 / self.store.changes.capacity() as f64)).unwrap();
    write!(f, "│ Tables: {:?}\n", self.store.tables.len()).unwrap();
    write!(f, "│ Blocks: {:?}\n", self.runtime.blocks.len()).unwrap();
    write!(f, "└────────────────────┘\n").unwrap();
    for table in self.store.tables.map.values() {
      write!(f, "{:?}", table).unwrap();
    }
    Ok(())
  }
}