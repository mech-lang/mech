#![feature(get_mut_unchecked)]

extern crate ahash;
extern crate core as rust_core;
extern crate hashbrown;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate num_traits;

use std::hash::Hasher;
use ahash::AHasher;

mod database;
mod runtime;
mod table;
mod indexes;
mod operations;
mod quantities;
mod errors;
mod core;
mod block;

pub use self::database::{Database, Store, Transaction, Change};
pub use self::block::{Block, BlockState, Transformation, IndexRepeater, IndexIterator, Register, ValueIterator};
pub use self::table::{Table, TableId, Index, Value, ValueMethods, ValueType};
pub use self::core::Core;
pub use self::quantities::{Quantity, QuantityMath, ToQuantity, make_quantity};
pub use self::errors::{Error, ErrorType};

pub fn hash_string(input: &str) -> u64 {
  let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);
  hasher.write(input.to_string().as_bytes());
  let mut hash = hasher.finish();
  hash & 0x00FFFFFFFFFFFFFF
}

pub fn humanize(hash: &u64) -> String {
  use std::mem::transmute;
  let bytes: [u8; 8] = unsafe { transmute(hash.to_be()) };
  let mut string = "".to_string();
  let mut ix = 0;
  for byte in bytes.iter() {
    if ix % 2 == 0 {
      ix += 1;
      continue;
    }
    string.push_str(&WORDLIST[*byte as usize]);
    if ix < 7 {
      string.push_str("-");
    }
    ix += 1;
  }
  string
}

pub const WORDLIST: &[&str;256] = &[
  "nil", "ama", "ine", "ska", "pha", "gel", "art", 
  "ona", "sas", "ist", "aus", "pen", "ust", "umn",
  "ado", "con", "loo", "man", "eer", "lin", "ium",
  "ack", "som", "lue", "ird", "avo", "dog", "ger",
  "ter", "nia", "bon", "nal", "ina", "pet", "cat",
  "ing", "lie", "ken", "fee", "ola", "old", "rad",
  "met", "cut", "azy", "cup", "ota", "dec", "del",
  "elt", "iet", "don", "ble", "ear", "rth", "eas", 
  "war", "eig", "tee", "ele", "emm", "ene", "qua",
  "fai", "fan", "fif", "fil", "fin", "fis", "fiv", 
  "flo", "for", "foo", "fou", "fot", "fox", "fre",
  "fri", "fru", "gee", "gia", "glu", "fol", "gre", 
  "ham", "hap", "har", "haw", "hel", "hig", "hot", 
  "hyd", "ida", "ill", "ind", "ini", "ink", "iwa",
  "and", "ite", "jer", "jig", "joh", "jul", "uly", 
  "kan", "ket", "kil", "kin", "kit", "lac", "lak", 
  "lem", "ard", "lim", "lio", "lit", "lon", "lou",
  "low", "mag", "nes", "mai", "gam", "arc", "mar",
  "mao", "mas", "may", "mex", "mic", "mik", "ril",
  "min", "mir", "mis", "mio", "mob", "moc", "ech",
  "moe", "tan", "oon", "ain", "mup", "sic", "neb",
  "une", "net", "nev", "nin", "een", "nit", "nor",
  "nov", "nut", "oct", "ohi", "okl", "one", "ora",
  "ges", "ore", "osc", "ove", "oxy", "pap", "par", 
  "pey", "pip", "piz", "plu", "pot", "pri", "pur",
  "que", "uqi", "qui", "red", "riv", "rob", "roi", 
  "rug", "sad", "sal", "sat", "sep", "sev", "eve",
  "sha", "sie", "sin", "sik", "six", "sit", "sky", 
  "soc", "sod", "sol", "sot", "tir", "ker", "spr",
  "sta", "ste", "mam", "mer", "swe", "tab", "tag", 
  "see", "nis", "tex", "thi", "the", "tim", "tri",
  "twe", "ent", "two", "unc", "ess", "uni", "ura", 
  "veg", "ven", "ver", "vic", "vid", "vio", "vir",
  "was", "est", "whi", "hit", "iam", "win", "his",
  "wis", "olf", "wyo", "ray", "ank", "yel", "zeb",
  "ulu", "fix", "gry", "hol", "jup", "lam", "pas",
  "rom", "sne", "ten", "uta"];

/*
// # Mech

/*
Mech is a programming language especially suited for developing reactive 
systems. 
*/

// ## Prelude

#![cfg_attr(feature = "no-std", no_std)]
#![feature(nll)]
#![feature(get_mut_unchecked)]

#[cfg(feature = "no-std")] extern crate rlibc;
#[cfg(feature="no-std")] #[macro_use] extern crate alloc;
#[cfg(not(feature = "no-std"))] extern crate core;
#[cfg(not(feature = "no-std"))] use std::rc::Rc;
#[cfg(not(feature = "no-std"))] use std::sync::Arc;

extern crate hashbrown;
#[macro_use]
extern crate serde_derive;
extern crate serde;

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::rc::RC;
#[cfg(feature = "no-std")] use alloc::sync::ArC;
#[cfg(not(feature = "no-std"))] use core::fmt;
use hashbrown::hash_set::HashSet;
use hashbrown::hash_map::{HashMap, Entry};

use std::cell::RefCell;

// ## Modules

mod database;
mod runtime;
mod table;
mod indexes;
mod operations;
mod quantities;
mod errors;

// ## Exported Modules

pub use self::database::{Transaction, Change, Interner};
pub use self::table::{Value, Index, TableId, Table};
pub use self::indexes::{TableIndex, Hasher};
pub use self::operations::{Parameter};
pub use self::runtime::{Runtime, Block, BlockState, Constraint, Register};
pub use self::quantities::{Quantity, ToQuantity, QuantityMath, make_quantity};
pub use self::errors::{Error, ErrorType};

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
  pub defined_tables: HashSet<Register>,
  pub input: HashSet<Register>,
  pub output: HashSet<Register>,
  pub paused: bool,
  pub remote_tables: Vec<Arc<Table>>,
  pub transaction_boundaries: Vec<usize>,
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
      defined_tables: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      paused: false,
      remote_tables: Vec::new(),
      transaction_boundaries: Vec::new(),
    }
  }

  pub fn clear(&mut self) {
    self.epoch = 0;
    self.round = 0;
    self.runtime.clear();
    self.store.clear();
    self.input.clear();
    self.output.clear();
    self.transaction_boundaries.clear();
  }

  pub fn get_table(&mut self, table_name: String) -> Option<&Rc<RefCell<Table>>> {
    let table_id = Hasher::hash_string(table_name);
    self.store.get_table(table_id)
  }

  pub fn get_table_by_id(&mut self, table_id: &TableId) -> Option<&Rc<RefCell<Table>>> {
    match table_id {
      TableId::Local(id) => None,
      TableId::Global(id) => self.store.get_table(*id)
    }
  }

  pub fn register_blocks(&mut self, blocks: Vec<Block>) {
    self.runtime.register_blocks(blocks, &mut self.store);
    for (id, block) in self.runtime.blocks.iter() {
      // Collect input
      for register in block.input_registers.iter() {
        self.input.insert(register.clone());
      }
      // Collect output
      for register in block.output_registers.iter() {
        self.output.insert(register.clone());
      }
      for (constraint_text, constraints) in &block.constraints {
        for constraint in constraints {
          match constraint {
            Constraint::Identifier{id, text} => {
              self.store.names.insert(id.clone() as u64, text.clone());
            },
            Constraint::DefineTable{to_table, ..} => {
              self.defined_tables.insert(Register{table: TableId::Global(*to_table), column: Index::Index(0)});
            }
            _ =>(),
          };
        }
      }
    }
  }

  pub fn register_table(&mut self, table: Table) {
    let table_id = table.id;
    self.store.tables.insert(table);
    let register = Register::new(TableId::Global(table_id),Index::Index(0));
    match self.runtime.pipes_map.entry(register.clone()) {
      Entry::Occupied(addresses) => {
        for address in addresses.get().iter() {
          let block_id = address.block;
          let block = self.runtime.blocks.get_mut(&block_id).unwrap();
          block.ready.insert(register.clone());
          if (block.is_ready()) {
            self.runtime.ready_blocks.insert(block.id);
          }
        }
      },
      _ => (),    
    }
  }

  pub fn remove_block(&mut self, block_id: &usize) {
    self.runtime.remove_block(&block_id);
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

  pub fn step(&mut self, max_iterations: u64) {
    self.runtime.run_network(&mut self.store, max_iterations);
    self.transaction_boundaries.push(self.store.change_pointer);
  }

  pub fn index(&mut self, table: u64, row: &Index, column: &Index) -> Option<&Value> {
    match self.store.tables.get(table) {
      Some(table_ref) => {
        match unsafe{(*table_ref.as_ptr()).index(row, column)} {
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
        let core = self as *mut Core;
        match &*self.store.changes[ix] {
          Change::Set{table, column, values} => {
            unsafe {
              (*core).store.process_transaction(&Transaction::from_change(
                Rc::new(Change::Remove{table: table.clone(), column: column.clone(), values: values.clone()})
              ));
            }
          },
          Change::Remove{table, column, values} => {
            unsafe {
              (*core).store.process_transaction(&Transaction::from_change(
                Rc::new(Change::Set{table: table.clone(), column: column.clone(), values: values.clone()})
              ));
            }
          },
          Change::NewTable{id, rows, columns} => {
            /*self.store.process_transaction(&Transaction::from_change(
              Change::RemoveTable{id: *id, rows: *rows, columns: *columns}
            ));*/
          },
          _ => (),
        };
      }

    }
    self.offset = self.store.offset;
  }

  pub fn step_forward(&mut self, steps: usize) {
    for i in 0..steps {
      self.step_forward_one();
    }
  }

  pub fn set_time(&mut self, time: usize) {
    let current_time = self.offset;
    if current_time > time {
      let dt = current_time - time;
      self.step_forward(dt); 
    } else if current_time < time {
      let dt = time - current_time;
      self.step_backward(dt); 
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
      self.store.offset -= 1;
    }
    self.offset = self.store.offset;
  }

  pub fn resume(&mut self) {
    for _ in 0..self.offset {
      self.step_forward_one();
    }
    self.paused = false;
  }

  pub fn pause(&mut self) {
    self.paused = true;
  }

  pub fn process_transaction(&mut self, txn: &Transaction) {
    if !self.paused {
      self.store.process_transaction(txn);
      self.runtime.run_network(&mut self.store, 10_000);

      self.transaction_boundaries.push(self.store.change_pointer);
      self.epoch = self.store.rollover;
    }
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
    write!(f, "│   Defined Tables: {:?}\n", self.defined_tables).unwrap();
    write!(f, "│   Input: {:?}\n", self.input).unwrap();
    write!(f, "│   Output: {:?}\n", self.output).unwrap();
    write!(f, "│   Errors:\n").unwrap();
    write!(f, "│     {:?}\n", self.runtime.errors).unwrap();
    write!(f, "└────────────────────┘\n").unwrap();
    for table in self.store.tables.map.values() {
      write!(f, "{:?}", table.borrow()).unwrap();
    }
    Ok(())
  }
}*/