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
    self.runtime.load_library_function("math/add",math_add);
    self.runtime.load_library_function("math/subtract",math_subtract);
    self.runtime.load_library_function("math/multiply",math_multiply);
    self.runtime.load_library_function("math/divide",math_divide);
    self.runtime.load_library_function("compare/greater-than-equal",compare_greater_than_equal);
    self.runtime.load_library_function("compare/greater-than",compare_greater_than);
    self.runtime.load_library_function("compare/less-than-equal",compare_less_than_equal);
    self.runtime.load_library_function("compare/less-than",compare_less_than);
    self.runtime.load_library_function("compare/equal",compare_equal);
    self.runtime.load_library_function("compare/not-equal",compare_not_equal);
    self.runtime.load_library_function("logic/and",logic_and);
    self.runtime.load_library_function("logic/or",logic_or);
    self.runtime.load_library_function("table/set",table_set);
    self.runtime.load_library_function("table/add-row",table_add_row);
    self.runtime.load_library_function("table/range",table_range);
    self.runtime.load_library_function("table/horizontal-concatenate",table_horizontal_concatenate);
    self.runtime.load_library_function("table/vertical-concatenate",table_vertical_concatenate);
    self.runtime.load_library_function("stats/sum",stats_sum);
    self.runtime.load_library_function("set/any",set_any);
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