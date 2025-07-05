use crate::*;
#[cfg(feature = "stdlib")]
use crate::function::{
  math::*,
  math_update::*,
  compare::*,
  stats::*,
  table::*,
  set::*,
  logic::*,
  matrix::*,
};
use crate::capabilities::*;

use hashbrown::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;

use rand::rngs::OsRng;
use rand::RngCore;
use ed25519_dalek::{self, SecretKey, SigningKey, Signature, Signer, Verifier, VerifyingKey};
use rand::Error;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use wasm_bindgen::JsValue;
#[cfg(feature = "wasm")]
use web_sys::{Crypto, Window,console};


/*
The Functions struct serves as a container for managing custom functions implemented as MechFunctionCompiler 
objects. Stored in a HashMap, these custom functions can be accessed, inserted, and extended during the execution 
of a Mech program. This enables the Mech runtime to integrate user-defined functionality, making it more versatile 
and adaptable to various use cases by allowing users to add their own functions tailored to their specific requirements.
*/

pub struct Functions{
  pub functions: HashMap<u64,Box<dyn MechFunctionCompiler>>,
}

impl Functions {
  fn new () -> Functions {
    Functions {
      functions: HashMap::new(),
    }
  }
  pub fn get(&self, key: u64) -> std::option::Option<&Box<dyn MechFunctionCompiler>> {
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


/*
The Core struct is the central component of the Mech programming language 
runtime, containing a variety of fields and methods for managing data 
structures used during program execution. It provides functionality for loading
and processing transactions, managing errors, interacting with tables, 
scheduling blocks for execution, and loading user-defined functions. 

Additionally, the Core struct supports dynamic loading of sections of a Mech 
program, making it useful for large and complex programs.
*/

pub struct Core {
  pub id: u64,
  pub verifying_key: VerifyingKey ,
  pub sections: Vec<HashMap<BlockId,BlockRef>>,
  pub blocks: HashMap<BlockId,BlockRef>,
  pub unsatisfied_blocks: HashMap<BlockId,BlockRef>,
  pub database: Rc<RefCell<Database>>,
  pub functions: Rc<RefCell<Functions>>,
  pub user_functions: Rc<RefCell<HashMap<u64,UserFunction>>>,
  pub required_functions: HashSet<u64>,
  pub errors: HashMap<MechErrorKind,Vec<BlockRef>>,
  pub full_errors: HashMap<MechError,Vec<BlockRef>>,
  pub input: HashSet<Register>,
  pub output: HashSet<Register>,
  pub defined_tables: HashSet<Register>,
  pub schedule: Schedule,
  pub dictionary: StringDictionary,
  pub capabilities: Vec<CapabilityToken>,
}

impl Core {

  pub fn new() -> Core {
    
    let mut functions = Functions::new();
    // -----------------
    // Standard Machines
    // -----------------
    let dictionary = Rc::new(RefCell::new(HashMap::new()));
    #[cfg(feature = "stdlib")]
    {
      let mut dict = dictionary.borrow_mut();
      // Math
      functions.insert(*MATH_ADD, Box::new(MathAdd{})); dict.insert(*MATH_ADD,MechString::from_str("math/add"));
      functions.insert(*MATH_SUBTRACT, Box::new(MathSub{})); dict.insert(*MATH_SUBTRACT,MechString::from_str("math/subtract"));
      functions.insert(*MATH_MULTIPLY, Box::new(MathMul{})); dict.insert(*MATH_MULTIPLY,MechString::from_str("math/multiply"));
      functions.insert(*MATH_DIVIDE, Box::new(MathDiv{})); dict.insert(*MATH_DIVIDE,MechString::from_str("math/divide"));
      functions.insert(*MATH_EXPONENT, Box::new(MathExp{})); dict.insert(*MATH_EXPONENT,MechString::from_str("math/exp"));
      functions.insert(*MATH_NEGATE, Box::new(MathNegate{})); dict.insert(*MATH_NEGATE,MechString::from_str("math/negate"));
      functions.insert(*MATH_ADD__UPDATE, Box::new(MathAddUpdate{})); dict.insert(*MATH_ADD__UPDATE,MechString::from_str("math/add-update"));
      functions.insert(*MATH_SUBTRACT__UPDATE, Box::new(MathSubtractUpdate{})); dict.insert(*MATH_SUBTRACT__UPDATE,MechString::from_str("math/subtract-update"));  
      functions.insert(*MATH_MULTIPLY__UPDATE, Box::new(MathMultiplyUpdate{})); dict.insert(*MATH_MULTIPLY__UPDATE,MechString::from_str("math/multiply-update"));
      functions.insert(*MATH_DIVIDE__UPDATE, Box::new(MathDivideUpdate{})); dict.insert(*MATH_DIVIDE__UPDATE,MechString::from_str("math/divide-update"));

      // Matrix
      functions.insert(*MATRIX_MULTIPLY, Box::new(MatrixMul{}));
      functions.insert(*MATRIX_TRANSPOSE, Box::new(MatrixTranspose{}));

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
      functions.insert(*TABLE_FLATTEN, Box::new(TableFlatten{}));
      functions.insert(*TABLE_DEFINE, Box::new(TableDefine{}));
      functions.insert(*TABLE_SET, Box::new(TableSet{}));
      functions.insert(*TABLE_HORIZONTAL__CONCATENATE, Box::new(TableHorizontalConcatenate{}));
      functions.insert(*TABLE_VERTICAL__CONCATENATE, Box::new(TableVerticalConcatenate{}));
      functions.insert(*TABLE_SIZE, Box::new(TableSize{}));
      functions.insert(*TABLE_FOLLOWED__BY, Box::new(TableFollowedBy{}));

      // Stats
      functions.insert(*STATS_SUM, Box::new(StatsSum{}));

      // Set
      functions.insert(*SET_ANY, Box::new(SetAny{}));
      functions.insert(*SET_ALL, Box::new(SetAll{}));
      functions.insert(*SET_CARTESIAN, Box::new(SetCartesian{}));
    }

    let core_id = generate_uuid();
    let name = format!("core-{:?}",core_id);
    let mut default_caps = HashSet::new();
    default_caps.insert(Capability::CoreDatabaseRead);
    default_caps.insert(Capability::CoreDatabaseWrite);
    let mut core_cap_token = CapabilityToken::new(name,default_caps,core_id,None);
    let keypair = generate_keypair();
    core_cap_token.sign(&keypair);
    let verifying_key = keypair.verifying_key();   
    
    Core {
      id: core_id,
      verifying_key,
      sections: Vec::new(),
      blocks: HashMap::new(),
      unsatisfied_blocks: HashMap::new(),
      database: Rc::new(RefCell::new(Database::new())),
      functions: Rc::new(RefCell::new(functions)),
      user_functions: Rc::new(RefCell::new(HashMap::new())),
      required_functions: HashSet::new(),
      errors: HashMap::new(),
      full_errors: HashMap::new(),
      schedule: Schedule::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      defined_tables: HashSet::new(),
      dictionary: dictionary,
      capabilities: vec![core_cap_token],
    }
  }

  pub fn get_name(&self, name_id: u64) -> Option<String> {
    match self.dictionary.borrow().get(&name_id) {
      Some(mech_string) => Some(mech_string.to_string()),
      None => None,
    }
  }

  pub fn load_function(&mut self, name: &str, mut fxn: Box<dyn MechFunctionCompiler>) -> Result<(),MechError> {
    let mut functions_brrw = self.functions.borrow_mut();
    functions_brrw.insert(hash_str(name),fxn);
    Ok(())
  }

  pub fn needed_registers(&self) -> HashSet<Register> {
    self.input.difference(&self.defined_tables).cloned().collect()
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(Vec<BlockRef>,HashSet<Register>),MechError> {
    let mut block_refs = vec![];
    let changed_registers = self.database.borrow_mut().process_transaction(txn)?;
    for (changed_table_id,_,_) in &changed_registers {
      let mut cured_block_refs = self.remove_error(*changed_table_id)?;
      block_refs.append(&mut cured_block_refs);
    }
    for register in &changed_registers {
      self.step(register);
    }
    Ok((block_refs,changed_registers))
  }

  pub fn has_pending_transactions(&mut self) -> bool {
    !(self.database.borrow().transaction_queue.is_empty())
  }

  pub fn process_transaction_queue(&mut self) -> Result<(Vec<BlockRef>,HashSet<Register>),MechError> {
    let mut block_refs = vec![];
    let changed_registers = self.database.borrow_mut().process_transaction_queue()?;
    for (changed_table_id,_,_) in &changed_registers {
      let mut cured_block_refs = self.remove_error(*changed_table_id)?;
      block_refs.append(&mut cured_block_refs);
    }
    for register in &changed_registers {
      self.step(register);
    }
    Ok((block_refs,changed_registers))
  }

  pub fn remove_error(&mut self, table_id: TableId) -> Result<Vec<BlockRef>,MechError> {
    let mut block_refs = vec![];
    match &self.errors.remove(&MechErrorKind::MissingTable(table_id)) {
      Some(ref ublocks) => {
        let mut mb = ublocks.clone();
        block_refs.append(&mut mb);
      }
      None => (),
    }
    match self.errors.remove(&MechErrorKind::PendingTable(table_id)) {
      Some(ref ublocks) => {
        let mut mb = ublocks.clone();
        block_refs.append(&mut mb);
      }
      None => (),
    }
    self.load_block_refs(block_refs.clone());
    self.schedule_blocks();
    let mut graph_output = vec![];
    match self.schedule.trigger_to_output.get(&(table_id,RegisterIndex::All,RegisterIndex::All)) {
      Some(output) => {
        for (table_id,_,_) in output {
          graph_output.push(table_id.clone());
        }
      }
      None => (),
    }
    for table_id in graph_output {
      self.remove_error(table_id);
    }
    Ok(block_refs)
  }

  pub fn insert_table(&mut self, table: Table) -> Result<TableRef,MechError> {
    self.database.borrow_mut().insert_table(table)
  }

  pub fn overwrite_tables(&mut self, tables: &Vec<Table>) -> Result<(),MechError> {
    let mut database_brrw = self.database.borrow_mut();
    for table in tables {
      let table2 = database_brrw.get_table_by_id(&table.id).unwrap();
    }
    Ok(())
  }

  pub fn get_table(&self, table_name: &str) -> Result<TableRef,MechError> {
    match self.database.borrow().get_table(table_name) {
      Some(table) => Ok(table.clone()),
      None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1004, kind: MechErrorKind::MissingTable(TableId::Global(hash_str(table_name)))});},
    }
  }

  pub fn get_table_by_id(&self, table_id: u64) -> Result<TableRef,MechError> {
    match self.database.borrow().get_table_by_id(&table_id) {
      Some(table) => Ok(table.clone()),
      None => {return Err(MechError{tokens: vec![], msg: "".to_string(), id: 1005, kind: MechErrorKind::MissingTable(TableId::Global(table_id))});},
    }
  }

  pub fn table_names(&self) -> Vec<String> {
    self.database.borrow().tables.iter().filter_map(|(_,t)| {
      t.borrow().name()
    }).collect::<Vec<String>>()
  }

  pub fn recompile_dynamic_tables(&mut self, register: Register) -> Result<(),MechError> {
    match self.schedule.schedules.get(&register) {
      Some(schedules) => {
        for schedule in schedules {
          schedule.recompile_blocks()?;
        }
      }
      None => (),
    }
    
    Ok(())
  }

  pub fn load_sections(&mut self, sections: Vec<Vec<SectionElement>>) -> Vec<((Vec<BlockId>,Vec<u64>,Vec<MechError>))> {
    let mut result = vec![];
    for elements in sections.iter() {
      let mut block_ids_agg = vec![];
      let mut fxn_ids = vec![];
      let mut errors_agg = vec![];
      for section_element in elements.iter() {
        match section_element {
          SectionElement::Block(block) => {
            let (mut block_ids, mut errors) = self.load_blocks(&vec![block.clone()]);
            block_ids_agg.append(&mut block_ids);
            errors_agg.append(&mut errors);
          }
          SectionElement::UserFunction(fxn) => {
            self.load_user_function(fxn);
          }
        }
      }
      result.push((block_ids_agg,fxn_ids,errors_agg));
    }
    result
  }

  pub fn load_user_function(&mut self, user_function: &UserFunction) -> Result<(),MechError> {

    let mut usr_fxns_brrw = self.user_functions.borrow_mut();

    usr_fxns_brrw.insert(user_function.name,user_function.clone());

    Ok(())
  }

  pub fn load_block_refs(&mut self, mut blocks: Vec<BlockRef>) -> (Vec<BlockId>,Vec<MechError>) {
    let mut block_ids = vec![];
    let mut block_errors = vec![];
    for block in blocks {
      let (mut new_block_ids, mut new_block_errors, mut new_block_output) = self.load_block(block.clone());
      block_ids.append(&mut new_block_ids);
      block_errors.append(&mut new_block_errors);
      for register in new_block_output.iter() {
        self.step(register);
      }
      self.schedule_blocks();
      //self.recompile_dynamic_tables();
    }
    (block_ids,block_errors)
  }

  pub fn load_blocks(&mut self, mut blocks: &Vec<Block>) -> (Vec<BlockId>,Vec<MechError>) {
    let mut block_ids = vec![];
    let mut block_errors = vec![];
    for block in blocks {
      let (mut new_block_ids, mut new_block_errors, mut new_block_output) = self.load_block(Rc::new(RefCell::new(block.clone())));
      block_ids.append(&mut new_block_ids);
      block_errors.append(&mut new_block_errors);
      for register in new_block_output.iter() {
        self.step(register);
      }
      self.schedule_blocks();
      //self.recompile_dynamic_tables();
    }
    (block_ids,block_errors)
  }

  pub fn load_block(&mut self, mut block_ref: BlockRef) -> (Vec<BlockId>,Vec<MechError>,HashSet<Register>) {
    let block_ref_c = block_ref.clone();
    let mut new_block_ids = vec![];
    let mut new_block_errors = vec![];
    let mut new_block_output = HashSet::new();
    {
      let mut block_brrw = block_ref.borrow_mut();
      let temp_db = block_brrw.global_database.clone();
      block_brrw.global_database = self.database.clone();
      block_brrw.functions = Some(self.functions.clone());
      block_brrw.user_functions = Some(self.user_functions.clone());
      // Merge databases
      {
        let mut temp_db_brrw = temp_db.borrow();
        match self.database.try_borrow_mut() {
          Ok(ref mut database_brrw) => {
            database_brrw.union(&mut temp_db_brrw);
          }
          Err(_) => ()
        }
      }
      // Merge dictionaries
      for (k,v) in block_brrw.strings.borrow().iter() {
        self.dictionary.borrow_mut().insert(*k,v.clone());
      }
      // Merge dictionaries
      for fxn_id in block_brrw.required_functions.iter() {
        self.required_functions.insert(*fxn_id);
      }
      // try to satisfy the block
      match block_brrw.ready() {
        Ok(()) => {
          
          let id = block_brrw.gen_id();

          // Merge input and output
          self.input = self.input.union(&mut block_brrw.input).cloned().collect();
          self.output = self.output.union(&mut block_brrw.output).cloned().collect();
          self.defined_tables = self.defined_tables.union(&mut block_brrw.defined_tables).cloned().collect();
          {
            let mut database_brrw = self.database.borrow_mut();
            database_brrw.dynamic_tables = database_brrw.dynamic_tables.union(&mut block_brrw.dynamic_tables).cloned().collect();
          }
          self.schedule.add_block(block_ref.clone());
          self.blocks.insert(id,block_ref_c.clone());

          // Try to satisfy other blocks
          let mut block_output = block_brrw.output.clone();
          new_block_output = new_block_output.union(&mut block_output).cloned().collect();
          let mut resolved_tables: Vec<MechErrorKind> = block_output.iter().map(|(table_id,_,_)| MechErrorKind::MissingTable(*table_id)).collect();
          let mut resolved_pending_tables: Vec<MechErrorKind> = block_output.iter().map(|(table_id,_,_)| MechErrorKind::PendingTable(*table_id)).collect();
          resolved_tables.append(&mut resolved_pending_tables);
          new_block_ids.push(id);
          let (mut newly_resolved_block_ids, mut new_errors, mut new_output) = self.resolve_errors(&resolved_tables);
          new_block_output = new_block_output.union(&mut new_output).cloned().collect();
          new_block_ids.append(&mut newly_resolved_block_ids);
          new_block_errors.append(&mut new_errors);
        }
        Err(x) => {
          // Merge input and output
          self.input = self.input.union(&mut block_brrw.input).cloned().collect();
          self.output = self.output.union(&mut block_brrw.output).cloned().collect();        
          let (mech_error,_) = block_brrw.unsatisfied_transformation.as_ref().unwrap();
          let blocks_with_errors = self.full_errors.entry(mech_error.clone()).or_insert(Vec::new());
          blocks_with_errors.push(block_ref_c.clone());
          let blocks_with_errors = self.errors.entry(mech_error.kind.clone()).or_insert(Vec::new());
          blocks_with_errors.push(block_ref_c.clone());
          self.unsatisfied_blocks.insert(0,block_ref_c.clone());
          let error = MechError{tokens: vec![], msg: "".to_string(), id: 1006, kind: MechErrorKind::GenericError(format!("{:?}", x))};
          new_block_errors.push(error);
        },
      };
    }
    self.unsatisfied_blocks.extract_if(|k,v| { 
      let state = {
        match v.try_borrow() {
          Ok(brrw) => brrw.state.clone(),
          Err(_) => BlockState::Pending,
        }
      };
      state == BlockState::Ready
    });

    (new_block_ids,new_block_errors,new_block_output)
  }
  
  pub fn resolve_errors(&mut self, resolved_errors: &Vec<MechErrorKind>) -> (Vec<u64>,Vec<MechError>,HashSet<Register>) {
    let mut new_block_ids =  vec![];
    let mut new_block_errors =  vec![];
    let mut new_block_output = HashSet::new();
    for error in resolved_errors.iter() {
      match self.errors.remove(error) {
        Some(mut ublocks) => {
          for ublock in ublocks {
            let (mut nbids,mut nberrs, mut nboutput) = self.load_block(ublock);
            {
              new_block_ids.append(&mut nbids);
              self.unsatisfied_blocks = self.unsatisfied_blocks.extract_if  (|k,v| {
                let state = {
                  match v.try_borrow() {
                    Ok(brrw) => brrw.state.clone(),
                    Err(_) => BlockState::Pending,
                  }
                };
                state != BlockState::Ready
              }).collect();
              // For each of the new blocks, check to see if any of the tables
              // it provides are pending.
              let mut new_block_pending_ids = vec![];
              for id in &new_block_ids {
                let mut output = {
                  let block_ref = self.blocks.get(id).unwrap();
                  let block_ref_brrw = block_ref.borrow();
                  block_ref_brrw.output.clone()
                };
                new_block_output = new_block_output.union(&mut output).cloned().collect();
                for (table_id,_,_) in &output {
                  let (mut resolved, mut errs, mut output) = self.resolve_errors(&vec![MechErrorKind::PendingTable(*table_id)]);
                  new_block_pending_ids.append(&mut resolved);
                  new_block_errors.append(&mut errs);
                  new_block_output = new_block_output.union(&mut output).cloned().collect();
                }
              }
              new_block_ids.append(&mut new_block_pending_ids);
            }
            new_block_errors.append(&mut nberrs)
          }
        }
        None => (),
      }
    }
    (new_block_ids,new_block_errors,new_block_output)
  }

  pub fn get_output_by_block_id(&self, block_id: BlockId) -> Result<HashSet<Register>,MechError> {
    match self.blocks.get(&block_id) {
      Some(block_ref) => {
        let output = block_ref.borrow().output.clone();
        Ok(output)
      }
      None => Err(MechError{tokens: vec![], msg: "".to_string(), id: 1008, kind: MechErrorKind::MissingBlock(block_id)}),
    }
  }

  pub fn schedule_blocks(&mut self) -> Result<(),MechError> {
    self.schedule.schedule_blocks()
  }

  pub fn step(&mut self, register: &Register) -> Result<(),MechError> {
    self.schedule.run_schedule(register)
  }

}

impl fmt::Debug for Core {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let dictionary = self.dictionary.clone();
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_title("🤖",&format!("CORE: {:?}", humanize(&self.id)));
    if self.errors.len() > 0 {
      box_drawing.add_title("🐛","errors");
      for (error,blocks) in self.errors.iter() {
        box_drawing.add_line(format!("  - {:?}", error));
      }
    }
    box_drawing.add_title("📭","input");
    for (table,row,col) in &self.input {
      let table = match dictionary.borrow().get(table.unwrap()) {
        Some(x) => x.to_string(),
        None => format!("{:?}", table),
      };
      box_drawing.add_line(format!("  - ({:?}, {:?}, {:?})", table,row,col));
    }
    box_drawing.add_title("📬","output");
    for (table,row,col) in &self.output {
      let table = match dictionary.borrow().get(table.unwrap()) {
        Some(x) => x.to_string(),
        None => format!("{:?}", table),
      };
      box_drawing.add_line(format!("  - ({:?}, {:?}, {:?})", table,row,col));
    }
    box_drawing.add_title("🧊","blocks");
    box_drawing.add_line(format!("{:#?}", &self.blocks.iter().map(|(k,v)|humanize(&k)).collect::<Vec<String>>()));
    if self.unsatisfied_blocks.len() > 0 {
      box_drawing.add_title("😔","unsatisfied blocks");
      box_drawing.add_line(format!("{:#?}", &self.unsatisfied_blocks));    
    }
    box_drawing.add_title("💻","functions");
    box_drawing.add_line("Compiled Functions".to_string());
    box_drawing.add_line(format!("{:#?}", &self.functions.borrow().functions.iter().map(|(k,v)|
    {
      match dictionary.borrow().get(k) {
        Some(x) => x.to_string(),
        None => humanize(&k),
      }
    }).collect::<Vec<String>>()));
    box_drawing.add_line("User Functions".to_string());
    box_drawing.add_line(format!("{:#?}", &self.user_functions.borrow().iter().map(|(k,v)|
    {
      match dictionary.borrow().get(k) {
        Some(x) => x.to_string(),
        None => humanize(&k),
      }
    }).collect::<Vec<String>>()));
    box_drawing.add_title("🦾","capabilities");
    box_drawing.add_line(format!("{:#?}", &self.capabilities));
    box_drawing.add_title("🗓️","schedule");
    box_drawing.add_line(format!("{:#?}", &self.schedule));
    box_drawing.add_title("💾","database");
    box_drawing.add_line(format!("{:#?}", &self.database.borrow()));
    write!(f,"{:?}",box_drawing)?;
    Ok(())
  }
}

