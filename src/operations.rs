// # Operations

// ## Prelude

use alloc::string::String;
use alloc::vec::Vec;
use alloc::fmt;
use runtime::{Constraint, Register};
use table::{Table, Value};
use indexes::{TableIndex};
use database::{Interner};

/*
Queries are compiled down to a Plan, which is a sequence of Operations that 
work on the supplied data.
*/

// ## Functions

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum Function {
  Add, 
  Subtract, 
  Multiply, 
  Divide,
  Power,
  Concatenate,
  Undefined
}

#[macro_export]
macro_rules! binary_math {
  ($func_name:ident, $op:tt) => (
    pub fn $func_name(lhs_table: Option<&Table>, lhs_rows: &Vec<u64>, lhs_columns: &Vec<u64>, 
                      rhs_table: Option<&Table>, rhs_rows: &Vec<u64>, rhs_columns: &Vec<u64>,
                      out: &mut Table) {
      match (lhs_table, rhs_table) {
        // we have both tables, so let's do some math
        (Some(lhs), Some(rhs)) => {
          // We're operating on the whole of both tables
          if lhs_rows.is_empty() && rhs_rows.is_empty() && lhs_columns.is_empty() && rhs_columns.is_empty() {
            // The tables are the same size
            if lhs.rows == rhs.rows && lhs.columns == rhs.columns {
              out.grow_to_fit(lhs.rows, lhs.columns);
              for i in (0..lhs.columns) {
                for j in (0..lhs.rows) {
                  match (&lhs.data[i][j], &rhs.data[i][j]) {
                    (Value::Number(x), Value::Number(y)) => {
                      out.data[i][j] = Value::from_i64(x $op y);
                    },
                    _ => (),
                  }
                  
                }
              }
            }
          }
        },
        _ => (),
      }
      /* 
      // Operate element wise
      if lhs.len() == rhs.len() {
        for i in 0 .. lhs.len() {     
          match (&lhs[i], &rhs[i]) {
            (Value::Number(x), Value::Number(y)) => {
              out.push(Value::from_i64(*x $op *y));
            },
            _ => {},
          } 
        }
      // Add vector to scalar  
      }
        /* else if lhs_length == 1 && rhs_length > 1 {
          for i in 1 .. rhs_length + 1 {     
            match (store.index(1, lhs), store.index(i, rhs)) {
              (Some(Value::Number(x)), Some(Value::Number(y))) => {
                store.set_cell(i, out, Value::from_i64(*x as i64 $op *y as i64)); 
              },
              _ => {store.set_cell(i, out, Value::Empty);},
            } 
            out_length = rhs_length;
          }
        } else if lhs_length > 1 && rhs_length == 1 {
          for i in 1 .. lhs_length + 1 {     
            match (store.index(i, lhs), store.index(1, rhs)) {
              (Some(Value::Number(x)), Some(Value::Number(y))) => {
                store.set_cell(i, out, Value::from_i64(*x as i64 $op *y as i64)); 
              },
              _ => {store.set_cell(i, out, Value::Empty);},
            }
            out_length = lhs_length; 
          }
        }
        lengths[out - 1] = out_length as u64;*/*/
    }
    
  )
}

binary_math!{math_add, +}
binary_math!{math_subtract, -}
binary_math!{math_multiply, *}
binary_math!{math_divide, /}
// FIXME this isn't actually right at all. ^ is not power in Rust
binary_math!{math_power, ^}

pub fn undefined(lhs_table: Option<&Table>, lhs_rows: &Vec<u64>, lhs_columns: &Vec<u64>, 
                 rhs_table: Option<&Table>, rhs_rows: &Vec<u64>, rhs_columns: &Vec<u64>,
                 out: &mut Table) {
}

pub fn concatenate(lhs_table: Option<&Table>, lhs_rows: &Vec<u64>, lhs_columns: &Vec<u64>, 
                   rhs_table: Option<&Table>, rhs_rows: &Vec<u64>, rhs_columns: &Vec<u64>,
                   out: &mut Table) {
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
      Comparator::GreaterThan => write!(f, ">"),
      Comparator::LessThan => write!(f, "<"),
      Comparator::LessThanOrEqual => write!(f, "<="),
      Comparator::GreaterThanOrEqual => write!(f, ">="),
      Comparator::Equal => write!(f, "="),
      Comparator::NotEqual => write!(f, "!="),
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