
// ## Prelude

use alloc::{Vec};
use core::fmt;
use table::{Value, Table};
use indexes::{TableIndex};
use hashmap_core::map::HashMap;

// ## Change

#[derive(Debug, Clone)]
pub enum Change {
  Add(AddChange),
  Remove(RemoveChange),
}

#[derive(Clone)]
pub struct AddChange {
    pub ix: usize,
    pub table: u64,
    pub entity: u64,
    pub attribute: u64,
    pub value: Value,
}

impl AddChange {

  pub fn new(table: u64, entity: u64, attribute: u64, value: Value) -> AddChange {  
    AddChange {
      ix: 0,
      table: table,
      entity: entity,
      attribute: attribute,
      value: value,
    }
  }
}

impl fmt::Debug for AddChange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "+ #{:?} [{:?} {:?}: {:?}]", self.table, self.entity, self.attribute, self.value)
    }
}

#[derive(Clone)]
pub struct RemoveChange {
    pub ix: usize,
    pub table: u64,
    pub entity: u64,
    pub attribute: u64,
    pub value: Value,
}

impl RemoveChange {

  pub fn new(table: u64, entity: u64, attribute: u64, value: Value) -> RemoveChange {  
    RemoveChange {
      ix: 0,
      table: table,
      entity: entity,
      attribute: attribute,
      value: value,
    }
  }
}

impl fmt::Debug for RemoveChange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "- #{:?} [{:?} {:?}: {:?}]", self.table, self.entity, self.attribute, self.value)
    }
}
  
// ## Transaction

pub struct Transaction {
  pub timestamp: u64,
  complete: u64,
  pub epoch: u64,
  pub round: u64,
  pub adds: Vec<Change>,
  pub removes: Vec<Change>,
}

impl Transaction {
  pub fn new() -> Transaction {
    Transaction {
      timestamp: 0,
      complete: 0,
      epoch: 0,
      round: 0,
      adds: Vec::new(),
      removes: Vec::new(),
    }
  }

  pub fn from_changeset(changes: Vec<Change>) -> Transaction {
    let mut txn = Transaction::new();
    for change in changes {
      match change {
        Change::Add(_) => txn.adds.push(change),
        _ => (),
      }
    }
    txn
  }

  pub fn process(&mut self) -> u64 {
    if self.complete == 0 {
      self.complete = 1;
    }
    self.complete
  }

  pub fn is_complete(&self) -> bool {
    self.complete == 1
  }
}

impl fmt::Debug for Transaction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f,"Adds:\n").unwrap();
      for ref add in &self.adds {
        write!(f, "{:?}\n", add).unwrap();
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
      Change::Add(add) => {
        self.changes.push(change.clone());
      },
      Change::Remove(remove) => {
        self.changes.push(change.clone());
      }
      
    }


    
    //interned_change.ix = self.store.len();
    //self.store.push(interned_change);
    //self.tables.register(change.table);
  }

  pub fn len(&self) -> usize {
    self.changes.len()
  }

}

// ## Database

pub struct Database {
    pub epoch: u64,
    pub round: u64,
    pub store: Interner,
    pub transactions: Vec<Transaction>, 
    pub scanned: usize,
    pub txn_pointer: usize,
}

impl Database {

  pub fn new(transaction_capacity: usize, change_capacity: usize, table_capacity: usize) -> Database {
    Database {
      epoch: 0,
      round: 0,
      transactions: Vec::with_capacity(transaction_capacity),
      store: Interner::new(change_capacity, table_capacity),
      scanned: 0,
      txn_pointer: 0,
    }
  }

  pub fn init(&self) {
    
  }

  pub fn register_transactions(&mut self, transactions: &mut Vec<Transaction>) {
    self.process_transactions(transactions);
    self.transactions.append(transactions);
    self.epoch = self.epoch + 1;
  }

  pub fn register_transaction(&mut self, transaction: Transaction) {
    self.register_transactions(&mut vec![transaction]);
  }

  fn process_transactions(&mut self, transactions: &mut Vec<Transaction>) {   
    for txn in transactions {
      if !txn.is_complete() {
        // Handle the adds
        for add in txn.adds.iter_mut() {
            self.store.intern_change(add);
        }
        // Handle the removes
        for remove in txn.removes.iter_mut() {
            self.store.intern_change(remove);
        }
        txn.process();
        txn.epoch = self.epoch;
        txn.round = self.round;
        self.round = self.round + 1;
      }
    }
    self.round = 0;
  }

}

impl fmt::Debug for Database {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Database:\n--------------------\nEpoch: {:?}\nTransactions: {:?}\nChanges: {:?}\nTables: {:?}\nScanned: {:?}\n--------------------\n", self.epoch, self.transactions.len(), self.store.changes.len(), self.store.tables.len(), self.scanned)
    }
}