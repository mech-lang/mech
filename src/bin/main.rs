extern crate time;
extern crate core;
use std::collections::{BTreeSet, BTreeMap};
use std::num::Wrapping;
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
      timestamp: time::precise_time_ns(),
      complete: 0,
      epoch: 0,
      round: 0,
      adds: Vec::new(),
      removes: Vec::new(),
    }
  }

  pub fn process(&mut self) -> u64 {
    if self.complete == 0 {
      self.complete = time::precise_time_ns ();
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

fn main() {
    
    println!("Starting:");
    
    // Init the DB
    tic();
    let mut db = Database::new();
    db.init();
    toc();

    let n = 100_000;
    let avg_txn = 50;
    let avg_change = 30;
    let mut txn_time: Vec<f64> = Vec::new();
    let mut gen_time: Vec<f64> = Vec::new();
    let start = tic();
    for i in 0..n {
        // Generate a random transaction
        tic();
        let mut txns = generate_random_transaction(avg_txn, avg_change);
        gen_time.push(toc());

        // Process transactions
        tic();
        db.register_transactions(&mut txns);
        txn_time.push(toc());
    }

    // Do it again

    let stop = tic();
    println!("Finished!");
    let run_time = (stop - start) / 1000.0;
    println!("Runtime: {:?}", run_time);
    println!("{:?}", db);
    let avg_gen_time = gen_time.into_iter().fold(0.0, |r,x| r + x);
    println!("Gen Time: {:?}", avg_gen_time / n as f64);
    let avg_txn_time = txn_time.into_iter().fold(0.0, |r,x| r + x);
    println!("Insert Time: {:?}", avg_txn_time / n as f64);
    println!("Txns/s: {:?}", db.transactions.len() as f64 / run_time / 1000.0);

    loop {}

}

pub fn generate_random_transaction(m: u32, n: u32) -> Vec<Transaction> {
    let mut seed = tic() as u32;
    let r1 = rand_between(&mut seed, 1, m);
    let r2 = rand_between(&mut seed, 1, n);      
    let mut txn_vec = Vec::with_capacity(r1 as usize);
    for i in 0..r1 {
        let txn = generate_transaction(r2);
        txn_vec.push(txn);
    }
    txn_vec
}

pub fn generate_transaction(n: u32) -> Transaction {

    let mut txn = Transaction::new();
    let changes = generate_changes(n);
    txn.adds = changes;
    txn

}

pub fn generate_changes(n: u32) -> Vec<Change> {
    let mut vec: Vec<Change> = Vec::with_capacity(n as usize);
    for i in 0..n {
        let entity = time::precise_time_ns() as u64; 
        let change = Change {
            kind: ChangeType::Add,
            entity,   
            attribute: i as u64,
            value: Value::from_int(0),
            marked: false,
        };
        vec.push(change);
    }
    vec
}




static mut TIC: u64 = 0;
static mut TOC: u64 = 0;

pub fn tic() -> f64 {
    unsafe {
        TIC = time::precise_time_ns();
        TIC as f64 / 1_000_000.0
    }
}

pub fn toc() -> f64 {
    unsafe {
        TOC = time::precise_time_ns();
        let dt = (TOC - TIC) as f64 / 1_000_000.0;
        //println!("{:?}", dt);
        dt
    }
}


fn rand(rseed:&mut u32) -> u32 {
    *rseed = ((Wrapping(*rseed) * Wrapping(1103515245) + Wrapping(12345)) & Wrapping(0x7fffffff)).0;
    return *rseed;
}

fn rand_between(rseed:&mut u32, from:u32, to:u32) -> u32 {
    rand(rseed);
    let range = (to - from) + 1;
    from + *rseed % range
}
/*

fn micro_benchmarks() {
        let n = 10_000;
    let m = 10;
    let mut insert_time = 0;
    let mut modify_time = 0;

    for j in 0..m {    
        let mut seed: u32 = tic() as u32;
        let mut _vec: Vec<Change> = Vec::with_capacity(n);
        let mut _tree: BTreeMap<u64, Change> = BTreeMap::new();
        // Bulk Insert
        tic();
        for i in 0..n {
            let entity = time::precise_time_ns() as u64; 
            let change = Change {
                kind: ChangeType::Add,
                entity,   
                attribute: i as u64,
                value: Value::from_int(0),
                marked: false,
            };
            _tree.insert(entity, change);
            //_vec.push(change); 
        }
        insert_time = insert_time + toc();

        // Linear Read/Modify        
        tic();
        for (key, change) in _tree.iter_mut() {
            change.marked = true;
        }
        modify_time = modify_time + toc();
        //println!("{:?}", _tree.len());
        //println!("{:?}", _vec.len());
    }
    println!("Insert: {}", insert_time as f64 / 1_000_000.0 / m as f64);
    println!("Modify: {}", modify_time as f64 / 1_000_000.0 / m as f64);
    println!("Done");
}*/