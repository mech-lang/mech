use block::{Block, Register, Error};
use database::{Database, Change, Transaction};
use runtime::Runtime;
use table::{Table, Index, TableId};
use value::Value;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use rust_core::fmt;
use operations::{
  math_add, math_subtract, 
  math_multiply, 
  math_divide, 
  compare_greater_than_equal,
  compare_greater_than,
  compare_less_than,
  compare_less_than_equal,
  compare_equal,
  compare_not_equal,
  logic_and,
  logic_or,
  table_range, 
  table_set,
  table_add_row,
  table_horizontal_concatenate,
  table_vertical_concatenate,
  stats_sum,
  set_any
};
use ::hash_string;
use operations::{MechFunction};

// ## Core

// Cores are the smallest unit of a mech program exposed to a user. They hold references to all the 
// subparts of Mech, including the database (defines the what) and the runtime (defines the how).
// The core accepts transactions and applies those to the database. Updated tables in the database
// trigger computation in the runtime, which can further update the database. Execution terminates
// when a steady state is reached, or an iteration limit is reached (whichever comes first). The 
// core then waits for further transactions.
pub struct Core {
  pub runtime: Runtime,
  pub database: Arc<RefCell<Database>>,
}

impl Core {
  pub fn new(capacity: usize) -> Core {
    let mut database = Arc::new(RefCell::new(Database::new(capacity)));
    Core {
      runtime: Runtime::new(database.clone(), 5),
      database,
    }
  }

  pub fn load_standard_library(&mut self) {
    {
      let name = "table/split";
      let name_hash = hash_string(&name);
      let mut db = self.runtime.database.borrow_mut();
      let mut store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
      store.strings.insert(name_hash, name.to_string());
    }
    self.runtime.load_library_function("math/add",Some(math_add));
    self.runtime.load_library_function("math/subtract",Some(math_subtract));
    self.runtime.load_library_function("math/multiply",Some(math_multiply));
    self.runtime.load_library_function("math/divide",Some(math_divide));
    self.runtime.load_library_function("compare/greater-than-equal",Some(compare_greater_than_equal));
    self.runtime.load_library_function("compare/greater-than",Some(compare_greater_than));
    self.runtime.load_library_function("compare/less-than-equal",Some(compare_less_than_equal));
    self.runtime.load_library_function("compare/less-than",Some(compare_less_than));
    self.runtime.load_library_function("compare/equal",Some(compare_equal));
    self.runtime.load_library_function("compare/not-equal",Some(compare_not_equal));
    self.runtime.load_library_function("logic/and",Some(logic_and));
    self.runtime.load_library_function("logic/or",Some(logic_or));
    self.runtime.load_library_function("table/set",Some(table_set));
    self.runtime.load_library_function("table/add-row",Some(table_add_row));
    self.runtime.load_library_function("table/range",Some(table_range));
    self.runtime.load_library_function("table/horizontal-concatenate",Some(table_horizontal_concatenate));
    self.runtime.load_library_function("table/vertical-concatenate",Some(table_vertical_concatenate));
    self.runtime.load_library_function("stats/sum",Some(stats_sum));
    self.runtime.load_library_function("set/any",Some(set_any));
  }

  pub fn get_string(&self, id: &u64) -> Option<String> {
    match self.database.borrow().store.strings.get(&id) {
      Some(string) => Some(string.clone()),
      None => None,
    }
  }

  pub fn insert_string(&self, string: &str) {
    let hashed_string = hash_string(string);
    let mut db = self.runtime.database.borrow_mut();
    let mut store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
    store.strings.insert(hashed_string, string.to_string());
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(),Error> {
    for change in &txn.changes {
      match change {
        Change::NewTable{table_id, rows, columns} => {
          self.runtime.output.insert(Register{table_id: TableId::Global(*table_id), row: Index::All, column: Index::All}.hash());
        }
        _ => (),
      }
    }
    self.database.borrow_mut().process_transaction(txn)?;
    self.runtime.run_network()?;
    Ok(())
  }

  pub fn get_cell_in_table(&mut self, table: u64, row: &Index, column: &Index) -> Option<Value> {
    match self.database.borrow().tables.get(&table) {
      Some(table_ref) => {
        table_ref.get(row, column)
      },
      None => None,
    }
  }

  pub fn register_blocks(&mut self, blocks: Vec<Block>) {
    self.runtime.register_blocks(blocks);
  }

  pub fn step(&mut self) {
    self.runtime.run_network();
  }

  pub fn get_table(&self, table_id: u64) -> Option<Table> {
    match self.runtime.database.borrow().tables.get(&table_id) {
      Some(table) => Some(table.clone()),
      None => None,
    }
  }

  pub fn get_table_by_name(&self, table_name: &str) -> Option<Table> {
    let table_id = hash_string(table_name);
    self.get_table(table_id)
  }

  pub fn clear_table(&self, table_id: u64) {
    match self.runtime.database.borrow_mut().tables.get_mut(&table_id) {
      Some(table) => table.clear(),
      None => (),
    };
  }

}

impl fmt::Debug for Core {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}\n", self.database.borrow())?;   
    write!(f, "{:?}\n", self.runtime)?;
    Ok(())
  }
}