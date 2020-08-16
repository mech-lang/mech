use block::{Block, BlockState, Register, Error, Transformation};
use ::{humanize, hash_string};
use database::{Database, Transaction, Change, Store};
use table::{Index, Table, TableId};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use hashbrown::{HashSet, HashMap};
use rust_core::fmt;
use operations::{MechFunction};


// ## Runtime

// Defines the function of a Mech program. The runtime consists of a series of blocks, defined
// by the user. Each block has a number of table dependencies, and produces new values that update
// existing tables. Blocks can also create new tables. The data dependencies of each block define
// a computational network of operations that runs until a steady state is reached (no more tables
// are updated after a computational round).
// For example, say we have three tables: #a, #b, and #c.
// Block1 takes #a as input and writes to #b. Block2 takes #b as input and writes to #c.
// If we update table #a with a transaction, this will trigger Block1 to execute, which will update
// #b. This in turn will trigger Block2 to execute and it will update block #c. After this, there is
// nothing left to update so the round of execution is complete.
//
// Now consider Block3 that takes #b as input and update #a and #c. Block3 will be triggered to execute
// after Block1, and it will update #a and #c. But since Block1 takes #a as input, this causes an infinite
// loop. This loop will terminate after a fixed number of iterations. Practically, this can be checked at
// compile time and the user can be warned of this and instructed to include some stop condition.
pub struct Runtime {
  pub recursion_limit: u64,
  pub database: Arc<RefCell<Database>>,
  pub blocks: HashMap<u64, Block>,
  pub ready_blocks: HashSet<u64>,
  pub errors: Vec<Error>,
  pub register_to_block: HashMap<u64,HashSet<u64>>,
  pub output_to_block:  HashMap<u64,HashSet<u64>>,
  pub changed_this_round: HashSet<u64>,
  pub defined_tables: HashSet<u64>,
  pub input: HashSet<u64>,
  pub output: HashSet<u64>,
  pub functions: HashMap<u64, Option<MechFunction>>,
}

impl Runtime {

  pub fn new(database: Arc<RefCell<Database>>, recursion_limit: u64) -> Runtime {
    Runtime {
      recursion_limit,
      database,
      blocks: HashMap::new(),
      errors: Vec::new(),
      ready_blocks: HashSet::new(),
      register_to_block: HashMap::new(),
      output_to_block: HashMap::new(),
      changed_this_round: HashSet::new(), // A cumulative list of all tables changed this round
      defined_tables: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      functions: HashMap::new(),
    }
  }

  pub fn load_library_function(&mut self, name: &str, fxn: Option<MechFunction>) {
    let name_hash = hash_string(name);
    let mut db = self.database.borrow_mut();
    let mut store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
    store.strings.insert(name_hash, name.to_string());
    self.functions.insert(name_hash, fxn);
  }

  pub fn run_network(&mut self) -> Result<(), Error> {    
    let mut recursion_ix = 0;

    // We are going to execute ready blocks until there aren't any left or until
    // the recursion limit is reached
    loop {
      // Solve all of the ready blocks
      for block_id in self.ready_blocks.drain() {
        let mut block = self.blocks.get_mut(&block_id).unwrap();
        block.process_changes(self.database.clone());
        block.solve(self.database.clone(), &self.functions);
        self.changed_this_round.extend(&block.output);
      }

      self.changed_this_round.extend(&self.database.borrow().changed_this_round);
      &self.database.borrow_mut().changed_this_round.clear();

      // Figure out which blocks are now ready and add them to the list
      // of ready blocks
      for register in self.changed_this_round.drain() {
        match self.output_to_block.get(&register) {
          Some(producing_block_ids) => {
            for block_id in producing_block_ids.iter() {
              let mut block = &mut self.blocks.get_mut(&block_id).unwrap();
              if block.state == BlockState::New {
                block.output_dependencies_ready.insert(register);
                if block.is_ready() {
                  self.ready_blocks.insert(block.id);
                }
              }
            }
          }
          _ => () // No producers
        }
        match self.register_to_block.get(&register) {
          Some(listening_block_ids) => {
            for block_id in listening_block_ids.iter() {
              let mut block = &mut self.blocks.get_mut(&block_id).unwrap();
              block.ready.insert(register);
              if block.is_ready() {
                self.ready_blocks.insert(block.id);
              }
            }
          },
          _ => (), // No listeners
        }
      }

      // Break the loop if there are no more blocks that are ready
      if self.ready_blocks.is_empty() {
        break;
      // Break the loop if we hit the recursion limit
      } else if self.recursion_limit == recursion_ix {
        // TODO Emit a warning here
        println!("Recursion limit reached");
        break;
      }
      recursion_ix += 1;
    }
    for (block_id, block) in self.blocks.iter() {
      match block.state {
        BlockState::Ready => {self.ready_blocks.insert(*block_id);}
        _ => (),
      }
    }
    Ok(())
  }

  pub fn remap_column(&self, table_id: u64, column: Index) -> Index {
    match column {
      Index::Alias(alias) => {
        match self.database.borrow().store.column_alias_to_index.get(&(table_id,alias)) {
          Some(ix) => Index::Index(*ix),
          None => Index::Alias(alias),
        }
      },
      x => x,
    }    
  }

  pub fn register_blocks(&mut self, blocks: Vec<Block>) {
    for block in blocks {
      self.register_block(block);
    }
  }

  pub fn register_block(&mut self, mut block: Block) {
    // Add the block id as a listener for a particular register
    for input_register in block.input.iter() {
      let listeners = self.register_to_block.entry(*input_register).or_insert(HashSet::new());
      listeners.insert(block.id);
    }

    // Keep track of which blocks produce which tables
    for output_register in block.output.iter() {
      let producers = self.output_to_block.entry(*output_register).or_insert(HashSet::new());
      producers.insert(block.id);
    }

    // If the block is new and has no input, it can be marked to run immediately
    if !block.errors.is_empty() {
      block.state == BlockState::Error;
    } else if block.state == BlockState::New && block.input.len() == 0 {
      block.state == BlockState::Ready;
    }

    // Extend block strings
    {
      let mut db = self.database.borrow_mut();
      let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
      for (k,v) in block.store.strings.iter() {
        store.strings.insert(*k,v.clone());
      } 
    }

    // Register functions
    for tfm in &block.plan {
      match tfm {
        Transformation::Function{name, ..} => {
          self.functions.entry(*name).or_insert(None);
        }
        _ => (),
      }
    }

    // Mark ready registers
    let ready: HashSet<u64> = block.input.intersection(&self.output).cloned().collect();
    block.ready.extend(&ready);

    let ready: HashSet<u64> = block.output_dependencies.intersection(&self.output).cloned().collect();
    block.output_dependencies_ready.extend(&ready);

    // Add to the list of runtime output registers
    self.output.extend(&block.output);

    if block.is_ready() {
      block.process_changes(self.database.clone());
      self.ready_blocks.insert(block.id);
    }

    // Add the block to the list of blocks
    self.blocks.insert(block.id, block);

    self.run_network();
  }

}

impl fmt::Debug for Runtime {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "blocks: \n")?;
    for (k,block) in self.blocks.iter() {
      write!(f, "{:?}\n", block)?;
    }
    
    Ok(())
  }
}