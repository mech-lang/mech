// # Operations

// ## Prelude

use alloc::{String, Vec, fmt};
use runtime::{Constraint, Register};
use table::{Table, Value};

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

pub fn math_add(parameters: &Vec<u64>, output: & Vec<u64>, store: &mut Table) {
  if parameters.len() == 2 && output.len() == 1 {
    let lhs = parameters[0] as usize;
    let rhs = parameters[1] as usize;
    let out = output[0] as usize;
    let lhs_count: usize = store.get_column(lhs).unwrap().len();
    let rhs_count: usize = store.get_column(rhs).unwrap().len();

    if lhs_count == rhs_count {
      for i in 1 .. lhs_count + 1 {     
        match (store.index(i, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 + *y as i64)); 
          },
          _ => (),
        } 
      }
    }

  }
}

pub fn math_subtract(parameters: &Vec<u64>, output: & Vec<u64>, store: &mut Table) {
  /*
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
  */
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

// ## Internal Functions

pub fn identity(source: &Vec<Value>, sink: u64, store: &mut Table) {
  store.grow_to_fit(source.len(), 0);
  for i in 1 .. source.len() + 1 {     
    store.set_cell(i, sink as usize, source[i - 1].clone());
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