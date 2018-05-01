// # Database

// ## Prelude

use alloc::{String, Vec};
use core::fmt;
use table::{Value, Table};
use indexes::{TableIndex, Hasher};
use hashmap_core::map::HashMap;
use runtime::{Runtime, Block};

// ## Changes

#[derive(Clone)]
pub enum Change {
  Add{ix: usize, table: u64, row: u64, column: u64, value: Value},
  Remove{ix: usize, table: u64, row: u64, column: u64, value: Value},
  NewTable{tag: String, entities: Vec<String>, attributes: Vec<String>, rows: usize, columns: usize},
}

impl fmt::Debug for Change {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        Change::Add{ix, table, row, column, value} => write!(f, "+>> #{:#x} [{:#x} {:#x}: {:?}]", table, row, column, value),
        Change::Remove{ix, table, row, column, value} => write!(f, "- #{:#x} [{:#x} {:#x}: {:?}]", table, row, column, value),
        Change::NewTable{tag, entities, attributes, rows, columns} => write!(f, "+ #{} [{:?} {:?} {:?} x {:?}]", tag, entities, attributes, rows, columns),
      }
    }
}
  
// ## Transaction

#[derive(Clone)]
pub struct Transaction {
  pub timestamp: u64,
  complete: bool,
  pub epoch: u64,
  pub round: u64,
  pub tables: Vec<Change>,
  pub adds: Vec<Change>,
  pub removes: Vec<Change>,
}

impl Transaction {
  pub fn new() -> Transaction {
    Transaction {
      timestamp: 0,
      complete: false,
      epoch: 0,
      round: 0,
      tables: Vec::new(),
      adds: Vec::new(),
      removes: Vec::new(),
    }
  }

  pub fn from_changeset(changes: Vec<Change>) -> Transaction {
    let mut txn = Transaction::new();
    for change in changes {
      match change {
        Change::Add{..} => txn.adds.push(change),
        Change::Remove{..} => txn.removes.push(change),
        Change::NewTable{..} => txn.tables.push(change),
      }
    }
    txn
  }

  pub fn from_change(change: Change) -> Transaction {
      let mut txn = Transaction::new();
      match change {
        Change::Add{..} => txn.adds.push(change),
        Change::Remove{..} => txn.removes.push(change),
        Change::NewTable{..} => txn.tables.push(change),
      }
      txn
  }

  pub fn is_complete(&self) -> bool {
    self.complete == true
  }
}

impl fmt::Debug for Transaction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      for ref table in &self.tables {
        write!(f, "{:?}\n", table).unwrap();
      }
      for ref add in &self.adds {
        write!(f, "{:?}\n", add).unwrap();
      }
      for ref remove in &self.removes {
        write!(f, "{:?}\n", remove).unwrap();
      }
      Ok(())
    }
}

// ## Interner

#[derive(Debug)]
pub struct Interner {
  pub tables: TableIndex,
  pub changes: Vec<Change>,
}

impl Interner {

  pub fn new(change_capacity: usize, table_capacity: usize) -> Interner {
    Interner {
      tables: TableIndex::new(table_capacity),
      changes: Vec::with_capacity(change_capacity),
    }
  }

  pub fn intern_change(&mut self, change: &Change) {
    match change {
      Change::Add{ix, table, row, column, value} => {
        match self.tables.get_mut(*table) {
          Some(table) => {
            table.grow_to_fit(*row as usize, *column as usize);
            table.set_cell(*row as usize, *column as usize, value.clone());
            self.changes.push(change.clone());
          }
          None => (),
        };
      },
      // TODO Implement removes
      Change::Remove{..} => {
        self.changes.push(change.clone());
      }
      Change::NewTable{tag, entities, attributes, rows, columns } => {
        let table_id = Hasher::hash_string(tag.clone());
        if !self.tables.name_map.contains_key(&table_id) {
          self.changes.push(change.clone());
          self.tables.name_map.insert(table_id, tag.to_string());
          self.tables.register(Table::new(table_id, *rows, *columns));
        }
      }  
    }
  }

  pub fn get_column(&self, table: u64, column_ix: usize) -> Option<&Vec<Value>> {
    match self.tables.get(table) {
      Some(stored_table) => {
        match stored_table.get_column(column_ix) {
          Some(column) => Some(column),
          None => None,
        }
      },
      None => None,
    }
  }

  pub fn len(&self) -> usize {
    self.changes.len()
  }

}

// ## Database

pub struct Database {
    pub epoch: u64,
    pub round: u64,
    pub processed: usize,
    pub store: Interner,
    pub transactions: Vec<Transaction>, 
    pub runtime: Runtime,
}

impl Database {

  pub fn new(transaction_capacity: usize, change_capacity: usize, table_capacity: usize) -> Database {
    Database {
      epoch: 0,
      round: 0,
      processed: 0,
      transactions: Vec::with_capacity(transaction_capacity),
      store: Interner::new(change_capacity, table_capacity),
      runtime: Runtime::new(),
    }
  }

  pub fn register_transactions(&mut self, transactions: &mut Vec<Transaction>) {
    self.transactions.append(transactions);
  }

  pub fn register_transaction(&mut self, transaction: Transaction) {
    self.register_transactions(&mut vec![transaction]);
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Vec<Change> {
    //self.epoch += 1;

            
      // First make any tables
      for table in txn.tables.iter() {
        self.store.intern_change(table);
      }
      // Handle the adds
      for add in txn.adds.iter() {
        self.store.intern_change(add);
        //self.runtime.process_change(add);
      }
      // Handle the removes
      for remove in txn.removes.iter() {
        self.store.intern_change(remove);
      }
      //txn.complete = true;
      //txn.epoch = self.epoch;
      //txn.round = self.round;
    //  self.round += 1;
    //}
    let changes = self.runtime.run_network(&self.store);
    //self.round = 0;
    //self.processed = self.transactions.len();
    changes
  }

}

impl fmt::Debug for Database {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "┌────────────────────┐\n").unwrap();
        write!(f, "│ Database           │\n").unwrap();
        write!(f, "├────────────────────┤\n").unwrap();
        write!(f, "│ Epoch: {:?}\n", self.epoch).unwrap();
        write!(f, "│ Transactions: {:?}\n", self.transactions.len()).unwrap();
        write!(f, "│ Changes: {:?}\n", self.store.changes.len()).unwrap();
        write!(f, "│ Tables: {:?}\n", self.store.tables.len()).unwrap();
        write!(f, "│ Blocks: {:?}\n", self.runtime.blocks.len()).unwrap();
        write!(f, "└────────────────────┘\n").unwrap();
        for (table, history) in self.store.tables.map.values() {
          write!(f, "{:?}", table).unwrap();
        }
        Ok(())
    }
}