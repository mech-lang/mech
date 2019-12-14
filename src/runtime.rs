// # Mech Runtime

/* 
 The Mech Runtime is the engine that drives computations in Mech. The runtime 
 is comprised of "Blocks", interconnected by "Pipes" of records. Blocks can 
 interact with the database, by Scanning for records that  match a pattern, or 
 by Projecting computed records into the database.
*/

// ## Prelude

use table::{Table, TableId, Value, Index};
use indexes::Hasher;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))] use core::fmt;
use database::{Transaction, Interner, Change};
use hashbrown::hash_map::{HashMap, Entry};
use hashbrown::hash_set::HashSet;
use indexes::TableIndex;
use operations;
use operations::{Function, Comparator, Parameter, Logic};
use quantities::{Quantity, ToQuantity, QuantityMath, make_quantity};
use libm::{sin, cos, fmod, round, floor};
use errors::{Error, ErrorType};

// ## Runtime

#[derive(Clone)]
pub struct Runtime {
  pub blocks: HashMap<usize, Block>,
  pub pipes_map: HashMap<Register, HashSet<Address>>,
  pub tables_map: HashMap<u64, u64>,
  pub ready_blocks: HashSet<usize>,
  pub functions: HashMap<String, Option<fn(Value)->Value>>,
  pub changed_this_round: HashSet<(u64, Index)>,
  pub errors: Vec<Error>,
}

impl Runtime {

  pub fn new() -> Runtime {
    Runtime {
      blocks: HashMap::new(),
      ready_blocks: HashSet::new(),
      pipes_map: HashMap::new(),
      tables_map: HashMap::new(),
      functions: HashMap::new(),
      changed_this_round: HashSet::new(),
      errors: Vec::new(),
    }
  }

  pub fn clear(&mut self) {
    self.blocks.clear();
    self.ready_blocks.clear();
    self.pipes_map.clear();
  }

  // Register a new block with the runtime
  pub fn register_block(&mut self, mut block: Block, store: &mut Interner) {
    if block.id == 0 {
      // TODO Better auto ID. Maybe hash constraints?
      block.id = self.blocks.len() + 1;
    }
    // Take the input registers from the block and add them to the pipes map
    for register in block.input_registers.iter() {
      let table = register.table;
      let column = register.column.clone();
      let new_address = Address{block: block.id, register: register.clone()};
      let listeners = self.pipes_map.entry(register.clone()).or_insert(HashSet::new());
      listeners.insert(new_address);

      // Set the register as ready if the referenced column exists
      /*
      match store.get_column_by_id(table as u64, column) {
        Some(_column) => block.ready = set_bit(block.ready, ix),
        None => (),
      }*/
    }

    // Record the functions used in block
    for fun in &block.functions {
      self.functions.insert(fun.to_string(), None);
    }

    // Register all local tables in the tables map
    for local_table in block.memory.map.keys() {
      self.tables_map.insert(*local_table, block.id as u64);
    }
    // Register all errors on the block with the runtime
    self.errors.append(&mut block.errors.clone());

    // Mark the block as ready for execution on the next available cycle
    if block.updated && block.input_registers.len() == 0 && block.errors.len() == 0 {
      self.ready_blocks.insert(block.id);
    }
    // Add the block to our list of blocks
    self.blocks.insert(block.id, block.clone());
  } 

  pub fn register_blocks(&mut self, blocks: Vec<Block>, store: &mut Interner) {
    for block in blocks {
      self.register_block(block, store);
    }
  }

  pub fn remove_block(&mut self, block_id: &usize) {
    {
      let block = self.blocks.get(block_id).unwrap();
      // Remove listeners
      for register in block.input_registers.iter() {
        let mut listeners = self.pipes_map.get_mut(&register).unwrap();
        let address = Address{block: block_id.clone(), register: register.clone()};
        listeners.remove(&address);
      }
      // Register all local tables in the tables map
      for local_table in block.memory.map.keys() {
        self.tables_map.remove(local_table);
      }
      self.ready_blocks.remove(block_id);
    }
    self.blocks.remove(&block_id);
  }

  // We've just interned some changes, and now we react to them by running the 
  // block graph. The graph is run until the tables reach a steady state or 
  // we hit the max_iteration limit
  pub fn run_network(&mut self, store: &mut Interner, max_iterations: u64) {
    
    let mut iteration_count = 0; 
    // Note: The way this while loop is written, it's actually a do-while loop.
    // This is a little trick in Rust. This means the network will always run
    // at least one time, and if there are no more ready blocks after that run,
    // the loop will terminate.
    while {
      let mut ready_blocks: Vec<usize> = self.ready_blocks.drain().map(|arg| {
        arg.clone()
      }).collect::<Vec<usize>>();
      ready_blocks.sort();
      for block_id in ready_blocks {
        let block = &mut self.blocks.get_mut(&block_id).unwrap();
        block.solve(store, &self.functions);
        // Register any new inputs
        for register in block.input_registers.iter() {
          let table = register.table;
          let column = register.column.clone();
          let new_address = Address{block: block.id, register: register.clone()};
          let listeners = self.pipes_map.entry(register.clone()).or_insert(HashSet::new());
          listeners.insert(new_address);
        }
      }
      // Queue up the next blocks based on tables that changed during this round.
      for (table, column) in store.tables.changed_this_round.drain() {
        self.changed_this_round.insert((table.clone(), column.clone()));
        let register = Register::new(table,column);
        match self.pipes_map.get(&register) {
          Some(register_addresses) => {
            for register_address in register_addresses.iter() {
              let mut block = &mut self.blocks.get_mut(&register_address.block).unwrap();
              block.ready.insert(register_address.register.clone());
              if block.is_ready() {
                self.ready_blocks.insert(register_address.block);
              }
            }
          },
          _ => (), // No listeners
        }
      }
      // Halt iterating if we've exceeded the maximum number of allowed iterations.
      iteration_count += 1;
      if iteration_count == max_iterations {
        // TODO Insert an error into the db here instead.
        //println!("Reached iteration limit {:?}", iteration_count);
        self.ready_blocks.clear();        
      }
      // Terminate if no blocks are ready to execute next round.
      !self.ready_blocks.is_empty()
    } {}
    // Reset blocks' updated status
    for mut block in &mut self.blocks.values_mut() {
      block.updated = false;
    }
  }

}

impl fmt::Debug for Runtime {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Runtime:\n").unwrap();
    write!(f, " Blocks:\n\n").unwrap();
    for ref block in &self.blocks {
      write!(f, "{:?}\n\n", block).unwrap();
    }
    Ok(())
  }
}

// ## Blocks

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Address {
  pub block: usize,
  pub register: Register,
}

impl fmt::Debug for Address {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "@(block: {:?}, register: {:?})", self.block, self.register)
  }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Register {
  pub table: u64,
  pub column: Index,
}

impl Register {
  
  pub fn new(table: u64, column: Index) -> Register { 
    Register {
      table: table,
      column: column,
    }
  }

}

impl fmt::Debug for Register {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:#x}, {:?})", self.table, self.column)
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockState {
  Ready,
  Error,
  Unsatisfied,
  Updated,
  Pending,
  Disabled,
  New,
}

#[derive(Clone, PartialEq)]
pub struct Block {
  pub id: usize,
  pub state: BlockState,
  pub name: String,
  pub text: String,
  pub ready: HashSet<Register>,
  pub updated: bool,
  pub plan: Vec<Constraint>,
  pub input_registers: HashSet<Register>,
  pub output_registers: HashSet<Register>,
  pub constraints: Vec<(String, Vec<Constraint>)>,
  pub errors: Vec<Error>,
  pub functions: HashSet<String>,
  memory: TableIndex,
  scratch: Table,
  lhs_rows_empty: Vec<Value>,
  lhs_columns_empty: Vec<Value>,
  rhs_rows_empty: Vec<Value>,
  rhs_columns_empty: Vec<Value>,
  block_changes: Vec<Change>,
}

impl Block {
  
  pub fn new() -> Block { 
    Block {
      id: 0,
      name: String::from(""),
      text: String::from(""),
      ready: HashSet::with_capacity(1),
      state: BlockState::New,
      updated: false,
      plan: Vec::new(),
      input_registers: HashSet::with_capacity(1),
      output_registers: HashSet::with_capacity(1),
      functions: HashSet::with_capacity(1),
      constraints: Vec::with_capacity(1),
      memory: TableIndex::new(1),
      errors: Vec::new(),
      scratch: Table::new(0,0,0),
      // allocate empty indices so we don't have to do this on each iteration
      lhs_rows_empty: Vec::new(),
      lhs_columns_empty: Vec::new(),
      rhs_rows_empty: Vec::new(),
      rhs_columns_empty: Vec::new(),
      block_changes: Vec::new(),
    }
  }

  pub fn get_table(&self, table_id: u64) -> Option<&Table> {
    self.memory.get(table_id)
  } 

  pub fn add_constraints(&mut self, constraint_tuple: (String, Vec<Constraint>)) {
    self.constraints.push(constraint_tuple.clone());
    let (constraint_text, constraints) = constraint_tuple;

    // Add relevant constraints to plan
    let mut reversed = constraints.clone();
    reversed.reverse();
    for constraint in reversed {
      match constraint {
        Constraint::Filter{..} |
        Constraint::Logic{..} |
        Constraint::Function{..} |
        Constraint::CopyTable{..} |
        Constraint::Range{..} |
        Constraint::ChangeScan{..} |
        Constraint::Append{..} |
        Constraint::Scan{..} |
        Constraint::Insert{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }

    // Do any work we can up front
    for (constraint_ix, constraint) in constraints.iter().enumerate() {
      match constraint {
        Constraint::CopyTable{from_table, to_table} => {
          self.output_registers.insert(Register::new(*to_table, Index::Index(0)));
        },
        Constraint::Append{from_table, to_table} => {
          match to_table {
            TableId::Global(id) => {
              self.input_registers.insert(Register::new(*id, Index::Index(0)));
              self.output_registers.insert(Register::new(*id, Index::Index(0)));
            }
            _ => (),
          };
        },
        Constraint::Insert{from: (from_table, ..), to: (to_table, ..)} => {
          match to_table {
            TableId::Global(id) => {
              self.input_registers.insert(Register::new(*id, Index::Index(0)));
              self.output_registers.insert(Register::new(*id, Index::Index(0)));
            }, 
            _ => (),
          };
        },
        Constraint::Scan{table, indices, output} => {
          // TODO Update this whole register adding process and marking tables ready
          //self.input_registers.push(Register::input(table, 1));
          match table {
            TableId::Global(id) => {
              self.input_registers.insert(Register{table: *id, column: Index::Index(0)});
            },
            _ => (),
          }
        },
        Constraint::ChangeScan{table, column} => {
          match (table, column.as_slice()) {
            (TableId::Global(id), [None, Some(Parameter::Index(index))]) => {
              self.input_registers.insert(Register{table: *id, column: index.clone()});
            },
            (TableId::Global(id), [None, None]) => {
              self.input_registers.insert(Register{table: *id, column: Index::Index(0)});
            },
            _ => (),
          }
        },
        Constraint::AliasTable{table, alias} => {
          // TODO Raise an error here if the alias already exists
          match table {
            TableId::Local(id) => {
              match self.memory.add_alias(*id, *alias) {
                Err(mech_error) => {
                  self.errors.push(Error{
                    block: self.id as u64,
                    constraint: constraint.clone(),
                    error_id: mech_error,
                  });
                },
                _ => (),
              }
            },
            TableId::Global(id) => (), // TODO Add global alias here
          }
        },
        Constraint::Function{operation, fnstring, parameters, output} => {
          self.functions.insert(fnstring.to_string());
          for (arg_name, table, indices) in parameters {
            match table {
              TableId::Global(id) => {
                self.input_registers.insert(Register{table: *id, column: Index::Index(0)});
              },
              _ => (),
            }
          }
        },
        Constraint::Filter{comparator, lhs, rhs, output} => {
          let (lhs_table, lhs_rows, lhs_columns) = lhs;
          let (rhs_table, rhs_rows, rhs_columns) = rhs;
          match lhs_table {
            TableId::Global(id) => {
              match lhs_columns {
                Some(Parameter::Index(index)) => self.input_registers.insert(Register{table: *id, column: index.clone()}),
                _ => self.input_registers.insert(Register{table: *id, column: Index::Index(0)}),
              };
            }
            _ => (),
          }
          match rhs_table {
            TableId::Global(id) => {
              match rhs_columns {
                Some(Parameter::Index(index)) => self.input_registers.insert(Register{table: *id, column: index.clone()}),
                _ => self.input_registers.insert(Register{table: *id, column: Index::Index(0)}),
              };
            }
            _ => (),
          }
        },
        Constraint::NewTable{id, rows, columns} => {
          match id {
            TableId::Local(id) => {
              self.memory.insert(Table::new(*id, *rows, *columns));              
            }
            _ => (),
          }
        },
        Constraint::Empty{table, row, column} => {
          let table_id = match table {
            TableId::Local(id) => *id,
            _ => 0,
          };
          match self.memory.map.entry(table_id) {
            Entry::Occupied(mut o) => {
              let table_ref = o.get_mut();
              table_ref.set_cell(&row, &column, Value::Empty);
            },
            Entry::Vacant(v) => {    
            },
          };
          self.updated = true;
        },
        Constraint::Constant{table, row, column, value, unit} => {
          let (domain, scale) = match unit {
            Some(unit_value) => match unit_value.as_ref() {
              "g"  => (1, 0),
              "kg" => (1, 3),
              "m" => (2, 0),
              "km" => (2, 3),
              _ => (0, 0),
            },
            _ => (0, 0),
          };
          let test = make_quantity(value.mantissa(), value.range() + scale, domain);
          let table_id = match table {
            TableId::Local(id) => *id,
            _ => 0,
          };
          match self.memory.map.entry(table_id) {
            Entry::Occupied(mut o) => {
              let table_ref = o.get_mut();
              table_ref.set_cell(&row, &column, Value::from_quantity(test));
            },
            Entry::Vacant(v) => {    
            },
          };  
          self.updated = true;
        },
        Constraint::Reference{table, destination} => {
          match self.memory.map.entry(*destination) {
            Entry::Occupied(mut o) => {
              let table_ref = o.get_mut();
              table_ref.set_cell(&Index::Index(1), &Index::Index(1), Value::Reference(*table));
            },
            Entry::Vacant(v) => {    
            },
          };
        },
        Constraint::String{table, row, column, value} => {
          let table_id = match table {
            TableId::Local(id) => *id,
            _ => 0,
          };
          match self.memory.map.entry(table_id) {
            Entry::Occupied(mut o) => {
              let table_ref = o.get_mut();
              table_ref.set_cell(&row, &column, Value::from_string(value.clone()));
            },
            Entry::Vacant(v) => {    
            },
          };
          self.updated = true;
        },
        Constraint::TableColumn{table, column_ix, column_alias} => {
          match self.memory.get_mut(*table) {
            Some(table_ref) => {
              table_ref.set_column_alias(*column_alias, *column_ix);
            }
            None => (), // TODO Note this as an error
          };
        },
        _ => (),
      }
    }

    if self.errors.len() > 0 {
      self.state = BlockState::Error;
    }


  }

  pub fn is_ready(&mut self) -> bool {
    if self.state == BlockState::Error || self.state == BlockState::Pending {
      false
    } else {
      let set_diff: HashSet<Register> = self.input_registers.difference(&self.ready).cloned().collect();
      // The block is ready if all input registers are ready i.e. the length of the set diff is 0
      if set_diff.len() == 0 {
        true
      } else {
        self.state = BlockState::Unsatisfied;
        false
      }
    }    
  }

  pub fn resolve_subscript(&mut self, store: &mut Interner, table: &TableId, indices: &Vec<(Option<Parameter>, Option<Parameter>)>) -> Table {
    let mut old: Table = Table::new(0,0,0);
    let mut table_id: TableId = table.clone();
    'solve_loop: for index in indices {
      let mut table_ref = match table_id {
        TableId::Local(id) => match self.memory.get(id) {
          Some(id) => id,
          None => store.get_table(id).unwrap(),
        },
        TableId::Global(id) => store.get_table(id).unwrap(),
      };
      let one = vec![Value::from_u64(1)];
      let (row_ixes, column_ixes) = match index {
        // If we only have one index, it's like this #x{3}
        (Some(parameter), None) => {
          // Get the ixes
          let ixes: &Vec<Value> = match &parameter {
            Parameter::TableId(TableId::Local(id)) => &self.memory.get(*id).unwrap().data[0],
            Parameter::TableId(TableId::Global(id)) => &store.get_table(*id).unwrap().data[0],
            _ => &self.rhs_rows_empty,
          };
          // The other dimension index will be one.
          // So if it's #x{3} where #x = [1 2 3], then it translates to #x{1,3}
          // If #x = [1; 2; 3] then it translates to #x{3,1}
          let (row_ixes, column_ixes) = match (table_ref.rows, table_ref.columns) {
            (1, columns) => (&one, ixes),
            (rows, 1) => (ixes, &one),
            _ => {
              // TODO Report an error here... or do matlab style ind2sub
              break 'solve_loop;
            }
          };
          (row_ixes, column_ixes)
        },
        // Otherwise we have a couple choices:
        // #x{1,2}
        // #x.y{1}
        (Some(row_parameter), Some(column_parameter)) => {
          let row_ixes: &Vec<Value> = match &row_parameter {
            Parameter::TableId(TableId::Local(id)) => &self.memory.get(*id).unwrap().data[0],
            Parameter::TableId(TableId::Global(id)) => &store.get_table(*id).unwrap().data[0],
            _ => &self.rhs_rows_empty,
          };
          let column_ixes: &Vec<Value> = match &column_parameter {
            Parameter::TableId(TableId::Local(id)) => &self.memory.get(*id).unwrap().data[0],
            Parameter::TableId(TableId::Global(id)) => &store.get_table(*id).unwrap().data[0],
            Parameter::Index(index) => {
              let ix = match table_ref.get_column_index(index) {
                Some(ix) => ix,
                // If the attribute is missing, note the error and bail
                None => { 
                  /* TODO Fix this
                  self.errors.push(
                    Error{
                      block: self.id as u64,
                      constraint: step.clone(),
                      error_id: ErrorType::MissingAttribute(index.clone()),
                    }
                  );*/
                  break 'solve_loop;
                }, 
              };
              self.lhs_columns_empty.push(Value::from_u64(ix));
              &self.lhs_columns_empty
            },
            _ => &self.lhs_rows_empty,
          };
          (row_ixes, column_ixes)
        }
        _ => (&self.lhs_rows_empty, &self.rhs_rows_empty),
      };
      let width  = if column_ixes.is_empty() { table_ref.columns }
                    else { column_ixes.len() as u64 };      
      let height = if row_ixes.is_empty() { table_ref.rows }
                    else { row_ixes.len() as u64 };
      self.scratch.grow_to_fit(height, width);
      let mut iix = 0;
      let mut actual_width = 0;
      let mut actual_height = 0;
      for i in 0..width as usize {
        let mut column_mask = true;
        let cix = if column_ixes.is_empty() { i }
                  else { 
                    match column_ixes[i] {
                      Value::Number(n) => n.to_u64() as usize  - 1,
                      Value::Bool(true) => i,
                      _ => {
                        column_mask = false;
                        0
                      },  
                    }
                  };
        let mut jix = 0;
        for j in 0..height as usize {
          let mut row_mask = true;
          let rix = if row_ixes.is_empty() { j }
                    else { 
                      match row_ixes[j] {
                        Value::Number(n) => n.to_u64() as usize - 1,
                        Value::Bool(true) => j,
                        _ => {
                          row_mask = false;
                          0
                        }, 
                      }
                    };
          if column_mask == true && row_mask == true {
            //let value = table_ref.data[cix][rix];
            // Check bounds
            if cix + 1 > table_ref.columns as usize || rix + 1 > table_ref.rows as usize {
              /* TODO Fix error
              self.errors.push(
                Error{
                  block: self.id as u64,
                  constraint: step.clone(),
                  error_id: ErrorType::IndexOutOfBounds(((rix as u64 + 1, cix as u64 + 1),(table_ref.rows, table_ref.columns))),
                }
              );*/
              break 'solve_loop;
            }
            match table_ref.column_index_to_alias.get(cix) {
              Some(Some(alias)) => {
                self.scratch.column_aliases.insert(*alias, iix as u64 + 1);
                if self.scratch.column_index_to_alias.len() < iix + 1 {
                  self.scratch.column_index_to_alias.resize_with(iix + 1, ||{None});
                }
                self.scratch.column_index_to_alias[iix] = Some(*alias);
              },
              _ => (),
            };
            self.scratch.data[iix][jix] = table_ref.data[cix][rix].clone();
            jix += 1;
            actual_height = jix;
          }
        }
        if column_mask == true {
          iix += 1;
          actual_width = iix;
        }
      }
      self.scratch.shrink_to_fit(actual_height as u64, actual_width as u64);
      old = self.scratch.clone();
      self.scratch.clear();
      match &old.data[0][0] {
        Value::Reference(id) => table_id = id.clone(),
        _ => (),
      };
    }
    self.rhs_columns_empty.clear();
    self.lhs_columns_empty.clear();
    let out = old.clone();
    self.scratch.clear();
    out
  }

  pub fn solve(&mut self, store: &mut Interner, functions: &HashMap<String, Option<fn(Value)->Value>>) {
    let block = self as *mut Block;
    let mut copy_tables: Vec<TableId> = vec![];
    'solve_loop: for step in &self.plan {
      match step {
        Constraint::Scan{table, indices, output} => {
          let out_table = &output;
          let scanned;
          unsafe {
            scanned = (*block).resolve_subscript(store,table,indices);
          }
          let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
          out.rows = scanned.rows;
          out.columns = scanned.columns;
          out.data = scanned.data.clone();
          out.column_index_to_alias = scanned.column_index_to_alias.clone();
          out.column_aliases = scanned.column_aliases.clone();
          self.scratch.clear();
          self.rhs_columns_empty.clear();
          self.lhs_columns_empty.clear();
        },
        Constraint::ChangeScan{table, column} => {
          match (table, column.as_slice()) {
            (TableId::Global(id), [None, Some(Parameter::Index(index))]) => {
              let register = Register{table: *id, column: index.clone()};
              self.ready.remove(&register);
            }
            (TableId::Global(id), [None, None]) => {
              let register = Register{table: *id, column: Index::Index(0)};
              self.ready.remove(&register);
            }
            (TableId::Local(id), _) => {
              // test value at table
              let table = self.memory.get(*id).unwrap();
              if table.data[0][0] == Value::Bool(false) {
                self.block_changes.clear();
                self.state = BlockState::Unsatisfied;
                break 'solve_loop;
              }
            },
            _ => (),
          }
        },
        // TODO move most of this into Operations.rs
        Constraint::Function{operation, fnstring, parameters, output} => {
          // Concat Functions
          if *operation == Function::HorizontalConcatenate {
            let out_table = &output[0];
            let mut cat_table = Table::new(0,0,0);
            for (name, table, indices) in parameters {
              let scanned;
              unsafe {
                scanned = (*block).resolve_subscript(store,table,indices);
              } 
              // Do all the work here:
              if cat_table.rows == 0 {
                cat_table.grow_to_fit(scanned.rows,scanned.columns);
                cat_table.data = scanned.data;
              // We're adding a scalar to the table. Auto fill to height
              } else if scanned.rows == 1 {
                let start_col: usize = cat_table.columns as usize;
                let end_col: usize = (cat_table.columns + scanned.columns) as usize;
                let start_row: usize = 0;
                let end_row: usize = cat_table.rows as usize;
                cat_table.grow_to_fit(end_row as u64, end_col as u64);
                for i in 0..scanned.columns {
                  for j in 0..cat_table.rows {
                    cat_table.data[i as usize + start_col][j as usize] = scanned.data[i as usize][0].clone();
                  }
                }
              } else if cat_table.rows == 1 {
                let old_width = cat_table.columns;
                let end_col: usize = (cat_table.columns + scanned.columns) as usize;
                cat_table.grow_to_fit(scanned.rows, end_col as u64);
                // copy old stuff
                for i in 0..old_width as usize {
                  for j in 1..cat_table.rows as usize {
                    cat_table.data[i][j] = cat_table.data[i][0].clone();
                  }
                }
                // copy new stuff
                for i in 0..scanned.columns as usize {
                  for j in 0..scanned.rows as usize {
                    cat_table.data[i + old_width as usize][j] = scanned.data[i][j].clone();
                  }
                }
              }
            }
            let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
            out.rows = cat_table.rows;
            out.columns = cat_table.columns;
            out.data = cat_table.data.clone();
            self.lhs_columns_empty.clear();
            self.rhs_columns_empty.clear();
            self.scratch.clear();
          }
          /*
          else if *operation == Function::VerticalConcatenate {
            let out_table = &output[0];
            for (table, rows, columns) in parameters {
              let table_ref = match table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
              };
              if self.scratch.columns == 0 {
                self.scratch.grow_to_fit(table_ref.rows, table_ref.columns);
                self.scratch.data = table_ref.data.clone();
              } else if self.scratch.columns == table_ref.columns {
                let mut i = 0;
                for column in &mut self.scratch.data {
                  let mut col = table_ref.data[i].clone();
                  column.append(&mut col);
                  i += 1;
                }
                self.scratch.grow_to_fit(self.scratch.rows + table_ref.rows, self.scratch.columns);
              }
            }
            
            let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.data = self.scratch.data.clone();
            self.scratch.clear();
            
          }
          else if *operation == Function::TableSplit {
            let out_table = &output[0];
            let (in_table, _, _) = &parameters[0];
            let table_ref = match in_table {
              TableId::Local(id) => self.memory.get(*id).unwrap(),
              TableId::Global(id) => store.get_table(*id).unwrap(),
            };        
            self.scratch.grow_to_fit(table_ref.rows, 1);
            let cc = table_ref.columns;
            let rr = table_ref.rows;
            let aliases = table_ref.column_aliases.clone();
            let alias_map = table_ref.column_index_to_alias.clone();
            let data = table_ref.data.clone();
            for i in 0..rr as usize {
              // Create a new table for each row in the original table
              let id = Hasher::hash_string(format!("{},table/split-{:?}-row::{},{:?}",i,in_table, i, aliases));
              let mut new_table = Table::new(id, 1, cc);
              new_table.column_aliases = aliases.clone();
              // fill data
              for j in 0..cc as usize {
                new_table.data[j][0] = data[j][i].clone();
              }
              self.memory.insert(new_table);
              self.scratch.data[0][i] = Value::Reference(TableId::Local(id));
            }
            let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.data = self.scratch.data.clone();
            self.scratch.clear();
          }
          else if *operation == Function::MathSin || *operation == Function::MathCos ||
                  *operation == Function::MathRound ||
                  *operation == Function::MathFloor ||
                  *operation == Function::StatSum ||
                  *operation == Function::TableSplit || 
                  *operation == Function::SetAny {
            let argument = match &parameters[0] {
              (TableId::Local(argument), _, _) => *argument,
              _ => 0,
            };
            let (value_table, value_rows, value_columns) = &parameters[1];            
            let out_table = &output[0];
            {
              let rhs = match value_table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
              };
              let rhs_rows: &Vec<Value> = match value_rows {
                Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
                Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
                _ => &self.rhs_rows_empty,
              };
              let rhs_columns: &Vec<Value> = match value_columns {
                Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
                Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
                Some(Parameter::Index(index)) => {
                  let ix = match rhs.get_column_index(index) {
                    Some(ix) => ix,
                    None => 0,
                  };
                  self.rhs_columns_empty.push(Value::from_u64(ix));
                  &self.rhs_columns_empty
                },
                _ => &self.rhs_columns_empty,
              };
              let rhs_width  = if rhs_columns.is_empty() { rhs.columns }
                               else { rhs_columns.len() as u64 };     
              let rhs_height = if rhs_rows.is_empty() { rhs.rows }
                               else { rhs_rows.len() as u64 }; 
              self.scratch.grow_to_fit(rhs_height, rhs_width);        
              for i in 0..rhs_width as usize {
                let rcix = if rhs_columns.is_empty() { i }
                          else { rhs_columns[i].as_u64().unwrap() as usize - 1 };
                for j in 0..rhs_height as usize {
                  let rrix = if rhs_rows.is_empty() { j }
                            else { rhs_rows[j].as_u64().unwrap() as usize - 1 };
                  let pi = 3.141592653589793238462643383279502884197169399375105820974944592307816406286;
                  match (operation, argument, &rhs.data[rcix][rrix]) {
                    // column
                    (Function::StatSum, 0x756cddd0, Value::Number(x)) => {
                      let previous = self.scratch.data[0][0].as_quantity().unwrap();
                      match previous.add(*x) {
                        Ok(op_result) => {
                          self.scratch.data[0][0] = Value::Number(op_result);
                          self.scratch.shrink_to_fit(1,1);
                        },
                        _ => (), // Throw an error here
                      }
                    }
                    // row
                    (Function::StatSum, 0x776f72, Value::Number(x)) => {
                      let previous = self.scratch.data[0][0].as_quantity().unwrap();
                      match previous.add(*x) {
                        Ok(op_result) => {
                          self.scratch.data[0][0] = Value::Number(op_result);
                          self.scratch.shrink_to_fit(1,1);
                        },
                        _ => (), // Throw an error here
                      }
                    }
                    // column
                    (Function::MathRound, 0x756cddd0, Value::Number(x)) => {
                      let result = round(x.to_float());
                      self.scratch.data[i][j] = Value::from_quantity(result.to_quantity());
                    },
                    // column
                    (Function::SetAny, 0x756cddd0, Value::Bool(x)) => {
                      let new = match (x, &self.scratch.data[0][0]) {
                        (false, Value::Empty) => Value::Bool(false),
                        (true, _) => Value::Bool(true),
                        (_, Value::Bool(true)) => Value::Bool(true),
                        (false, _) => Value::Bool(false),
                      };
                      self.scratch.data[0][0] = new;
                    },
                    // column
                    (Function::MathFloor, 0x756cddd0, Value::Number(x)) => {
                      let result = floor(x.to_float());
                      self.scratch.data[i][j] = Value::from_quantity(result.to_quantity());
                    },
                    // column
                    (Function::TableSplit, 0x756cddd0, Value::Number(x)) => {
                    },
                    // row
                    (Function::TableSplit, 0x776f72, Value::Number(x)) => {

                    },
                    // radians
                    (Function::MathSin, 0x69d7cfd3, Value::Number(x)) => {
                      let result = sin(x.to_float());
                      self.scratch.data[i][j] = Value::from_quantity(result.to_quantity());
                    },
                    // degrees
                    (Function::MathCos, 0x72dacac9, Value::Number(x)) => {
                      let result = match fmod(x.to_float(), 360.0) {
                        0.0 => 1.0,
                        90.0 => 0.0,
                        180.0 => -1.0,
                        270.0 => 0.0,
                        _ => cos(x.to_float() * pi / 180.0),
                      };
                      self.scratch.data[i][j] = Value::from_quantity(result.to_quantity());
                    },
                    // radians
                    (Function::MathCos, 0x69d7cfd3, Value::Number(x)) => {
                      let result = cos(x.to_float());
                      self.scratch.data[i][j] = Value::from_quantity(result.to_quantity());
                    },
                    // degrees
                    (_, _, Value::Number(x)) => {
                      let result = match functions.get(fnstring) {
                        Some(Some(fn_ptr)) => {
                          fn_ptr(Value::Number(*x))
                        }
                        _ => Value::Empty,
                      };
                      self.scratch.data[i][j] = result;
                    },
                    _ => (),
                  }
                }
              } 
            }
            let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.data = self.scratch.data.clone();
            self.scratch.clear();
          }
          // Infix Math
          else if parameters.len() == 2 {
            // Pass the parameters to the appropriate function
            let op_fun = match operation {
              Function::Add => operations::math_add,
              Function::Subtract => operations::math_subtract,
              Function::Multiply => operations::math_multiply,
              Function::Divide => operations::math_divide,
              Function::Power => operations::math_power,
              _ => operations::undefined, 
            };
            // Execute the function. Results are placed on the memory registers
            let (lhs_table, lhs_rows, lhs_columns) = &parameters[0];
            let (rhs_table, rhs_rows, rhs_columns) = &parameters[1];
            let out_table = &output[0];
            // TODO This seems very inefficient. Find a better way to do this. 
            // I'm having trouble getting the borrow checker to understand what I'm doing here
            let mut errors: Vec<ErrorType> = Vec::new();
            {     
              let lhs = match lhs_table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
              };
              let rhs = match rhs_table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
              };
              let lhs_rows: &Vec<Value> = match lhs_rows {
                Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
                Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
                _ => &self.lhs_rows_empty,
              };
              let rhs_rows: &Vec<Value> = match rhs_rows {
                Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
                Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
                _ => &self.rhs_rows_empty,
              };
              let lhs_columns: &Vec<Value> = match lhs_columns {
                Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
                Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
                Some(Parameter::Index(index)) => {
                  let ix = match lhs.get_column_index(index) {
                    Some(ix) => ix,
                    None => 0,
                  };
                  self.lhs_columns_empty.push(Value::from_u64(ix));
                  &self.lhs_columns_empty
                },
                _ => &self.lhs_rows_empty,
              };
              let rhs_columns: &Vec<Value> = match rhs_columns {
                Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
                Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
                Some(Parameter::Index(index)) => {
                  let ix = match rhs.get_column_index(index) {
                    Some(ix) => ix,
                    None => 0,
                  };
                  self.rhs_columns_empty.push(Value::from_u64(ix));
                  &self.rhs_columns_empty
                },
                _ => &self.rhs_columns_empty,
              };
              op_fun(lhs, lhs_rows, lhs_columns,
                     rhs, rhs_rows, rhs_columns, &mut self.scratch, &mut errors);
            }
            // If there are no errors, copy the data over
            if errors.len() == 0 {
              let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
              out.rows = self.scratch.rows;
              out.columns = self.scratch.columns;
              out.data = self.scratch.data.clone();
            } 
            // Clear scratch no matter what
            self.scratch.clear();
            self.rhs_columns_empty.clear();
            self.lhs_columns_empty.clear();
            // record error on block and quit the solve loop if there are any errors
            for error in &errors {
              self.errors.push(
                Error{
                  block: self.id as u64,
                  constraint: step.clone(),
                  error_id: error.clone(),
                }
              );
            }
            if errors.len() > 0 {
              break 'solve_loop;
            }
          }
        
        
        */},
        Constraint::Filter{comparator, lhs, rhs, output} => {
          let op_fun = match comparator {
            Comparator::NotEqual => operations::compare_not_equal,
            Comparator::Equal => operations::compare_equal,
            Comparator::LessThanEqual => operations::compare_less_than_equal,
            Comparator::GreaterThanEqual => operations::compare_greater_than_equal,
            Comparator::GreaterThan => operations::compare_greater_than,
            Comparator::LessThan => operations::compare_less_than,
            _ => operations::compare_undefined, 
          };
          let (lhs_table, lhs_rows, lhs_columns) = &lhs;
          let (rhs_table, rhs_rows, rhs_columns) = &rhs;
          let out_table = output;
          {
            let lhs = match lhs_table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
            };
            let rhs = match rhs_table {
              TableId::Local(id) => self.memory.get(*id).unwrap(),
              TableId::Global(id) => store.get_table(*id).unwrap(),
            };
            let lhs_rows: &Vec<Value> = match lhs_rows {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              _ => &self.lhs_rows_empty,
            };
            let rhs_rows: &Vec<Value> = match rhs_rows {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              _ => &self.rhs_rows_empty,
            };
            
            let lhs_columns: &Vec<Value> = match lhs_columns {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              Some(Parameter::Index(index)) => {
                let ix = match lhs.get_column_index(index) {
                  Some(ix) => ix,
                  None => 0,
                };
                self.lhs_columns_empty.push(Value::from_u64(ix));
                &self.lhs_columns_empty
              },
              _ => &self.lhs_rows_empty,
            };
            let rhs_columns: &Vec<Value> = match rhs_columns {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              Some(Parameter::Index(index)) => {
                let ix = match rhs.get_column_index(index) {
                  Some(ix) => ix,
                  None => 0,
                };
                self.rhs_columns_empty.push(Value::from_u64(ix));
                &self.rhs_columns_empty
              },
              _ => &self.rhs_columns_empty,
            };
            op_fun(lhs, lhs_rows, lhs_columns,
                    rhs, rhs_rows, rhs_columns, &mut self.scratch);
          }
          let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
          out.rows = self.scratch.rows;
          out.columns = self.scratch.columns;
          out.data = self.scratch.data.clone();
          self.scratch.clear();
          self.rhs_columns_empty.clear();
          self.lhs_columns_empty.clear();
        },
        Constraint::Logic{logic, lhs, rhs, output} => {
          let op_fun = match logic {
            Logic::And => operations::logic_and,
            Logic::Or => operations::logic_or,
            _ => operations::logic_undefined, 
          };
          let (lhs_table, lhs_rows, lhs_columns) = &lhs;
          let (rhs_table, rhs_rows, rhs_columns) = &rhs;
          let out_table = output;
          {
            let lhs = match lhs_table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
            };
            let rhs = match rhs_table {
              TableId::Local(id) => self.memory.get(*id).unwrap(),
              TableId::Global(id) => store.get_table(*id).unwrap(),
            };
            let lhs_rows: &Vec<Value> = match lhs_rows {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              _ => &self.lhs_rows_empty,
            };
            let rhs_rows: &Vec<Value> = match rhs_rows {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              _ => &self.rhs_rows_empty,
            };
            let lhs_columns: &Vec<Value> = match lhs_columns {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              Some(Parameter::Index(index)) => {
                let ix = match lhs.get_column_index(index) {
                  Some(ix) => ix,
                  None => 0,
                };
                self.lhs_columns_empty.push(Value::from_u64(ix));
                &self.lhs_columns_empty
              },
              _ => &self.lhs_rows_empty,
            };
            let rhs_columns: &Vec<Value> = match rhs_columns {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              Some(Parameter::Index(index)) => {
                let ix = match rhs.get_column_index(index) {
                  Some(ix) => ix,
                  None => 0,
                };
                self.rhs_columns_empty.push(Value::from_u64(ix));
                &self.rhs_columns_empty
              },
              _ => &self.rhs_columns_empty,
            };
            op_fun(lhs, lhs_rows, lhs_columns,
                   rhs, rhs_rows, rhs_columns, &mut self.scratch);
          }
          let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
          out.rows = self.scratch.rows;
          out.columns = self.scratch.columns;
          out.data = self.scratch.data.clone();
          self.scratch.clear();
          self.rhs_columns_empty.clear();
          self.lhs_columns_empty.clear();
        },
        Constraint::Range{table, start, end} => {
          {
            let start_value = &self.memory.get(*start.unwrap()).unwrap().data[0][0].as_u64().unwrap().clone();
            let end_value = &self.memory.get(*end.unwrap()).unwrap().data[0][0].as_u64().unwrap().clone();
            self.scratch.grow_to_fit(end_value - start_value + 1, 1);
            let mut row = 1;
            for i in *start_value..*end_value + 1 {
              self.scratch.set_cell(&Index::Index(row), &Index::Index(1), Value::from_u64(i));
              row += 1;
            }
          }
          let out = self.memory.get_mut(*table.unwrap()).unwrap();
          out.data = self.scratch.data.clone();
          out.rows = self.scratch.rows;
          out.columns = self.scratch.columns;
          self.scratch.clear();
        },
        Constraint::Insert{from, to} => {/*
          
          let (from_table, from_ixes) = from;
          let (to_table, to_ixes) = to;

          let from = match from_table {
            TableId::Local(id) => self.memory.get(*id).unwrap(),
            TableId::Global(id) => store.get_table(*id).unwrap(),
          };

          let (to, to_table_id) = match to_table {
            TableId::Local(id) => (self.memory.get(*id).unwrap(), id.clone()),
            TableId::Global(id) => (store.get_table(*id).unwrap(), id.clone()),
          };

          let from_column_values: &Vec<Value> = match &from_ixes[1] {
            Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
            Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
            Some(Parameter::Index(index)) => {
              let ix = match from.get_column_index(&index) {
                Some(ix) => ix,
                None => 0,
              };
              self.rhs_columns_empty.push(Value::from_u64(ix));
              &self.rhs_columns_empty
            },
            _ => &self.rhs_columns_empty,
          };

          let from_row_values: &Vec<Value> = match &from_ixes[0] {
            Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
            Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
            Some(Parameter::Index(index)) => {
              let ix = match from.get_row_index(&index) {
                Some(ix) => ix,
                None => 0,
              };
              self.rhs_rows_empty.push(Value::from_u64(ix));
              &self.rhs_rows_empty
            },
            _ => &self.rhs_rows_empty,
          };


         // If we only have one index, it's like this #x{3} := ...
          let one = vec![Value::from_u64(1)];
          let (to_row_values, to_column_values) = if to_ixes.len() == 1 {
            // Get the ixes
            let ixes: &Vec<Value> = match &to_ixes[0] {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              _ => &self.rhs_rows_empty,
            };
            // Now the other dimension index will be one.
            // So if it's #x{3} := 7 where #x = [1 2 3], then it translates to #x{1,3} := 7
            // If #x = [1; 2; 3] then it translates to #x{3,1} := 7
            let (row_ixes, column_ixes) = match (to.rows, to.columns) {
              (1, columns) => (&one, ixes),
              (rows, 1) => (ixes, &one),
              _ => {
                // TODO Report an error here... or do matlab style ind2sub
                break 'solve_loop;
              }
            };
            (row_ixes, column_ixes)
          // Otherwise we have a couple choices:
          // #x{1,2}
          } else {
            let to_column_values: &Vec<Value> = match &to_ixes[1] {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              Some(Parameter::Index(index)) => {
                let ix = match to.get_column_index(&index) {
                  Some(ix) => ix,
                  None => 0,
                };
                self.lhs_columns_empty.push(Value::from_u64(ix));
                &self.lhs_columns_empty
              },
              _ => &self.lhs_columns_empty,
            };

            let to_row_values: &Vec<Value> = match &to_ixes[0] {
              Some(Parameter::TableId(TableId::Local(id))) => &self.memory.get(*id).unwrap().data[0],
              Some(Parameter::TableId(TableId::Global(id))) => &store.get_table(*id).unwrap().data[0],
              Some(Parameter::Index(index)) => {
                let ix = match to.get_row_index(&index) {
                  Some(ix) => ix,
                  None => 0,
                };
                self.lhs_rows_empty.push(Value::from_u64(ix));
                &self.lhs_rows_empty
              },
              _ => &self.lhs_rows_empty,
            };
            (to_row_values, to_column_values)
          };

          let to_width = if to_column_values.is_empty() { to.columns }
                         else { to_column_values.len() as u64 };
          let from_width = if from_column_values.is_empty() { from.columns }
                           else { from_column_values.len() as u64 };      
          let to_height = if to_row_values.is_empty() { to.rows }
                          else { to_row_values.len() as u64 };
          let from_height = if from_row_values.is_empty() { from.rows }
                            else { from_row_values.len() as u64 };

          let to_is_scalar = to_width == 1 && to_height == 1;
          let from_is_scalar = from_width == 1 && from_height == 1;
          // TODO MAKE THIS REAL
          if from_is_scalar {
            for i in 0..to_width as usize {
              let cix = if to_column_values.is_empty() { i }
                        else { to_column_values[i].as_u64().unwrap() as usize - 1 };
              for j in 0..to_height as usize {
                // If to_row_values are empty, it means we're matching over all the rows
                // otherwise we take the truth value from the vector and use it
                let truth = if to_row_values.is_empty() {
                  &Value::Bool(true)
                } else {
                  &to_row_values[j]
                };
                match truth {
                  Value::Bool(true) => {
                    let change = Change::Set{table: to_table_id.clone(), 
                                             row: Index::Index(j as u64 + 1), 
                                             column: Index::Index(cix as u64 + 1),
                                             value: from.data[0][0].clone() 
                                            };
                    self.block_changes.push(change);
                  },
                  Value::Number(index) => {
                    let ix = index.mantissa() as usize;
                    if ix <= to.rows as usize {
                      let change = Change::Set{table: to_table_id.clone(), 
                                              row: Index::Index(ix as u64), 
                                              column: Index::Index(cix as u64 + 1),
                                              value: from.data[0][0].clone() 
                                              };
                      self.block_changes.push(change); 
                    }
                  }
                  _ => (),
                }
              }
            }
          // from and to are the same size
          } else if to_height == from_height && to_width == from_width {
            for i in 0..from_width as usize {
              let fcix = if from_column_values.is_empty() { i }
                         else {
                           match from_column_values[i] {
                             Value::Number(x) => x.mantissa() as usize  - 1,
                             Value::Bool(true) => i,
                             _ => {continue; 0}, // This continues before the return
                           }
                         };
              let tcix = if to_column_values.is_empty() { i }
                         else {
                           match to_column_values[i] {
                             Value::Number(x) => x.mantissa() as usize  - 1,
                             Value::Bool(true) => i,
                             _ => {continue; 0},
                           }
                         };
              for j in 0..from_height as usize {
                let frix = if from_row_values.is_empty() { j }
                           else {
                             match from_row_values[j] {
                               Value::Number(x) => x.mantissa() as usize  - 1,
                               Value::Bool(true) => j,
                               _ => {continue; 0},
                             }
                           };

                let trix = if to_row_values.is_empty() { j }
                           else {
                             match to_row_values[j] {
                               Value::Number(x) => x.mantissa() as usize  - 1,
                               Value::Bool(true) => j,
                               _ => {continue; 0},
                             }
                           };
                let change = Change::Set{table: to_table_id.clone(), 
                                          row: Index::Index(trix as u64 + 1), 
                                          column: Index::Index(tcix as u64 + 1),
                                          value: from.data[fcix][frix].clone() 
                                        };
                self.block_changes.push(change);
              }
            }
          }
          
          self.rhs_columns_empty.clear();
          self.lhs_columns_empty.clear();
          self.rhs_rows_empty.clear();
          self.lhs_rows_empty.clear();
        */},
        Constraint::Append{from_table, to_table} => {
          let from = match from_table {
            TableId::Local(id) => self.memory.get(*id).unwrap(),
            TableId::Global(id) => store.get_table(*id).unwrap(),
          };

          let (to, to_id) = match to_table {
            TableId::Local(id) => (self.memory.get(*id).unwrap(), id),
            TableId::Global(id) => (store.get_table(*id).unwrap(), id),
          };

          let from_width = from.columns;
          let to_width = to.columns;

          if from_width == to_width {
            for i in 0..from_width as usize {
              for j in 0..from.rows as usize {
                self.block_changes.push(Change::Set{table: *to_id, row: Index::Index((j as u64 + to.rows) + 1), column: Index::Index(i as u64 + 1), value: from.data[i][j].clone() });
              }
            }
          }
        },
        Constraint::CopyTable{from_table, to_table} => {
          let mut from_table_ref = self.memory.get(*from_table).unwrap();
          let mut changes = vec![Change::NewTable{id: *to_table, rows: from_table_ref.rows, columns: from_table_ref.columns}];
          for (alias, ix) in from_table_ref.column_aliases.iter() {
            changes.push(Change::RenameColumn{table: *to_table, column_ix: *ix, column_alias: *alias});
          }
          for (col_ix, column) in from_table_ref.data.iter().enumerate() {
            for (row_ix, data) in column.iter().enumerate() {
              match data {
                Value::Reference(id) => {
                  copy_tables.push(*id);
                }, 
                _ => (),
              }
              changes.push(Change::Set{table: *to_table, row: Index::Index(row_ix as u64 + 1), column: Index::Index(col_ix as u64 + 1), value: data.clone()});
            }
          }
          //println!("{:?}", self);
          self.block_changes.append(&mut changes);
        },
        Constraint::NewTable{id, rows, columns} => {
          match id {
            TableId::Global(id) => {
              self.block_changes.push(Change::NewTable{id: *id, rows: *rows, columns: *columns});
            }
            _ => (),
          }
        },
        _ => (),
      } 
      
    }
    for c in copy_tables {
      self.copy_table(c, c, store);
    }

    if self.errors.len() > 0 {
      self.state = BlockState::Error;
    } else {
      store.process_transaction(&Transaction::from_changeset(self.block_changes.clone()));
      self.updated = true;
    }
    self.block_changes.clear();
  }

  fn copy_table(&mut self, from_table: TableId, to_table: TableId, store: &Interner) {
    let mut copy_tables: Vec<TableId> = vec![];
    let mut from_table_ref = match from_table {
      TableId::Local(id) => {
        match self.memory.get(id) {
          None => store.get_table(id).unwrap(),
          Some(table) => table,
        }
      },
      TableId::Global(id) => store.get_table(id).unwrap(),
    };
    let to_table_id = match to_table {
      TableId::Local(id) => id,
      TableId::Global(id) => id,
    };
    let mut changes = vec![Change::NewTable{id: to_table_id, rows: from_table_ref.rows, columns: from_table_ref.columns}];
    for (alias, ix) in from_table_ref.column_aliases.iter() {
      changes.push(Change::RenameColumn{table: to_table_id, column_ix: *ix, column_alias: *alias});
    }
    for (col_ix, column) in from_table_ref.data.iter().enumerate() {
      for (row_ix, data) in column.iter().enumerate() {
        match data {
          Value::Reference(id) => {
            copy_tables.push(*id);
          }, 
          _ => (),
        }
        changes.push(Change::Set{table: to_table_id, row: Index::Index(row_ix as u64 + 1), column: Index::Index(col_ix as u64 + 1), value: data.clone()});
      }
    }
    for c in copy_tables {
      self.copy_table(c, c, store);
    }
    self.block_changes.append(&mut changes);
  }
}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "┌────────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Block {:?} ({:#x})\n", self.name, self.id).unwrap();
    write!(f, "├────────────────────────────────────────┤\n").unwrap();
    write!(f, "│ \n{}\n",self.text).unwrap();
    write!(f, "├────────────────────────────────────────┤\n").unwrap();
    write!(f, "│ Errors:\n").unwrap();
    write!(f, "│ {:?}\n", self.errors).unwrap();
    write!(f, "├────────────────────────────────────────┤\n").unwrap();
    write!(f, "│ State: {:?}\n", self.state).unwrap();
    write!(f, "│ Ready: {:?}\n", self.ready).unwrap();
    write!(f, "│ Updated: {:?}\n", self.updated).unwrap();
    write!(f, "│ Input: {:?}\n", self.input_registers.len()).unwrap();
    for (ix, register) in self.input_registers.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, register).unwrap();
    }
    write!(f, "│ Output: {:?}\n", self.output_registers.len()).unwrap();
    for (ix, register) in self.output_registers.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, register).unwrap();
    }
    write!(f, "│ Constraints: {:?}\n", self.constraints.len()).unwrap();
    for (ix, (text, constraint)) in self.constraints.iter().enumerate() {
      write!(f, "│  {}. {}\n", ix + 1, text).unwrap();
      for constraint_step in constraint {
        write!(f, "│    > {:?}\n", constraint_step).unwrap();
      }
    }
    write!(f, "│ Plan: {:?}\n", self.plan.len()).unwrap();
    for (ix, step) in self.plan.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, step).unwrap();
    }
    write!(f, "└────────────────────────────────────────┘\n").unwrap();
    write!(f, "{:?}\n", self.memory).unwrap();
    Ok(())
  }
}

// ## Pipe

// Pipes are conduits of records between blocks.

/*pub struct Pipe {
  input: Address,
  output: Address,
}*/

// ## Constraints

// Constraints put bounds on the data available for a block to work with. For 
// example, Scan constraints could bring data into the block, and a Join 
// constraint could match elements from one table to another.

#[derive(Clone, PartialEq)]
pub enum Constraint {
  NewTable{id: TableId, rows: u64, columns: u64},
  TableColumn{table: u64, column_ix: u64, column_alias: u64},
  // Input Constraints
  Reference{table: TableId, destination: u64},
  Scan {table: TableId, indices: Vec<(Option<Parameter>, Option<Parameter>)>, output: TableId},
  ChangeScan {table: TableId, column: Vec<Option<Parameter>>},
  Identifier {id: u64, text: String},
  Range{table: TableId, start: TableId, end: TableId},
  // Transform Constraints
  Filter {comparator: operations::Comparator, lhs: (TableId, Option<Parameter>, Option<Parameter>), rhs: (TableId, Option<Parameter>, Option<Parameter>), output: TableId},
  Logic {logic: operations::Logic, lhs: (TableId, Option<Parameter>, Option<Parameter>), rhs: (TableId, Option<Parameter>, Option<Parameter>), output: TableId},
  Function {operation: operations::Function, fnstring: String, parameters: Vec<(String, TableId, Vec<(Option<Parameter>, Option<Parameter>)>)>, output: Vec<TableId>},
  Constant {table: TableId, row: Index, column: Index, value: Quantity, unit: Option<String>},
  String {table: TableId, row: Index, column: Index, value: String},
  // Identity Constraints
  CopyTable {from_table: u64, to_table: u64},
  AliasTable {table: TableId, alias: u64},
  // Output Constraints
  Insert {from: (TableId, Vec<(Option<Parameter>,Option<Parameter>)>), to: (TableId, Vec<(Option<Parameter>,Option<Parameter>)>)},
  Append {from_table: TableId, to_table: TableId},
  Empty{table: TableId, row: Index, column: Index},
  Null,
}

impl fmt::Debug for Constraint {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Constraint::Reference{table, destination} => write!(f, "Reference({:?} -> {:#x})", table, destination),
      Constraint::NewTable{id, rows, columns} => write!(f, "NewTable(#{:?}({:?}x{:?}))", id, rows, columns),
      Constraint::Scan{table, indices, output} => write!(f, "Scan(#{:?}({:?}) -> {:?})", table, indices, output),
      Constraint::ChangeScan{table, column} => write!(f, "ChangeScan(#{:?}({:?}))", table, column),
      Constraint::Filter{comparator, lhs, rhs, output} => write!(f, "Filter({:?} {:?} {:?} -> {:?})", lhs, comparator, rhs, output),
      Constraint::Logic{logic, lhs, rhs, output} => write!(f, "Logic({:?} {:?} {:?} -> {:?})", lhs, logic, rhs, output),
      Constraint::Function{operation, fnstring, parameters, output} => write!(f, "Fxn::{:?}{:?} -> {:?}", operation, parameters, output),
      Constraint::Constant{table, row, column, value, unit} => write!(f, "Constant({}{:?} -> #{:?})", value.to_float(), unit, table),
      Constraint::String{table, row, column, value} => write!(f, "String({:?} -> #{:?})", value, table),
      Constraint::CopyTable{from_table, to_table} => write!(f, "CopyTable({:#x} -> {:#x})", from_table, to_table),
      Constraint::AliasTable{table, alias} => write!(f, "AliasTable({:?} -> {:#x})", table, alias),
      Constraint::Identifier{id, text} => write!(f, "Identifier(\"{}\" = {:#x})", text, id),
      Constraint::Insert{from, to} => write!(f, "Insert({:?} -> {:?})",  from, to),
      Constraint::Append{from_table, to_table} => write!(f, "Append({:?} -> {:?})", from_table, to_table),
      Constraint::TableColumn{table, column_ix, column_alias}  => write!(f, "TableColumn(#{:#x}({:#x}) -> {:#x})",  table, column_ix, column_alias),
      Constraint::Range{table, start, end} => write!(f, "Range({:?} -> {:?} to {:?})", table, start, end),
      Constraint::Empty{table, row, column} => write!(f, "Empty -> #{:?} {:?} {:?}", table, row, column),
      Constraint::Null => write!(f, "Null"),
    }
  }
}

// ## Bit helpers

// Lifted from Eve v0.4

pub fn check_bits(solved: u64, checking: u64) -> bool {
    solved & checking == checking
}

pub fn has_any_bits(solved: u64, checking: u64) -> bool {
    solved & checking != 0
}

pub fn set_bit(solved: u64, bit: usize) -> u64 {
    solved | (1 << bit)
}

pub fn clear_bit(solved: u64, bit: usize) -> u64 {
    solved & !(1 << bit)
}

pub fn check_bit(solved: u64, bit: usize) -> bool {
    solved & (1 << bit) != 0
}
