use alloc::{BTreeSet, BTreeMap, Vec, String};
use core::fmt;

#[derive(Clone)]
pub enum Value {
  Null,
  Number(u64),
  Bool(bool),
  String(String),
}

impl Value {

  pub fn from_string(string: String) -> Value {
    Value::String(string)
  }

  pub fn from_str(string: &str) -> Value {
    Value::String(String::from(string))
  }

  pub fn from_int(int: u64) -> Value {
    Value::Number(int)
  }

}

impl fmt::Debug for Value {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        &Value::Number(ref x) => write!(f, "{}", x),
        &Value::String(ref x) => write!(f, "{}", x),
        &Value::Bool(ref x) => write!(f, "{}", x),
        &Value::Null => write!(f, "Null"),
      }
    }
}

#[derive(Debug, Clone)]
pub enum ChangeType {
  Add,
  Remove,
}

#[derive(Clone)]
pub struct Change {
    pub kind: ChangeType,
    pub entity: u64,
    pub attribute: u64,
    pub value: Value,
    pub marked: bool,
}

impl Change {
  pub fn from_eav(entity: &str, attribute: &str, value: Value) -> Change {  
    let e = 0;
    let a = 0;
    Change {
      kind: ChangeType::Add,
      entity: e,
      attribute: a,
      value,
      marked: false,
    }
  }
}

impl fmt::Debug for Change {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: [{:?} {:?}: {:?}]", self.kind, self.entity, self.attribute, self.value)
    }
}

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

#[derive(Debug)]
pub struct Interner {
  pub store: Vec<Change>,
}

impl Interner {

  pub fn new() -> Interner {
    Interner {
      store: Vec::new(),
    }
  }

  pub fn intern_change(&mut self, change: &Change) {
    self.store.push(change.clone());
  }
}

pub struct Database {
    pub epoch: u64,
    pub round: u64,
    pub index: BTreeMap<u64, Change>,
    pub store: Interner,
    pub transactions: Vec<Transaction>, 
    pub scanned: usize,
    txn_pointer: usize,
}

impl Database {

  pub fn new() -> Database {
    Database {
      epoch: 0,
      round: 0,
      transactions: Vec::with_capacity(1_000_000),
      index: BTreeMap::new(),
      store: Interner::new(),
      scanned: 0,
      txn_pointer: 0,
    }
  }

  pub fn init(&self) {
    
  }

  pub fn register_transactions(&mut self, transactions: &mut Vec<Transaction>) {
    self.transactions.append(transactions);
    self.process_transactions();
    self.txn_pointer = self.transactions.len();
    self.update_indices();
    self.epoch = self.epoch + 1;
  }

  fn process_transactions(&mut self) {   
    for txn in self.transactions.iter_mut().skip(self.txn_pointer) {
      if !txn.is_complete() {
        // Handle the adds
        for add in txn.adds.iter() {
            self.store.intern_change(add);
        }
        // Handle the removes
        for remove in txn.removes.iter() {
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

  fn update_indices(&mut self) {
    for change in self.store.store.iter().skip(self.scanned) {
      match change.kind {
        ChangeType::Add => {
          self.index.insert(change.entity.clone(), change.clone());          
          //self.attribute_index.insert(change.attribute.clone(), Attribute::new());
        },
        ChangeType::Remove => {
          self.index.remove(&change.entity);
        },
      }
    }
    self.scanned = self.store.store.len();
  }

}

impl fmt::Debug for Database {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Database:\n--------------------\nEpoch: {:?}\nTransactions: {:?}\nChanges: {:?}\nScanned: {:?}\n--------------------\n", self.epoch, self.transactions.len(), self.store.store.len(), self.scanned)
    }
}