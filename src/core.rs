use block::Error;
use database::{Database, Transaction};
use runtime::Runtime;
use table::{Value, Index};
use std::rc::Rc;
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


// ## Core

// Cores are the smallest unit of a mech program exposed to a user. They hold references to all the 
// subparts of Mech, including the database (defines the what) and the runtime (defines the how).
// The core accepts transactions and applies those to the database. Updated tables in the database
// trigger computation in the runtime, which can further update the database. Execution terminates
// when a steady state is reached, or an iteration limit is reached (whichever comes first). The 
// core then waits for further transactions.
pub struct Core {
  pub runtime: Runtime,
  pub database: Rc<RefCell<Database>>,
}

impl Core {
  pub fn new(capacity: usize) -> Core {
    let mut database = Rc::new(RefCell::new(Database::new(capacity)));
    Core {
      runtime: Runtime::new(database.clone(), 1000),
      database,
    }
  }

  pub fn load_standard_library(&mut self) {
    self.runtime.functions.insert(hash_string("math/add"),Some(math_add));
    self.runtime.functions.insert(hash_string("math/subtract"),Some(math_subtract));
    self.runtime.functions.insert(hash_string("math/multiply"),Some(math_multiply));
    self.runtime.functions.insert(hash_string("math/divide"),Some(math_divide));
    self.runtime.functions.insert(hash_string("compare/greater-than-equal"),Some(compare_greater_than_equal));
    self.runtime.functions.insert(hash_string("compare/greater-than"),Some(compare_greater_than));
    self.runtime.functions.insert(hash_string("compare/less-than-equal"),Some(compare_less_than_equal));
    self.runtime.functions.insert(hash_string("compare/less-than"),Some(compare_less_than));
    self.runtime.functions.insert(hash_string("compare/equal"),Some(compare_equal));
    self.runtime.functions.insert(hash_string("compare/not-equal"),Some(compare_not_equal));
    self.runtime.functions.insert(hash_string("logic/and"),Some(logic_and));
    self.runtime.functions.insert(hash_string("logic/or"),Some(logic_or));
    self.runtime.functions.insert(hash_string("table/set"),Some(table_set));
    self.runtime.functions.insert(hash_string("table/add-row"),Some(table_add_row));
    self.runtime.functions.insert(hash_string("table/range"),Some(table_range));
    self.runtime.functions.insert(hash_string("table/horizontal-concatenate"),Some(table_horizontal_concatenate));
    self.runtime.functions.insert(hash_string("table/vertical-concatenate"),Some(table_vertical_concatenate));
    self.runtime.functions.insert(hash_string("stats/sum"),Some(stats_sum));
    self.runtime.functions.insert(hash_string("set/any"),Some(set_any));
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(),Error> {

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

}

impl fmt::Debug for Core {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}\n", self.database.borrow())?;   
    write!(f, "{:?}\n", self.runtime)?;
    Ok(())
  }
}