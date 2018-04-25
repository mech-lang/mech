// # Operations

// ## Prelude

use alloc::{String, Vec};

/*
Queries are compiled down to a Plan, which is a sequence of Operations that 
work on the supplied data.
*/

// ## Functions

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Function {
  Add,
  Subtract,
  Multiply,
  Divide,
  Power,
}

// ## Comparators

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Comparators {
  LessThan,
  GreaterThan,
  LessThanOrEqual,
  GreaterThanOrEqual,
  Equal,
  NotEqual
}

// ## Plans

// Plans are an ordered list of operations.

#[derive(Debug, Clone)]
pub struct Plan {
  pub operations: Vec<Operation>,
}

impl Plan {
  pub fn new() -> Plan {
    Plan {
      operations: Vec::new(),
    }
  }
}

// Operations

// Operations are the core of Mech. They define what the language can do with data.

#[derive(Debug, Clone)]
pub enum Operation {
  Filter,
  Function,
}

impl Operation {

}