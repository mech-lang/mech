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
  pub fn insert(&mut self, key: u64, mut fxn: Box<dyn MechFunctionCompiler>) {
    self.functions.insert(key,fxn);
  }

  pub fn extend(&mut self, other: HashMap<u64,Box<dyn MechFunctionCompiler>>) {
    self.functions.extend(other); 
  }
  
}

impl fmt::Debug for Functions {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f,"Functions...")?;
    Ok(())
  }
}



pub struct Core {
  pub blocks: HashMap<BlockId,BlockRef>,
  unsatisfied_blocks: Vec<BlockRef>,
  database: Rc<RefCell<Database>>,
  pub functions: Rc<RefCell<Functions>>,
  pub errors: HashMap<MechErrorKind,Vec<BlockRef>>,
  pub input: HashSet<(TableId,TableIndex,TableIndex)>,
  pub output: HashSet<(TableId,TableIndex,TableIndex)>,
  pub schedule: Schedule,
  pub dictionary: StringDictionary,
}

impl Core {

  pub fn new() -> Core {
    
    let mut functions = Functions::new();
    // -----------------
    // Standard Machines
    // -----------------

    // Math
    functions.insert(*MATH_ADD, Box::new(MathAdd{}));
    functions.insert(*MATH_SUBTRACT, Box::new(MathSub{}));
    functions.insert(*MATH_MULTIPLY, Box::new(MathMul{}));
    functions.insert(*MATH_DIVIDE, Box::new(MathDiv{}));
    //functions.insert(*MATH_EXPONENT, MathExp{});
    functions.insert(*MATH_NEGATE, Box::new(MathNegate{}));

    // Logic
    functions.insert(*LOGIC_NOT, Box::new(LogicNot{}));
    functions.insert(*LOGIC_AND, Box::new(LogicAnd{}));
    functions.insert(*LOGIC_OR, Box::new(LoigicOr{}));
    functions.insert(*LOGIC_XOR, Box::new(LogicXor{}));

    // Compare
    functions.insert(*COMPARE_GREATER__THAN, Box::new(CompareGreater{}));
    functions.insert(*COMPARE_LESS__THAN, Box::new(CompareLess{}));
    functions.insert(*COMPARE_GREATER__THAN__EQUAL, Box::new(CompareGreaterEqual{}));
    functions.insert(*COMPARE_LESS__THAN__EQUAL, Box::new(CompareLessEqual{}));
    functions.insert(*COMPARE_EQUAL, Box::new(CompareEqual{}));
    functions.insert(*COMPARE_NOT__EQUAL, Box::new(CompareNotEqual{}));

    // Table
    functions.insert(*TABLE_APPEND, Box::new(TableAppend{}));
    functions.insert(*TABLE_RANGE, Box::new(TableRange{}));
    functions.insert(*TABLE_SPLIT, Box::new(TableSplit{}));
    functions.insert(*TABLE_DEFINE, Box::new(TableDefine{}));
    functions.insert(*TABLE_SET, Box::new(TableSet{}));
    functions.insert(*TABLE_HORIZONTAL__CONCATENATE, Box::new(TableHorizontalConcatenate{}));
    functions.insert(*TABLE_VERTICAL__CONCATENATE, Box::new(TableVerticalConcatenate{}));
    functions.insert(*TABLE_SIZE, Box::new(TableSize{}));
    
    // Stats
    functions.insert(*STATS_SUM, Box::new(StatsSum{}));

    // Set
    functions.insert(*SET_ANY, Box::new(SetAny{}));
    functions.insert(*SET_ALL, Box::new(SetAll{}));
     
    Core {
      blocks: HashMap::new(),
      unsatisfied_blocks: Vec::new(),
      database: Rc::new(RefCell::new(Database::new())),
      functions: Rc::new(RefCell::new(functions)),
      errors: HashMap::new(),
      schedule: Schedule::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      dictionary: Rc::new(RefCell::new(HashMap::new())),
    }
  }

  pub fn needed_registers(&self) -> HashSet<(TableId,TableIndex,TableIndex)> {
    self.input.difference(&self.output).cloned().collect()
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<Vec<BlockRef>,MechError> {
    let mut changed_registers = HashSet::new();
    let mut block_refs = Vec::new();
    for change in txn {
      match change {
        Change::Set((table_id, adds)) => {
          match self.database.borrow().get_table_by_id(table_id) {
            Some(table) => {
              let table_brrw = table.borrow();
              for (row,col,val) in adds {
                match table_brrw.set(row, col, val.clone()) {
                  Ok(()) => {
                    // TODO This is inserting a {:,:} register instead of the one passed in, and that needs to be fixed.
                    changed_registers.insert((TableId::Global(*table_id),TableIndex::All,TableIndex::All));
                  },
                  Err(x) => { return Err(MechError{id: 1000, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
                }
              }
            }
            None => {return Err(MechError{id: 1001, kind: MechErrorKind::MissingTable(TableId::Global(*table_id))});},
          }
        }
        Change::NewTable{table_id, rows, columns} => {
          let table = Table::new(*table_id,*rows,*columns);
          self.database.borrow_mut().insert_table(table)?;
        }
        Change::ColumnAlias{table_id, column_ix, column_alias} => {
          match self.database.borrow_mut().get_table_by_id(table_id) {
            Some(table) => {
              let mut table_brrw = table.borrow_mut();   
              let rows = table_brrw.rows;
              if *column_ix + 1 > table_brrw.cols {
                table_brrw.resize(rows, column_ix + 1);
              }    
              table_brrw.set_col_alias(*column_ix,*column_alias);     
            }
            x => {return Err(MechError{id: 1002, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
        Change::ColumnKind{table_id, column_ix, column_kind} => {
          match self.database.borrow_mut().get_table_by_id(table_id) {
            Some(table) => {
              let mut table_brrw = table.borrow_mut();   
              let rows = table_brrw.rows;
              if *column_ix + 1 > table_brrw.cols {
                table_brrw.resize(rows, column_ix + 1);
              }    
              table_brrw.set_col_kind(*column_ix,column_kind.clone());     
            }
            x => {return Err(MechError{id: 1003, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
      }
    }
    for (changed_table_id,_,_) in &changed_registers {
      match self.errors.remove(&MechErrorKind::MissingTable(*changed_table_id)) {
        Some(mut ublocks) => {
          block_refs.append(&mut ublocks);
        }
        None => (),
      }
    }
    self.load_block_refs(block_refs.clone());
    self.schedule_blocks();
    for register in &changed_registers {
      self.step(register);
    }
    Ok(block_refs)
  }

  pub fn insert_table(&mut self, table: Table) -> Result<Rc<RefCell<Table>>,MechError> {
    self.database.borrow_mut().insert_table(table)
  }

  pub fn get_table(&mut self, table_name: &str) -> Result<Rc<RefCell<Table>>,MechError> {
    match self.database.borrow().get_table(table_name) {
      Some(table) => Ok(table.clone()),
      None => {return Err(MechError{id: 1004, kind: MechErrorKind::MissingTable(TableId::Global(hash_str(table_name)))});},
    }
  }

  pub fn get_table_by_id(&mut self, table_id: u64) -> Result<Rc<RefCell<Table>>,MechError> {
    match self.database.borrow().get_table_by_id(&table_id) {
      Some(table) => Ok(table.clone()),
      None => {return Err(MechError{id: 1005, kind: MechErrorKind::MissingTable(TableId::Global(table_id))});},
    }
  }

  pub fn load_block_refs(&mut self, mut blocks: Vec<BlockRef>) -> Result<Vec<BlockId>,MechError> {
    let mut block_ids = vec![];
    for block in blocks {
      let mut new_block_ids = self.load_block(block.clone())?;
      block_ids.append(&mut new_block_ids);
    }
    Ok(block_ids)
  }

  pub fn load_blocks(&mut self, mut blocks: Vec<Block>) -> (Vec<BlockId>,Vec<MechError>) {
    let mut block_ids = vec![];
    let mut block_errors = vec![];
    for block in blocks {
      match self.load_block(Rc::new(RefCell::new(block.clone()))) {
        Ok(mut new_block_id) => block_ids.append(&mut new_block_id),
        Err(x) => block_errors.push(x),
      }
    }
    (block_ids,block_errors)
  }

  pub fn load_block(&mut self, mut block_ref: BlockRef) -> Result<Vec<BlockId>,MechError> {
    let block_ref_c = block_ref.clone();
    let result;
    {
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
      // Merge dictionaries
      for (k,v) in block_brrw.strings.borrow().iter() {
        self.dictionary.borrow_mut().insert(*k,v.clone());
      }
      // try to satisfy the block
      result = match block_brrw.ready() {
        Ok(()) => {
          
          let id = block_brrw.gen_id();

          // Merge input and output
          self.input = self.input.union(&mut block_brrw.input).cloned().collect();
          self.output = self.output.union(&mut block_brrw.output).cloned().collect();

          self.schedule.add_block(block_ref.clone());
          self.blocks.insert(id,block_ref_c.clone());

          // Try to satisfy other blocks
          let block_output = block_brrw.output.clone();
          let resolved_tables: Vec<MechErrorKind> = block_output.iter().map(|(table_id,_,_)| MechErrorKind::MissingTable(*table_id)).collect();
          let mut newly_resolved_block_ids = self.resolve_errors(&resolved_tables)?;

          let mut result = vec![id];
          result.append(&mut newly_resolved_block_ids);
          Ok(result)
        }
        Err(x) => {
          // Merge input and output
          self.input = self.input.union(&mut block_brrw.input).cloned().collect();
          self.output = self.output.union(&mut block_brrw.output).cloned().collect();        
          let (mech_error,_) = block_brrw.unsatisfied_transformation.as_ref().unwrap();
          let blocks_with_errors = self.errors.entry(mech_error.kind.clone()).or_insert(Vec::new());
          blocks_with_errors.push(block_ref_c.clone());
          self.unsatisfied_blocks.push(block_ref_c.clone());
          Err(MechError{id: 1006, kind: MechErrorKind::GenericError(format!("{:?}", x))})
        },
      };
    }
    self.unsatisfied_blocks.drain_filter(|b| b.borrow().state == BlockState::Ready);
    result
  }
  
  pub fn resolve_errors(&mut self, resolved_errors: &Vec<MechErrorKind>) -> Result<Vec<u64>,MechError> {
    let mut new_block_ids =  vec![];
    for error in resolved_errors.iter() {
      match self.errors.remove(error) {
        Some(mut ublocks) => {
          for ublock in ublocks {
            match self.load_block(ublock) {
              Ok(mut nbid) => {
                new_block_ids.append(&mut nbid);
                self.unsatisfied_blocks = self.unsatisfied_blocks.iter().filter(|x| {
                  x.borrow().state != BlockState::Ready
                }).cloned().collect();
                // For each of the new blocks, check to see if any of the tables
                // it provides are pending.
                let mut new_block_pending_ids = vec![];
                for id in &new_block_ids {
                  let output = {
                    let block_ref = self.blocks.get(&id).unwrap();
                    let block_ref_brrw = block_ref.borrow();
                    block_ref_brrw.output.clone()
                  };
                  for (table_id,_,_) in &output {
                    let mut resolved = self.resolve_errors(&vec![MechErrorKind::PendingTable(*table_id)])?;
                    new_block_pending_ids.append(&mut resolved);
                  }
                }
                new_block_ids.append(&mut new_block_pending_ids);
              },
              err => {return err;}
            }
          }
        }
        None => (),
      }
    }
    Ok(new_block_ids)
  }

  
  pub fn get_output_by_block_id(&self, block_id: BlockId) -> Result<HashSet<(TableId,TableIndex,TableIndex)>,MechError> {
    match self.blocks.get(&block_id) {
      Some(block_ref) => {
        let output = block_ref.borrow().output.clone();
        Ok(output)
      }
      None => Err(MechError{id: 1008, kind: MechErrorKind::MissingBlock(block_id)}),
    }
  }


  pub fn schedule_blocks(&mut self) -> Result<(),MechError> {
    self.schedule.schedule_blocks()
  }

  pub fn step(&mut self, register: &(TableId,TableIndex,TableIndex)) -> Result<(),MechError> {
    self.schedule.run_schedule(register)
  }
}

impl fmt::Debug for Core {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_title("🤖","CORE");
    if self.errors.len() > 0 {
      box_drawing.add_title("🐛","errors");
      box_drawing.add_line(format!("{:#?}", &self.errors));
    }
    box_drawing.add_title("📭","input");
    for (table,row,col) in &self.input {
      box_drawing.add_line(format!("  - ({:?}, {:?}, {:?})", table,row,col));
    }
    box_drawing.add_title("📬","output");
    for (table,row,col) in &self.output {
      box_drawing.add_line(format!("  - ({:?}, {:?}, {:?})", table,row,col));
    }
    box_drawing.add_title("🧊","blocks");
    box_drawing.add_line(format!("{:#?}", &self.blocks.iter().map(|(k,v)|humanize(&k)).collect::<Vec<String>>()));
    if self.unsatisfied_blocks.len() > 0 {
      box_drawing.add_title("😔","unsatisfied blocks");
      box_drawing.add_line(format!("{:#?}", &self.unsatisfied_blocks));    
    }
    box_drawing.add_title("💻","functions");
    box_drawing.add_line(format!("{:#?}", &self.functions.borrow().functions.iter().map(|(k,v)|humanize(&k)).collect::<Vec<String>>()));
    box_drawing.add_title("🗓️","schedule");
    box_drawing.add_line(format!("{:#?}", &self.schedule));
    box_drawing.add_title("💾","database");
    box_drawing.add_line(format!("{:#?}", &self.database.borrow()));
    write!(f,"{:?}",box_drawing)?;
    Ok(())
  }
}
