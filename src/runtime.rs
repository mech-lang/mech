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
use operations::{set_any, table_vertical_concatenate, table_horizontal_concatenate, logic_and, logic_or, table_range, stats_sum, math_add, math_subtract, math_multiply, math_divide, compare_equal, compare_greater_than, compare_greater_than_equal, compare_less_than, compare_less_than_equal, compare_not_equal, Parameter};
use quantities::{Quantity, ToQuantity, QuantityMath, make_quantity};
use errors::{Error, ErrorType};
use std::rc::Rc;
use std::cell::RefCell;

// ## Runtime
pub struct Runtime {
  pub blocks: HashMap<usize, Block>,
  pub pipes_map: HashMap<Register, HashSet<Address>>,
  pub output: HashSet<Register>,
  pub tables_map: HashMap<u64, u64>,
  pub ready_blocks: HashSet<usize>,
  pub functions: HashMap<String, Option<extern "C" fn(Vec<(String, Table)>)->Table>>,
  pub changed_this_round: HashSet<(u64, Index)>,
  pub errors: Vec<Error>,
}

impl Runtime {

  pub fn new() -> Runtime {
    let mut runtime = Runtime {
      blocks: HashMap::new(),
      ready_blocks: HashSet::new(),
      pipes_map: HashMap::new(),
      output: HashSet::new(),
      tables_map: HashMap::new(),
      functions: HashMap::new(),
      changed_this_round: HashSet::new(),
      errors: Vec::new(),
    };
    runtime.functions.insert("math/add".to_string(),Some(math_add));
    runtime.functions.insert("math/multiply".to_string(),Some(math_multiply));
    runtime.functions.insert("math/divide".to_string(),Some(math_divide));
    runtime.functions.insert("math/subtract".to_string(),Some(math_subtract));
    runtime.functions.insert("compare/greater-than".to_string(),Some(compare_greater_than));
    runtime.functions.insert("compare/less-than".to_string(),Some(compare_less_than));
    runtime.functions.insert("compare/greater-than-equal".to_string(),Some(compare_greater_than_equal));
    runtime.functions.insert("compare/less-than-equal".to_string(),Some(compare_less_than_equal));
    runtime.functions.insert("compare/equal".to_string(),Some(compare_equal));
    runtime.functions.insert("compare/not-equal".to_string(),Some(compare_not_equal));
    runtime.functions.insert("stats/sum".to_string(),Some(stats_sum));
    runtime.functions.insert("table/range".to_string(),Some(table_range));
    runtime.functions.insert("logic/and".to_string(),Some(logic_and));
    runtime.functions.insert("logic/or".to_string(),Some(logic_or));
    runtime.functions.insert("table/horizontal-concatenate".to_string(),Some(table_horizontal_concatenate));
    runtime.functions.insert("table/vertical-concatenate".to_string(),Some(table_vertical_concatenate));
    runtime.functions.insert("set/any".to_string(),Some(set_any));
    runtime
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
      if self.output.contains(&register) && store.contains(table) {
        block.ready.insert(register.clone());
      }
    }

    for register in block.output_registers.iter() {
      self.output.insert(register.clone());
    }

    // Record the functions used in block
    for fun in &block.functions {
      self.functions.entry(fun.to_string()).or_insert(None);
    }

    // Register all local tables in the tables map
    for local_table in block.memory.map.keys() {
      self.tables_map.insert(*local_table, block.id as u64);
    }
    // Register all errors on the block with the runtime
    self.errors.append(&mut block.errors.clone());

    // Mark the block as ready for execution on the next available cycle
    if block.is_ready() {
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
        let register = Register::new(TableId::Global(table),column);
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
    write!(f, "@(block: {:x}, register: {:?})", self.block, self.register)
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Register {
  pub table: TableId,
  pub column: Index,
}

impl Register {
  
  pub fn new(table: TableId, column: Index) -> Register { 
    Register {
      table: table,
      column: column,
    }
  }

}

impl fmt::Debug for Register {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:?}, {:?})", self.table, self.column)
  }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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
  tables_modified: HashSet<u64>,
  scratch: Table,
  lhs_rows_empty: Vec<Value>,
  lhs_columns_empty: Vec<Value>,
  rhs_rows_empty: Vec<Value>,
  rhs_columns_empty: Vec<Value>,
  block_changes: Vec<Change>,
  current_step: Option<Constraint>,
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
      tables_modified: HashSet::new(),
      errors: Vec::new(),
      scratch: Table::new(0,0,0),
      // allocate empty indices so we don't have to do this on each iteration
      lhs_rows_empty: Vec::new(),
      lhs_columns_empty: Vec::new(),
      rhs_rows_empty: Vec::new(),
      rhs_columns_empty: Vec::new(),
      block_changes: Vec::new(),
      current_step: None,
    }
  }

  pub fn get_table(&self, table_id: u64) -> Option<&Rc<RefCell<Table>>> {
    self.memory.get(table_id)
  } 

  pub fn gen_block_id(&self) -> usize {
    let mut constraint_string = String::new();
    for constraint in &self.constraints {
      constraint_string = format!("{}{:?}",constraint_string, constraint);
    }
    Hasher::hash_string(constraint_string) as usize
  }

  pub fn add_constraints(&mut self, constraint_tuple: (String, Vec<Constraint>)) {
    self.constraints.push(constraint_tuple.clone());
    let (constraint_text, constraints) = constraint_tuple;

    // Add relevant constraints to plan
    let mut reversed = constraints.clone();
    reversed.reverse();
    for constraint in reversed {
      match constraint {
        Constraint::Function{..} |
        Constraint::CopyTable{..} |
        Constraint::DefineTable{..} |
        Constraint::Whenever{..} |
        Constraint::Wait{..} |
        Constraint::Until{..} |
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
          self.output_registers.insert(Register::new(TableId::Global(*to_table), Index::Index(0)));
        },
        Constraint::DefineTable{from_table, to_table} => {
          self.output_registers.insert(Register::new(TableId::Global(*to_table), Index::Index(0)));
        },
        Constraint::Append{from_table, to_table} => {
          match to_table {
            TableId::Global(id) => {
              //self.input_registers.insert(Register::new(*to_table, Index::Index(0)));
              self.output_registers.insert(Register::new(*to_table, Index::Index(0)));
            }
            _ => (),
          };
        },
        Constraint::Insert{from: (from_table, ..), to: (to_table, ..)} => {
          match to_table {
            TableId::Global(id) => {
              self.input_registers.insert(Register::new(*to_table, Index::Index(0)));
              self.output_registers.insert(Register::new(*to_table, Index::Index(0)));
            }, 
            _ => (),
          };
        },
        Constraint::Scan{table, indices, output} => {
          match table {
            TableId::Global(id) => {
              self.input_registers.insert(Register{table: *table, column: Index::Index(0)});
            },
            _ => (),
          }
        },
        Constraint::Whenever{tables} => {
          for (table, indices) in tables {
            match (table, indices) {
              (TableId::Global(id), x) => {
                let column = match x[0] {
                  (None, Some(Parameter::Index(index))) => index,
                  _ => Index::Index(0),
                };
                self.input_registers.insert(Register{table: *table, column});
              },
              (TableId::Local(id), _) => {
                self.input_registers.insert(Register{table: *table, column: Index::Index(0)});
              },
              _ => (),
            }
          }
        },
        Constraint::Wait{tables} => {
          for (table, indices) in tables {
            match (table, indices) {
              (TableId::Global(id), x) => {
                let column = match x[0] {
                  (None, Some(Parameter::Index(index))) => index,
                  _ => Index::Index(0),
                };
                self.input_registers.insert(Register{table: *table, column});
              },
              (TableId::Local(id), _) => {
                self.input_registers.insert(Register{table: *table, column: Index::Index(0)});
              },
              _ => (),
            }
          }
        },
        Constraint::Until{tables} => {
          for (table, indices) in tables {
            match (table, indices) {
              (TableId::Global(id), x) => {
                let column = match x[0] {
                  (None, Some(Parameter::Index(index))) => index,
                  _ => Index::Index(0),
                };
                self.input_registers.insert(Register{table: *table, column});
              },
              (TableId::Local(id), _) => {
                self.input_registers.insert(Register{table: *table, column: Index::Index(0)});
              },
              _ => (),
            }
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
        Constraint::Function{fnstring, parameters, output} => {
          self.functions.insert(fnstring.to_string());
          for (arg_name, table, indices) in parameters {
            match (table, indices) {
              (TableId::Global(id), x) => {
                let column = match x[0] {
                  (None, Some(Parameter::Index(index))) => index,
                  _ => Index::Index(0),
                };
                self.input_registers.insert(Register{table: *table, column});
              },
              _ => (),
            }
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
              table_ref.borrow_mut().set_cell(&row, &column, Value::Empty);
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
              let table_ref = o.get();
              table_ref.borrow_mut().set_cell(&row, &column, Value::from_quantity(test));
            },
            Entry::Vacant(v) => {    
            },
          };  
          self.updated = true;
        },
        Constraint::Reference{table, destination} => {
          match self.memory.map.entry(*destination) {
            Entry::Occupied(mut o) => {
              let table_ref = o.get();
              table_ref.borrow_mut().set_cell(&Index::Index(1), &Index::Index(1), Value::Reference(*table));
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
              let table_ref = o.get();
              table_ref.borrow_mut().set_cell(&row, &column, Value::from_string(value.clone()));
            },
            Entry::Vacant(v) => {    
            },
          };
          self.updated = true;
        },
        Constraint::TableColumn{table, column_ix, column_alias} => {
          match self.memory.get(*table) {
            Some(table_ref) => {
              table_ref.borrow_mut().set_column_alias(*column_alias, *column_ix);
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
        // Mark as unsatisfied if there are any global tables still waiting.
        // Dependency on a local table is still possible.
        let mut result = true;
        for x in set_diff.iter() {
          match x {
            Register{table: TableId::Global(y), ..} => {
              self.state = BlockState::Unsatisfied;
              result = false;
            },
            _ => (),
          }
        }
        result

      }
    }    
  }

  pub fn resolve_subscript(&mut self, store: &mut Interner, table: &TableId, indices: &Vec<(Option<Parameter>, Option<Parameter>)>) -> Table {
    let mut old: Table = Table::new(0,0,0);
    let mut table_id: TableId = table.clone();
    'solve_loop: for index in indices {
      let mut table_ref = match table_id {
        TableId::Local(id) => match self.memory.get(id) {
          Some(id) => id.borrow(),
          None => store.get_table(id).unwrap().borrow(),
        },
        TableId::Global(id) => store.get_table(id).unwrap().borrow(),
      };
      let one = vec![Value::from_u64(1)];
      let (row_ixes, column_ixes) = match index {
        // If we only have one index, we have two options:
        // #x{3}
        (Some(parameter), None) => {
          // Get the ixes
          let ixes: &Vec<Value> = match &parameter {
            Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
            Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
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
            _ => &self.rhs_rows_empty,
          };
          // The other dimension index will be one.
          // So if it's #x{3} where #x = [1 2 3], then it translates to #x{1,3}
          // If #x = [1; 2; 3] then it translates to #x{3,1}
          let (row_ixes, column_ixes) = match (table_ref.rows, table_ref.columns) {
            (1, columns) => (&one, ixes),
            (rows, 1) => (ixes, &one),
            _ => (&self.rhs_rows_empty, ixes),
            _ => {
              // TODO Report an error here... or do matlab style ind2sub
              break 'solve_loop;
            }
          };
          (row_ixes, column_ixes)
        },
        // #x.y
        (None, Some(parameter)) => {
          // Get the ixes
          let ixes: &Vec<Value> = match &parameter {
            Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
            Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
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
            _ => &self.rhs_rows_empty,
          };
          (&self.lhs_rows_empty, ixes)
        },
        // Otherwise we have a couple choices:
        // #x{1,2}
        // #x.y{1}
        (Some(row_parameter), Some(column_parameter)) => {
          let row_ixes: &Vec<Value> = match &row_parameter {
            Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
            Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
            _ => &self.rhs_rows_empty,
          };
          let column_ixes: &Vec<Value> = match &column_parameter {
            Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
            Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
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
        },
        (Some(parameter), Some(Parameter::All)) => {
          let ixes: &Vec<Value> = match &parameter {
            Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
            Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
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
            _ => &self.rhs_rows_empty,
          };
          (ixes ,&self.rhs_columns_empty)
        },
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
      match &old.index(&Index::Index(1),&Index::Index(1)) {
        Some(Value::Reference(id)) => {
          // Make block depend on reference
          match self.current_step {
            Some(Constraint::Scan{..}) => {
              let register = Register{table: TableId::Global(*id.unwrap()), column: Index::Index(0)};
              self.ready.insert(register.clone());
              self.input_registers.insert(register.clone());
            }
            _ => (),
          };
          // Swap the old table with a new id
          table_id = id.clone()
        },
        _ => (),
      };
    }
    self.rhs_columns_empty.clear();
    self.lhs_columns_empty.clear();
    let out = old.clone();
    self.scratch.clear();
    out
  }

  pub fn solve(&mut self, store: &mut Interner, functions: &HashMap<String, Option<extern "C" fn(Vec<(String, Table)>)->Table>>) {
    let block = self as *mut Block;
    let mut copy_tables: HashSet<TableId> = HashSet::new();
    self.tables_modified.clear();
    'solve_loop: for step in &self.plan {
      self.current_step = Some(step.clone());
      match step {
        Constraint::Scan{table, indices, output} => {
          let out_table = &output;
          let scanned;
          unsafe {
            scanned = (*block).resolve_subscript(store,table,indices);
          }
          let mut out = self.memory.get(*out_table.unwrap()).unwrap().borrow_mut();
          out.rows = scanned.rows;
          out.columns = scanned.columns;
          out.data = scanned.data.clone();
          out.column_index_to_alias = scanned.column_index_to_alias.clone();
          out.column_aliases = scanned.column_aliases.clone();
          self.tables_modified.insert(out.id);
          self.scratch.clear();
          self.rhs_columns_empty.clear();
          self.lhs_columns_empty.clear();
        },
        Constraint::Whenever{tables} => {
          for (table, indices) in tables {
            match (table, indices.as_slice()) {
              (TableId::Global(id), [(None, Some(Parameter::Index(index)))]) => {
                let register = Register{table: TableId::Global(*id), column: index.clone()};
                self.ready.remove(&register);
              }
              (TableId::Global(id), _) => {
                let register = Register{table:TableId::Global(*id), column: Index::Index(0)};
                self.ready.remove(&register);
              }
              /*
              (TableId::Global(id), [None, None]) => {
                let register = Register{table: *id, column: Index::Index(0)};
                self.ready.remove(&register);
              }*/
              (TableId::Local(id), _) => {
                // test value at table
                let table = self.memory.get(*id).unwrap().borrow_mut();
                if table.data[0][0] == Value::Bool(false) || self.tables_modified.get(id) == None {
                  self.block_changes.clear();
                  self.state = BlockState::Unsatisfied;
                  break 'solve_loop;
                }
              },
              _ => (),
            }
          }
        },
        Constraint::Wait{tables} => {
          for (table, indices) in tables {
            match (table, indices.as_slice()) {
              (TableId::Global(id), [(None, Some(Parameter::Index(index)))]) => {
                let register = Register{table: TableId::Global(*id), column: index.clone()};
                //self.ready.remove(&register);
              }
              (TableId::Global(id), _) => {
                let register = Register{table:TableId::Global(*id), column: Index::Index(0)};
                //self.ready.remove(&register);
              }
              /*
              (TableId::Global(id), [None, None]) => {
                let register = Register{table: *id, column: Index::Index(0)};
                self.ready.remove(&register);
              }*/
              (TableId::Local(id), _) => {
                // test value at table
                let table = self.memory.get(*id).unwrap().borrow_mut();
                if table.data[0][0] == Value::Bool(true) {
                  self.state = BlockState::Ready;
                  break 'solve_loop;
                }
              },
              _ => (),
            }
          }
        },
        Constraint::Until{tables} => {
          for (table, indices) in tables {
            match (table, indices.as_slice()) {
              (TableId::Global(id), [(None, Some(Parameter::Index(index)))]) => {
                let register = Register{table: TableId::Global(*id), column: index.clone()};
                self.ready.remove(&register);
              }
              (TableId::Global(id), _) => {
                let register = Register{table:TableId::Global(*id), column: Index::Index(0)};
                self.ready.remove(&register);
              }
              /*
              (TableId::Global(id), [None, None]) => {
                let register = Register{table: *id, column: Index::Index(0)};
                self.ready.remove(&register);
              }*/
              (TableId::Local(id), _) => {
                // test value at table
                let table = self.memory.get(*id).unwrap().borrow_mut();
                if table.data[0][0] == Value::Bool(true) {
                  self.block_changes.clear();
                  self.state = BlockState::Unsatisfied;
                  break 'solve_loop;
                }
              },
              _ => (),
            }
          }
        },
        Constraint::Function{fnstring, parameters, output} => {
          if *fnstring == "table/split" {
            let out_table = &output[0];
            let (_, in_table, _) = &parameters[0];
            let table_ref = match in_table {
              TableId::Local(id) => self.memory.get(*id).unwrap().borrow(),
              TableId::Global(id) => store.get_table(*id).unwrap().borrow(),
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
              unsafe{
                (*block).memory.insert(new_table);
              }
              self.scratch.data[0][i] = Value::Reference(TableId::Local(id));
            }
            let mut out = self.memory.get(*out_table.unwrap()).unwrap().borrow_mut();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.data = self.scratch.data.clone();
            self.tables_modified.insert(out.id);
            self.scratch.clear();
          } else {
            let out_table = &output[0];
            let arguments = parameters.iter().map(|(arg_name, table_id, indices)| {
              let table_ref: Table;
              unsafe {
                table_ref = (*block).resolve_subscript(store,table_id,indices);
              }
              (arg_name.clone(), table_ref)
            }).collect::<Vec<_>>();
            let result = match functions.get(fnstring) {
              Some(Some(fn_ptr)) => {
                fn_ptr(arguments)
              }
              _ => Table::new(0,1,1),
            };
            self.scratch = result;
            let mut out = self.memory.get(*out_table.unwrap()).unwrap().borrow_mut();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.data = self.scratch.data.clone();
            self.tables_modified.insert(out.id);
            self.scratch.clear();
          }
        },
        Constraint::Insert{from, to} => {
                  
          let (from_table, from_ixes) = from;
          let (to_table, to_ixes) = to;

          let to_table_id = match to_table {
            TableId::Global(id) => id.clone(),
            TableId::Local(id) => 0,
          };

          let from_table_ref: Table;
          unsafe {
            from_table_ref = (*block).resolve_subscript(store,from_table,from_ixes);
          } 

          let to_table_ref: Table;
          unsafe {
            to_table_ref = (*block).resolve_subscript(store,to_table,&vec![(None, None)]);
          } 

          // TODO This thing is a little hacky. It merges two subscripts together. Maybe this should be in the compiler?
          let to_ixes = if to_ixes.len() == 2 {
            match (to_ixes[0], to_ixes[1]) {
              ((None, x),(y,None)) => {
                (y,x)
              }
              _ => to_ixes[0]
            }
          } else {
            to_ixes[0]
          };

          let one = vec![Value::from_u64(1)];
          let (to_row_values, to_column_values) = match to_ixes {
            // If we only have one index, we have two options:
            // #x{3}
            // #x.y
            (None, Some(parameter)) |
            (Some(parameter), None) => {
              // Get the ixes
              let ixes: &Vec<Value> = match &parameter {
                Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
                Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
                Parameter::Index(index) => {
                  let ix = match to_table_ref.get_column_index(index) {
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
                _ => &self.rhs_rows_empty,
              };
              // The other dimension index will be one.
              // So if it's #x{3} where #x = [1 2 3], then it translates to #x{1,3}
              // If #x = [1; 2; 3] then it translates to #x{3,1}
              let (row_ixes, column_ixes) = match (to_table_ref.rows, to_table_ref.columns) {
                (1, columns) => (&one, ixes),
                (rows, 1) => (ixes, &one),
                _ => (&self.rhs_rows_empty, ixes),
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
                Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
                Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
                _ => &self.rhs_rows_empty,
              };
              let column_ixes: &Vec<Value> = match &column_parameter {
                Parameter::TableId(TableId::Local(id)) => unsafe{&(*self.memory.get(*id).unwrap().as_ptr()).data[0]},
                Parameter::TableId(TableId::Global(id)) => unsafe{&(*store.get_table(*id).unwrap().as_ptr()).data[0]},
                Parameter::Index(index) => {
                  let ix = match to_table_ref.get_column_index(index) {
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

          let to_width = if to_column_values.is_empty() { to_table_ref.columns }
                         else { to_column_values.len() as u64 };     
          let to_height = if to_row_values.is_empty() { to_table_ref.rows }
                          else { to_row_values.len() as u64 };
          let from_height = from_table_ref.rows;
          let from_width = from_table_ref.columns;

          let to_is_scalar = to_width == 1 && to_height == 1;
          let from_is_scalar = from_table_ref.columns == 1 && from_table_ref.rows == 1;

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
                                             value: from_table_ref.data[0][0].clone() 
                                            };
                    self.block_changes.push(change);
                  },
                  Value::Number(index) => {
                    let ix = index.mantissa() as usize;
                    if ix <= to_table_ref.rows as usize {
                      let change = Change::Set{table: to_table_id.clone(), 
                                              row: Index::Index(ix as u64), 
                                              column: Index::Index(cix as u64 + 1),
                                              value: from_table_ref.data[0][0].clone() 
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
              let tcix = if to_column_values.is_empty() { i }
                         else {
                           match to_column_values[i] {
                             Value::Number(x) => x.mantissa() as usize  - 1,
                             Value::Bool(true) => i,
                             _ => {continue; 0},
                           }
                         };
              for j in 0..from_height as usize {
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
                                          value: from_table_ref.data[i][j].clone() 
                                        };
                self.block_changes.push(change);
              }
            }
          }
          self.rhs_columns_empty.clear();
          self.lhs_columns_empty.clear();
          self.rhs_rows_empty.clear();
          self.lhs_rows_empty.clear();
        },
        Constraint::Append{from_table, to_table} => {
          let from = match from_table {
            TableId::Local(id) => self.memory.get(*id).unwrap().borrow(),
            TableId::Global(id) => store.get_table(*id).unwrap().borrow(),
          };

          let (to, to_id) = match to_table {
            TableId::Local(id) => (self.memory.get(*id).unwrap().borrow_mut(), id),
            TableId::Global(id) => (store.get_table(*id).unwrap().borrow_mut(), id),
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
          let mut from_table_ref = self.memory.get(*from_table).unwrap().borrow();
          let mut changes = vec![Change::NewTable{id: *to_table, rows: from_table_ref.rows, columns: from_table_ref.columns}];
          for (alias, ix) in from_table_ref.column_aliases.iter() {
            changes.push(Change::RenameColumn{table: *to_table, column_ix: *ix, column_alias: *alias});
          }
          for (col_ix, column) in from_table_ref.data.iter().enumerate() {
            for (row_ix, data) in column.iter().enumerate() {
              match data {
                Value::Reference(id) => {
                  copy_tables.insert(*id);
                }, 
                _ => (),
              }
              changes.push(Change::Set{table: *to_table, row: Index::Index(row_ix as u64 + 1), column: Index::Index(col_ix as u64 + 1), value: data.clone()});
            }
          }
          self.block_changes.append(&mut changes);
        },
        // Like copy table but we do this only once!
        Constraint::DefineTable{from_table, to_table} => {
          match store.tables.get(*to_table) {
            //Some(_) => (), // The table has already been created
            _ => {
              let mut from_table_ref = self.memory.get(*from_table).unwrap().borrow();
              let mut changes = vec![Change::NewTable{id: *to_table, rows: from_table_ref.rows, columns: from_table_ref.columns}];
              for (alias, ix) in from_table_ref.column_aliases.iter() {
                changes.push(Change::RenameColumn{table: *to_table, column_ix: *ix, column_alias: *alias});
              }
              for (col_ix, column) in from_table_ref.data.iter().enumerate() {
                for (row_ix, data) in column.iter().enumerate() {
                  match data {
                    Value::Reference(id) => {
                      copy_tables.insert(*id);
                    }, 
                    _ => (),
                  }
                  changes.push(Change::Set{table: *to_table, row: Index::Index(row_ix as u64 + 1), column: Index::Index(col_ix as u64 + 1), value: data.clone()});
                }
              }
              self.block_changes.append(&mut changes);
            }
          }
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
    for c in copy_tables.iter() {
      self.copy_table(*c, *c, store);
    }

    if self.errors.len() > 0 {
      self.state = BlockState::Error;
    } else {
      store.process_transaction(&Transaction::from_changeset(self.block_changes.clone()));
      self.updated = true;
    }
    self.block_changes.clear();
    self.state = BlockState::Updated;
    self.current_step = None;
  }

  fn copy_table(&mut self, from_table: TableId, to_table: TableId, store: &Interner) {
    let block = self as *mut Block;
    let mut copy_tables: HashSet<TableId> = HashSet::new();
    let mut from_table_ref = match from_table {
      TableId::Local(id) => {
        match unsafe{(*block).memory.get(id)} {
          None => store.get_table(id).unwrap().borrow(),
          Some(table) => table.borrow(),
        }
      },
      TableId::Global(id) => store.get_table(id).unwrap().borrow(),
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
            copy_tables.insert(*id);
          }, 
          _ => (),
        }
        changes.push(Change::Set{table: to_table_id, row: Index::Index(row_ix as u64 + 1), column: Index::Index(col_ix as u64 + 1), value: data.clone()});
      }
    }
    for c in copy_tables.iter() {
      self.copy_table(*c, *c, store);
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

// ## Constraints

// Constraints put bounds on the data available for a block to work with. For 
// example, Scan constraints could bring data into the block, and a Join 
// constraint could match elements from one table to another.

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Constraint {
  NewTable{id: TableId, rows: u64, columns: u64},
  TableColumn{table: u64, column_ix: u64, column_alias: u64},
  // Input Constraints
  Reference{table: TableId, destination: u64},
  Scan {table: TableId, indices: Vec<(Option<Parameter>, Option<Parameter>)>, output: TableId},
  Whenever {tables: Vec<(TableId, Vec<(Option<Parameter>, Option<Parameter>)>)>},
  Wait {tables: Vec<(TableId, Vec<(Option<Parameter>, Option<Parameter>)>)>},
  Until {tables: Vec<(TableId, Vec<(Option<Parameter>, Option<Parameter>)>)>},
  Identifier {id: u64, text: String},
  // Transform Constraints
  Function {fnstring: String, parameters: Vec<(String, TableId, Vec<(Option<Parameter>, Option<Parameter>)>)>, output: Vec<TableId>},
  Constant {table: TableId, row: Index, column: Index, value: Quantity, unit: Option<String>},
  String {table: TableId, row: Index, column: Index, value: String},
  // Identity Constraints
  CopyTable {from_table: u64, to_table: u64},
  DefineTable {from_table: u64, to_table: u64},
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
      Constraint::Whenever{tables} => write!(f, "Whenever({:?})", tables),
      Constraint::Wait{tables} => write!(f, "Wait({:?})", tables),
      Constraint::Until{tables} => write!(f, "Until({:?})", tables),
      Constraint::Function{fnstring, parameters, output} => write!(f, "Function({:?}({:?}) -> {:?})", fnstring, parameters, output),
      Constraint::Constant{table, row, column, value, unit} => write!(f, "Constant({}{:?} -> #{:?})", value.to_float(), unit, table),
      Constraint::String{table, row, column, value} => write!(f, "String({:?} -> #{:?})", value, table),
      Constraint::CopyTable{from_table, to_table} => write!(f, "CopyTable({:#x} -> {:#x})", from_table, to_table),
      Constraint::DefineTable{from_table, to_table} => write!(f, "DefineTable({:#x} -> {:#x})", from_table, to_table),
      Constraint::AliasTable{table, alias} => write!(f, "AliasTable({:?} -> {:#x})", table, alias),
      Constraint::Identifier{id, text} => write!(f, "Identifier(\"{}\" = {:#x})", text, id),
      Constraint::Insert{from, to} => write!(f, "Insert({:?} -> {:?})",  from, to),
      Constraint::Append{from_table, to_table} => write!(f, "Append({:?} -> {:?})", from_table, to_table),
      Constraint::TableColumn{table, column_ix, column_alias}  => write!(f, "TableColumn(#{:#x}({:#x}) -> {:#x})",  table, column_ix, column_alias),
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
