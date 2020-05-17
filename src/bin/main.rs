extern crate mech_core;
extern crate serde; // 1.0.68
#[macro_use]
extern crate serde_derive; // 1.0.68

use mech_core::{Index, Value, Quantity, ToQuantity, QuantityMath, make_quantity};

extern crate hashbrown;
use hashbrown::hash_map::HashMap;
use serde::*;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap};
use std::rc::Rc;
use std::cell::RefCell;
extern crate core;
use core::fmt;
use std::time::{Duration, SystemTime};
use std::io;
use std::io::prelude::*;


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
    };

    let column_ix = match column {
      Index::Index(ix) => ix,
      Index::Alias(alias) => 0, // TODO get ix from alias
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

// Holds changes to be applied to the database
struct Transaction {
  changes: Vec<Change>,
}

// Updates the database
enum Change {
  Set{table_id: u64, values: Vec<(Index, Index, Value)>},
  NewTable{table_id: u64, rows: usize, columns: usize},
}

// The database holds a map of tables, and a data store that holds a data array of values. 
// Cells in the tables contain memory addresses that point to elements of the store data array.
// The database processes transactions, which are arrays of changes that ar applies to the tables
// in the database.
struct Database {
  pub tables: HashMap<u64, Rc<RefCell<Table>>>,
  pub store: Rc<RefCell<Store>>,
}


impl Database {

  pub fn new(store: Rc<RefCell<Store>>) -> Database {
    Database {
      tables: HashMap::new(),
      store,
    }
  }

  pub fn process_transaction(&mut self, txn: Transaction) -> Result<(), Error> {
    for change in txn.changes {
      match change {
        Change::NewTable{table_id, rows, columns} => {
          self.tables.insert(table_id, Rc::new(RefCell::new(Table::new(table_id, rows, columns, self.store.clone()))));
        },
        Change::Set{table_id, values} => {
          match self.tables.get(&table_id) {
            Some(table) => {
              for (row, column, value) in values {
                table.borrow_mut().set(row, column, value);
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
    for (id,table) in self.tables.iter() {
      write!(f, "{:?}\n", table.borrow())?;   
    }
    Ok(())
  }
}

// Cores are the smallest unit of a mech program exposed to a user. They hold references to all the 
// subparts of Mech, including the database (defines the what) and the runtime (defines the how).
// The core accepts transactions and applies those to the database. Updated tables in the database
// trigger computation in the runtime, which can further update the database. Execution terminates
// when a steady state is reached, or an iteration limit is reached (whichever comes first). The 
// core then waits for further transactions.
struct Core {
  runtime: Runtime,
  database: Database,
}

impl Core {
  pub fn new(capacity: usize) -> Core {
    let mut store = Rc::new(RefCell::new(Store::new(capacity)));
    Core {
      runtime: Runtime::new(store.clone()),
      database: Database::new(store.clone()),
    }
  }

  pub fn process_transaction(&mut self, txn: Transaction) -> Result<(),Error> {

    self.database.process_transaction(txn)?;
    self.runtime.run_network()?;

    Ok(())
  }

}

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
  pub store: Rc<RefCell<Store>>,
  pub blocks: HashMap<u64, Block>,
}

impl Runtime {

  pub fn new(store: Rc<RefCell<Store>>) -> Runtime {
    Runtime {
      store,
      blocks: HashMap::new(),
    }
  }

  pub fn run_network(&mut self) -> Result<(), Error> {
    Ok(())
  }

}

// Blocks are the ubiquitous unit of code in a Mech program. Users do not write functions in Mech, as in
// other languages. Blocks consist of a number of "Transforms" that read values from tables and reshape 
// them or perform computations on them. Blocks can be thought of as pure functions where the input and 
// output are tables. Blocks have their own internal table store. Local tables can be defined within a 
// block, which allows the programmer to break a computation down into steps. The result of the computation 
// is then output to one or more global tables, which triggers the execution of other blocks in the network.
struct Block {
  pub id: usize,
  pub status: BlockStatus,
  pub tables: HashMap<u64, Table>,
  pub store: Store,
  pub transformations: Vec<Transformation>,
  pub plan: Vec<Transformation>,
}

impl Block {
  pub fn new(capacity: usize) -> Block {
    Block {
      id: 0,
      status: BlockStatus::New,
      tables: HashMap::new(),
      store: Store::new(capacity),
      transformations: Vec::new(),
      plan: Vec::new(),
    }
  }
}

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

enum Transformation {
  Scan
}



fn main() {


  let balls = 10;

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
  println!("{:?}", core.database);

 /*
  println!("{:?}\n", gravity);

  print!("Running computation...");
  io::stdout().flush().unwrap();
  let rounds = 1000.0;
  let start_ns = time::precise_time_ns();
  for j in 0..rounds as usize {
    for i in 1..balls+1 {
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
      table.set(i,4,v3);
    }
  }
  let end_ns = time::precise_time_ns();
  let time = (end_ns - start_ns) as f64 / 1000000.0;   
  let per_iteration_time = time / rounds;
  println!("Done!");
  println!("{:?}s total", time / 1000.0);  
  println!("{:?}ms per iteration", per_iteration_time);  

  println!("{:?}\n", table);

  //println!("{:?}", store);
  */

}