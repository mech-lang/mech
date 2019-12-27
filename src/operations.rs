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
  All,
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(input: Vec<(String, Table)>) -> Table {
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

pub extern "C" fn stat_sum(input: Vec<(String, Table)>) -> Table {
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
  } else if field == "row" {
    let mut total = 0.to_quantity();
    for i in 0..table_ref.columns as usize {
      match table_ref.data[i][0] {
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

pub extern "C" fn table_range(input: Vec<(String, Table)>) -> Table {
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

pub extern "C" fn set_any(input: Vec<(String, Table)>) -> Table {
  let mut out = Table::new(0,1,1);
  let (field, table_ref) = &input[0];
  if field == "column" {
    let mut result = Value::Bool(false);
    for i in 0..table_ref.rows as usize {
      match table_ref.data[0][i] {
        Value::Bool(true) => {
          result = Value::Bool(true);
        }
        _ => (),
      }
    }
    out.data[0][0] = result;
  } else if field == "row" {
    let mut result = Value::Bool(false);
    for i in 0..table_ref.columns as usize {
      match table_ref.data[i][0] {
        Value::Bool(true) => {
          result = Value::Bool(true);
        }
        _ => (),
      }
    }
    out.data[0][0] = result;    
  }
  out
}

pub extern "C" fn table_horizontal_concatenate(input: Vec<(String, Table)>) -> Table {
  let mut cat_table = Table::new(0,0,0);
  for (_, scanned) in input {
    // Do all the work here:
    if cat_table.rows == 0 {
      cat_table.grow_to_fit(scanned.rows,scanned.columns);
      cat_table.data = scanned.data;
    // We're adding a scalar to the table. Auto fill to height
    } else if scanned.rows == 1 {
      let start_col: usize = cat_table.columns as usize;
      let end_col: usize = (cat_table.columns + scanned.columns) as usize;
      let start_row: usize = 0;
      let end_row: usize = cat_table.rows as usize;
      cat_table.grow_to_fit(end_row as u64, end_col as u64);
      for i in 0..scanned.columns {
        for j in 0..cat_table.rows {
          cat_table.data[i as usize + start_col][j as usize] = scanned.data[i as usize][0].clone();
        }
      }
    } else if cat_table.rows == 1 {
      let old_width = cat_table.columns;
      let end_col: usize = (cat_table.columns + scanned.columns) as usize;
      cat_table.grow_to_fit(scanned.rows, end_col as u64);
      // copy old stuff
      for i in 0..old_width as usize {
        for j in 1..cat_table.rows as usize {
          cat_table.data[i][j] = cat_table.data[i][0].clone();
        }
      }
      // copy new stuff
      for i in 0..scanned.columns as usize {
        for j in 0..scanned.rows as usize {
          cat_table.data[i + old_width as usize][j] = scanned.data[i][j].clone();
        }
      }
    // We are cating two tables of the same height
    } else if cat_table.rows == scanned.rows {
      let cols = cat_table.columns as usize;
      cat_table.grow_to_fit(cat_table.rows, cat_table.columns + scanned.columns);
      for i in 0..scanned.columns as usize {
        for j in 0..cat_table.rows as usize {
          cat_table.data[cols+i][j] = scanned.data[i][j].clone();
        }
      }
    }
  }
  cat_table
}

pub extern "C" fn table_vertical_concatenate(input: Vec<(String, Table)>) -> Table {
  let mut cat_table = Table::new(0,0,0);
  for (_, scanned) in input {
    if cat_table.rows == 0 {
      cat_table.grow_to_fit(scanned.rows, scanned.columns);
      cat_table.data = scanned.data.clone();
    } else if cat_table.columns == scanned.columns {
      let mut i = 0;
      for column in &mut cat_table.data {
        let mut col = scanned.data[i].clone();
        column.append(&mut col);
        i += 1;
      }
      cat_table.grow_to_fit(cat_table.rows + scanned.rows, cat_table.columns);
    } else {
      // TODO Throw size error
    }
  }
  cat_table
}

// ## Logic

#[macro_export]
macro_rules! logic {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(input: Vec<(String, Table)>) -> Table {
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