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
  Function,
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

lazy_static! {
  static ref COLUMN: u64 = hash_str("column");
  static ref ROW: u64 = hash_str("row");
  static ref TABLE: u64 = hash_str("table");
  static ref STATS_SUM: u64 = hash_str("stats/sum");
  static ref MATH_ADD: u64 = hash_str("math/add");
  static ref MATH_DIVIDE: u64 = hash_str("math/divide");
  static ref MATH_MULTIPLY: u64 = hash_str("math/multiply");
  static ref MATH_SUBTRACT: u64 = hash_str("math/subtract");
  static ref MATH_EXPONENT: u64 = hash_str("math/exponent");
  static ref MATH_NEGATE: u64 = hash_str("math/negate");
  static ref TABLE_RANGE: u64 = hash_str("table/range");
  static ref TABLE_SPLIT: u64 = hash_str("table/split");
  static ref TABLE_HORIZONTAL__CONCATENATE: u64 = hash_str("table/horizontal-concatenate");
  static ref TABLE_VERTICAL__CONCATENATE: u64 = hash_str("table/vertical-concatenate");
  static ref TABLE_APPEND: u64 = hash_str("table/append");
  static ref LOGIC_AND: u64 = hash_str("logic/and");  
  static ref LOGIC_OR: u64 = hash_str("logic/or");
  static ref LOGIC_NOT: u64 = hash_str("logic/not");  
  static ref LOGIC_XOR: u64 = hash_str("logic/xor");    
  static ref COMPARE_GREATER__THAN: u64 = hash_str("compare/greater-than");
  static ref COMPARE_LESS__THAN: u64 = hash_str("compare/less-than");
  static ref COMPARE_GREATER__THAN__EQUAL: u64 = hash_str("compare/greater-than-equal");
  static ref COMPARE_LESS__THAN__EQUAL: u64 = hash_str("compare/less-than-equal");
  static ref COMPARE_EQUAL: u64 = hash_str("compare/equal");
  static ref COMPARE_NOT__EQUAL: u64 = hash_str("compare/not-equal");
  static ref SET_ANY: u64 = hash_str("set/any");
  static ref SET_ALL: u64 = hash_str("set/all");  
}

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

/*
if *name == *MATH_ADD { math_add(self,&arguments,&out)?; }
        else if *name == *MATH_SUBTRACT { math_sub(self,&arguments,&out)?; } 
        else if *name == *MATH_MULTIPLY { math_mul(self,&arguments,&out)?; } 
        else if *name == *MATH_DIVIDE { math_div(self,&arguments,&out)?; } 
        else if *name == *MATH_EXPONENT { math_exp(self,&arguments,&out)?; } 
        else if *name == *MATH_NEGATE { math_negate(self,&arguments,&out)?; } 
        else if *name == *LOGIC_NOT { logic_not(self,&arguments,&out)?; } 
        else if *name == *LOGIC_AND { logic_and(self,&arguments,&out)?; } 
        else if *name == *LOGIC_OR { logic_or(self,&arguments,&out)?; } 
        else if *name == *LOGIC_XOR { logic_xor(self,&arguments,&out)?; } 
        else if *name == *COMPARE_EQUAL { compare_equal(self,&arguments,&out)?; } 
        else if *name == *COMPARE_NOT__EQUAL { compare_not__equal(self,&arguments,&out)?; } 
        else if *name == *COMPARE_LESS__THAN { compare_less__than(self,&arguments,&out)?; } 
        else if *name == *COMPARE_LESS__THAN__EQUAL { compare_less__than__equal(self,&arguments,&out)?; } 
        else if *name == *COMPARE_GREATER__THAN { compare_greater__than(self,&arguments,&out)?; } 
        else if *name == *COMPARE_GREATER__THAN__EQUAL { compare_greater__than__equal(self,&arguments,&out)?; } 
        else if *name == *STATS_SUM { stats_sum(self,&arguments,&out)?; } 
        else if *name == *SET_ANY { set_any(self,&arguments,&out)?; } 
        else if *name == *SET_ALL { set_all(self,&arguments,&out)?; } 
        else if *name == *TABLE_SPLIT { table_split(self,&arguments,&out)?;}
        else if *name == *TABLE_VERTICAL__CONCATENATE { table_vertical__concatenate(self,&arguments,&out)?; } 
        else if *name == *TABLE_HORIZONTAL__CONCATENATE { table_horizontal__concatenate(self,&arguments,&out)?; }       
        else if *name == *TABLE_APPEND { table_append(self,&arguments,&out)?; } 
        else if *name == *TABLE_RANGE { table_range(self,&arguments,&out)?; }
        */

impl Core {

  pub fn new() -> Core {

    let mut functions = Functions::new();
    functions.insert(*MATH_ADD, math_add{});
    functions.insert(*MATH_SUBTRACT, math_sub{});
    functions.insert(*MATH_MULTIPLY, math_mul{});
    functions.insert(*MATH_DIVIDE, math_div{});
    functions.insert(*MATH_EXPONENT, math_exp{});

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