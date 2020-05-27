use block::Error;
use database::{Database, Transaction};
use runtime::Runtime;
use table::{Value, Index};
use std::rc::Rc;
use std::cell::RefCell;
use rust_core::fmt;


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