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

pub fn math_add(parameters: Vec<&Register>) -> Vec<Vec<Value>> {
  let mut output: Vec<Vec<Value>> = vec![];
  if parameters.len() == 2 {
    let lhs = parameters[0];
    let rhs = parameters[1];
    let result: Vec<Value> = lhs.data.iter().zip(rhs.data.iter()).map(|(x, y)| { 
      match x {
        Value::Number(lhs_val) => match y {
          Value::Number(rhs_val) => Value::from_u64(lhs_val + rhs_val),
          _ => Value::Empty,
        }
        _ => Value::Empty,
      }
    }).collect();
    output.push(result);
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