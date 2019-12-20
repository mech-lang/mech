// # Operations

// ## Prelude

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(not(feature = "no-std"))] use core::fmt;
use table::{Table, Value, TableId, Index};
use errors::ErrorType;
use quantities::{Quantity, QuantityMath, ToQuantity};

/*
Queries are compiled down to a Plan, which is a sequence of Operations that 
work on the supplied data.
*/

// ## Parameters

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Parameter {
  TableId (TableId),
  Index (Index),
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub fn $func_name(input: Vec<(String, Table)>) -> Table {
      // TODO Test for the right amount of inputs
      let (_, lhs) = &input[0];
      let (_, rhs) = &input[1];

      // Get the math dimensions
      let lhs_width  = lhs.columns;
      let rhs_width  = rhs.columns;
      let lhs_height = lhs.rows;
      let rhs_height = rhs.rows;

      let lhs_is_scalar = lhs_width == 1 && lhs_height == 1;
      let rhs_is_scalar = rhs_width == 1 && rhs_height == 1;

      // The tables are the same size
      let result: Table = if lhs_width == rhs_width && lhs_height == rhs_height {
        let mut out = Table::new(0, lhs_height, lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&lhs.data[i][j], &rhs.data[i][j]) {
              (Value::Number(x), Value::Number(y)) => {
                match x.$op(*y) {
                  Ok(op_result) => out.data[i][j] = Value::from_quantity(op_result),
                  //Err(error) => errors.push(error), // TODO Throw an error here
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
        out
      // Operate with scalar on the left
      } else if lhs_is_scalar {
        let mut out = Table::new(0, rhs_height, rhs_width);
        for i in 0..rhs_width as usize {
          for j in 0..rhs_height as usize {
            match (&lhs.data[0][0], &rhs.data[i][j]) {
              (Value::Number(x), Value::Number(y)) => {
                match x.$op(*y) {
                  Ok(op_result) => out.data[i][j] = Value::from_quantity(op_result),
                  //Err(error) => errors.push(error), // TODO Throw an error here
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
        out
      // Operate with scalar on the right
      } else if rhs_is_scalar {
        let mut out = Table::new(0, lhs_height, lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&lhs.data[i][j], &rhs.data[0][0]) {
              (Value::Number(x), Value::Number(y)) => {
                match x.$op(*y) {
                  Ok(op_result) => out.data[i][j] = Value::from_quantity(op_result),
                  //Err(error) => errors.push(error), // TODO Throw an error here
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
        out
      } else {
        Table::new(0, 1, 1)
      };
      result
    }
  )
}

binary_infix!{math_add, add}
binary_infix!{math_subtract, sub}
binary_infix!{math_multiply, multiply}
binary_infix!{math_divide, divide}
// FIXME this isn't actually right at all. ^ is not power in Rust
//binary_math!{math_power, add}

binary_infix!{compare_not_equal, not_equal}
binary_infix!{compare_equal, equal}
binary_infix!{compare_less_than_equal, less_than_equal}
binary_infix!{compare_greater_than_equal, greater_than_equal}
binary_infix!{compare_greater_than, greater_than}
binary_infix!{compare_less_than, less_than}

pub fn stat_sum(input: Vec<(String, Table)>) -> Table {
  let mut out = Table::new(0,1,1);
  let (field, table_ref) = &input[0];
  if field == "column" {
    let mut total = 0.to_quantity();
    for i in 0..table_ref.rows as usize {
      match table_ref.data[0][i] {
        Value::Number(x) => {
          total = total.add(x).unwrap();
        }
        _ => (),
      }
    }
    out.data[0][0] = Value::Number(total);
  }
  out
}

pub fn table_range(input: Vec<(String, Table)>) -> Table {
  let (_, lhs) = &input[0];
  let (_, rhs) = &input[1];
  let start = lhs.data[0][0].as_i64().unwrap();
  let end = rhs.data[0][0].as_i64().unwrap();
  let steps = (end - start) as usize + 1;
  let mut out = Table::new(0,steps as u64,1);
  for i in 0..steps {
    out.data[0][i] = Value::Number((start + i as i64).to_quantity())
  }
  out
}

// ## Logic

#[macro_export]
macro_rules! logic {
  ($func_name:ident, $op:tt) => (
    pub fn $func_name(input: Vec<(String, Table)>) -> Table {
      // TODO Test for the right amount of inputs
      let (_, lhs) = &input[0];
      let (_, rhs) = &input[1];

      // Get the math dimensions
      let lhs_width  = lhs.columns;
      let rhs_width  = rhs.columns;
      let lhs_height = lhs.rows;
      let rhs_height = rhs.rows;

      let lhs_is_scalar = lhs_width == 1 && lhs_height == 1;
      let rhs_is_scalar = rhs_width == 1 && rhs_height == 1;

      // The tables are the same size
      let result: Table = if lhs_width == rhs_width && lhs_height == rhs_height {
        let mut out = Table::new(0, lhs_height, lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&lhs.data[i][j], &rhs.data[i][j]) {
              (Value::Bool(x), Value::Bool(y)) => {
                out.data[i][j] = Value::Bool(*x $op *y);
              },
              _ => (),
            }
          }
        }
        out
      // Operate with scalar on the left
      } else if lhs_is_scalar {
        let mut out = Table::new(0, rhs_height, rhs_width);
        for i in 0..rhs_width as usize {
          for j in 0..rhs_height as usize {
            match (&lhs.data[0][0], &rhs.data[i][j]) {
              (Value::Bool(x), Value::Bool(y)) => {
                out.data[i][j] = Value::Bool(*x $op *y);
              },
              _ => (),
            }
          }
        }
        out
      // Operate with scalar on the right
      } else if rhs_is_scalar {
        let mut out = Table::new(0, lhs_height, lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&lhs.data[i][j], &rhs.data[0][0]) {
              (Value::Bool(x), Value::Bool(y)) => {
                out.data[i][j] = Value::Bool(*x $op *y);
              },
              _ => (),
            }
          }
        }
        out
      } else {
        Table::new(0, 1, 1)
      };
      result
    }
  )
}

logic!{logic_and, &&}
logic!{logic_or, ||}