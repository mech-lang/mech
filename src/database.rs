use crate::*;
use hashbrown::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;

type FunctionName = u64;

#[derive(Clone)]
pub enum Change {
  Set((FunctionName, Vec<(usize, usize, Value)>)),
  NewTable{table_id: u64, rows: usize, columns: usize},
  ColumnAlias{table_id: u64, column_ix: usize, column_alias: u64},
}

impl fmt::Debug for Change {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Change::Set((function_name,args)) => write!(f,"Set({},{:#?})",function_name,args)?,
      Change::NewTable{table_id,rows,columns} => write!(f,"NewTable({},{:?},{:?})",humanize(table_id),rows,columns)?,
      Change::ColumnAlias{table_id,column_ix,column_alias} => write!(f,"ColumnAlias({},{:?},{})",humanize(table_id),column_ix,humanize(column_alias))?,
    }
    Ok(())
  }
}

pub type Transaction = Vec<Change>;

#[derive(Clone)]
pub struct Database {
  pub tables: HashMap<u64,Rc<RefCell<Table>>>,
  pub table_alias_to_id: HashMap<u64,TableId>,
}

impl Database {
  pub fn new() -> Database {
    Database {
      tables: HashMap::new(),
      table_alias_to_id: HashMap::new(),
    }
  }

  pub fn insert_alias(&mut self, alias: u64, table_id: TableId) -> Result<TableId,MechError> {
    match self.table_alias_to_id.try_insert(alias, table_id) {
      Err(_) => Err(MechError::GenericError(6333)),
      Ok(x) => Ok(*x), 
    }
  }

  pub fn insert_table(&mut self, table: Table) -> Result<Rc<RefCell<Table>>,MechError> {
    match self.tables.try_insert(table.id, Rc::new(RefCell::new(table))) {
      Ok(x) => Ok(x.clone()),
      Err(_) => Err(MechError::GenericError(4211)),
    }
  }

  pub fn get_table(&self, table_name: &str) -> Option<&Rc<RefCell<Table>>> {
    let alias = hash_str(table_name);
    match self.table_alias_to_id.get(&alias) {
      Some(table_id) => {
        self.tables.get(table_id.unwrap())
      }
      _ => self.tables.get(&alias),
    }
  }

  pub fn get_table_by_id(&self, table_id: &u64) -> Option<&Rc<RefCell<Table>>> {
    match self.tables.get(table_id) {
      None => {
        match self.table_alias_to_id.get(&table_id) {
          None => None,
          Some(table_id) => {
            self.tables.get(table_id.unwrap())
          }
        }
      }
      x => x
    }
  }

  pub fn get_table_by_id_mut(&self, table_id: u64) -> Option<&Rc<RefCell<Table>>> {
    let table_id = match self.tables.contains_key(&table_id) {
      true => table_id,
      false => match self.table_alias_to_id.get(&table_id) {
        Some(table_id) => *table_id.unwrap(),
        None => return None,
      }
    };
    self.tables.get(&table_id)
  }
}

impl fmt::Debug for Database {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut db_drawing = BoxPrinter::new();
    
    db_drawing.add_header("tables");
    for table in self.tables.values() {
      db_drawing.add_line(format!("{:?}", table.borrow()));
    }
    db_drawing.add_header("table alias -> table id");
    for (alias,id) in self.table_alias_to_id.iter() {
      db_drawing.add_line(format!("{} -> {:?}", humanize(alias), id));
    }
    write!(f,"{:?}",db_drawing)?;
    Ok(())
  }
}


/*
use table::{Table, TableId, TableIndex};
use value::{Value, ValueMethods, NumberLiteral};
use block::{Register};
use errors::{Error, ErrorType};
use ::hash_str;
use std::sync::Arc;
use std::cell::RefCell;
use hashbrown::{HashSet, HashMap};
use rust_core::fmt;

// ## Store

// Holds all of the values of the program in a 1D vector. We keep track of how many times a value
// is referenced using a counter. When the counter goes to zero, the memory location is marked as
// free and is available to be overwritten by a new value.
#[derive(Debug)]
pub struct Store {
  pub changed: bool,
  pub column_alias_to_index: HashMap<(u64,u64),usize>,
  pub table_id_to_alias: HashMap<TableId, u64>,
  pub table_alias_to_id: HashMap<u64, TableId>,
  pub column_index_to_alias: HashMap<(u64,usize),u64>,
  pub strings: HashMap<u64, String>,        // This is where we store string literals and other strings
  pub number_literals: HashMap<u64, NumberLiteral>,   // This is where we store number literals and other numbers
}

impl Store {
  pub fn new(capacity: usize) -> Store {
    Store {
      changed: false,
      column_alias_to_index: HashMap::new(),
      column_index_to_alias: HashMap::new(),
      table_id_to_alias: HashMap::new(),
      table_alias_to_id: HashMap::new(),
      strings: HashMap::new(),
      number_literals: HashMap::new(),
    }
  }
}

// ## Database

// The database holds a map of tables, and a data store that holds a data array of values. 
// Cells in the tables contain memory addresses that point to elements of the store data array.
// The database processes transactions, which are arrays of changes that ar applies to the tables
// in the database.
pub struct Database {
  pub tables: HashMap<u64, Arc<RefCell<Table>>>,
  pub changed_this_round: HashSet<Register>,
  pub store: Arc<Store>,
}

impl Database {

  pub fn new(capacity: usize) -> Database {    
    Database {
      tables: HashMap::new(),
      changed_this_round: HashSet::new(),
      store: Arc::new(Store::new(capacity)),
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(), Error> {
    self.changed_this_round.clear();
    for change in &txn.changes {
      match change {
        Change::NewTable{table_id, rows, columns} => {

          match self.tables.get_mut(&table_id) {
            Some(_table) => {
              // TODO warn user the table exists already
            },
            None => {
              let register = Register{table_id: TableId::Global(*table_id), row: TableIndex::All, column: TableIndex::All};
              //self.changed_this_round.insert(register);
              self.tables.insert(*table_id, Arc::new(RefCell::new(Table::new(
                *table_id, 
                *rows, 
                *columns, self.store.clone()))));
            }
          }
        },
        Change::SetColumnAlias{table_id, column_ix, column_alias} => {
          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
          match store.column_alias_to_index.get(&(*table_id,*column_alias)) {
            None => {
              store.column_index_to_alias.insert((*table_id,*column_ix),*column_alias);
              store.column_alias_to_index.insert((*table_id,*column_alias),*column_ix);
              let register = Register{table_id: TableId::Global(*table_id), row: TableIndex::All, column: TableIndex::Alias(*column_alias)};
              //self.changed_this_round.insert(register);
            }
            _ => (),
          }
        }
        Change::Set{table_id, values} => {
          match self.tables.get_mut(&table_id) {
            Some(table) => {
              for (row, column, value) in values {
                // Set the value
                table.borrow_mut().set(row, column, *value);
                // Mark the table as updated
                let register = Register{table_id: TableId::Global(*table_id), row: *row, column: *column};
                self.changed_this_round.insert(register);
                let register = Register{table_id: TableId::Global(*table_id), row: TableIndex::All, column: TableIndex::All};
                self.changed_this_round.insert(register);
              }
            },
            None => {
              // TODO Throw an error here and roll back all changes
            }
          }
        },
        Change::Table{table_id, data} => {
          match self.tables.get_mut(&table_id) {
            Some(table) => {
              table.borrow_mut().set_data(data);
              // Mark the table as updated
              let register = Register{table_id: TableId::Global(*table_id), row: TableIndex::All, column: TableIndex::All};
              self.changed_this_round.insert(register);
            },
            None => {
              // TODO Throw an error here and roll back all changes
            }
          }          
        },
        Change::InternString{string} => {
          let store = unsafe{&mut *Arc::get_mut_unchecked(&mut self.store)};
          store.strings.insert(Value::from_string(&string), string.to_string());
        }
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
      write!(f, "       {:?}\n", changed)?;
    }
    write!(f,"{:?}", self.store).ok();
    write!(f, "tables: \n")?;
    for (_id,table) in self.tables.iter() {
      write!(f, "{:?}\n", table.borrow())?;   
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
  Table{table_id: u64, data: Vec<Value>},
  Set{table_id: u64, values: Vec<(TableIndex, TableIndex, Value)>},
  SetColumnAlias{table_id: u64, column_ix: usize, column_alias: u64},
  NewTable{table_id: u64, rows: usize, columns: usize},
  InternString{string: String},
}
*/