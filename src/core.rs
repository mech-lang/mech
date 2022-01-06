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
  pub schedules: HashMap<(u64,usize,usize),Vec<Vec<usize>>>,
}

impl Core {

  pub fn new() -> Core {
    Core {
      blocks: Vec::new(),
      unsatisfied_blocks: Vec::new(),
      database: Rc::new(RefCell::new(Database::new())),
      errors: HashMap::new(),
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
                  Ok(()) => {
                    registers.push((*table_id,*row,*col));
                  },
                  Err(x) => {return Err(x);}
                }
              }
            }
            None => {return Err(MechError::GenericError(4219));}
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
    let temp_db = block_brrw.global_database.clone();
    block_brrw.global_database = self.database.clone();

    // Merge databases
    let mut temp_db_brrw = temp_db.borrow();
    match self.database.try_borrow_mut() {
      Ok(ref mut database_brrw) => {
        database_brrw.union(&mut temp_db_brrw);
      }
      Err(_) => ()
    }
    // try to satisfy the block
    match block_brrw.ready() {
      true => {
        block_brrw.gen_id();
        let block_output = block_brrw.output.clone();
        self.blocks.push(block_ref_c.clone());
        for table_id in block_output {
          match self.errors.remove(&MechError::MissingTable(table_id)) {
            Some(mut ublocks) => {
              for ublock in ublocks {
                self.insert_block(ublock);
              }
            }
            None => (),
          }
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