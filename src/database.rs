// # Database

// ## Prelude

use alloc::{Vec};
use core::fmt;
use table::{Value, Table};
use indexes::{TableIndex, Hasher};
use hashmap_core::map::HashMap;

// ## Changes

#[derive(Debug, Clone)]
pub enum Change {
  Add(AddChange),
  Remove(RemoveChange),
  NewTable(NewTableChange)
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

#[derive(Clone)]
pub struct NewTableChange {
    pub ix: usize,
    pub tag: String,
    pub entities: Vec<String>,
    pub attributes: Vec<String>,
    pub rows: usize,
    pub cols: usize,
}

impl NewTableChange {

  pub fn new(tag: String, entities: Vec<String>, attributes: Vec<String>, rows: usize, cols: usize) -> NewTableChange {  
    NewTableChange {
      ix: 0,
      tag,
      entities,
      attributes,
      rows,
      cols,
    }
  }
}

impl fmt::Debug for NewTableChange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "+ #{:?} [{:?} {:?} {:?} x {:?}]", self.tag, self.entities, self.attributes, self.rows, self.cols)
    }
}
  
  
// ## Transaction

pub struct Transaction {
  pub timestamp: u64,
  complete: u64,
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
      complete: 0,
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
        Change::Add(_) => txn.adds.push(change),
        Change::Remove(_) => txn.removes.push(change),
        Change::NewTable(_) => txn.tables.push(change),
      }
    }
    txn
  }

  pub fn from_change(change: Change) -> Transaction {
      let mut txn = Transaction::new();
      match change {
        Change::Add(_) => txn.adds.push(change),
        Change::Remove(_) => txn.removes.push(change),
        Change::NewTable(_) => txn.tables.push(change),
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
        match self.tables.get_mut(add.table) {
          Some(table) => {
            // Only add change if the new value is different from the old one
            if table.index(add.entity, add.attribute) != Some(&add.value) {
              self.changes.push(change.clone());
              table.set(add.entity, add.attribute, add.value.clone());
            }
          },
          None => (),
        };
      },
      Change::Remove(remove) => {
        self.changes.push(change.clone());
      }
      Change::NewTable(new_table) => {
        let tag = new_table.tag.clone();
        let table_id = Hasher::hash_string(tag.clone());
        if !self.tables.name_map.contains_key(&table_id) {
          self.tables.name_map.insert(table_id, tag);
          self.tables.register(Table::new(table_id, new_table.rows, new_table.cols));
        }
      }  
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
        // First make any tables
        for table in txn.tables.iter_mut() {
          self.store.intern_change(table);
        }
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
        write!(f, "┌────────────────────┐\n").unwrap();
        write!(f, "│ Database:          │\n").unwrap();
        write!(f, "├────────────────────┤\n").unwrap();
        write!(f, "│ Epoch: {:?}\n", self.epoch).unwrap();
        write!(f, "│ Transactions: {:?}\n", self.transactions.len()).unwrap();
        write!(f, "│ Changes: {:?}\n", self.store.changes.len()).unwrap();
        write!(f, "│ Tables: {:?}\n", self.store.tables.len()).unwrap();
        write!(f, "│ Scanned: {:?}\n", self.scanned).unwrap();
        write!(f, "└────────────────────┘\n").unwrap();
        /*for change in self.store.changes.iter() {
          println!("{:?}", change);
        }*/
        for (table, history) in self.store.tables.map.values() {
          write!(f, "{:?}", table).unwrap();
        }
        Ok(())
    }
}