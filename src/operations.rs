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
  Subtract,
  //Multiply,
  //Divide,
  //Power,
}

pub fn math_add(parameters: &Vec<&Vec<Value>>, register: &mut Vec<Value>) -> Vec<Value> {
  let mut output = Vec::new();
  if parameters.len() == 2 {
    let lhs = &parameters[0];
    let rhs = &parameters[1];
    for i in 0 .. lhs.len() {     
      match (&lhs[i], &rhs[i]) {
        (Value::Number(x), Value::Number(y)) => {
          let a = Value::from_i64(x + y);
          if register.len() <= i {
            register.push(a);
          } else {
            register[i] = a;
          }
        },
        _ => (),
      } 
    }
  }
  output
}

pub fn math_subtract(parameters: &Vec<&Vec<Value>>, register: &mut Vec<Value>) -> Vec<Value> {
  let mut output = Vec::new();
  if parameters.len() == 2 {
    let lhs = &parameters[0];
    let rhs = &parameters[1];
    for i in 0 .. lhs.len() {     
      match (&lhs[i], &rhs[i]) {
        (Value::Number(x), Value::Number(y)) => {
          let a = Value::from_i64(x - y);
          if register.len() <= i {
            register.push(a);
          } else {
            register[i] = a;
          }
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