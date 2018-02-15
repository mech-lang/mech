// # Mech Runtime

// The Mech Runtime is the engine that drives computations in Mech. The runtime
// is comprised of "Blocks", interconnected by "Pipes" of data they query and 
// publish. Blocks can interact with the database, by Scanning for records that 
// match a pattern, or by Projecting compted records into the database.

// ## Prelude

use eav::{Entity, Attribute, Value};

// ## Blocks

pub struct Address {
  pub node: u64,
  pub block: u64,
  pub register: u16,
}

pub struct Register {

  data: Vec<(Entity, Attribute, Value)>,
}

pub struct Block {
  pub ix: u64,
  pub input_registers: Vec<Register>,
  pub intermediate_registers: Vec<Register>,
  pub output_registers: Vec<Register>,
  pub constraints: Vec<Constraint>,
}


impl Block {
  
  pub fn new() -> Block {
    Block {
      ix: 0,
      input_registers: Vec::with_capacity(32),
      intermediate_registers: Vec::with_capacity(32),
      output_registers: Vec::with_capacity(32),
      constraints: Vec::with_capacity(32),
    }
  }

}

// ## Pipe

pub struct Pipe {
  input: Address,
  output: Address,
}

// ## Constraints

// Constraints put bounds on the data available for a block to work with. For 
// example, Scan constraints could bring data into the block, and a Join 
// constraint could match elements from 


pub enum Constraint {
  Scan { entity: u64, Attribute: u64, value: Value, register: Register },
}