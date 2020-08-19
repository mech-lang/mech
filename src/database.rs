use table::{Table, TableId, Value, Index, ValueMethods};
use block::{Error, Register};
use ::humanize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use hashbrown::{HashSet, HashMap};
use rust_core::fmt;

// ## Store

// Holds all of the values of the program in a 1D vector. We keep track of how many times a value
// is referenced using a counter. When the counter goes to zero, the memory location is marked as
// free and is available to be overwritten by a new value.
pub struct Store {
  pub capacity: usize,
  pub next: usize,
  pub free_end: usize,
  pub free_next: usize,
  pub free: Vec<usize>,
  pub data_end: usize,
  pub reference_counts: Vec<u16>,
  pub data: Vec<Value>,
  pub column_alias_to_index: HashMap<(u64,u64),usize>,
  pub column_index_to_alias: HashMap<(u64,usize),u64>,
  pub strings: HashMap<u64, String>,
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
      data: vec![Value::from_u64(0); capacity],
      column_alias_to_index: HashMap::new(),
      column_index_to_alias: HashMap::new(),
      strings: HashMap::new(),
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
  // and choose one there.
  pub fn intern(&mut self, value: Value) -> usize {
    self.reference_counts[self.next] = 1;
    let address = self.next;
    self.data[address] = value;
    // The next address is taken from the free pile because our main memory is full
    if self.data_end + 1 == self.capacity && self.free_next != self.free_end {
      self.next = self.free[self.free_next];
      if self.free_next + 1 == self.free.len() {
        self.free_next = 0;
      } else {
        self.free_next += 1;
      }
    // Extend the data if it's full and there is no free space
    } else if self.data_end + 1 == self.capacity && self.free_next == self.free_end {
      self.capacity = self.capacity * 2;
      self.data.resize(self.capacity, Value::from_u64(0));
      self.reference_counts.resize(self.capacity, 0);
      self.free.resize(self.capacity, 0);
      self.data_end += 1;
      self.next = self.data_end;
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
    
    write!(f, "       ")?;
    let data_len = if self.data.len() > 40 {
      40
    } else {
      self.data.len()
    };
    for i in 0..data_len {
      write!(f, "{:3?}", i)?;
    }
    write!(f, "\n")?;
    write!(f, "data: [")?;
    let data_len = if self.data.len() > 40 {
      40
    } else {
      self.data.len()
    };
    for i in 0..data_len {
      write!(f, "{:3?}", self.data[i])?;
    }
    write!(f, "]({:?})\n", self.data.len())?; 
    write!(f, "free: [")?;
    let data_len = if self.free.len() > 40 {
      40
    } else {
      self.free.len()
    };
    for i in 0..data_len {
      write!(f, "{:3?}", self.free[i])?;
    }
    write!(f, "]({:?})\n", self.free.len())?;
    write!(f, "rc  : [")?;
    let data_len = if self.reference_counts.len() > 40 {
      40
    } else {
      self.reference_counts.len()
    };
    for i in 0..data_len {
      write!(f, "{:3?}", self.reference_counts[i])?;
    }
    write!(f, "]({:?})\n", self.reference_counts.len())?;

    
    Ok(())
  }
}

// ## Database

// The database holds a map of tables, and a data store that holds a data array of values. 
// Cells in the tables contain memory addresses that point to elements of the store data array.
// The database processes transactions, which are arrays of changes that ar applies to the tables
// in the database.
pub struct Database {
  pub tables: HashMap<u64, Table>,
  pub changed_this_round: HashSet<u64>,
  pub store: Arc<Store>,
  pub transactions: Vec<Transaction>,
  pub register_map: HashMap<u64, Register>,
}

impl Database {

  pub fn new(capacity: usize) -> Database {    
    Database {
      tables: HashMap::new(),
      changed_this_round: HashSet::new(),
      store: Arc::new(Store::new(capacity)),
      transactions: Vec::with_capacity(100_000),
      register_map: HashMap::new(),
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(), Error> {
    self.changed_this_round.clear();
    for change in &txn.changes {
      println!("{:?}", change);
      match change {
        Change::NewTable{table_id, rows, columns} => {
          let register = Register{table_id: TableId::Global(*table_id), row: Index::All, column: Index::All};
          self.changed_this_round.insert(register.hash());
          self.register_map.insert(register.hash(), register);
          self.tables.insert(*table_id, Table::new(
            *table_id, 
            *rows, 
            *columns, self.store.clone()));
        },
        Change::SetColumnAlias{table_id, column_ix, column_alias} => {
          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
          store.column_index_to_alias.insert((*table_id,*column_ix),*column_alias);
          store.column_alias_to_index.insert((*table_id,*column_alias),*column_ix);
          let register = Register{table_id: TableId::Global(*table_id), row: Index::All, column: Index::Alias(*column_alias)};
          self.changed_this_round.insert(register.hash());
          self.register_map.insert(register.hash(), register);
        }
        Change::Set{table_id, values} => {
          match self.tables.get_mut(&table_id) {
            Some(table) => {
              for (row, column, value) in values {
                // Set the value
                table.set(row, column, *value);
                // Mark the table as updated
                let register = Register{table_id: TableId::Global(*table_id), row: Index::All, column: *column};
                self.changed_this_round.insert(register.hash());
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
    println!("{:?}", self.tables);
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
    write!(f,"{:?}", self.store);
    write!(f, "tables: \n")?;
    for (id,table) in self.tables.iter() {
      write!(f, "{:?}\n", table)?;   
    }
    Ok(())
  }
}

// Holds changes to be applied to the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
  pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Updates the database
pub enum Change {
  Set{table_id: u64, values: Vec<(Index, Index, Value)>},
  SetColumnAlias{table_id: u64, column_ix: usize, column_alias: u64},
  NewTable{table_id: u64, rows: usize, columns: usize},
}