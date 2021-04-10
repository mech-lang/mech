use block::{Block, BlockState, Register, Error, Transformation, format_register};
use ::{humanize, hash_string};
use database::{Database};
use table::{TableIndex, TableId};
use std::cell::RefCell;
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
  pub output_to_block:  HashMap<Register,HashSet<u64>>,
  pub input_to_block:  HashMap<Register,HashSet<u64>>,
  pub changed_this_round: HashSet<Register>,
  pub aggregate_changed_this_round: HashSet<Register>,
  pub aggregate_tables_changed_this_round: HashSet<TableId>,
  pub register_aliases: HashMap<Register, HashSet<Register>>,
  pub defined_registers: HashSet<Register>,
  pub needed_registers: HashSet<Register>,
  pub input: HashSet<Register>,
  pub output: HashSet<Register>,
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
      output_to_block: HashMap::new(),
      input_to_block: HashMap::new(),
      changed_this_round: HashSet::new(), 
      aggregate_changed_this_round: HashSet::new(), // A cumulative list of all registers changed this round
      aggregate_tables_changed_this_round: HashSet::new(),
      register_aliases: HashMap::new(),
      defined_registers: HashSet::new(),
      needed_registers: HashSet::new(),
      input: HashSet::new(),
      output: HashSet::new(),
      functions: HashMap::new(),
    }
  }

  pub fn load_library_function(&mut self, name: &str, fxn: Option<MechFunction>) {
    let name_hash = hash_string(name);
    let mut db = self.database.borrow_mut();
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
    store.strings.insert(name_hash, name.to_string());
    self.functions.insert(name_hash, fxn);
  }

  pub fn run_network(&mut self) -> Result<(), Error> {   
    self.aggregate_changed_this_round.clear(); 
    let mut recursion_ix = 0;
    let mut changed_last_round = true;
    
    // We are going to execute ready blocks until there aren't any left or until
    // the recursion limit is reached
    loop {
      {
        let mut db = self.database.borrow_mut();
        let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
        store.changed = false;
      }

      // Solve all of the ready blocks
      for block_id in self.ready_blocks.drain() {
        let block = self.blocks.get_mut(&block_id).unwrap();
        block.process_changes(self.database.clone());
        block.solve(self.database.clone(), &self.functions);
        self.changed_this_round.extend(&block.output);
      }

      self.changed_this_round.extend(&self.database.borrow().changed_this_round);
      &self.database.borrow_mut().changed_this_round.clear();

      // Figure out which blocks are now ready and add them to the list
      // of ready blocks
      for register in self.changed_this_round.drain() {
        self.aggregate_changed_this_round.insert(register);
        self.aggregate_tables_changed_this_round.insert(register.table_id);
        // Do the output dependencies first
        match self.output_to_block.get(&register) {
          Some(producing_block_ids) => {
            for block_id in producing_block_ids.iter() {
              let block = &mut self.blocks.get_mut(&block_id).unwrap();
              if block.state == BlockState::New {
                block.output_dependencies_ready.insert(register);
                if block.is_ready() {
                  // Add to the list of runtime output registers
                  self.output.extend(&block.output);
                  self.input.extend(&block.input);
                  self.input.extend(&block.output_dependencies);
                  self.ready_blocks.insert(block.id);
                }
              }
            }
          }
          _ => () // No producers
        }
        // Now look over all the tables which have this register as an input
        match self.input_to_block.get(&register) {
          Some(listening_block_ids) => {
            for block_id in listening_block_ids.iter() {
              let block = &mut self.blocks.get_mut(&block_id).unwrap();
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
        println!("Recursion limit of {:?} reached", self.recursion_limit);
        break;
      }
      // Check if there were any updates to the store. If not, we are at a set point, and we are done.
      {
        let mut db = self.database.borrow_mut();
        let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
        // If the store wasn't changed and there are no more ready blocks, we're done.
        if !store.changed && self.ready_blocks.is_empty() {
          break;
        // If the store wasn't changed for two consecutive rounds but there are still ready blocks, 
        // this means they aren't doing any work and we're at a set point, so we're done.
        } else if !store.changed && !changed_last_round {
          for block_id in self.ready_blocks.iter() {
            let mut block = &mut self.blocks.get_mut(&block_id).unwrap();
            block.state = BlockState::Done;
          }
          break;
        // If the store was changed, we did work this round
        } else if store.changed {
          changed_last_round = true;
        // If the store didn't change we might be done for good, but we'll need to go one more round
        // to find out for sure.
        } else if !store.changed {
          changed_last_round = false;
        }
      }
      recursion_ix += 1;
    }
    for (block_id, block) in self.blocks.iter() {
      match block.state {
        BlockState::Ready => {self.ready_blocks.insert(*block_id);}
        _ => (),
      }
    }
    for table_id in self.aggregate_tables_changed_this_round.drain() {
      let mut db = self.database.borrow_mut();
      let table = db.tables.get_mut(table_id.unwrap()).unwrap();
      table.reset_changed();
    }
    Ok(())
  }

  pub fn remap_column(&self, table_id: u64, column: TableIndex) -> TableIndex {
    match column {
      TableIndex::Alias(alias) => {
        match self.database.borrow().store.column_alias_to_index.get(&(table_id,alias)) {
          Some(ix) => TableIndex::Index(*ix),
          None => TableIndex::Alias(alias),
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
      let listeners = self.input_to_block.entry(*input_register).or_insert(HashSet::new());
      listeners.insert(block.id);
    }

    // Keep track of which blocks produce which tables
    for output_register in block.output.iter() {
      let producers = self.output_to_block.entry(*output_register).or_insert(HashSet::new());
      producers.insert(block.id);
    }

    // If the block is new and has no input, it can be marked to run immediately
    if !block.errors.is_empty() {
      block.state = BlockState::Error;
    } else if block.state == BlockState::New && block.input.len() == 0 && block.output_dependencies.len() == 0 {
      block.state = BlockState::Ready;
    }

    // Extend database strings
    {
      let mut db = self.database.borrow_mut();
      let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
      for (k,v) in block.store.strings.iter() {
        store.strings.insert(*k,v.clone());
      }       
      let block_store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
      for (k,v) in store.strings.iter() {
        block_store.strings.insert(*k,v.clone());
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
    let ready: HashSet<Register> = block.input.intersection(&self.output).cloned().collect();
    block.ready.extend(ready);

    let ready: HashSet<Register> = block.output_dependencies.intersection(&self.output).cloned().collect();
    block.output_dependencies_ready.extend(ready);

    // Get the list of tables defined by the block
    for (_, tfms) in &block.transformations {
      for tfm in tfms {
        match tfm {
          Transformation::NewTable{table_id, ..} => {
            let register = Register{table_id: *table_id, row: TableIndex::All, column: TableIndex::All};
            self.defined_registers.insert(register);
          }
          Transformation::ColumnAlias{table_id, column_ix, column_alias} => {
            let register = Register{table_id: *table_id, row: TableIndex::All, column: TableIndex::Alias(*column_alias)};
            self.defined_registers.insert(register);
            let register = Register{table_id: *table_id, row: TableIndex::All, column: TableIndex::Index(*column_ix)};
            self.defined_registers.insert(register);
          }
          _ => (),
        }
      }
    }

    // Extend register aliases 
    for (register, aliases) in block.register_aliases.iter() {
      match self.register_aliases.get_mut(&register) {
        Some(aliases2) => aliases2.extend(aliases),
        None => {self.register_aliases.insert(register.clone(), aliases.clone());},
      }
    }

    // Keep track of needed tables
    self.needed_registers.extend(&block.input);
    self.needed_registers.extend(&block.output_dependencies);

    // Figure out if all the requirements are met
    for register in &block.input {
      match self.output.contains(&register) {
        true => {block.ready.insert(*register);},
        false => {
          // If the runtime doesn't output a needed register, check if there's an alias it does output
          match self.register_aliases.get(&register) {
            Some(register_aliases) => {
              for alias in register_aliases.iter() {
                match self.output.contains(&alias) {
                  true => {
                    block.ready.insert(*register);
                  }
                  false => (),
                }
              }
            }
            _ => (),
          }
        },
      }
    }

    if block.is_ready() {
      self.output.extend(&block.output);
      self.input.extend(&block.input);
      self.input.extend(&block.output_dependencies);
      block.process_changes(self.database.clone());
      self.ready_blocks.insert(block.id);
    }

    let mut new_input_register_mapping: HashMap<Register, u64> = HashMap::new();

    // Check to see if this new block makes any other blocks ready to execute
    for block_output_register in block.output.iter() {
      let mut alternative_registers: HashSet<Register> = HashSet::new();
      alternative_registers.insert(*block_output_register);

      // Go through the block input mappings
      for (block_input_register, listening_blocks) in self.input_to_block.iter() {

        // The current block output could be an alias for the required input
        let table_id = *block_output_register.table_id.unwrap();
        let column_alias = match block_output_register.column {
          TableIndex::Index(ix) => {
            match self.database.borrow().store.column_index_to_alias.get(&(table_id,ix)) {
              Some(alias) => TableIndex::Alias(*alias),
              None => TableIndex::Index(ix),
            }
          }
          TableIndex::Alias(alias) => {
            match self.database.borrow().store.column_alias_to_index.get(&(table_id,alias)) {
              Some(ix) => TableIndex::Index(*ix),
              None => TableIndex::Alias(alias),
            }
          }
          x => x,
        };

        // We're talking about the same table, so we need to see if the current register is an alias or generalization of the required one.
        if block_output_register.table_id == block_input_register.table_id {
          match (block_output_register.row, block_output_register.column, block_input_register.row, block_input_register.column) {
            // The current block output could be a generalization of the required input
            // If I produce x{:,:}, a consumer of x{1,2} would be triggered
            (TableIndex::All, TableIndex::All, in_row, in_col) => {
              alternative_registers.insert(Register{table_id: block_output_register.table_id, row: in_row, column: in_col});
            }
            // The current block could be an alias of the required input
            // If I produce x{:,.x}, a consumer of x{:,1} would be triggered
            // If I produce x{:,1}, a consumer of x{:,.x} would be triggered
            (TableIndex::All, TableIndex::Index(_), in_row, in_col) |
            (TableIndex::All, TableIndex::Alias(_), in_row, in_col) => {
              if in_col == column_alias  {
                alternative_registers.insert(Register{table_id: block_output_register.table_id, row: in_row, column: in_col});
              }
            }
            _ => (),
          };
        }
      }

      // Create new mappings based on generalized and aliased registers
      for alternative_output_register in alternative_registers.iter() {
        match self.input_to_block.get(&alternative_output_register) {
          Some(listening_blocks) => {
            for listening_block_id in listening_blocks.iter() {
              match self.blocks.get_mut(&listening_block_id) {
                Some(listening_block) => {
                  listening_block.ready.insert(*alternative_output_register);
                  listening_block.ready.insert(*block_output_register);
                  listening_block.input.insert(*block_output_register);
                  new_input_register_mapping.insert(*block_output_register, *listening_block_id);
                }
                None => (),
              }
            }
          }
          None => (),
        }
      }
    }

    // Add an alias for the input to block
    for (register, block_id) in new_input_register_mapping.iter() {
      let listeners = self.input_to_block.entry(*register).or_insert(HashSet::new());
      listeners.insert(*block_id);
    }

    // Add the block to the list of blocks
    self.blocks.insert(block.id, block);

    self.run_network().ok();
  }

}

impl fmt::Debug for Runtime {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut db = self.database.borrow_mut();
    let store = unsafe{&mut *Arc::get_mut_unchecked(&mut db.store)};
    write!(f, "input: \n")?;
    for k in self.input.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
    }
    write!(f, "output: \n")?;
    for k in self.output.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
    }
    write!(f, "input to block: \n")?;
    for (k,v) in self.input_to_block.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
      for block_id in v.iter() {
        write!(f, "    {}\n", humanize(block_id))?;
      }
    }
    write!(f, "output to block: \n")?;
    for (k,v) in self.output_to_block.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
      for block_id in v.iter() {
        write!(f, "    {}\n", humanize(block_id))?;
      }
    }
    write!(f, "register aliases: \n")?;
    for (k,v) in self.register_aliases.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
      for register in v.iter() {
        write!(f, "    {}\n", format_register(&store.strings,register))?;
      }
    }

    write!(f, "defined registers: \n")?;
    for k in self.defined_registers.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
    }

    write!(f, "needed registers: \n")?;
    for k in self.needed_registers.iter() {
      write!(f, "  {}\n", format_register(&store.strings,k))?;
    }

    write!(f, "blocks: \n")?;
    for (_k,block) in self.blocks.iter() {
      write!(f, "{:?}\n", block)?;
    }
    
    Ok(())
  }
}
