// # Mech Runtime

/* 
 The Mech Runtime is the engine that drives computations in Mech. The 
 runtime is comprised of "Blocks", interconnected by "Pipes" of records.
 Blocks can interact with the database, by Scanning for records that 
 match a pattern, or by Projecting computed records into the database.
*/

// ## Prelude

use table::{Table, Value};
use alloc::{fmt, Vec};
use database::{Interner, Change};
use hashmap_core::map::HashMap;
use indexes::Hasher;
use operations;
use operations::Function;

// ## Runtime

#[derive(Clone)]
pub struct Runtime {
  pub blocks: Vec<Block>,
  pub pipes_map: HashMap<(u64, u64), Vec<Address>>,
}

impl Runtime {

  pub fn new() -> Runtime {
    Runtime {
      blocks: Vec::new(),
      pipes_map: HashMap::new(),
    }
  }

  // Register a new block with the runtime
  pub fn register_block(&mut self, mut block: Block, store: &mut Interner) {
    // @TODO better block ID
    block.id = self.blocks.len() + 1;
    for ((table, column), register) in &block.pipes {
      let register_id = *register as usize - 1;
      self.pipes_map.insert((*table, *column), vec![Address{block: block.id, register: *register as usize}]);
      // Put associated values on the registers if we have them in the DB already
      block.input_registers[register_id].set(&(*table, *column));
      block.ready = set_bit(block.ready, register_id);      
    }
    self.blocks.push(block.clone());
    self.run_network(store);
  } 

  pub fn register_blocks(&mut self, blocks: Vec<Block>, store: &mut Interner) {
    for block in blocks {
      self.register_block(block, store);
    }
  }

  pub fn process_change(&mut self, change: &Change) {
    match change {
      Change::Add{table, row, column, value} => {
        match self.pipes_map.get(&(*table, *column)) {
          Some(addresses) => {
            for address in addresses {
              let register_ix = address.register - 1;
              let block_id = address.block - 1;
              if block_id < self.blocks.len() {
                let block = &mut self.blocks[block_id];
                if register_ix < block.input_registers.len() {
                  let register = &mut block.input_registers[register_ix];
                  block.ready = set_bit(block.ready, register_ix);
                }
              }
            }
          },
          _ => (),
        }
      },
      _ => (),
    }
  }

  pub fn run_network(&mut self, store: &mut Interner) {
    for block in &mut self.blocks {
      if block.is_ready() {
        block.solve(store);
      }
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

  pub fn intermediate(n: u64) -> Register {
    Register {
      table: 0,
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
  pub ready: u64,
  pub plan: Vec<Constraint>,
  pub pipes: HashMap<(u64, u64), u64>,
  pub input_registers: Vec<Register>,
  pub intermediate_registers: Vec<Register>,
  pub output_registers: Vec<Register>,
  pub constraints: Vec<Constraint>,
  memory: Table,
}

impl Block {
  
  pub fn new() -> Block { 
    Block {
      id: 0,
      ready: 0,
      pipes: HashMap::new(),
      plan: Vec::new(),
      input_registers: Vec::with_capacity(32),
      intermediate_registers: Vec::with_capacity(32),
      output_registers: Vec::with_capacity(32),
      constraints: Vec::with_capacity(32),
      memory: Table::new(0, 1, 1),
    }
  }

  pub fn add_constraint(&mut self, constraint: Constraint) {
    let new_column_ix = (self.memory.columns + 1) as u64;
    match constraint {
      Constraint::Scan{table, column, input} => {
        self.input_registers.push(Register::input(table, column));
        self.pipes.insert((table, column), input);
      },
      Constraint::Insert{output, table, column} => {
        self.output_registers.push(Register::new());
      },
      Constraint::Function{ref operation, ref parameters, output} => {
        self.intermediate_registers.push(Register::intermediate(output));
        self.memory.add_column(new_column_ix);
      },
      Constraint::Filter{ref comparator, lhs, rhs, intermediate} => {
        self.intermediate_registers.push(Register::intermediate(intermediate));
        self.memory.add_column(new_column_ix);
      }
      Constraint::Constant{value, input} => {
        self.intermediate_registers.push(Register::intermediate(input));
        self.memory.grow_to_fit(1, new_column_ix as usize);
        self.memory.set_cell(1, input as usize, Value::from_i64(value));
      }
      Constraint::Identity{source, sink} => {
        self.intermediate_registers.push(self.input_registers[source as usize - 1].clone());
        self.memory.add_column(new_column_ix);
      }
      Constraint::Condition{truth, result, default, output} => {
        self.intermediate_registers.push(Register::intermediate(output));
        self.memory.add_column(new_column_ix);
      }
    }
    self.constraints.push(constraint);

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
        Constraint::Function{operation, parameters, output} => {
          // Pass the parameters to the appropriate function
          let op_fun = match operation {
            Function::Add => operations::math_add,
            Function::Subtract => operations::math_subtract,
            Function::Multiply => operations::math_multiply,
            Function::Divide => operations::math_divide,
          };
          // Execute the function. Results are placed on the intermediate registers
          op_fun(parameters, &vec![*output], &mut self.memory);
        },
        Constraint::Insert{output, table, column} => {
          let output_memory_ix = self.intermediate_registers[*output as usize - 1].column;
          let column_data = &mut self.memory.get_column(output_memory_ix as usize).unwrap();
          for (row_ix, cell) in column_data.iter().enumerate() {
            store.intern_change(
              &Change::Add{table: *table, row: row_ix as u64 + 1, column: *column, value: cell.clone()}
            );
          }
        },
        Constraint::Filter{comparator, lhs, rhs, intermediate} => {
          operations::compare(comparator, *lhs as usize, *rhs as usize, *intermediate as usize, &mut self.memory);
        },
        Constraint::Identity{source, sink} => {
          let register = &self.intermediate_registers[*sink as usize - 1];
          match store.get_column(register.table, register.column as usize) {
            Some(column) => {
              operations::identity(column, *sink, &mut self.memory);
            },
            None => (),
          }
        },
        Constraint::Constant{value, input} => {
          for i in 1 .. self.memory.rows + 1 {
            self.memory.set_cell(i, *input as usize, Value::from_i64(*value));
          }
        },
        Constraint::Condition{truth, result, default, output} => {
          for i in 1 .. self.memory.rows + 1 {
            match self.memory.index(i, *truth as usize) {
              Some(Value::Bool(true)) => {
                let value = self.memory.index(i, *result as usize).unwrap().clone();
                self.memory.set_cell(i, *output as usize, value);
              },
              Some(Value::Bool(false)) => {
                let value = self.memory.index(i, *default as usize).unwrap().clone();
                self.memory.set_cell(i, *output as usize, value);
              },
              _ => (),
            };
          }
        }
        _ => (),
      } 
    }
  }

}

impl fmt::Debug for Block {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "┌────────────────────────────────────────┐\n").unwrap();
    write!(f, "│ Block #{:?}\n", self.id).unwrap();
    write!(f, "├────────────────────────────────────────┤\n").unwrap();
    write!(f, "│ Ready: {:b}\n", self.ready).unwrap();
    write!(f, "│ Input: {:?}\n", self.input_registers.len()).unwrap();
    for (ix, register) in self.input_registers.iter().enumerate() {
      write!(f, "│  {:?}. {:?}\n", ix + 1, register).unwrap();
    }
    write!(f, "│ Intermediate: {:?}\n", self.intermediate_registers.len()).unwrap();
    for (ix, register) in self.intermediate_registers.iter().enumerate() {
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
  // A Scan monitors a supplied cell
  Scan {table: u64, column: u64, input: u64},
  Insert {output: u64, table: u64, column: u64},
  Filter {comparator: operations::Comparator, lhs: u64, rhs: u64, intermediate: u64},
  Function {operation: operations::Function, parameters: Vec<u64>, output: u64},
  Constant {value: i64, input: u64},
  Identity {source: u64, sink: u64},
  Condition {truth: u64, result: u64, default: u64, output: u64},
}

impl fmt::Debug for Constraint {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Constraint::Scan{table, column, input} => write!(f, "Scan(#{:#x}({:#x})) -> I{:?}", table, column, input),
      Constraint::Insert{output, table, column} => write!(f, "Insert({:?}) -> #{:#x}({:#x})",  output, table, column),
      Constraint::Filter{comparator, lhs, rhs, intermediate} => write!(f, "Filter({:#x} {:?} {:#x}) -> {:?}", lhs, comparator, rhs, intermediate),
      Constraint::Function{operation, parameters, output} => write!(f, "Fxn::{:?}{:?} -> {:?}", operation, parameters, output),
      Constraint::Constant{value, input} => write!(f, "Constant({:?}) -> {:?}", value, input),
      Constraint::Identity{source, sink} => write!(f, "Identity({:?}) -> {:?}", source, sink),
      Constraint::Condition{truth, result, default, output} => write!(f, "Condition({:?} ? {:?} | {:?}) -> {:?}", truth, result, default, output),
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
