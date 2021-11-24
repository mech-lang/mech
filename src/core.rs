// ## Core

// Cores are the smallest unit of a mech program exposed to a user. They hold references to all the
// subparts of Mech, including the database (defines the what) and the runtime (defines the how).
// The core accepts transactions and applies those to the database. Updated tables in the database
// trigger computation in the runtime, which can further update the database. Execution terminates
// when a steady state is reached, or an iteration limit is reached (whichever comes first). The
// core then waits for further transactions.

use crate::*;
use hashbrown::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;

pub type BlockRef = Rc<RefCell<Block>>;

#[derive(Clone, Debug)]
pub struct Core {
  blocks: Vec<BlockRef>,
  unsatisfied_blocks: Vec<BlockRef>,
  database: Rc<RefCell<Database>>,
  errors: HashMap<MechError,Vec<BlockRef>>,
  potentially_ready: Vec<BlockRef>,
  pub schedules: HashMap<(u64,usize,usize),Vec<Vec<usize>>>,
}

impl Core {

  pub fn new() -> Core {
    Core {
      blocks: Vec::new(),
      unsatisfied_blocks: Vec::new(),
      database: Rc::new(RefCell::new(Database::new())),
      errors: HashMap::new(),
      potentially_ready: Vec::new(),
      schedules: HashMap::new(),
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<Vec<BlockRef>,MechError> {
    let mut registers = Vec::new();
    let mut block_refs = Vec::new();
    for change in txn {
      match change {
        Change::Set((table_id, adds)) => {
          match self.database.borrow().get_table_by_id(table_id) {
            Some(table) => {
              for (row,col,val) in adds {
                match table.borrow().set(*row, *col, val.clone()) {
                  Err(_) => {
                    // Index out of bounds.
                    return Err(MechError::GenericError(9131));
                  }
                  _ => {
                    registers.push((*table_id,*row,*col));
                  },
                }
              }
            }
            _ => {
              // Table doesn't exist
              return Err(MechError::GenericError(9132));
            }
          }
        }
        Change::NewTable{table_id, rows, columns} => {
          let table = Table::new(*table_id,*rows,*columns);
          self.database.borrow_mut().insert_table(table)?;
          match self.errors.remove(&MechError::MissingTable(TableId::Global(*table_id))) {
            Some(mut ublocks) => {
              block_refs.append(&mut ublocks);
            }
            None => (),
          }
        }
        Change::CopyTable{table_id,table} => {
          match self.database.borrow_mut().tables.try_insert(*table_id, table.clone()) {
            Ok(x) => (),
            Err(_) => {return Err(MechError::GenericError(4214));},
          }
        }
        Change::ColumnAlias{table_id, column_ix, column_alias} => {
          match self.database.borrow_mut().get_table_by_id(table_id) {
            Some(table) => {
              let mut table_brrw = table.borrow_mut();   
              let rows = table_brrw.rows;
              if *column_ix + 1 > table_brrw.cols {
                table_brrw.resize(rows, column_ix + 1);
              }    
              table_brrw.set_column_alias(*column_ix,*column_alias);     
            }
            _ => {return Err(MechError::GenericError(9139));}
          }
        }
      }
    }
    for register in registers {
      self.step(&register);
    }
    Ok(block_refs)
  }

  pub fn insert_table(&mut self, table: Table) -> Result<Rc<RefCell<Table>>,MechError> {
    self.database.borrow_mut().insert_table(table)
  }

  pub fn get_table(&mut self, table_name: &str) -> Option<Rc<RefCell<Table>>> {
    match self.database.borrow().get_table(table_name) {
      Some(table) => Some(table.clone()),
      None => None,
    }
  }

  pub fn get_table_by_id(&mut self, table_id: u64) -> Option<Rc<RefCell<Table>>> {
    match self.database.borrow().get_table_by_id(&table_id) {
      Some(table) => Some(table.clone()),
      None => None,
    }
  }

  pub fn insert_block(&mut self, mut block_ref: BlockRef) -> Result<(),MechError> {
    let block_ref_c = block_ref.clone();
    let mut block_brrw = block_ref.borrow_mut();
    block_brrw.global_database = self.database.clone();
    let mut potentially_ready = Vec::new();
    // Processing a transaction can lead to subsequent changes
    // that need to be processed.
    loop {
      let changes = block_brrw.changes.clone();
      block_brrw.changes.clear();
      let mut pr_blocks = self.process_transaction(&changes)?;
      potentially_ready.append(&mut pr_blocks);
      block_brrw.ready();
      if block_brrw.changes.len() == 0 {
        break;
      }
    }
    // try to satisfy the block
    match block_brrw.ready() {
      true => {
        block_brrw.gen_id();
        self.blocks.push(block_ref_c.clone());
        for pr_block in potentially_ready {
          self.insert_block(pr_block);
        }
        Ok(())
      }
      false => {
        let (mech_error,_) = block_brrw.unsatisfied_transformation.as_ref().unwrap();
        let blocks_with_errors = self.errors.entry(mech_error.clone()).or_insert(Vec::new());
        blocks_with_errors.push(block_ref_c.clone());
        Err(MechError::GenericError(8963))
      },
    }
  }

  pub fn step(&mut self, register: &(u64,usize,usize)) {
    match &mut self.schedules.get(register) {
      Some(schedule) => {
        for blocks in schedule.iter() {
          for block_ix in blocks {
            self.blocks[*block_ix].borrow_mut().solve();
          }
        }
      }
      _ => (),
    }

  }
}




/*
use block::{Block, Register};
use errors::{Error, ErrorType};
use database::{Database, Change, Transaction};
use runtime::Runtime;
use table::{Table, TableIndex, TableId};
use value::{Value, NumberLiteral, ValueMethods};
use std::sync::Arc;
use std::cell::RefCell;
use rust_core::fmt;
use operations::{
  math_add, 
  math_subtract, 
  math_multiply, 
  math_divide, 
  math_exponent,
  compare_greater__than__equal,
  compare_greater__than,
  compare_less__than,
  compare_less__than__equal,
  compare_equal,
  compare_not__equal,
  logic_and,
  logic_or,
  logic_xor,
  table_range,
  table_set,
  table_horizontal__concatenate,
  table_vertical__concatenate,
  table_append__row,
  table_copy,
  stats_sum,
  set_any,
  set_all,
};
use ::hash_str;


pub struct Core {
  pub runtime: Runtime,
  pub database: Arc<RefCell<Database>>,
}

impl Core {
  pub fn new(capacity: usize, recursion_limit: u64) -> Core {
    let database = Arc::new(RefCell::new(Database::new(capacity)));
    Core {
      runtime: Runtime::new(database.clone(), recursion_limit),
      database,
    }
  }

  pub fn load_standard_library(&mut self) {
    {
      let name = "table/split";
      let name_hash = hash_str(&name);
      let mut db = self.runtime.database.borrow_mut();
      let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
      store.strings.insert(name_hash, name.to_string());
    }
    self.runtime.load_library_function("math/add",Some(math_add));
    self.runtime.load_library_function("math/subtract",Some(math_subtract));
    self.runtime.load_library_function("math/multiply",Some(math_multiply));
    self.runtime.load_library_function("math/divide",Some(math_divide));
    self.runtime.load_library_function("math/exponent",Some(math_exponent));
    self.runtime.load_library_function("compare/greater-than-equal",Some(compare_greater__than__equal));
    self.runtime.load_library_function("compare/greater-than",Some(compare_greater__than));
    self.runtime.load_library_function("compare/less-than-equal",Some(compare_less__than__equal));
    self.runtime.load_library_function("compare/less-than",Some(compare_less__than));
    self.runtime.load_library_function("compare/equal",Some(compare_equal));
    self.runtime.load_library_function("compare/not-equal",Some(compare_not__equal));
    self.runtime.load_library_function("logic/and",Some(logic_and));
    self.runtime.load_library_function("logic/or",Some(logic_or));
    self.runtime.load_library_function("logic/xor",Some(logic_xor));
    self.runtime.load_library_function("table/append-row",Some(table_append__row));
    self.runtime.load_library_function("table/range",Some(table_range));
    self.runtime.load_library_function("table/set",Some(table_set));
    self.runtime.load_library_function("table/horizontal-concatenate",Some(table_horizontal__concatenate));
    self.runtime.load_library_function("table/vertical-concatenate",Some(table_vertical__concatenate));
    self.runtime.load_library_function("table/copy",Some(table_copy));
    self.runtime.load_library_function("stats/sum",Some(stats_sum));
    self.runtime.load_library_function("set/any",Some(set_any));
    self.runtime.load_library_function("set/all",Some(set_all));
  }

  pub fn get_string(&self, id: &u64) -> Option<String> {
    match self.database.borrow().store.strings.get(&id) {
      Some(string) => Some(string.clone()),
      None => None,
    }
  }

  pub fn get_number_literal(&self, id: u64) -> Option<Vec<u8>> {
    match self.database.borrow().store.number_literals.get(&id) {
      Some(number_literal) => Some(number_literal.bytes.clone()),
      None => {
        if id.is_number_literal() {
          let len = id.len().unwrap();
          let mut bytes: Vec<u8> = Vec::with_capacity(len);
          for i in 0..len {
            bytes.push((id >> i * 8) as u8);
          }
          bytes.reverse();
          Some(bytes)
        } else {
          None
        }
      },
    }
  }

  pub fn insert_string(&self, string: &str) {
    let hashed_string = hash_str(string);
    let mut db = self.runtime.database.borrow_mut();
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
    store.strings.insert(hashed_string, string.to_string());
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(),Error> {
    for change in &txn.changes {
      match change {
        Change::NewTable{table_id, ..} => {
          let register = Register{table_id: TableId::Global(*table_id), row: TableIndex::All, column: TableIndex::All};
          self.runtime.output.insert(register);
          self.runtime.set_ready(&register);
        }
        Change::SetColumnAlias{table_id, column_ix, column_alias} => {
          let register = Register{table_id: TableId::Global(*table_id), row: TableIndex::All, column: TableIndex::Alias(*column_alias)};
          self.runtime.set_ready(&register);
        }
        _ => (),
      }
    }
    self.database.borrow_mut().process_transaction(txn)?;
    self.runtime.run_network()?;
    Ok(())
  }

  pub fn get_cell_in_table(&mut self, table: u64, row: &TableIndex, column: &TableIndex) -> Option<(Value,bool)> {
    match self.database.borrow().tables.get(&table) {
      Some(table_ref) => {
        table_ref.borrow().get(row, column)
      },
      None => None,
    }
  }

  pub fn register_blocks(&mut self, blocks: Vec<Block>) {
    self.runtime.register_blocks(blocks);
  }

  pub fn step(&mut self) {
    self.runtime.run_network().ok();
  }

  pub fn get_table(&self, table_id: u64) -> Option<Table> {
    match self.runtime.database.borrow().tables.get(&table_id) {
      Some(table) => Some(table.borrow().clone()),
      None => None,
    }
  }

  pub fn get_table_by_name(&self, table_name: &str) -> Option<Table> {
    let table_id = hash_str(table_name);
    self.get_table(table_id)
  }

  pub fn clear_table(&self, table_id: u64) {
    match self.runtime.database.borrow_mut().tables.get_mut(&table_id) {
      Some(table) => table.borrow_mut().clear(),
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
*/