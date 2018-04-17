// # Mech Runtime

/* 
 The Mech Runtime is the engine that drives computations in Mech. The 
 runtime is comprised of "Blocks", interconnected by "Pipes" of records.
 Blocks can interact with the database, by Scanning for records that 
 match a pattern, or by Projecting computed records into the database.
*/

// ## Prelude

use table::{Value};
use alloc::{fmt, Vec};
use database::{Interner, Change};
use hashmap_core::map::HashMap;
use indexes::Hasher;

// ## Runtime

#[derive(Clone)]
pub struct Runtime {
  pub ready_mask: usize,
  pub blocks: Vec<Block>,
  pub pipes_map: HashMap<(u64, u64), Vec<Address>>,
}

impl Runtime {

  pub fn new() -> Runtime {
    Runtime {
      ready_mask: 0,
      blocks: Vec::new(),
      pipes_map: HashMap::new(),
    }
  }

  pub fn register_block(&mut self, mut block: Block, store: &Interner) {
    block.id = self.blocks.len() + 1;
    for constraint in &block.constraints {
      match constraint {
        Constraint::Scan{table, attribute, register_mask} => {
          self.pipes_map.insert((*table, *attribute), vec![Address{block: block.id, register: block.input_registers.len()}]);
          block.input_registers.push(Register::new());
        },
        Constraint::Insert{table, attribute, register_mask} => {
          block.output_registers.push(Register::new());
        },
      }
    }
    self.blocks.push(block.clone());
  } 

  pub fn process_change(&mut self, change: &Change) {
    match change {
      Change::Add(add) => {
        match self.pipes_map.get(&(add.table, add.attribute)) {
          Some(address) => {
            println!("{:?} {:?}", add, address);
          },
          _ => (),
        }
      },
      _ => (),
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
    write!(f, "@({:?}, {:?})", self.block, self.register)
  }
}


#[derive(Clone)]
pub struct Register {
  pub data: Vec<Value>,
}

impl Register {
  
  pub fn new() -> Register { 
    Register {
      data: Vec::new(),
    }
  }

}

impl fmt::Debug for Register {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{:?}", self.data).unwrap();
      Ok(())
    }
}


#[derive(Clone)]
pub struct Block {
  pub id: usize,
  pub input_registers: Vec<Register>,
  pub intermediate_registers: Vec<Register>,
  pub output_registers: Vec<Register>,
  pub constraints: Vec<Constraint>,
}

impl Block {
  
  pub fn new() -> Block { 
    Block {
      id: 0,
      input_registers: Vec::with_capacity(32),
      intermediate_registers: Vec::with_capacity(32),
      output_registers: Vec::with_capacity(32),
      constraints: Vec::with_capacity(32),
    }
  }

}

impl fmt::Debug for Block {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "┌────────────────────────────────────────┐\n").unwrap();
      write!(f, "│ Block #{:?}\n", self.id).unwrap();
      write!(f, "├────────────────────────────────────────┤\n").unwrap();
      write!(f, "│ Input: {:?}\n", self.input_registers.len()).unwrap();
      for register in &self.input_registers {
        write!(f, "│  > {:?}\n", register).unwrap();
      }
      write!(f, "│ Intermediate: {:?}\n", self.intermediate_registers.len()).unwrap();
      for register in &self.intermediate_registers {
        write!(f, "│  > {:?}\n", register).unwrap();
      }
      write!(f, "│ Output: {:?}\n", self.output_registers.len()).unwrap();
      for register in &self.output_registers {
        write!(f, "│  > {:?}\n", register).unwrap();
      }
      write!(f, "│ Constraints: {:?}\n", self.constraints.len()).unwrap();
      for constraint in &self.constraints {
        write!(f, "│  > {:?}\n", constraint).unwrap();
      }
      write!(f, "└────────────────────────────────────────┘\n").unwrap();
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
  Scan { table: u64, attribute: u64, register_mask: u64 },
  Insert {table: u64, attribute: u64, register_mask: u64},
}

impl fmt::Debug for Constraint {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      match self {
        Constraint::Scan{table, attribute, register_mask} => write!(f, "Scan({:#x}, {:#x})", table, attribute).unwrap(),
        Constraint::Insert{table, attribute, register_mask} => write!(f, "Insert({:#x}, {:#x})", table, attribute).unwrap(),
        _ => (),
      }
      Ok(())
    }
}