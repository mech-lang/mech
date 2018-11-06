// # Operations

// ## Prelude

use alloc::vec::Vec;
use alloc::fmt;
use table::{Table, Value, Index};

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
  HorizontalConcatenate,
  VerticalConcatenate,
  Undefined
}

#[macro_export]
macro_rules! binary_math {
  ($func_name:ident, $op:tt) => (
    pub fn $func_name(lhs: &Table, lhs_rows: &Vec<Index>, lhs_columns: &Vec<Index>, 
                      rhs: &Table, rhs_rows: &Vec<Index>, rhs_columns: &Vec<Index>,
                      out: &mut Table) {
      // Get the math dimensions
      let lhs_height = if lhs_rows.is_empty() { lhs.rows }
                       else { lhs_rows.len() as u64 };
      let lhs_width  = if lhs_columns.is_empty() { lhs.columns }
                       else { lhs_columns.len() as u64 };
      let rhs_height = if rhs_rows.is_empty() { rhs.rows }
                       else { rhs_rows.len() as u64 };
      let rhs_width  = if rhs_columns.is_empty() { rhs.columns }
                       else { rhs_columns.len() as u64 }; 
      let lhs_is_scalar = lhs_columns.is_empty() && lhs_width == 1 && lhs_rows.is_empty() && lhs_height == 1;
      let rhs_is_scalar = rhs_columns.is_empty() && rhs_width == 1 && rhs_rows.is_empty() && rhs_height == 1;

      // The tables are the same size, and we're operating over the whole of both
      if lhs_columns.is_empty() && rhs_columns.is_empty() {
        out.grow_to_fit(lhs_height, lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&lhs.data[i][j], &rhs.data[i][j]) {
              (Value::Number(x), Value::Number(y)) => {
                out.data[i][j] = Value::from_i64(x $op y);
              },
              _ => (),
            }
          }
        }
      // Operate with scalar on the left
      } else if lhs_is_scalar {
        out.grow_to_fit(rhs_height, rhs_width);
        for column in rhs_columns {
          for j in 1..rhs_height + 1 {
            match (&lhs.data[0][0], rhs.index(&Index::Index(j), column).unwrap()) {
              (Value::Number(x), Value::Number(y)) => {
                
                out.data[0][j as usize - 1] = Value::from_i64(x $op y);
              },
              _ => (),
            }
          }
        }
      // Operate with scalar on the right
      } else if rhs_is_scalar {
        out.grow_to_fit(lhs_height, lhs_width);
        for column in lhs_columns {
          for j in 1..lhs_height + 1 {
            match (lhs.index(&Index::Index(j), column).unwrap(), &rhs.data[0][0]) {
              (Value::Number(x), Value::Number(y)) => {
                out.data[0][j as usize - 1] = Value::from_i64(x $op y);
              },
              _ => (),
            }
          }
        }
      // Operate on a selection of columns
      } else if rhs_width == lhs_width && lhs_height == rhs_height &&!lhs_columns.is_empty() && !lhs_columns.is_empty()  {
        out.grow_to_fit(lhs_height, lhs_width);
        for (i, (lhs_column, rhs_column)) in lhs_columns.iter().zip(rhs_columns).enumerate() {
          for j in 1..lhs_height + 1 {
            match (lhs.index(&Index::Index(j), lhs_column).unwrap(), rhs.index(&Index::Index(j), rhs_column).unwrap()) {
              (Value::Number(x), Value::Number(y)) => {
                out.data[i][j as usize - 1] = Value::from_i64(x $op y);
              },
              _ => (),
            }
          }
        }
      }
    }
  )
}

binary_math!{math_add, +}
binary_math!{math_subtract, -}
binary_math!{math_multiply, *}
binary_math!{math_divide, /}
// FIXME this isn't actually right at all. ^ is not power in Rust
binary_math!{math_power, ^}
binary_math!{undefined, +}

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
    }
  }
}

/*
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
*/
