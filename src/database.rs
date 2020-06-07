use table::{Table, Value, Index};
use block::{Error, Register};
use ::humanize;
use std::cell::RefCell;
use std::rc::Rc;
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
  pub identifiers: HashMap<u64, String>,
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
      identifiers: HashMap::new(),
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
    if self.data_end + 1 == self.capacity && self.free[0] != 0 {
      self.next = self.free[self.free_next];
      if self.free_next + 1 == self.free.len() {
        self.free_next = 0;
      } else {
        self.free_next += 1;
      }
    // Extend the data if it's full and there is no free space
    } else if self.data_end + 1 == self.capacity && self.free[0] == 0 {
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
pub struct Database {
  pub tables: HashMap<u64, Table>,
  pub changed_this_round: HashSet<u64>,
  pub store: Rc<Store>,
  pub transactions: Vec<Transaction>,
}

impl Database {

  pub fn new(capacity: usize) -> Database {    
    Database {
      tables: HashMap::new(),
      changed_this_round: HashSet::new(),
      store: Rc::new(Store::new(capacity)),
      transactions: Vec::with_capacity(100_000),
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(), Error> {
    self.changed_this_round.clear();
    for change in &txn.changes {
      match change {
        Change::NewTable{table_id, rows, columns} => {
          self.tables.insert(*table_id, Table::new(
            *table_id, 
            *rows, 
            *columns, self.store.clone()));
        },
        Change::SetColumnAlias{table_id, column_ix, column_alias} => {
          let store = unsafe{&mut *Rc::get_mut_unchecked(&mut self.store)};
          store.column_index_to_alias.insert((*table_id,*column_ix),*column_alias);
          store.column_alias_to_index.insert((*table_id,*column_alias),*column_ix);
        }
        Change::Set{table_id, values} => {
          match self.tables.get_mut(&table_id) {
            Some(table) => {
              for (row, column, value) in values {
                // Set the value
                table.set(row, column, *value);
                // Mark the table as updated
                let register_hash = Register{table_id: *table_id, row: Index::All, column: *column};
                self.changed_this_round.insert(register_hash.hash());
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
    write!(f,"{:?}", self.store);
    write!(f, "tables: \n")?;
    for (id,table) in self.tables.iter() {
      write!(f, "{:?}\n", table)?;   
    }
    Ok(())
  }
}

// Holds changes to be applied to the database
#[derive(Debug, Clone)]
pub struct Transaction {
  pub changes: Vec<Change>,
}

#[derive(Debug, Clone)]
// Updates the database
pub enum Change {
  Set{table_id: u64, values: Vec<(Index, Index, Value)>},
  SetColumnAlias{table_id: u64, column_ix: usize, column_alias: u64},
  NewTable{table_id: u64, rows: usize, columns: usize},
}

/*
// # Database

// ## Prelude

#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use core::fmt;
use table::{Value, Table, TableId, Index};
use indexes::TableIndex;
use hashbrown::hash_map::{HashMap, Entry};
use std::rc::Rc;
use std::cell::RefCell;

// ## Changes

#[derive(Clone, PartialEq)]
pub enum Change {
  Set{table: u64, column: Index, values: Vec<(Index, Rc<Value>)>},
  Remove{table: u64, column: Index, values: Vec<(Index, Rc<Value>)>},
  NewTable{id: u64, rows: u64, columns: u64},
  RenameColumn{table: u64, column_ix: u64, column_alias: u64},
  RemoveTable{id: u64, rows: u64, columns: u64},
}

impl fmt::Debug for Change {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Change::Set{table, column, values} => write!(f, "<set> #{:#x} {:?} {:?}", table, column, values),
      Change::Remove{table, column, values} => write!(f, "<remove> #{:#x} {:?} {:?}", table, column, values),
      Change::NewTable{id, rows, columns} => write!(f, "<newtable> #{:#x} [{:?} x {:?}]", id, rows, columns),
      Change::RenameColumn{table, column_ix, column_alias} => write!(f, "<renamecolumn> #{:#x} {:#x} -> {:#x}", table, column_ix, column_alias),
      Change::RemoveTable{id, rows, columns} => write!(f, "<removetable> #{:#x} [{:?} x {:?}]", id, rows, columns),
    }
  }
}

// ## Transaction

#[derive(Clone)]
pub struct Transaction {
  pub tables: Vec<Rc<Change>>,
  pub adds: Vec<Rc<Change>>,
  pub removes: Vec<Rc<Change>>,
  pub names: Vec<Rc<Change>>,
}

impl Transaction {
  pub fn new() -> Transaction {
    Transaction {
      tables: Vec::new(),
      adds: Vec::new(),
      removes: Vec::new(),
      names: Vec::new(),
    }
  }

  pub fn from_changeset(changes: Vec<Rc<Change>>) -> Transaction {
    let mut txn = Transaction::new();
    for change in changes {
      match *change {
        Change::Set{..} => txn.adds.push(change.clone()),
        Change::Remove{..} => txn.removes.push(change.clone()),
        Change::RemoveTable{..} |
        Change::NewTable{..} => txn.tables.push(change.clone()),
        Change::RenameColumn{..} => txn.names.push(change.clone()),
      }
    }
    txn
  }

  pub fn from_change(change: Rc<Change>) -> Transaction {
    let mut txn = Transaction::new();
    match *change {
      Change::Set{..} => txn.adds.push(change.clone()),
      Change::Remove{..} => txn.removes.push(change.clone()),
      Change::RemoveTable{..} |
      Change::NewTable{..} => txn.tables.push(change.clone()),
      Change::RenameColumn{..} => txn.names.push(change.clone()),
    }
    txn
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
    for ref name in &self.names {
      write!(f, "{:?}\n", name).unwrap();
    }
    Ok(())
  }
}

// ## Interner

#[derive(Debug, Clone)]
pub struct Interner {
  pub offset: usize,
  pub tables: TableIndex,
  pub names: HashMap<u64,String>,
  pub changes: Vec<Rc<Change>>,
  pub changes_count: usize,
  pub change_pointer: usize, // points at the next available slot in memory that can hold a change
  pub rollover: usize,
  pub last_round: usize,
}

impl Interner {

  pub fn new(change_capacity: usize, table_capacity: usize) -> Interner {
    Interner {
      offset: 0,
      tables: TableIndex::new(table_capacity),
      names: HashMap::new(),
      changes: Vec::with_capacity(change_capacity),
      changes_count: 0,
      change_pointer: 0,
      rollover: 0,
      last_round: 0,
    }
  }

  pub fn clear(&mut self) {
    self.tables.clear();
    self.changes.clear();
    self.changes_count = 0;
    self.change_pointer = 0;
  }

  pub fn process_transaction(&mut self, txn: &Transaction) {
    // First make any tables
    for table in txn.tables.iter() {
      self.intern_change(table.clone());
    }
    // Change names
    for name in txn.names.iter() {
      self.intern_change(name.clone());
    }
    // Handle the removes
    for remove in txn.removes.iter() {
      self.intern_change(remove.clone());
    }
    // Handle the adds
    for add in txn.adds.iter() {
      self.intern_change(add.clone());
    }    
  }

  fn intern_change(&mut self, change: Rc<Change>) { 
    match &*change {
      Change::Set{table, column, values} => {
        let mut changed = false;
        let mut alias: Option<u64> = None;
        match self.tables.get(*table) {
          Some(table_ref) => {
            alias = table_ref.borrow().get_column_alias(&column);
            for (row, value) in values {
              table_ref.borrow_mut().set_cell(&row, &column, value.clone());
              changed = true;
              /*if old_value != *value {
                changed = true;
              }
              if self.offset == 0 && changed == true {
                match *old_value {
                  Value::Empty => (),
                  // Save a remove so that we can rewind
                  _ => (), //self.save_change(&Change::Remove{table: *table, row: row.clone(), column: column.clone(), value: old_value}),
                }
              }*/
            }
            if changed == true {
              match alias {
                Some(id) => {
                  self.tables.changed_this_round.insert((table.clone(), Index::Alias(id as usize)))
                },
                _ => false,
              };
              self.tables.changed_this_round.insert((table.clone(), column.clone()));
              self.tables.changed_this_round.insert((table.clone(), Index::Index(0)));
            }
          }
          None => (),
        };
      },
      Change::Remove{table, column, values} => {
        /*
        match value {
          Value::Empty => (),
          _ => {
            match self.tables.get_mut(*table) {
              Some(table_ref) => {
                table_ref.set_cell_by_id(*row as usize, *column as usize, Value::Empty);
              }
              None => (),
            };            
          },
        };
        self.tables.changed_this_round.insert((*table as usize, *column as usize));
        */
      },
      Change::NewTable{id, rows, columns } => {
        match self.tables.get(*id) {
          None => {
            self.tables.insert(Table::new(*id, *rows, *columns));
            self.tables.changed_this_round.insert((*id, Index::Index(0)));
          }
          _ => (),
        }
        
      }
      Change::RemoveTable{id, rows: _, columns: _} => {
        self.tables.remove(&id);
      }
      Change::RenameColumn{table, column_ix, column_alias} => { 
        match self.tables.get(*table) {
          Some(table_ref) => {
            table_ref.borrow_mut().set_column_alias(*column_alias, *column_ix);
          }
          None => (),
        };
        self.tables.changed_this_round.insert((*table, Index::Alias(*column_alias  as usize)));
      },
    }
    if self.offset == 0 {
      //self.save_change(change);
    }    
  }

  // Save the change. If there's enough room in memory, store it there. 
  // If not, make room by evicting some old change and throw that on disk. 
  // For now, we'll make the policy that the oldest record get evicted first.
  fn save_change(&mut self, change: Rc<Change>) {
    if self.changes.len() < self.changes.capacity() {
      self.changes.push(change.clone());
    } else if self.change_pointer == self.changes.capacity() {
      self.change_pointer = 0;
      self.rollover += 1;
      self.changes[self.change_pointer] = change.clone();
    } else {
      self.changes[self.change_pointer] = change.clone();
    }
    self.change_pointer += 1;
    self.changes_count += 1;
  }

  pub fn get_table(&self, table: u64) -> Option<&Rc<RefCell<Table>>> {
    self.tables.get(table)
  }

  pub fn contains(&mut self, table: TableId) -> bool {
    self.tables.contains(*table.unwrap())
  }

  pub fn get_column(&self, table: TableId, column: Index) -> Option<&Vec<Rc<Value>>> {
    match self.tables.get(*table.unwrap()) {
      Some(stored_table) => {
        match unsafe{(*stored_table.as_ptr()).get_column(&column)} {
          Some(column) => Some(column),
          None => None,
        }
      },
      None => None,
    }
  }

  /*
  pub fn get_cell(&self, table: u64, row_ix: usize, column_ix: usize) -> Option<&Value> {
    match self.tables.get(table) {
      Some(stored_table) => {
        stored_table.index(row_ix, column_ix)
      },
      None => None,
    }
  }*/

  pub fn len(&self) -> usize {
    self.changes_count as usize
  }

}*/