// # Operations

// ## Prelude

use alloc::vec::Vec;
use alloc::fmt;
use table::{Table, Value, TableId, Index};

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
    pub fn $func_name(lhs: &Table, lhs_rows: &Vec<Value>, lhs_columns: &Vec<Value>, 
                      rhs: &Table, rhs_rows: &Vec<Value>, rhs_columns: &Vec<Value>,
                      out: &mut Table) {
      // Get the math dimensions
      let lhs_width  = if lhs_columns.is_empty() { lhs.columns }
                       else { lhs_columns.len() as u64 };
      let rhs_width  = if rhs_columns.is_empty() { rhs.columns }
                       else { rhs_columns.len() as u64 };      
      let lhs_height = if lhs_rows.is_empty() { lhs.rows }
                       else { lhs_rows.len() as u64 };
      let rhs_height = if rhs_rows.is_empty() { rhs.rows }
                       else { rhs_rows.len() as u64 };

      let lhs_is_scalar = lhs_width == 1 && lhs_height == 1;
      let rhs_is_scalar = rhs_width == 1 && rhs_height == 1;

      println!("{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n", lhs.data, lhs_rows, lhs_columns, rhs.data, rhs_rows, rhs_columns);
println!("OUT {:?}", out);

      // The tables are the same size
      if lhs_width == rhs_width && lhs_height == rhs_height {
        out.grow_to_fit(lhs_height, lhs_width);        
        for i in 0..lhs_width as usize {
          let lcix = if lhs_columns.is_empty() { i }
                    else { lhs_columns[i].as_u64().unwrap() as usize - 1 };
          let rcix = if rhs_columns.is_empty() { i }
                    else { rhs_columns[i].as_u64().unwrap() as usize - 1 };
          for j in 0..lhs_height as usize {
            let lrix = if lhs_rows.is_empty() { j }
                       else { lhs_rows[i].as_u64().unwrap() as usize - 1 };
            let rrix = if rhs_rows.is_empty() { j }
                       else { rhs_rows[i].as_u64().unwrap() as usize - 1 };
            println!("{:?}x{:?} {:?}x{:?}", lcix, lrix, rcix, rrix);
            match (&lhs.data[lcix][lrix], &rhs.data[rcix][rrix]) {
              (Value::Number(x), Value::Number(y)) => {
                out.data[i][j] = Value::from_i64(x $op y);
              },
              _ => (),
            }
            println!("{:?}", out);
          }
        }
      // Operate with scalar on the left
      } /*else if lhs_is_scalar {
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
      }*/
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
  NotEqual,
  Undefined
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
      Comparator::Undefined => write!(f, "Undefined Comparator"),
    }
  }
}

#[macro_export]
macro_rules! comparator {
  ($func_name:ident, $op:tt) => (
    pub fn $func_name(lhs: &Table, lhs_rows: &Option<TableId>, lhs_columns: &Option<TableId>, 
                      rhs: &Table, rhs_rows: &Option<TableId>, rhs_columns: &Option<TableId>,
                      out: &mut Table) {
                       /* 
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
                out.data[i][j] = Value::Bool(x $op y);
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
                
                out.data[0][j as usize - 1] = Value::Bool(x $op y);
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
                out.data[0][j as usize - 1] = Value::Bool(x $op y);
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
                out.data[i][j as usize - 1] = Value::Bool(x $op y);
              },
              _ => (),
            }
          }
        }
      }*/
    }
  )
}

comparator!{compare_greater_than, >}
comparator!{compare_less_than, <}
comparator!{compare_undefined, >}

// ## Logic

#[repr(u8)]
#[derive(Clone)]
pub enum Logic {
  And,
  Or,
  Undefined
}

impl fmt::Debug for Logic {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Logic::And => write!(f, "&"),
      Logic::Or => write!(f, "|"),
      Logic::Undefined => write!(f, "Undefined Logic"),
    }
  }
}