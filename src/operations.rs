// # Operations

// ## Prelude

use alloc::{String, Vec};
use runtime::{Constraint, Register};
use table::Value;

/*
Queries are compiled down to a Plan, which is a sequence of Operations that 
work on the supplied data.
*/

// ## Functions

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Function {
  Add,
  //Subtract,
  //Multiply,
  //Divide,
  //Power,
}

pub fn math_add(parameters: Vec<&Vec<Value>>) -> Vec<Value> {
  let mut output = Vec::new();
  if parameters.len() == 2 {
    let lhs = &parameters[0];
    let rhs = &parameters[1];
    let mut result: Vec<Value> = Vec::with_capacity(lhs.len());
    
    for i in 1 .. lhs.len() + 1 {     
      match (lhs.get(i), rhs.get(i)) {
        (Some(Value::Number(x)), Some(Value::Number(y))) => {
          let a = Value::from_u64(x + y);
          output.push(a);
        },
        _ => (),
      } 
    }
  }
  output
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
  pub constraints: Vec<Constraint>,
}

impl Plan {
  pub fn new() -> Plan {
    Plan {
      constraints: Vec::new(),
    }
  }
}