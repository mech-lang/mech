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
  Multiply,
  Divide,
  //Power,
}

// TODO: Abstract this.... this is quick and dirty now just so I can get it working.

pub fn math_add(parameters: &Vec<u64>, output: & Vec<u64>, store: &mut Table, lengths: &mut Vec<u64>) {
  if parameters.len() == 2 && output.len() == 1 {
    // Extract parameters
    let lhs = parameters[0] as usize;
    let rhs = parameters[1] as usize;
    let out = output[0] as usize;
    let lhs_length = lengths[lhs - 1] as usize;
    let rhs_length = lengths[rhs - 1] as usize;
    let mut out_length = 0;
    // Operate element wise
    if lhs_length == rhs_length {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 + *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = lhs_length;
      }
    // Add vector to scalar  
    } else if lhs_length == 1 && rhs_length > 1 {
      for i in 1 .. rhs_length + 1 {     
        match (store.index(1, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 + *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = rhs_length;
      }
    } else if lhs_length > 1 && rhs_length == 1 {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(1, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 + *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        }
        out_length = lhs_length; 
      }
    }
    lengths[out - 1] = out_length as u64;
  }
}

pub fn math_subtract(parameters: &Vec<u64>, output: & Vec<u64>, store: &mut Table, lengths: &mut Vec<u64>) {
  if parameters.len() == 2 && output.len() == 1 {
    // Extract parameters
    let lhs = parameters[0] as usize;
    let rhs = parameters[1] as usize;
    let out = output[0] as usize;
    let lhs_length = lengths[lhs - 1] as usize;
    let rhs_length = lengths[rhs - 1] as usize;
    let mut out_length = 0;
    // Operate element wise
    if lhs_length == rhs_length {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 - *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = lhs_length;
      }
    // Add vector to scalar  
    } else if lhs_length == 1 && rhs_length > 1 {
      for i in 1 .. rhs_length + 1 {     
        match (store.index(1, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 - *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = rhs_length;
      }
    } else if lhs_length > 1 && rhs_length == 1 {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(1, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 - *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        }
        out_length = lhs_length; 
      }
    }
    lengths[out - 1] = out_length as u64;
  }
}

pub fn math_multiply(parameters: &Vec<u64>, output: & Vec<u64>, store: &mut Table, lengths: &mut Vec<u64>) {
  if parameters.len() == 2 && output.len() == 1 {
    // Extract parameters
    let lhs = parameters[0] as usize;
    let rhs = parameters[1] as usize;
    let out = output[0] as usize;
    let lhs_length = lengths[lhs - 1] as usize;
    let rhs_length = lengths[rhs - 1] as usize;
    let mut out_length = 0;
    // Operate element wise
    if lhs_length == rhs_length {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 * *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = lhs_length;
      }
    // Add vector to scalar  
    } else if lhs_length == 1 && rhs_length > 1 {
      for i in 1 .. rhs_length + 1 {     
        match (store.index(1, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 * *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = rhs_length;
      }
    } else if lhs_length > 1 && rhs_length == 1 {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(1, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 * *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        }
        out_length = lhs_length; 
      }
    }
    lengths[out - 1] = out_length as u64;
  }
}

pub fn math_divide(parameters: &Vec<u64>, output: & Vec<u64>, store: &mut Table, lengths: &mut Vec<u64>) {
  if parameters.len() == 2 && output.len() == 1 {
    // Extract parameters
    let lhs = parameters[0] as usize;
    let rhs = parameters[1] as usize;
    let out = output[0] as usize;
    let lhs_length = lengths[lhs - 1] as usize;
    let rhs_length = lengths[rhs - 1] as usize;
    let mut out_length = 0;
    // Operate element wise
    if lhs_length == rhs_length {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 / *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = lhs_length;
      }
    // Add vector to scalar  
    } else if lhs_length == 1 && rhs_length > 1 {
      for i in 1 .. rhs_length + 1 {     
        match (store.index(1, lhs), store.index(i, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 / *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        } 
        out_length = rhs_length;
      }
    } else if lhs_length > 1 && rhs_length == 1 {
      for i in 1 .. lhs_length + 1 {     
        match (store.index(i, lhs), store.index(1, rhs)) {
          (Some(Value::Number(x)), Some(Value::Number(y))) => {
            store.set_cell(i, out, Value::from_i64(*x as i64 / *y as i64)); 
          },
          _ => {store.set_cell(i, out, Value::Empty);},
        }
        out_length = lhs_length; 
      }
    }
    lengths[out - 1] = out_length as u64;
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

pub fn compare(comparator: &Comparator, lhs: usize, rhs: usize, output: usize, store: &mut Table, lengths: &mut Vec<u64>) {
  let lhs_length = lengths[lhs - 1] as usize;
  let rhs_length = lengths[rhs - 1] as usize;
  let out = output as usize;
  let mut out_length = 0;
  if lhs_length == rhs_length {
    for i in 1 .. lhs_length + 1 {     
      match (store.index(i, lhs), store.index(i, rhs)) {
        (Some(&Value::Number(lhs_val)), Some(&Value::Number(rhs_val))) => {
          let truth = match comparator {
            Comparator::LessThan           => Value::Bool(lhs_val < rhs_val),
            Comparator::GreaterThan        => Value::Bool(lhs_val > rhs_val),
            Comparator::LessThanOrEqual    => Value::Bool(lhs_val <= rhs_val),
            Comparator::GreaterThanOrEqual => Value::Bool(lhs_val >= rhs_val),
            Comparator::Equal              => Value::Bool(lhs_val == rhs_val),
            Comparator::NotEqual           => Value::Bool(lhs_val != rhs_val),
          };
          store.set_cell(i, output, truth);
        }, 
        _ => {store.set_cell(i, out, Value::Empty);},
      } 
      out_length = lhs_length;
    }
  // Add vector to scalar  
  } else if lhs_length == 1 && rhs_length > 1 {
    for i in 1 .. rhs_length + 1 {     
      match (store.index(1, lhs), store.index(i, rhs)) {
        (Some(&Value::Number(lhs_val)), Some(&Value::Number(rhs_val))) => {
          let truth = match comparator {
            Comparator::LessThan           => Value::Bool(lhs_val < rhs_val),
            Comparator::GreaterThan        => Value::Bool(lhs_val > rhs_val),
            Comparator::LessThanOrEqual    => Value::Bool(lhs_val <= rhs_val),
            Comparator::GreaterThanOrEqual => Value::Bool(lhs_val >= rhs_val),
            Comparator::Equal              => Value::Bool(lhs_val == rhs_val),
            Comparator::NotEqual           => Value::Bool(lhs_val != rhs_val),
          };
          store.set_cell(i, output, truth);
        }, 
        _ => {store.set_cell(i, out, Value::Empty);},
      } 
      out_length = rhs_length;
    }
  } else if lhs_length > 1 && rhs_length == 1 {
    for i in 1 .. lhs_length + 1 {     
      match (store.index(i, lhs), store.index(1, rhs)) {
        (Some(&Value::Number(lhs_val)), Some(&Value::Number(rhs_val))) => {
          let truth = match comparator {
            Comparator::LessThan           => Value::Bool(lhs_val < rhs_val),
            Comparator::GreaterThan        => Value::Bool(lhs_val > rhs_val),
            Comparator::LessThanOrEqual    => Value::Bool(lhs_val <= rhs_val),
            Comparator::GreaterThanOrEqual => Value::Bool(lhs_val >= rhs_val),
            Comparator::Equal              => Value::Bool(lhs_val == rhs_val),
            Comparator::NotEqual           => Value::Bool(lhs_val != rhs_val),
          };
          store.set_cell(i, output, truth);
        }, 
        _ => {store.set_cell(i, out, Value::Empty);},
      }
      out_length = lhs_length; 
    }
  }
  lengths[out - 1] = out_length as u64;
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