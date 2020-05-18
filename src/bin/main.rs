extern crate mech_core;
extern crate serde; // 1.0.68
#[macro_use]
extern crate serde_derive; // 1.0.68

use mech_core::{Index, Value, Quantity, ToQuantity, QuantityMath, make_quantity};

extern crate hashbrown;
use hashbrown::hash_map::HashMap;
use hashbrown::hash_set::HashSet;
use serde::*;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap};
use std::rc::Rc;
use std::cell::RefCell;
extern crate core;
use core::fmt;
use std::time::{Duration, SystemTime};
use std::io;
use std::io::prelude::*;
extern crate ahash;
use std::hash::Hasher;
use ahash::AHasher;

// ## Table

// A 2D table of values.
pub struct Table {
  pub id: u64,
  pub store:  Rc<RefCell<Store>>,
  pub rows: usize,
  pub columns: usize,
  pub data: Vec<usize>, // Each entry is a memory address into the store
}

impl Table {

  pub fn new(table_id: u64, rows: usize, columns: usize, store: Rc<RefCell<Store>>) -> Table {
    Table {
      id: table_id,
      store,
      rows,
      columns,
      data: vec![0; rows*columns], // Initialize with zeros, indicating Value::Empty (always the zeroth element of the store)
    }
  }

  // Transform a (row, column) into a linear address into the data. If it's out of range, return None
  pub fn index(&self, row: usize, column: usize) -> Option<usize> {
    if row <= self.rows && column <= self.columns && row > 0 && column > 0 {
      Some((row - 1) * self.columns + (column - 1))
    } else {
      None
    }
  }

  // Get the memory address into the store at a (row, column)
  pub fn get(&self, row: usize, column: usize) -> Option<usize> {
    match self.index(row, column) {
      Some(ix) => Some(self.data[ix]),
      None => None,
    }
  }

  // Set the value of at a (row, column). This will decrement the reference count of the value
  // at the old address, and insert the new value into the store while pointing the cell to the
  // new address.
  pub fn set(&mut self, row: Index, column: Index, value: Value) {
    
    let row_ix = match row {
      Index::Index(ix) => ix,
      Index::Alias(alias) => 0, // TODO get ix from alias
      Index::All => 0,
    };

    let column_ix = match column {
      Index::Index(ix) => ix,
      Index::Alias(alias) => 0, // TODO get ix from alias
      Index::All => 0,
    };

    let mut s = self.store.borrow_mut();
    let ix = self.index(row_ix, column_ix).unwrap();
    let old_address = self.data[ix];
    s.dereference(old_address);
    let new_address = s.intern(value);
    self.data[ix] = new_address;
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let rows = if self.rows > 10 {
      10
    } else {
      self.rows
    };
    write!(f, "#{:x}\n", self.id)?;
    for i in 0..rows {
      write!(f, "│ ", )?;
      for j in 0..self.columns {
        match self.get(i+1,j+1) {
          Some(x) => {
            let value = &self.store.borrow().data[x];
            write!(f, "{:?} │ ", value)?;
          },
          _ => (),
        }
        
      }
      write!(f, "\n")?;
    }
    
    Ok(())
  }
}

// ## Store

// Holds all of the values of the program in a 1D vector. We keep track of how many times a value
// is referenced using a counter. When the counter goes to zero, the memory location is marked as
// free and is available to be overwritten by a new value.
pub struct Store {
  capacity: usize,
  next: usize,
  free_end: usize,
  free_next: usize,
  free: Vec<usize>,
  data_end: usize,
  reference_counts: Vec<u16>,
  data: Vec<Value>,
}

impl Store {
  pub fn new(capacity: usize) -> Store {
    let mut rc = vec![0; capacity];
    rc[0] = 1;
    Store {
      capacity,
      next: 1,
      free_end: 0,
      free_next: 0,
      free: vec![0; capacity],
      data_end: 1,
      reference_counts: rc,
      data: vec![Value::Empty; capacity],
    }
  }

  // Decrement the reference counter for a given address. If the reference counter goes to zero,
  // mark that address as available for allocation
  pub fn dereference(&mut self, address: usize) {
    if address == 0 {
      // Do nothing, Value::Empty stays here, and is always referenced
    } else if self.reference_counts[address] == 1 {
      self.reference_counts[address] = 0;
      self.free[self.free_end] = address;
      if self.free_end + 1 == self.free.len() {
        self.free_end = 0;
      } else {
        self.free_end += 1;
      }
    } else {
      self.reference_counts[address] = self.reference_counts[address] - 1;
    }
  }

  // Intern a value into the store at the next available memory address.
  // If we are out of memory, we have to look at the list of free spaces
  // and choice one there.
  pub fn intern(&mut self, value: Value) -> usize {
    self.reference_counts[self.next] = 1;
    let address = self.next;
    self.data[address] = value;
    if self.data_end + 1 == self.capacity {
      self.next = self.free[self.free_next];
      if self.free_next + 1 == self.free.len() {
        self.free_next = 0;
      } else {
        self.free_next += 1;
      }
    } else {
      self.data_end += 1;
      self.next = self.data_end;
    }
    address
  }
}

impl fmt::Debug for Store {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "capacity: {:?}\n", self.capacity)?;
    write!(f, "next: {:?}\n", self.next)?;
    write!(f, "end: {:?}\n", self.data_end)?;
    write!(f, "free-next: {:?}\n", self.free_next)?;
    write!(f, "free-end: {:?}\n", self.free_end)?;
    //write!(f, "free: {:?}\n", self.free)?;
    //write!(f, "rc  : {:?}\n", self.reference_counts)?;
    //write!(f, "data: {:?}\n", self.data)?;
    
    Ok(())
  }
}

// ## Database

// The database holds a map of tables, and a data store that holds a data array of values. 
// Cells in the tables contain memory addresses that point to elements of the store data array.
// The database processes transactions, which are arrays of changes that ar applies to the tables
// in the database.
struct Database {
  pub tables: HashMap<u64, Rc<RefCell<Table>>>,
  pub changed_this_round: HashSet<u64>,
  pub store: Rc<RefCell<Store>>,
}

impl Database {

  pub fn new(capacity: usize) -> Database {    
    Database {
      tables: HashMap::new(),
      changed_this_round: HashSet::new(),
      store: Rc::new(RefCell::new(Store::new(capacity))),
    }
  }

  pub fn process_transaction(&mut self, txn: Transaction) -> Result<(), Error> {
    self.changed_this_round.clear();
    for change in txn.changes {
      match change {
        Change::NewTable{table_id, rows, columns} => {
          self.tables.insert(table_id, Rc::new(RefCell::new(Table::new(table_id, rows, columns, self.store.clone()))));
        },
        Change::Set{table_id, values} => {
          match self.tables.get(&table_id) {
            Some(table) => {
              for (row, column, value) in values {
                // Set the value
                table.borrow_mut().set(row, column, value);
                // Mark the table as updated
                let register_hash = Register{table_id, row: Index::All, column}.hash();
                self.changed_this_round.insert(register_hash);
              }
            },
            None => {
              // TODO Throw an error here and roll back all changes
            }
          }
        },
        _ => (),
      }
    }
    Ok(())
  }

}

impl fmt::Debug for Database {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "changed this round: \n")?;
    for changed in self.changed_this_round.iter() {
      write!(f, "       {}\n", humanize(changed))?;
    }
    for (id,table) in self.tables.iter() {
      write!(f, "{:?}\n", table.borrow())?;   
    }
    Ok(())
  }
}

// Holds changes to be applied to the database
#[derive(Clone)]
struct Transaction {
  changes: Vec<Change>,
}

#[derive(Clone)]
// Updates the database
enum Change {
  Set{table_id: u64, values: Vec<(Index, Index, Value)>},
  NewTable{table_id: u64, rows: usize, columns: usize},
}

// ## Core

// Cores are the smallest unit of a mech program exposed to a user. They hold references to all the 
// subparts of Mech, including the database (defines the what) and the runtime (defines the how).
// The core accepts transactions and applies those to the database. Updated tables in the database
// trigger computation in the runtime, which can further update the database. Execution terminates
// when a steady state is reached, or an iteration limit is reached (whichever comes first). The 
// core then waits for further transactions.
struct Core {
  runtime: Runtime,
  database: Rc<RefCell<Database>>,
}

impl Core {
  pub fn new(capacity: usize) -> Core {
    let mut database = Rc::new(RefCell::new(Database::new(capacity)));
    Core {
      runtime: Runtime::new(database.clone()),
      database,
    }
  }

  pub fn process_transaction(&mut self, txn: Transaction) -> Result<(),Error> {

    self.database.borrow_mut().process_transaction(txn)?;
    //self.runtime.run_network()?;

    Ok(())
  }

}

// ## Runtime

// Defines the function of a Mech program. The runtime consists of a series of blocks, defined
// by the user. Each block has a number of table dependencies, and produces new values that update
// existing tables. Blocks can also create new tables. The data dependencies of each block define
// a computational network of operations that runs until a steady state is reached (no more tables
// are updated after a computational round).
// For example, say we have three tables: #a, #b, and #c.
// Block1 takes #a as input and writes to #b. Block2 takes #b as input and writes to #c.
// If we update table #a with a transaction, this will trigger Block1 to execute, which will update
// #b. This in turn will trigger Block2 to execute and it will update block #c. After this, there is
// nothing left to update so the round of execution is complete.
//
// Now consider Block3 that takes #b as input and update #a and #c. Block3 will be triggered to execute
// after Block1, and it will update #a and #c. But since Block1 takes #a as input, this causes an infinite
// loop. This loop will terminate after a fixed number of iterations. Practically, this can be checked at
// compile time and the user can be warned of this and instructed to include some stop condition.
struct Runtime {
  pub database: Rc<RefCell<Database>>,
  pub blocks: HashMap<u64, Block>,
}

impl Runtime {

  pub fn new(database: Rc<RefCell<Database>>) -> Runtime {
    Runtime {
      database,
      blocks: HashMap::new(),
    }
  }

  pub fn run_network(&mut self) -> Result<(), Error> {
    Ok(())
  }

  pub fn register_block(&mut self, block: Block) {
    self.blocks.insert(block.id, block);
  }

}

// ## Block

// Blocks are the ubiquitous unit of code in a Mech program. Users do not write functions in Mech, as in
// other languages. Blocks consist of a number of "Transforms" that read values from tables and reshape 
// them or perform computations on them. Blocks can be thought of as pure functions where the input and 
// output are tables. Blocks have their own internal table store. Local tables can be defined within a 
// block, which allows the programmer to break a computation down into steps. The result of the computation 
// is then output to one or more global tables, which triggers the execution of other blocks in the network.
struct Block {
  pub id: u64,
  pub status: BlockStatus,
  pub input: HashSet<u64>,
  pub tables: HashMap<u64, Table>,
  pub store: Store,
  pub transformations: Vec<Transformation>,
  pub plan: Vec<Transformation>,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    Block {
      id: 0,
      input: HashSet::new(),
      status: BlockStatus::New,
      tables: HashMap::new(),
      store: Store::new(capacity),
      transformations: Vec::new(),
      plan: Vec::new(),
    }
  }

  pub fn register_transformation(&mut self, tfm: Transformation) {
    match tfm {
      Transformation::Whenever{table_id, row, column} => {
        self.input.insert(Register{table_id, row, column}.hash());
      }
      _ => (),
    }
    self.transformations.push(tfm);


  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "id: {:?}\n", self.id)?;
    write!(f, "status: {:?}\n", self.status)?;
    write!(f, "input: \n")?;
    for input in self.input.iter() {
      write!(f, "       {}\n", humanize(input))?;
    }
    write!(f, "transformations: {:?}\n", self.transformations)?;
    
    Ok(())
  }
}

#[derive(Debug)]
enum BlockStatus {
  New,          // Has just been created, but has not been tested for satisfaction
  Ready,        // All inputs are satisfied and the block is ready to execute
  Done,         // All inputs are satisfied and the block has executed
  Unsatisfied,  // One or more inputs are not satisfied
  Error,        // One or more errors exist on the block
  Disabled,     // The block is disabled will not execute if it otherwise would
}

enum Error {
  TableNotFound,
}

#[derive(Debug)]
enum Transformation {
  Whenever{table_id: u64, row: Index, column: Index},
  Scan,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Register {
  pub table_id: u64,
  pub row: Index,
  pub column: Index,
}

impl Register {
  pub fn hash(&self) -> u64 {
    let mut hasher = AHasher::new_with_keys(329458495230, 245372983457);
    hasher.write_u64(self.table_id);
    hasher.write_u64(*self.row.unwrap() as u64);
    hasher.write_u64(*self.column.unwrap() as u64);
    hasher.finish()
  }
}

fn main() {


  let balls = 4000;

  print!("Allocating memory...");
  let mut core = Core::new(balls * 4 * 4);
  println!("Done!");

  let mut txn = Transaction{
    changes: vec![
      Change::NewTable{table_id: 123, rows: balls, columns: 4},
    ]
  };
  let mut values = vec![];
  for i in 1..balls+1 {
    let mut v = vec![
      (Index::Index(i), Index::Index(1), Value::from_u64(i as u64)),
      (Index::Index(i), Index::Index(2), Value::from_u64(i as u64)),
      (Index::Index(i), Index::Index(3), Value::from_u64(20)),
      (Index::Index(i), Index::Index(4), Value::from_u64(0)),
    ];
    values.append(&mut v);
  }
  txn.changes.push(Change::Set{table_id: 123, values});
  core.process_transaction(txn);

  let mut txn = Transaction{
    changes: vec![
      Change::NewTable{table_id: 456, rows: 1, columns: 1},
      Change::Set{table_id: 456, values: vec![(Index::Index(1), Index::Index(1), Value::from_u64(9))]},
      Change::NewTable{table_id: 789, rows: 1, columns: 2},
      Change::Set{table_id: 789, values: vec![
        (Index::Index(1), Index::Index(1), Value::from_u64(10)),
        (Index::Index(1), Index::Index(2), Value::from_u64(0))
      ]},
    ]
  };

  core.process_transaction(txn);
  

  let mut block = Block::new(1000);
  block.register_transformation(Transformation::Whenever{table_id: 789, row: Index::All, column: Index::Index(2)});

  println!("{:?}", core.database);
  println!("{:?}", block);

  core.runtime.register_block(block);



  
  
  // Hand compile this...
  /*
  ~ #time/timer.ticks
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity"#);*
  */


 
  print!("Running computation...");
  io::stdout().flush().unwrap();
  let rounds = 1000.0;
  let start_ns = time::precise_time_ns();

  let mut values = vec![];
  for i in 1..balls+1 {
    let mut v = vec![
      (Index::Index(i), Index::Index(1), Value::from_u64(i as u64 + 5)),
      (Index::Index(i), Index::Index(2), Value::from_u64(i as u64 + 10)),
      (Index::Index(i), Index::Index(4), Value::from_u64(i as u64)),
    ];
    values.append(&mut v);
  }

  let txn = Transaction{
    changes: vec![
      Change::Set{table_id: 123, values}
    ]
  };

  for j in 0..rounds as usize {
    /*let mut values = vec![];
    for i in 1..balls+1 {
      let mut v = vec![
        (Index::Index(i), Index::Index(1), Value::from_u64(i as u64)),
        (Index::Index(i), Index::Index(2), Value::from_u64(i as u64)),
        (Index::Index(i), Index::Index(3), Value::from_u64(20)),
        (Index::Index(i), Index::Index(4), Value::from_u64(0)),
      ];
      values.append(&mut v);
      /*match core.database.tables.get(&123) {
        Some(table) => {
          // Set the value
          let mut t = table.borrow_mut();
          t.set(Index::Index(i), Index::Index(1), Value::from_u64(j as u64));
          t.set(Index::Index(i), Index::Index(2), Value::from_u64(j as u64));
          t.set(Index::Index(i), Index::Index(4), Value::from_u64(j as u64));
          // Mark the table as updated
          //self.changed_this_round.insert(Register{table_id, row: Index::All, column}.hash());
        },
        None => {
          // TODO Throw an error here and roll back all changes
        }
      }*/
      //table.borrow_mut().set(row, column, value);
      /*
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,1).unwrap()];
        let v2 = &s.data[table.get(i,3).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,1,v3);
    
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,2).unwrap()];
        let v2 = &s.data[table.get(i,4).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,2,v3);
    
      let v3;
      {
        let s = store.borrow();
        let v1 = &s.data[table.get(i,4).unwrap()];
        let v2 = &s.data[gravity.get(1,1).unwrap()];
        v3 = v1.as_quantity().unwrap().add(v2.as_quantity().unwrap()).unwrap();
      }
      let v3 = Value::from_quantity(v3);
      table.set(i,4,v3);*/
    }
    /*let mut txn = Transaction{
      changes: vec![
        Change::Set{table_id: 123, values},
      ]
    };*/*/
    core.process_transaction(txn.clone());
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  println!("Done!");
  println!("{:?}s total", time / 1000.0);  
  println!("{:?}ms per iteration", per_iteration_time);  

  //println!("{:?}", core.database.tables);
  

}


pub fn humanize(hash: &u64) -> String {
  use std::mem::transmute;
  let bytes: [u8; 8] = unsafe { transmute(hash.to_be()) };
  let mut string = "".to_string();
  let mut ix = 0;
  for byte in bytes.iter() {
    string.push_str(&DEFAULT_WORDLIST[*byte as usize]);
    if ix < 7 {
      string.push_str("-");
    }
    ix += 1;
  }
  string
}

pub const DEFAULT_WORDLIST: &[&str;256] = &[
    "ack", "ama", "ine", "ska", "pha", "gel", "art", "ril",
    "ona", "sas", "ist", "agus", "pen", "ust", "umn",
    "ado", "con", "loo", "man", "eer", "lin", "ium",
    "ack", "som", "lue", "ird", "avo", "dog", "ger",
    "ter", "nia", "bon", "nal", "ina", "pet", "cat",
    "ing", "lie", "ken", "fee", "ola", "old", "rad",
    "met", "cut", "azy", "cup", "ota", "dec", "del",
    "elt", "iet", "don", "ble", "ear", "rth", "eas", "ech",
    "war", "eig", "tee", "ele", "emm", "enemy", "equal",
    "failed", "fanta", "fifteen", "fillet", "finch", "fish", "five", "fix",
    "floor", "florida", "football", "four", "fourteen", "foxtrot", "freddie",
    "friend", "fruit", "gee", "gia", "glu", "olf", "gre", "grey",
    "hamper", "happy", "harry", "hawaii", "helium", "high", "hot", "hotel",
    "hydrogen", "idaho", "illinois", "india", "indigo", "ink", "iowa",
    "island", "item", "jersey", "jig", "joh", "juliet", "uly", "jupiter",
    "kansas", "kentucky", "kil", "kin", "kitten", "lactose", "lake", "lam",
    "lemon", "ard", "lima", "lion", "lithium", "london", "louisiana",
    "low", "magazine", "magnesium", "maine", "mango", "arc", "mar",
    "maryland", "massachusetts", "may", "mex", "michigan", "mike",
    "minnesota", "mirror", "mis", "missouri", "mobile", "mockingbird",
    "monkey", "tan", "oon", "ain", "mup", "sic", "neb",
    "une", "network", "nevada", "nine", "een", "nitrogen", "north",
    "november", "nuts", "october", "ohio", "oklahoma", "one", "ora",
    "ges", "oregon", "oscar", "oven", "oxygen", "papa", "paris", "pasta",
    "pennsylvania", "pip", "pizza", "pluto", "potato", "princess", "purple",
    "quebec", "queen", "quiet", "red", "river", "robert", "robin", "romeo",
    "rugby", "sad", "salami", "saturn", "september", "seven", "eve",
    "shade", "sierra", "single", "sink", "six", "sixteen", "skylark", "snake",
    "soc", "sodium", "solar", "south", "tti", "ker", "spr",
    "stairway", "steak", "stream", "mer", "swe", "table", "tango", "ten",
    "tennessee", "tennis", "texas", "thirteen", "three", "timing", "triple",
    "twe", "twenty", "two", "uncle", "ess", "uniform", "uranus", "uta",
    "vegan", "venus", "vermont", "vic", "video", "violet", "vir",
    "was", "est", "whiskey", "white", "iam", "win", "his",
    "wisconsin", "wolfram", "wyo", "xray", "yankee", "yellow", "zebra",
    "zulu" ];