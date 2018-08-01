// # Mech Runtime

/* 
 The Mech Runtime is the engine that drives computations in Mech. The 
 runtime is comprised of "Blocks", interconnected by "Pipes" of records.
 Blocks can interact with the database, by Scanning for records that 
 match a pattern, or by Projecting computed records into the database.
*/

// ## Prelude

use table::{Table, Value};
use alloc::{fmt, Vec, String};
use database::{Interner, Change};
use hashmap_core::map::HashMap;
use hashmap_core::set::HashSet;
use indexes::Hasher;
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

  // Register a new block with the runtime
  pub fn register_block(&mut self, mut block: Block, store: &mut Interner) {
    if block.id == 0 {
      block.id = self.blocks.len() + 1;
    }
    for ((table, column), registers) in &block.pipes {
      for register in registers {
        let register_id = *register as usize - 1;
        let new_address = Address{block: block.id, register: *register as usize};
        let mut listeners = self.pipes_map.entry((*table as usize, *column as usize)).or_insert(vec![]);
        listeners.push(new_address);

        // Put associated values on the registers if we have them in the DB already
        block.input_registers[register_id].set(&(*table, *column));
        match store.get_column(*table, *column as usize) {
          Some(column) => block.ready = set_bit(block.ready, register_id),
          None => (),
        }
      }      
    }
    if block.updated && block.input_registers.len() == 0 {
      self.ready_blocks.insert(block.id);
    }
    self.blocks.insert(block.id, block.clone());
  } 

  pub fn register_blocks(&mut self, blocks: Vec<Block>, store: &mut Interner) {
    for block in blocks {
      self.register_block(block, store);
    }
  }

  // We've just interned some changes, and now we react to them by running the block graph.
  pub fn run_network(&mut self, store: &mut Interner) {
    // Run the compute graph until it reaches a steady state.
    // TODO Make this a parameter
    let max_iterations = 10_000;
    let mut n = 0;    
    while {
      println!("Ready: {:?}", self.ready_blocks);
      for block_id in self.ready_blocks.drain() {
        let mut block = &mut self.blocks.get_mut(&block_id).unwrap();
        block.solve(store);
      }
      // Queue up the next blocks
      for table_address in store.tables.changed_this_round.drain() {
        match self.pipes_map.get(&table_address) {
          Some(register_addresses) => {
            for register_address in register_addresses {
              let block_ix = register_address.block - 1;
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
      !self.ready_blocks.is_empty()
    } {}
    // Reset blocks updated status
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

  pub fn output(n: u64) -> Register {
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
  pub name: String,
  pub text: String,
  pub ready: u64,
  pub updated: bool,
  pub plan: Vec<Constraint>,
  pub column_lengths: Vec<u64>,
  pub pipes: HashMap<(u64, u64), Vec<u64>>,
  pub input_registers: Vec<Register>,
  pub memory_registers: Vec<Register>,
  pub output_registers: Vec<Register>,
  pub constraints: Vec<Constraint>,
  memory: Table,
}

impl Block {
  
  pub fn new() -> Block { 
    Block {
      id: 0,
      name: String::from(""),
      text: String::from(""),
      ready: 0,
      updated: false,
      pipes: HashMap::new(),
      plan: Vec::new(),
      column_lengths: Vec::new(),
      input_registers: Vec::with_capacity(32),
      memory_registers: Vec::with_capacity(32),
      output_registers: Vec::with_capacity(32),
      constraints: Vec::with_capacity(32),
      memory: Table::new(0, 1, 1),
    }
  }

  pub fn add_constraint(&mut self, constraint: Constraint) {
    let new_column_ix = (self.memory.columns + 1) as u64;
    match constraint {
      Constraint::Scan{table, column, input} | 
      Constraint::ChangeScan{table, column, input} => {
        if self.input_registers.len() < input as usize {
          self.input_registers.resize(input as usize, Register::new());
        }
        self.input_registers[input as usize - 1] = Register::input(table, column);
        let mut listeners = self.pipes.entry((table, column)).or_insert(vec![]);
        listeners.push(input);
      },
      Constraint::Function{ref operation, ref parameters, memory} => {
        self.memory_registers.push(Register::memory(memory));
        self.memory.grow_to_fit(1, memory as usize);
        if self.column_lengths.len() < memory as usize {
          self.column_lengths.resize(memory as usize, 0);
        }
        self.column_lengths[memory as usize - 1] = 0;
      },
      Constraint::Filter{ref comparator, lhs, rhs, memory} => {
        self.memory_registers.push(Register::memory(memory));
        self.memory.add_column(new_column_ix);
        self.column_lengths.push(0);
      }
      Constraint::Constant{value, memory} => {
        if self.memory_registers.len() < memory as usize {
          self.memory_registers.resize(memory as usize, Register::new());
        }
        self.memory_registers[memory as usize - 1] = Register::memory(memory);
        self.memory.grow_to_fit(1, memory as usize);
        let result = self.memory.set_cell(1, memory as usize, Value::from_i64(value));
        if self.column_lengths.len() < memory as usize {
          self.column_lengths.resize(memory as usize, 0);
        }
        self.column_lengths[memory as usize - 1] = 1;
        self.updated = true;
      }
      Constraint::CopyInput{input, memory} => {
        let source = self.input_registers[input as usize - 1].clone();
        if self.memory_registers.len() < memory as usize {
          self.memory_registers.resize(memory as usize, Register::new());
        }
        self.memory_registers[memory as usize - 1] = self.input_registers[input as usize - 1].clone();
        self.memory.add_column(source.column);
        self.column_lengths.push(0);
      }
      Constraint::Condition{truth, result, default, memory} => {
        self.memory_registers.push(Register::memory(memory));
        self.memory.add_column(new_column_ix);
        self.column_lengths.push(0);
      }
      Constraint::IndexMask{source, truth, memory} => {
        self.memory_registers.push(Register::memory(memory));
        self.memory.add_column(new_column_ix);
        self.column_lengths.push(0);
      },
      Constraint::Insert{memory, output, table, column} => {
        self.output_registers.push(Register::output(column));
      },
      Constraint::Set{output, table, column} => {
        self.output_registers.push(Register::new());
      },
      Constraint::Identifier{id, memory} => {
        self.memory.attributes.insert(id, memory as usize);
      },
      Constraint::Data{..} => (),
      Constraint::NewTable{..} => (),
      Constraint::CopyOutput{..} => (),
    }
    self.constraints.push(constraint);

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
      println!("{:?}", step);
      match step {
        Constraint::ChangeScan{table, column, input} => {
          self.ready = clear_bit(self.ready, *input as usize - 1);
        }
        Constraint::Function{operation, parameters, memory} => {
          // Pass the parameters to the appropriate function
          let op_fun = match operation {
            Function::Add => operations::math_add,
            Function::Subtract => operations::math_subtract,
            Function::Multiply => operations::math_multiply,
            Function::Divide => operations::math_divide,
          };
          // Execute the function. Results are placed on the memory registers
          op_fun(parameters, &vec![*memory], &mut self.memory, &mut self.column_lengths);
        },
        Constraint::Filter{comparator, lhs, rhs, memory} => {
          operations::compare(comparator, *lhs as usize, *rhs as usize, *memory as usize, &mut self.memory, &mut self.column_lengths);
        },
        Constraint::CopyInput{input, memory} => {
          println!("SOLING COPY INPUT I{:?} -> M{:?}", input, memory);
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
            match self.memory.index(i, *truth as usize) {
              Some(Value::Bool(true)) => {
                let value = self.memory.index(i, source_ix).unwrap().clone();
                self.memory.set_cell(i, memory_ix, value);
              },
              Some(Value::Bool(false)) => {
                let value = self.memory.index(i, source_ix).unwrap().clone();
                self.memory.set_cell(i, memory_ix, Value::Empty);
              },
              _ => (),
            };
          }
          self.column_lengths[memory_ix - 1] = source_length as u64;
        },
        Constraint::Insert{memory, output, table, column} => {
          let output_memory_ix = self.output_registers[*output as usize - 1].column;
          match &mut self.memory.get_column(output_memory_ix as usize) {
            Some(column_data) => {
              for (row_ix, cell) in column_data.iter().enumerate() {
                match cell {
                  Value::Empty => (),
                  _ => {
                    store.intern_change(
                      &Change::Add{ table: *table, row: row_ix as u64 + 1, column: *column, value: cell.clone() }
                    );
                  }
                }
              }
            },
            None => (),
          }
        },
        Constraint::NewTable{id, rows, columns} => {
          store.intern_change(
            &Change::NewTable{id: *id, rows: *rows as usize, columns: *columns as usize}
          );
        },
        Constraint::Set{output, table, column} => {
          let target_rows;
          {
            let table_ref = store.tables.get(*table).unwrap();
            target_rows = table_ref.rows as u64;
          }
          let output_length = self.column_lengths[*output as usize - 1];
          let output_memory_ix = self.memory_registers[*output as usize - 1].column;
          let column_data = &mut self.memory.get_column(output_memory_ix as usize).unwrap();
          for i in 1 .. target_rows + 1 {
            if output_length == 1 {
              store.intern_change(
                &Change::Add{table: *table, row: i, column: *column, value: column_data[0].clone()}
              );
            } else if output_length == target_rows {
              // TODO
            }
          }
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
        Constraint::CopyInput{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
        Constraint::Function{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }
    for constraint in &self.constraints {
      match constraint {
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
  // Input Constraints
  Scan {table: u64, column: u64, input: u64},
  Identifier {id: u64, memory: u64},
  ChangeScan {table: u64, column: u64, input: u64},
  // Transform Constraints
  Filter {comparator: operations::Comparator, lhs: u64, rhs: u64, memory: u64},
  Function {operation: operations::Function, parameters: Vec<u64>, memory: u64},
  Constant {value: i64, memory: u64},
  Condition {truth: u64, result: u64, default: u64, memory: u64},
  IndexMask {source: u64, truth: u64, memory: u64},
  // Identity Constraints
  CopyInput {input: u64, memory: u64},
  CopyOutput {memory: u64, output: u64},
  // Output Constraints
  Insert {memory: u64, output: u64, table: u64, column: u64},
  Set{output: u64, table: u64, column: u64},
}

impl fmt::Debug for Constraint {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Constraint::Data{table, column} => write!(f, "Data(#{:#x}({:#x}))", table, column),
      Constraint::NewTable{id, rows, columns} => write!(f, "NewTable(#{:#x}({:?}x{:?}))", id, rows, columns),
      Constraint::Scan{table, column, input} => write!(f, "Scan(#{:#x}({:#x}) -> I{:#x})", table, column, input),
      Constraint::ChangeScan{table, column, input} => write!(f, "ChangeScan(#{:#x}({:#x}) -> I{:?})", table, column, input),
      Constraint::Filter{comparator, lhs, rhs, memory} => write!(f, "Filter({:#x} {:?} {:#x} -> M{:?})", lhs, comparator, rhs, memory),
      Constraint::Function{operation, parameters, memory} => write!(f, "Fxn::{:?}{:?} -> M{:#x}", operation, parameters, memory),
      Constraint::Constant{value, memory} => write!(f, "Constant({:?} -> M{:#x})", value, memory),
      Constraint::CopyInput{input, memory} => write!(f, "CopyInput(I{:#x} -> M{:#x})", input, memory),
      Constraint::CopyOutput{memory, output} => write!(f, "CopyOutput(M{:#x} -> O{:#x})", memory, output),
      Constraint::Condition{truth, result, default, memory} => write!(f, "Condition({:?} ? {:?} | {:?} -> M{:?})", truth, result, default, memory),
      Constraint::Identifier{id, memory} => write!(f, "Identifier({:?} -> M{:?})", id, memory),
      Constraint::IndexMask{source, truth, memory} => write!(f, "IndexMask({:#x}, {:#x} -> M{:#x})", source, truth, memory),
      Constraint::Insert{memory, output, table, column} => write!(f, "Insert(O{:#x} -> #{:#x}[{:#x}])",  output, table, column),
      Constraint::Set{output, table, column} => write!(f, "Set(O{:#x} -> #{:#x}({:#x}))",  output, table, column),
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
