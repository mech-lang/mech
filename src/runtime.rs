// # Mech Runtime

/* 
 The Mech Runtime is the engine that drives computations in Mech. The runtime 
 is comprised of "Blocks", interconnected by "Pipes" of records. Blocks can 
 interact with the database, by Scanning for records that  match a pattern, or 
 by Projecting computed records into the database.
*/

// ## Prelude

use table::{Table, TableId, Value, Index};
use alloc::fmt;
use alloc::string::String;
use alloc::vec::Vec;
use database::{Transaction, Interner, Change};
use hashmap_core::map::{HashMap, Entry};
use hashmap_core::set::HashSet;
use indexes::TableIndex;
use operations;
use operations::Function;

// ## Runtime

#[derive(Clone)]
pub struct Runtime {
  pub blocks: HashMap<usize, Block>,
  pub pipes_map: HashMap<(u64, Index), Vec<Address>>,
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
      let table = register.table;
      let column = register.column.clone();
      let new_address = Address{block: block.id, register: ix + 1};
      let listeners = self.pipes_map.entry((table, column)).or_insert(vec![]);
      listeners.push(new_address);

      // Set the register as ready if the referenced column exists
      /*
      match store.get_column_by_id(table as u64, column) {
        Some(_column) => block.ready = set_bit(block.ready, ix),
        None => (),
      }*/
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
        let block = &mut self.blocks.get_mut(&block_id).unwrap();
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
  pub column: Index,
}

impl Register {
  
  pub fn new(table: u64, column: Index) -> Register { 
    Register {
      table: 0,
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

#[derive(Clone)]
pub struct Block {
  pub id: usize,
  pub name: String,
  pub text: String,
  pub ready: u64,
  pub updated: bool,
  pub plan: Vec<Constraint>,
  pub input_registers: Vec<Register>,
  pub memory_registers: Vec<Register>,
  pub output_registers: Vec<Register>,
  pub constraints: Vec<(String, Vec<Constraint>)>,
  memory: TableIndex,
  scratch: Table,
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
      input_registers: Vec::with_capacity(1),
      memory_registers: Vec::with_capacity(1),
      output_registers: Vec::with_capacity(1),
      constraints: Vec::with_capacity(1),
      memory: TableIndex::new(1),
      scratch: Table::new(0,0,0),
    }
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
        Constraint::Insert{..} => self.plan.push(constraint.clone()),
        _ => (),
      }
    }

    // Do any work we can up front
    for constraint in constraints {
      println!("Adding Constraint {:?}", constraint);
      match constraint {
        Constraint::Scan{table, rows, columns} => {
          // TODO Update this whole register adding process and marking tables ready
          //self.input_registers.push(Register::input(table, 1));
        },
        Constraint::AliasTable{table, alias} => {
          // TODO Raise an error here if the alias already exists
          match table {
            TableId::Local(id) => self.memory.add_alias(id, alias),
            TableId::Global(id) => (), // TODO Add global alias here
          }
        },
        Constraint::Function{operation, parameters, output} => {

        },
        Constraint::NewTable{id, rows, columns} => {
          match id {
            TableId::Local(id) => {
              self.memory.insert(Table::new(id, rows, columns));
            }
            _ => (),
          }
        },
        Constraint::Constant{table, row, column, value} => {
          let table_id = match table {
            TableId::Local(id) => id,
            _ => 0,
          };
          match self.memory.map.entry(table_id) {
            Entry::Occupied(mut o) => {
              let table_ref = o.get_mut();
              table_ref.set_cell(&row, &column, Value::from_i64(value));
            },
            Entry::Vacant(v) => {    
            },
          };
          self.updated = true;
        },
        Constraint::TableColumn{table, column_ix, column_id} => {
          /*
          match self.memory.get_mut(table) {
            Some(table_ref) => {
              table_ref.set_column_id(column_id, column_ix as usize);
            }
            None => (),
          };*/
        },
        _ => (),
      }
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
      println!("Step: {:?}", step);
      match step {
        Constraint::Function{operation, parameters, output} => { 
          
          // Concat Functions  
          if *operation == Function::HorizontalConcatenate {
            let out_table = &output[0];

            for (table, rows, columns) in parameters {
              let table_ref = match table {
                TableId::Local(id) => self.memory.get(*id).unwrap(),
                TableId::Global(id) => store.get_table(*id).unwrap(),
              };
              println!("{:?}", self.scratch);
              if self.scratch.rows == 0 {
                self.scratch.grow_to_fit(table_ref.rows, table_ref.columns);
                self.scratch.data = table_ref.data.clone();
              } else if self.scratch.rows == table_ref.rows {
                self.scratch.data.append(&mut table_ref.data.clone());
                self.scratch.grow_to_fit(self.scratch.rows, self.scratch.columns + table_ref.columns);
              }
              
            }

            let out = self.memory.get_mut(*out_table.unwrap()).unwrap();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.column_aliases = self.scratch.column_aliases.clone();
            out.row_aliases = self.scratch.row_aliases.clone();
            out.data = self.scratch.data.clone();
            self.scratch.clear();
          }
          /*
          // Concat Functions  
          else if *operation == Function::VerticalConcatenate {
            let out_table = &output[0];
            for (table, rows, columns) in parameters {
              let table_ref = match self.memory.get(*table) {
                Some(table_ref) => table_ref,
                None => store.get_table(*table).unwrap(),
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
            let out = self.memory.get_mut(*out_table).unwrap();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.row_ids = self.scratch.row_ids.clone();
            //out.column_ids = self.scratch.column_ids.clone();
            out.column_aliases = self.scratch.column_aliases.clone();
            out.row_aliases = self.scratch.row_aliases.clone();
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
            {     
              let lhs = match self.memory.get(*lhs_table) {
                Some(table_ref) => Some(table_ref),
                None => store.get_table(*lhs_table),
              };
              let rhs = match self.memory.get(*rhs_table) {
                Some(table_ref) => Some(table_ref),
                None => store.get_table(*rhs_table),
              };
              op_fun(lhs,lhs_rows,lhs_columns,rhs,rhs_rows,rhs_columns, &mut self.scratch);
            }
            let out = self.memory.get_mut(*out_table).unwrap();
            out.rows = self.scratch.rows;
            out.columns = self.scratch.columns;
            out.row_ids = self.scratch.row_ids.clone();
            out.column_ids = self.scratch.column_ids.clone();
            out.column_aliases = self.scratch.column_aliases.clone();
            out.row_aliases = self.scratch.row_aliases.clone();
            out.data = self.scratch.data.clone();
            self.scratch.clear();
          }*/
        },
        /*
        Constraint::Filter{comparator, lhs, rhs, memory} => {
          operations::compare(comparator, *lhs as usize, *rhs as usize, *memory as usize, &mut self.memory, &mut self.column_lengths);
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
          /*
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
        Constraint::CopyTable{from_table, to_table} => {
          let from_table_ref = self.memory.get(*from_table).unwrap();
          let mut changes = vec![Change::NewTable{id: *to_table, rows: from_table_ref.rows, columns: from_table_ref.columns}];
          for (alias, ix) in from_table_ref.column_aliases.iter() {
            changes.push(Change::RenameColumn{table: *to_table, column_ix: *ix, column_alias: *alias});
          }
          for (col_ix, column) in from_table_ref.data.iter().enumerate() {
            for (row_ix, data) in column.iter().enumerate() {
              changes.push(Change::Set{table: *to_table, row: Index::Index(row_ix as u64 + 1), column: Index::Index(col_ix as u64 + 1), value: data.clone()});
            }
          }
          store.process_transaction(&Transaction::from_changeset(changes));
        },
        Constraint::NewTable{id, rows, columns} => {
          match id {
            TableId::Global(id) => {
              store.process_transaction(&Transaction::from_change(
                Change::NewTable{id: *id, rows: *rows, columns: *columns},
              ));
            }
            _ => (),
          }
        },
        _ => (),
      } 
      
    }
    self.updated = true;
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
  NewTable{id: TableId, rows: u64, columns: u64},
  TableColumn{table: u64, column_ix: u64, column_id: u64},
  // Input Constraints
  Reference{table: u64, rows: Vec<u64>, columns: Vec<u64>, destination: (u64, u64, u64)},
  Scan {table: TableId, rows: Vec<Index>, columns: Vec<Index>},
  Identifier {id: u64},
  ChangeScan {table: u64, column: u64, input: u64},
  // Transform Constraints
  Filter {comparator: operations::Comparator, lhs: u64, rhs: u64, memory: u64},
  Function {operation: operations::Function, parameters: Vec<(TableId, Vec<Index>, Vec<Index>)>, output: Vec<TableId>},
  Constant {table: TableId, row: Index, column: Index, value: i64},
  Condition {truth: u64, result: u64, default: u64, memory: u64},
  IndexMask {source: u64, truth: u64, memory: u64},
  // Identity Constraints
  CopyTable {from_table: u64, to_table: u64},
  AliasTable {table: TableId, alias: u64},
  CopyOutput {memory: u64, output: u64},
  // Output Constraints
  Insert {from: (u64, u64, u64), to: (u64, u64, u64)},
  Append {memory: u64, table: u64, column: u64},
}

impl fmt::Debug for Constraint {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Constraint::Reference{table, rows, columns, destination} => write!(f, "Reference(@{:#x}(rows: {:?}, cols: {:?}) -> {:?})", table, rows, columns, destination),
      Constraint::Data{table, column} => write!(f, "Data(#{:#x}({:#x}))", table, column),
      Constraint::NewTable{id, rows, columns} => write!(f, "NewTable(#{:?}({:?}x{:?}))", id, rows, columns),
      Constraint::Scan{table, rows, columns} => write!(f, "Scan(#{:?}({:?} x {:?}))", table, rows, columns),
      Constraint::ChangeScan{table, column, input} => write!(f, "ChangeScan(#{:#x}({:#x}) -> I{:?})", table, column, input),
      Constraint::Filter{comparator, lhs, rhs, memory} => write!(f, "Filter({:#x} {:?} {:#x} -> M{:?})", lhs, comparator, rhs, memory),
      Constraint::Function{operation, parameters, output} => write!(f, "Fxn::{:?}{:?} -> {:?}", operation, parameters, output),
      Constraint::Constant{table, row, column, value} => write!(f, "Constant({:?} -> #{:?}({:?}, {:?}))", value, table, row, column),
      Constraint::CopyTable{from_table, to_table} => write!(f, "CopyTable({:#x} -> {:#x})", from_table, to_table),
      Constraint::AliasTable{table, alias} => write!(f, "AliasLocalTable({:?} -> {:#x})", table, alias),
      Constraint::CopyOutput{memory, output} => write!(f, "CopyOutput(M{:#x} -> O{:#x})", memory, output),
      Constraint::Condition{truth, result, default, memory} => write!(f, "Condition({:?} ? {:?} | {:?} -> M{:?})", truth, result, default, memory),
      Constraint::Identifier{id} => write!(f, "Identifier({:#x})", id),
      Constraint::IndexMask{source, truth, memory} => write!(f, "IndexMask({:#x}, {:#x} -> M{:#x})", source, truth, memory),
      Constraint::Insert{from, to} => write!(f, "Insert({:?} -> {:?})",  from, to),
      Constraint::Append{memory, table, column} => write!(f, "Append(M{:#x} -> #{:#x}[{:#x}])",  memory, table, column),
      Constraint::TableColumn{table, column_ix, column_id}  => write!(f, "TableColumn(#{:#x}({:#x}) -> {:#x})",  table, column_ix, column_id),
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
