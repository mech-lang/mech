
// ## Prelude

use alloc::{BTreeSet, BTreeMap, Vec, String};
use core::fmt;
use table::{Value, Table};
use indexes::{TableIndex};

// ## Change

#[derive(Debug, Clone)]
pub enum ChangeType {
  Add,
  Remove,
}

#[derive(Clone)]
pub struct Change {
    pub ix: usize,
    pub kind: ChangeType,
    pub table: u64,
    pub entity: u64,
    pub attribute: u64,
    pub value: Value,
    pub transaction: u64, 
}

impl Change {

  pub fn new(table: u64, entity: u64, attribute: u64, value: Value, change_type: ChangeType) -> Change {  
    Change {
      ix: 0,
      kind: change_type,
      table: table,
      entity: entity,
      attribute: attribute,
      value: value,
      transaction: 0,
    }
  }
}

impl fmt::Debug for Change {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: [{:?} {:?}: {:?}]", self.kind, self.entity, self.attribute, self.value)
    }
}
  
// ## Transaction

pub struct Transaction {
  pub timestamp: u64,
  pub complete: u64,
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
      match change.kind {
        ChangeType::Add => txn.adds.push(change),
        ChangeType::Remove => txn.removes.push(change),
      }
    }
    txn
  }

  pub fn process(&mut self) -> u64 {
    if self.complete == 0 {
      self.complete = 0;
    }
    self.complete
  }

  pub fn is_complete(&self) -> bool {
    self.complete != 0
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
  pub tables: Vec<Table>,
  pub store: Vec<Change>,
}

impl Interner {

  pub fn new(change_capacity: usize, table_capacity: usize) -> Interner {
    Interner {
      tables: Vec::with_capacity(table_capacity),
      store: Vec::with_capacity(change_capacity),
    }
  }

  pub fn intern_change(&mut self, change: &mut Change) {
    change.ix = self.store.len();
    self.store.push(change.clone());
  }

  pub fn len(&self) -> usize {
    self.store.len()
  }

}

// ## Database

pub struct Database {
    pub epoch: u64,
    pub round: u64,
    pub attribute_index: BTreeMap<u64, u64>,
    pub table_index: TableIndex,
    pub store: Interner,
    pub transactions: Vec<Transaction>, 
    pub scanned: usize,
    pub txn_pointer: usize,
}

impl Database {

  pub fn new(txn_capacity: usize, change_capacity: usize, table_capacity: usize) -> Database {
    Database {
      epoch: 0,
      round: 0,
      transactions: Vec::with_capacity(txn_capacity),
      table_index: TableIndex::new(),
      attribute_index: BTreeMap::new(),
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
            self.update_indices(add);
        }
        // Handle the removes
        for remove in txn.removes.iter_mut() {
            self.store.intern_change(remove);
            self.update_indices(remove);
        }
        txn.process();
        txn.epoch = self.epoch;
        txn.round = self.round;
        self.round = self.round + 1;
      }
    }
    self.round = 0;
  }

  fn update_indices(&mut self, change: &mut Change) {
    match change.kind {
      ChangeType::Add => {
        //self.entity_index.insert(change.clone());          
        //self.attribute_index.insert(change.attribute.id.clone(), change.attribute.clone());
      },
      ChangeType::Remove => {
        //self.entity_index.remove(&change.entity);
        //self.attribute_index.remove(&change.attribute.id);
      },
    }
  }

}

impl fmt::Debug for Database {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Database:\n--------------------\nEpoch: {:?}\nTransactions: {:?}\nChanges: {:?}\nScanned: {:?}\n--------------------\n", self.epoch, self.transactions.len(), self.store.store.len(), self.scanned)
    }
}