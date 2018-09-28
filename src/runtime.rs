// # Mech Runtime

/* 
 The Mech Runtime is the engine that drives computations in Mech. The runtime 
 is comprised of "Blocks", interconnected by "Pipes" of records. Blocks can 
 interact with the database, by Scanning for records that  match a pattern, or 
 by Projecting computed records into the database.
*/

// ## Prelude

use table::{Table, Value};
use alloc::{fmt, Vec, String};
use database::{Transaction, Interner, Change};
use hashmap_core::map::{HashMap, Entry};
use hashmap_core::set::HashSet;
use indexes::{Hasher, TableIndex};
use operations;
use operations::Function;

// ## Runtime

#[derive(Clone)]
pub struct Runtime {
  pub blocks: HashMap<usize, Block>,
  pub pipes_map: HashMap<(usize, usize), Vec<Address>>,
  pub ready_blocks: HashSet<usize>,
}

impl Runtime {

  pub fn new() -> Runtime {
    Runtime {
      blocks: HashMap::new(),
      ready_blocks: HashSet::new(),
      pipes_map: HashMap::new(),
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
    for (ix, register) in block.input_registers.iter().enumerate() {
      let table = register.table as usize;
      let column = register.column as usize;
      let new_address = Address{block: block.id, register: ix + 1};
      let mut listeners = self.pipes_map.entry((table, column)).or_insert(vec![]);
      listeners.push(new_address);

      // Set the register as ready if the referenced column exists
      match store.get_column(table as u64, column) {
        Some(column) => block.ready = set_bit(block.ready, ix),
        None => (),
      }
    }
    // Mark the block as ready for execution on the next available cycle
    if block.updated && block.input_registers.len() == 0 {
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

  // We've just interned some changes, and now we react to them by running the 
  // block graph.
  pub fn run_network(&mut self, store: &mut Interner) {
    // Run the compute graph until it reaches a steady state, or until it hits 
    // an iteration limit
    // TODO Make this a parameter
    let max_iterations = 10_000;
    let mut n = 0; 
    // Note: The way this while loop is written, it's actually a do-while loop.
    // This is a little trick in Rust. This means the network will always run
    // at least one time, and if there are no more ready blocks after that run,
    // the loop will terminate.
    while {
      for block_id in self.ready_blocks.drain() {
        let mut block = &mut self.blocks.get_mut(&block_id).unwrap();
        block.solve(store);
      }
      // Queue up the next blocks based on tables that changed during this round.
      for table_address in store.tables.changed_this_round.drain() {
        match self.pipes_map.get(&table_address) {
          Some(register_addresses) => {
            for register_address in register_addresses {
              let mut block = &mut self.blocks.get_mut(&register_address.block).unwrap();
              block.ready = set_bit(block.ready, register_address.register - 1);
              if block.is_ready() {
                self.ready_blocks.insert(register_address.block);
              }
            }
          },
          _ => (),
        }
      }
      // Halt iterating if we've exceeded the maximum number of allowed iterations.
      n += 1;
      if n > max_iterations {
        // TODO Insert an error into the db here.
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

#[derive(Clone)]
pub struct Address {
  pub block: usize,
  pub register: usize,
}

impl fmt::Debug for Address {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "@(block: {:?}, register: {:?})", self.block, self.register)
  }
}

#[derive(Clone)]
pub struct Register {
  pub table: u64,
  pub column: u64,
}

impl Register {
  
  pub fn new() -> Register { 
    Register {
      table: 0,
      column: 0,
    }
  }

  pub fn input(t: u64, n: u64) -> Register {
    Register {
      table: t,
      column: n,
    }
  }

  pub fn memory(n: u64) -> Register {
    Register {
      table: 0,
      column: n,
    }
  }

  pub fn output(t: u64, n: u64) -> Register {
    Register {
      table: t,
      column: n,
    }
  }

  pub fn get(&self) -> (u64, u64) {
    (self.table, self.column)
  }

  pub fn set(&mut self, index: &(u64, u64)) {
    let (table, column) = index;
    self.table = *table;
    self.column = *column;
  }

  pub fn table(&self) -> u64 {
    self.table
  }

  pub fn column(&self) -> u64 {
    self.column
  }

}

impl fmt::Debug for Register {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({:#x}, {:#x})", self.table, self.column)
  }
}

#[derive(Clone)]
pub struct Block {
  pub id: usize,
  pub name: String,
  pub text: String,
  pub ready: u64,
  pub updated: bool,
  pub plan: Vec<Constraint>,
  pub column_lengths: Vec<u64>,
  pub input_registers: Vec<Register>,
  pub memory_registers: Vec<Register>,
  pub output_registers: Vec<Register>,
  pub constraints: Vec<Constraint>,
  memory: TableIndex,
  scratch: Vec<Value>,
}

impl Block {
  
  pub fn new() -> Block { 
    Block {
      id: 0,
      name: String::from(""),
      text: String::from(""),
      ready: 0,
      updated: false,
      plan: Vec::new(),
      column_lengths: Vec::new(),
      input_registers: Vec::with_capacity(1),
      memory_registers: Vec::with_capacity(1),
      output_registers: Vec::with_capacity(1),
      constraints: Vec::with_capacity(1),
      memory: TableIndex::new(1),
      scratch: Vec::new(),
    }
  }

  pub fn add_constraint(&mut self, constraint: Constraint) {
    self.constraints.push(constraint.clone());
    match constraint {
      Constraint::Function{operation, parameters, output} => {
        for (table, column) in output {
          let mut table_ref = self.memory.get_mut(table).unwrap();
          table_ref.grow_to_fit(table_ref.rows, column as usize);
          table_ref.set_column_id(column, column as usize);
        }
      },
      Constraint::NewTable{..} => self.updated = true,
      Constraint::NewBlockTable{id, rows, columns} => {
        self.memory.register(Table::new(id, rows as usize, columns as usize));
      },
      Constraint::Constant{table, row, column, value} => {
        match self.memory.map.entry(table) {
          Entry::Occupied(mut o) => {
            let mut table_ref = o.get_mut();
            table_ref.grow_to_fit(row as usize, column as usize);
            table_ref.set_cell(row as usize, column as usize, Value::from_i64(value));
            table_ref.set_column_id(column, column as usize);
          },
          Entry::Vacant(v) => {    
          },
        };
        self.updated = true;
      },
      _ => (),
    }


  }

  pub fn add_constraints(&mut self, constraints: Vec<Constraint>) {
    for constraint in constraints {
      self.add_constraint(constraint)
    }
  }

  pub fn is_ready(&self) -> bool {
    let input_registers_count = self.input_registers.len();
    // TODO why does the exponent have to be u32?
    if input_registers_count > 0 {
      self.ready == 2_u64.pow(input_registers_count as u32) - 1
    } else {
      false
    }
  }

  pub fn solve(&mut self, store: &mut Interner) {
    for step in &self.plan {
      
      match step {
        /*
        Constraint::ChangeScan{table, column, input} => {
          self.ready = clear_bit(self.ready, *input as usize - 1);
        }*/
        Constraint::Function{operation, parameters, output} => {
          // Pass the parameters to the appropriate function
          let op_fun = match operation {
            Function::Add => operations::math_add,
            Function::Subtract => operations::math_subtract,
            Function::Multiply => operations::math_multiply,
            Function::Divide => operations::math_divide,
            Function::Power => operations::math_power,
          };
          // Execute the function. Results are placed on the memory registers
          let (lhs_table, lhs_column) = parameters[0];
          let (rhs_table, rhs_column) = parameters[1];
          let (out_table, out_column) = output[0];
          {         
            let lhs = match self.memory.get(lhs_table) {
              Some(table_ref) => {
                table_ref.get_column(lhs_column as usize).unwrap()
              },
              None => store.get_column(lhs_table, lhs_column as usize).unwrap(),
            };
            let rhs = match self.memory.get(rhs_table) {
              Some(table_ref) => {
                table_ref.get_column(rhs_column as usize).unwrap()
              },
              None => store.get_column(rhs_table, rhs_column as usize).unwrap(),
            };
            op_fun(lhs, rhs, &mut self.scratch);
          }
          let out = self.memory.get_mut(out_table).unwrap().get_column_mut(out_column as usize).unwrap();
          out[0] = self.scratch[0].clone();
          self.scratch.clear();
        },
        /*
        Constraint::Filter{comparator, lhs, rhs, memory} => {
          operations::compare(comparator, *lhs as usize, *rhs as usize, *memory as usize, &mut self.memory, &mut self.column_lengths);
        },
        Constraint::CopyInput{input, memory} => {
          let register = &self.input_registers[*input as usize - 1];
          match store.get_column(register.table, register.column as usize) {
            Some(column) => {
              self.column_lengths[*memory as usize - 1] = column.len() as u64;
              operations::identity(column, *memory, &mut self.memory);
            },
            None => (),
          }
        },
        Constraint::Condition{truth, result, default, memory} => {
          for i in 1 .. self.memory.rows + 1 {
            match self.memory.index(i, *truth as usize) {
              Some(Value::Bool(true)) => {
                let value = self.memory.index(i, *result as usize).unwrap().clone();
                self.memory.set_cell(i, *memory as usize, value);
              },
              Some(Value::Bool(false)) => {
                let value = self.memory.index(i, *default as usize).unwrap().clone();
                self.memory.set_cell(i, *memory as usize, value);
              },
              _ => (),
            };
          }
        }
        Constraint::IndexMask{source, truth, memory} => {
          let source_ix = *source as usize;
          let memory_ix = *memory as usize;
          let source_length = self.column_lengths[source_ix - 1] as usize;
          for i in 1 .. source_length + 1 {
            let value = self.memory.index(i, source_ix).unwrap().clone();
            match self.memory.index_by_alias(i, truth) {
              Some(Value::Bool(true)) =>  self.memory.set_cell(i, memory_ix, value),
              Some(Value::Bool(false)) => self.memory.set_cell(i, memory_ix, Value::Empty),
              otherwise => Ok(Value::Empty),
            };
          }
          self.column_lengths[memory_ix - 1] = source_length as u64;
        },*/
        Constraint::Insert{from, to} => {
          let (from_table, from_row, from_column) = from;
          let (to_table, to_row, to_column) = to;
          match &mut self.memory.get_mut(*from_table) {
            Some(table_ref) => {
              match &mut table_ref.get_column_by_ix(*from_column as usize) {
                Some(column_data) => {
                  for (row_ix, cell) in column_data.iter().enumerate() {
                    match cell {
                      Value::Empty => (),
                      _ => {
                        store.process_transaction(&Transaction::from_change(
                          Change::Set{ table: *to_table, row: row_ix as u64 + 1, column: *to_column, value: cell.clone() },
                        ));
                      }
                    }
                  }
                },
                None => (),
              };
            },
            None => (),
          }
          /*match &mut self.memory.get_column_by_ix(*memory as usize) {
            Some(column_data) => {
              for (row_ix, cell) in column_data.iter().enumerate() {
                match cell {
                  Value::Empty => (),
                  _ => {
                    store.process_transaction(&Transaction::from_change(
                      Change::Set{ table: *table, row: row_ix as u64 + 1, column: *column, value: cell.clone() },
                    ));
                  }
                }
              }
            },
            None => (),
          }*/
        },/*
        Constraint::Append{memory, table, column} => {
          match &mut self.memory.get_column_by_ix(*memory as usize) {
            Some(column_data) => {
              for (row_ix, cell) in column_data.iter().enumerate() {
                let length = column_data.len() as u64;
                match cell {
                  Value::Empty => (),
                  _ => {
                    store.process_transaction(&Transaction::from_change(
                      Change::Append{ table: *table, column: *column, value: cell.clone() }
                    ));
                  }
                }
              }
            },
            None => (),
          }
        },
        */
        Constraint::NewTable{id, rows, columns} => {
          store.process_transaction(&Transaction::from_change(
            Change::NewTable{id: *id, rows: *rows as usize, columns: *columns as usize},
          ));
        },
        _ => (),
      } 
    }
    self.updated = true;
  }

  // Right now, the planner works just by giving the constraint an order to execute.
  // This is accomplished with a weighted sort. First scans, then transforms, then
  // inserts.
  // This could be an entire thing all by itself, so let's just keep it simple at first.
  pub fn plan(&mut self) {
    for constraint in &self.constraints {
      match constraint {
        Constraint::NewTable{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
        Constraint::ChangeScan{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
        Constraint::CopyInput{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
        Constraint::Filter{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
        Constraint::IndexMask{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    // TODO Actually sort the function constraints
    let mut reversed = self.constraints.clone();
    reversed.reverse();
    for constraint in &reversed {
      match constraint {
        Constraint::Function{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
        Constraint::Append{..} |
        Constraint::Insert{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "┌────────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Block {:?} ({:?})\n", self.name, self.id).unwrap();
    write!(f, "├────────────────────────────────────────┤\n").unwrap();
    write!(f, "│ {}\n",self.text).unwrap();
    write!(f, "├────────────────────────────────────────┤\n").unwrap();
    write!(f, "│ Ready: {:?} ({:b})\n", self.is_ready(), self.ready).unwrap();
    write!(f, "│ Updated: {:?}\n", self.updated).unwrap();
    write!(f, "│ Input: {:?}\n", self.input_registers.len()).unwrap();
    for (ix, register) in self.input_registers.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, register).unwrap();
    }
    write!(f, "│ Memory: {:?}\n", self.memory_registers.len()).unwrap();
    for (ix, register) in self.memory_registers.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, register).unwrap();
    }
    write!(f, "│ Output: {:?}\n", self.output_registers.len()).unwrap();
    for (ix, register) in self.output_registers.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, register).unwrap();
    }
    write!(f, "│ Constraints: {:?}\n", self.constraints.len()).unwrap();
    for constraint in &self.constraints {
      write!(f, "│  > {:?}\n", constraint).unwrap();
    }
    write!(f, "│ Plan: {:?}\n", self.plan.len()).unwrap();
    for (ix, step) in self.plan.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, step).unwrap();
    }
    write!(f, "└────────────────────────────────────────┘\n").unwrap();
    write!(f, "{:?}\n", self.column_lengths).unwrap();
    write!(f, "{:?}\n", self.memory).unwrap();
    Ok(())
  }
}

// ## Pipe

// Pipes are conduits of records between blocks.

pub struct Pipe {
  input: Address,
  output: Address,
}

// ## Constraints

// Constraints put bounds on the data available for a block to work with. For 
// example, Scan constraints could bring data into the block, and a Join 
// constraint could match elements from one table to another.

#[derive(Clone)]
pub enum Constraint {
  Data {table: u64, column: u64},
  NewTable{id: u64, rows: u64, columns: u64},
  NewBlockTable{id: u64, rows: u64, columns: u64},
  // Input Constraints
  Scan {table: u64, column: u64, input: u64},
  Identifier {id: u64},
  ChangeScan {table: u64, column: u64, input: u64},
  // Transform Constraints
  Filter {comparator: operations::Comparator, lhs: u64, rhs: u64, memory: u64},
  Function {operation: operations::Function, parameters: Vec<(u64, u64)>, output: Vec<(u64,u64)>},
  Constant {table: u64, row: u64, column: u64, value: i64},
  Condition {truth: u64, result: u64, default: u64, memory: u64},
  IndexMask {source: u64, truth: u64, memory: u64},
  // Identity Constraints
  CopyInput {input: u64, memory: u64},
  CopyOutput {memory: u64, output: u64},
  // Output Constraints
  Insert {from: (u64, u64, u64), to: (u64, u64, u64)},
  Append {memory: u64, table: u64, column: u64},
}

impl fmt::Debug for Constraint {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Constraint::Data{table, column} => write!(f, "Data(#{:#x}({:#x}))", table, column),
      Constraint::NewTable{id, rows, columns} => write!(f, "NewTable(#{:#x}({:?}x{:?}))", id, rows, columns),
      Constraint::NewBlockTable{id, rows, columns} => write!(f, "NewBlockTable(#{:#x}({:?}x{:?}))", id, rows, columns),
      Constraint::Scan{table, column, input} => write!(f, "Scan(#{:#x}({:#x}) -> I{:#x})", table, column, input),
      Constraint::ChangeScan{table, column, input} => write!(f, "ChangeScan(#{:#x}({:#x}) -> I{:?})", table, column, input),
      Constraint::Filter{comparator, lhs, rhs, memory} => write!(f, "Filter({:#x} {:?} {:#x} -> M{:?})", lhs, comparator, rhs, memory),
      Constraint::Function{operation, parameters, output} => write!(f, "Fxn::{:?}{:?} -> {:?}", operation, parameters, output),
      Constraint::Constant{table, row, column, value} => write!(f, "Constant({:?} -> #{:#x}({:#x}, {:#x}))", value, table, row, column),
      Constraint::CopyInput{input, memory} => write!(f, "CopyInput(I{:#x} -> M{:#x})", input, memory),
      Constraint::CopyOutput{memory, output} => write!(f, "CopyOutput(M{:#x} -> O{:#x})", memory, output),
      Constraint::Condition{truth, result, default, memory} => write!(f, "Condition({:?} ? {:?} | {:?} -> M{:?})", truth, result, default, memory),
      Constraint::Identifier{id} => write!(f, "Identifier({:#x})", id),
      Constraint::IndexMask{source, truth, memory} => write!(f, "IndexMask({:#x}, {:#x} -> M{:#x})", source, truth, memory),
      Constraint::Insert{from, to} => write!(f, "Insert({:?} -> {:?})",  from, to),
      Constraint::Append{memory, table, column} => write!(f, "Append(M{:#x} -> #{:#x}[{:#x}])",  memory, table, column),
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
