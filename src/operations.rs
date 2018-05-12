// # Operations

// ## Prelude

use alloc::{String, Vec, fmt};
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

pub fn math_add(parameters: &Vec<&Vec<Value>>, register: &mut Vec<Value>) {
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
}

pub fn math_subtract(parameters: &Vec<&Vec<Value>>, register: &mut Vec<Value>) {
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
}

// ## Comparators

#[repr(u8)]
#[derive(Clone)]
pub enum Comparator {
  LessThan,
  GreaterThan,
  LessThanOrEqual,
  GreaterThanOrEqual,
  Equal,
  NotEqual
}

pub fn compare(comparator: &Comparator, lhs: &Vec<Value>, rhs: &Vec<Value>, register: &mut Vec<Value>) {
  for i in 0 .. lhs.len() {
    let x = &lhs[i];
    let y = &rhs[i];
    match (x, y) {
      (Value::Number(lhs_val), Value::Number(rhs_val)) => {
        let truth = match comparator {
          Comparator::LessThan           => Value::Bool(lhs_val < rhs_val),
          Comparator::GreaterThan        => Value::Bool(lhs_val > rhs_val),
          Comparator::LessThanOrEqual    => Value::Bool(lhs_val <= rhs_val),
          Comparator::GreaterThanOrEqual => Value::Bool(lhs_val >= rhs_val),
          Comparator::Equal              => Value::Bool(lhs_val == rhs_val),
          Comparator::NotEqual           => Value::Bool(lhs_val != rhs_val),
        };
        if register.len() <= i {
          register.push(truth);
        } else {
          register[i] = truth;
        }
      }, 
      _ => (),
    }
  }
}

impl fmt::Debug for Comparator {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      GreaterThan => write!(f, ">"),
      LessThan => write!(f, "<"),
      LessThanOrEqual => write!(f, "<="),
      GreaterThanOrEqual => write!(f, ">="),
      Equal => write!(f, "="),
      NotEqual => write!(f, "!="),
      _ => Ok(()),
    }
  }
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