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

#[repr(u8)]
#[derive(Clone, PartialEq)]
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

#[macro_export]
macro_rules! logic {
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
                       else { lhs_rows[j].as_u64().unwrap() as usize - 1 };
            let rrix = if rhs_rows.is_empty() { j }
                       else { rhs_rows[j].as_u64().unwrap() as usize - 1 };
            match (&lhs.data[lcix][lrix], &rhs.data[rcix][rrix]) {
              (Value::Bool(x), Value::Bool(y)) => {
                out.data[i][j] = Value::Bool(*x $op *y);
              },
              _ => (),
            }
          }
        }
      // Operate with scalar on the left
      } else if lhs_is_scalar {
        out.grow_to_fit(rhs_height, rhs_width);        
        for i in 0..rhs_width as usize {
          let lcix = if lhs_columns.is_empty() { 0 }
                    else { lhs_columns[0].as_u64().unwrap() as usize - 1 };
          let rcix = if rhs_columns.is_empty() { i }
                    else { rhs_columns[i].as_u64().unwrap() as usize - 1 };
          for j in 0..rhs_height as usize {
            let lrix = if lhs_rows.is_empty() { 0 }
                       else { lhs_rows[0].as_u64().unwrap() as usize - 1 };
            let rrix = if rhs_rows.is_empty() { j }
                       else { rhs_rows[j].as_u64().unwrap() as usize - 1 };
            match (&lhs.data[lcix][lrix], &rhs.data[rcix][rrix]) {
              (Value::Bool(x), Value::Bool(y)) => {
                out.data[i][j] = Value::Bool(*x $op *y);
              },
              _ => (),
            }
          }
        }
      // Operate with scalar on the right
      } else if rhs_is_scalar {
        out.grow_to_fit(lhs_height, lhs_width);        
        for i in 0..lhs_width as usize {
          let lcix = if lhs_columns.is_empty() { i }
                    else { lhs_columns[i].as_u64().unwrap() as usize - 1 };
          let rcix = if rhs_columns.is_empty() { 0 }
                    else { rhs_columns[0].as_u64().unwrap() as usize - 1 };
          for j in 0..lhs_height as usize {
            let lrix = if lhs_rows.is_empty() { j }
                       else { lhs_rows[j].as_u64().unwrap() as usize - 1 };
            let rrix = if rhs_rows.is_empty() { 0 }
                       else { rhs_rows[0].as_u64().unwrap() as usize - 1 };
            match (&lhs.data[lcix][lrix], &rhs.data[rcix][rrix]) {
              (Value::Bool(x), Value::Bool(y)) => {
                out.data[i][j] = Value::Bool(*x $op *y);
              },
              _ => (),
            }
          }
        }
      }
    }
  )
}

logic!{logic_and, &&}
logic!{logic_or, ||}
logic!{logic_undefined, &&}