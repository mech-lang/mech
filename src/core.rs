// ## Core

// Cores are the smallest unit of a mech program exposed to a user. They hold references to all the
// subparts of Mech, including the database (defines the what) and the runtime (defines the how).
// The core accepts transactions and applies those to the database. Updated tables in the database
// trigger computation in the runtime, which can further update the database. Execution terminates
// when a steady state is reached, or an iteration limit is reached (whichever comes first). The
// core then waits for further transactions.

use crate::*;
use crate::function::{
  MechFunction,
  math::*,
  compare::*,
  stats::*,
  table::*,
  set::*,
  logic::*,
};

use hashbrown::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;

pub type BlockRef = Rc<RefCell<Block>>;

pub struct Functions{
  pub functions: HashMap<u64,Box<dyn MechFunctionCompiler>>,
}

impl Functions {
  fn new () -> Functions {
    Functions {
      functions: HashMap::new(),
    }
  }
  pub fn get(&mut self, key: u64) -> std::option::Option<&Box<dyn function::MechFunctionCompiler>> {
    self.functions.get(&key)
  }
  pub fn insert<S: MechFunctionCompiler + 'static>(&mut self, key: u64, mut fxn: S) {
    self.functions.insert(key,Box::new(fxn));
  }
  
}

#[derive(Clone)]
pub struct Core {
  pub blocks: HashMap<BlockId,BlockRef>,
  unsatisfied_blocks: Vec<BlockRef>,
  database: Rc<RefCell<Database>>,
  functions: Rc<RefCell<Functions>>,
  pub errors: HashMap<MechError,Vec<BlockRef>>,
  pub schedules: HashMap<(u64,usize,usize),Vec<Vec<usize>>>,
}

impl Core {

  pub fn new() -> Core {

    let mut functions = Functions::new();
    functions.insert(*MATH_ADD, MathAdd{});
    functions.insert(*MATH_SUBTRACT, MathSub{});
    functions.insert(*MATH_MULTIPLY, MathMul{});
    functions.insert(*MATH_DIVIDE, MathDiv{});
    //functions.insert(*MATH_EXPONENT, MathExp{});
    functions.insert(*MATH_NEGATE, MathNegate{});

    functions.insert(*LOGIC_NOT, LogicNot{});
    functions.insert(*LOGIC_AND, logic_and{});
    functions.insert(*LOGIC_OR, logic_or{});
    functions.insert(*LOGIC_XOR, logic_xor{});

    functions.insert(*COMPARE_GREATER__THAN, compare_greater__than{});
    functions.insert(*COMPARE_LESS__THAN, compare_less__than{});
    functions.insert(*COMPARE_GREATER__THAN__EQUAL, compare_greater__than__equal{});
    functions.insert(*COMPARE_LESS__THAN__EQUAL, compare_less__than__equal{});
    functions.insert(*COMPARE_EQUAL, compare_equal{});
    functions.insert(*COMPARE_NOT__EQUAL, compare_not__equal{});

    functions.insert(*TABLE_APPEND, TableAppend{});
    functions.insert(*TABLE_RANGE, TableRange{});
    functions.insert(*TABLE_SPLIT, TableSplit{});
    functions.insert(*TABLE_HORIZONTAL__CONCATENATE, TableHorizontalConcatenate{});
    functions.insert(*TABLE_VERTICAL__CONCATENATE, TableVerticalConcatenate{});

    functions.insert(*STATS_SUM, StatsSum{});

    functions.insert(*SET_ANY, SetAny{});
    functions.insert(*SET_ALL, SetAll{});

    Core {
      blocks: HashMap::new(),
      unsatisfied_blocks: Vec::new(),
      database: Rc::new(RefCell::new(Database::new())),
      functions: Rc::new(RefCell::new(functions)),
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

  pub fn get_table(&mut self, table_name: &str) -> Result<Rc<RefCell<Table>>,MechError> {
    match self.database.borrow().get_table(table_name) {
      Some(table) => Ok(table.clone()),
      None => Err(MechError::GenericError(2951)),
    }
  }

  pub fn get_table_by_id(&mut self, table_id: u64) -> Result<Rc<RefCell<Table>>,MechError> {
    match self.database.borrow().get_table_by_id(&table_id) {
      Some(table) => Ok(table.clone()),
      None => Err(MechError::GenericError(2952)),
    }
  }

  pub fn insert_blocks(&mut self, mut blocks: Vec<Block>) -> Result<Vec<BlockId>,MechError> {
    let mut block_ids = vec![];
    for block in blocks {
      let block_id = self.insert_block(Rc::new(RefCell::new(block.clone())))?;
      block_ids.push(block_id);
    }
    Ok(block_ids)
  }

  pub fn insert_block(&mut self, mut block_ref: BlockRef) -> Result<BlockId,MechError> {
    let block_ref_c = block_ref.clone();
    let mut block_brrw = block_ref.borrow_mut();
    let temp_db = block_brrw.global_database.clone();
    block_brrw.global_database = self.database.clone();
    block_brrw.functions = Some(self.functions.clone());

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
        let id = block_brrw.gen_id();
        let block_output = block_brrw.output.clone();
        self.blocks.insert(id,block_ref_c.clone());
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
        Ok(id)
      }
      false => {
        let (mech_error,_) = block_brrw.unsatisfied_transformation.as_ref().unwrap();
        println!("{:?}", mech_error);
        let blocks_with_errors = self.errors.entry(mech_error.clone()).or_insert(Vec::new());
        blocks_with_errors.push(block_ref_c.clone());
        Err(MechError::GenericError(8963))
      },
    }
  }

  pub fn step(&mut self, register: &(u64,usize,usize)) {
    /*
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
    */
  }
}

impl fmt::Debug for Core {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f,"{:?}",self.blocks)?;
    Ok(())
  }
}