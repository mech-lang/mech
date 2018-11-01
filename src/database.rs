// # Database

// ## Prelude

use alloc::{String, Vec};
use core::fmt;
use table::{Value, Table};
use indexes::{TableIndex, Hasher};
use hashmap_core::set::{HashSet};
use hashmap_core::map::{HashMap};
use runtime::{Runtime, Block};

// ## Changes

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Change {
  Append{table: u64, column: u64, value: Value},
  Set{table: u64, row: u64, column: u64, value: Value},
  Remove{table: u64, row: u64, column: u64, value: Value},
  NewTable{id: u64, rows: usize, columns: usize},
  RenameColumn{table: u64, column_ix: u64, column_id: u64},
  RemoveTable{id: u64, rows: usize, columns: usize},
}

impl fmt::Debug for Change {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Change::Append{table, column, value} => write!(f, "<append> #{:#x} [{:#x}: {:?}]", table, column, value),
      Change::Set{table, row, column, value} => write!(f, "<set> #{:#x} [{:#x} {:#x}: {:?}]", table, row, column, value),
      Change::Remove{table, row, column, value} => write!(f, "<remove> #{:#x} [{:#x} {:#x}: {:?}]", table, row, column, value),
      Change::NewTable{id, rows, columns} => write!(f, "<newtable> #{:#x} [{:?} x {:?}]", id, rows, columns),
      Change::RenameColumn{table, column_ix, column_id} => write!(f, "<renamecolumn> #{:#x} {:#x} -> {:#x}", table, column_ix, column_id),
      Change::RemoveTable{id, rows, columns} => write!(f, "<removetable> #{:#x} [{:?} x {:?}]", id, rows, columns),
    }
  }
}
  
// ## Transaction

#[derive(Clone)]
pub struct Transaction {
  pub tables: Vec<Change>,
  pub adds: Vec<Change>,
  pub removes: Vec<Change>,
  pub names: Vec<Change>,
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

  pub fn from_changeset(changes: Vec<Change>) -> Transaction {
    let mut txn = Transaction::new();
    for change in changes {
      match change {
        Change::Append{..} |
        Change::Set{..} => txn.adds.push(change),
        Change::Remove{..} => txn.removes.push(change),
        Change::RemoveTable{..} |
        Change::NewTable{..} => txn.tables.push(change),
        Change::RenameColumn{..} => txn.names.push(change),
      }
    }
    txn
  }

  pub fn from_change(change: Change) -> Transaction {
    let mut txn = Transaction::new();
    match change {
      Change::Append{..} |
      Change::Set{..} => txn.adds.push(change),
      Change::Remove{..} => txn.removes.push(change),
      Change::RemoveTable{..} |
      Change::NewTable{..} => txn.tables.push(change),
      Change::RenameColumn{..} => txn.names.push(change),
    }
    txn
  }

  pub fn from_adds_removes(adds: Vec<(u64, u64, u64, String)>, removes: Vec<(u64, u64, u64, String)>) -> Transaction {
    let mut txn = Transaction::new();
    for (table, row,column, value) in adds {
      txn.adds.push(Change::Set{table, row, column, value: Value::from_string(value)});
    }
    for (table, row,column, value) in removes {
      txn.removes.push(Change::Remove{table, row, column, value: Value::from_string(value)});
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

#[derive(Debug)]
pub struct Interner {
  pub offset: usize,
  pub tables: TableIndex,
  pub changes: Vec<Change>,
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
      self.intern_change(table);
    }
    // Change names
    for name in txn.names.iter() {
      self.intern_change(name);
    }
    // Handle the removes
    for remove in txn.removes.iter() {
      self.intern_change(remove);
    }
    // Handle the adds
    for add in txn.adds.iter() {
      self.intern_change(add);
    }    

  }

  fn intern_change(&mut self, change: &Change) {  
    match change {
      Change::Set{table, row, column, value} => {
        match self.tables.get_mut(*table) {
          Some(table_ref) => {
            match table_ref.set_cell_by_id(*row as usize, *column as usize, value.clone()) {
              Ok(old_value) => {
                if self.offset == 0 {
                  match old_value {
                    Value::Empty => (),
                    // Save a remove so that we can rewind
                    _ => self.save_change(&Change::Remove{table: *table, row: *row, column: *column, value: old_value}),
                  }
                }
              },
              _ => (),
            };
          }
          None => (),
        };
        self.tables.changed_this_round.insert((*table as usize, *column as usize));
      },
      Change::Remove{table, row, column, value} => {
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
      },
      Change::Append{table, column, value} => {
        match self.tables.get_mut(*table) {
          Some(table_ref) => {
            let row: usize = table_ref.rows + 1;;
            table_ref.set_cell_by_id(row, *column as usize, value.clone());
          }
          None => (),
        };
        self.tables.changed_this_round.insert((*table as usize, *column as usize));
      },
      Change::NewTable{id, rows, columns } => {
        self.tables.register(Table::new(*id, *rows, *columns));
      }
      Change::RemoveTable{id, rows, columns } => {
        self.tables.remove(&id);
      }
      Change::RenameColumn{table, column_ix, column_id} => {
        match self.tables.get_mut(*table) {
          Some(table_ref) => {
            table_ref.set_column_id(*column_id, *column_ix as usize);
          }
          None => (),
        };
        self.tables.changed_this_round.insert((*table as usize, *column_id as usize));
      },
    }
    if self.offset == 0 {
      self.save_change(change);
    }    
  }

  // Save the change. If there's enough room in memory, store it there. 
  // If not, make room by evicting some old change and throw that on disk. 
  // For now, we'll make the policy that the oldest record get evicted first.
  fn save_change(&mut self, change: &Change) {
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

  pub fn get_table(&self, table: u64) -> Option<&Table> {
    self.tables.get(table)
  }

  pub fn get_column_by_id(&self, table: u64, column_id: usize) -> Option<&Vec<Value>> {
    match self.tables.get(table) {
      Some(stored_table) => {
        match stored_table.get_column_by_id(column_id) {
          Some(column) => Some(column),
          None => None,
        }
      },
      None => None,
    }
  }

  pub fn get_column_by_ix(&self, table: u64, column_ix: usize) -> Option<&Vec<Value>> {
    match self.tables.get(table) {
      Some(stored_table) => {
        match stored_table.get_column_by_ix(column_ix) {
          Some(column) => Some(column),
          None => None,
        }
      },
      None => None,
    }
  }

  pub fn get_cell(&self, table: u64, row_ix: usize, column_ix: usize) -> Option<&Value> {
    match self.tables.get(table) {
      Some(stored_table) => {
        stored_table.index(row_ix, column_ix)
      },
      None => None,
    }
  }

  pub fn len(&self) -> usize {
    self.changes_count as usize
  }

}