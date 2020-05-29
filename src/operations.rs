
// # Operations

// ## Prelude

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use table::{Table, Value, TableId, Index};
use runtime::Runtime;
use block::{Block, IndexIterator};
use database::Database;
use errors::ErrorType;
use quantities::{Quantity, QuantityMath, ToQuantity};
use std::rc::Rc;
use std::cell::RefCell;
use hashbrown::HashMap;


pub type MechFunction = extern "C" fn(&Vec<(u64, TableId, Index, Index)>, &(TableId, Index, Index), block_tables: &mut HashMap<u64, Table>, database: &Rc<RefCell<Database>>);


pub extern "C" fn table_horizontal_concatenate(arguments: &Vec<(u64, TableId, Index, Index)>, out: &(TableId, Index, Index), block_tables: &mut HashMap<u64, Table>, database: &Rc<RefCell<Database>>) {
  let (out_table_id, out_rows, out_columns) = out;
  let mut db = database.borrow_mut();
  let mut column = 0;
  let mut out_rows = 0;
  // First pass, make sure the dimensions work out
  for (_, table_id, rows, columns) in arguments {
    let table = match table_id {
      TableId::Global(id) => db.tables.get(id).unwrap(),
      TableId::Local(id) => block_tables.get(id).unwrap(),
    };
    if out_rows == 0 {
      out_rows = table.rows;
    } else if table.rows != 1 && out_rows != table.rows {
      // TODO Throw an error here
    } else if table.rows > out_rows && out_rows == 1 {
      out_rows = table.rows
    }
  }
  let mut out_table = match out_table_id {
    TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
  };
  for (_, table_id, rows, columns) in arguments {
    let table = match table_id {
      TableId::Global(id) => db.tables.get(id).unwrap(),
      TableId::Local(id) => block_tables.get(id).unwrap(),
    };
    let rows_iter = if table.rows == 1 {
      IndexIterator::Constant(Index::Index(1))
    } else {
      IndexIterator::Range(1..=table.rows)
    };
    for (i,k) in (1..=out_rows).zip(rows_iter) {
      for j in 1..=table.columns {
        let value = table.get(&k,&Index::Index(j)).unwrap();
        unsafe {
          (*out_table).set(&Index::Index(i), &Index::Index(column+j), value);
        }
      }
    }
    column += 1;
  }
}

pub extern "C" fn table_range(arguments: &Vec<(u64, TableId, Index, Index)>, out: &(TableId, Index, Index), block_tables: &mut HashMap<u64, Table>, database: &Rc<RefCell<Database>>) {
  // TODO test argument count is 2 or 3
  // 2 -> start, end
  // 3 -> start, increment, end
  let (_, start_table_id, start_rows, start_columns) = &arguments[0];
  let (_, end_table_id, end_rows, end_columns) = &arguments[1];
  let (out_table_id, out_rows, out_columns) = out;
  let db = database.borrow_mut();
  let start_table = match start_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };
  let end_table = match end_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };
  let start_value = start_table.get(&Index::Index(1),&Index::Index(1)).unwrap();
  let end_value = end_table.get(&Index::Index(1),&Index::Index(1)).unwrap();
  let range = end_value.as_u64().unwrap() - start_value.as_u64().unwrap();
  match out_table_id {
    TableId::Local(id) => {
      let mut out_table = block_tables.get_mut(id).unwrap();
      for i in 1..=range as usize {
        out_table.set(&Index::Index(i), &Index::Index(1), Value::from_u64(i as u64));
      }
    }
    TableId::Global(id) => {

    }
  }
}

pub extern "C" fn math_add(arguments: &Vec<(u64, TableId, Index, Index)>, out: &(TableId, Index, Index), block_tables: &mut HashMap<u64, Table>, database: &Rc<RefCell<Database>>) {
  // TODO test argument count is 2
  let (_, lhs_table_id, lhs_rows, lhs_columns) = &arguments[0];
  let (_, rhs_table_id, rhs_rows, rhs_columns) = &arguments[1];
  let (out_table_id, out_rows, out_columns) = out;
  let mut db = database.borrow_mut();

  let mut out_table = match out_table_id {
    TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
  };

  let lhs_table = match lhs_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };
  let rhs_table = match rhs_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };
  let store = &db.store;

  // Figure out dimensions
  let equal_dimensions = if lhs_table.rows == rhs_table.rows
  { true } else { false };
  let lhs_scalar = if lhs_table.rows == 1 && lhs_table.columns == 1 
  { true } else { false };
  let rhs_scalar = if rhs_table.rows == 1 && rhs_table.columns == 1
  { true } else { false };

  let out_rows_count = unsafe{(*out_table).rows};

  let (mut lrix, mut lcix, mut rrix, mut rcix, mut out_rix, mut out_cix) = if rhs_scalar && lhs_scalar {
    (
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),               
    )
  } else if equal_dimensions {
    (
      IndexIterator::Range(1..=lhs_table.rows),
      IndexIterator::Constant(*lhs_columns),
      IndexIterator::Range(1..=rhs_table.rows),
      IndexIterator::Constant(*rhs_columns),
      IndexIterator::Range(1..=out_rows_count),
      IndexIterator::Constant(*out_columns),
    )
  } else if rhs_scalar {
    (
      IndexIterator::Range(1..=lhs_table.rows),
      IndexIterator::Constant(*lhs_columns),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Range(1..=out_rows_count),
      IndexIterator::Constant(*out_columns),
    )
  } else {
    (
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Range(1..=rhs_table.rows),
      IndexIterator::Constant(*rhs_columns),
      IndexIterator::Range(1..=out_rows_count),
      IndexIterator::Constant(*out_columns),
    )
  };

  let mut i = 1;

  loop {
    let l1 = lrix.next().unwrap().unwrap();
    let l2 = lcix.next().unwrap().unwrap();
    let r1 = rrix.next().unwrap().unwrap();
    let r2 = rcix.next().unwrap().unwrap();
    let o1 = out_rix.next().unwrap().unwrap();
    let o2 = out_cix.next().unwrap().unwrap();
    match (lhs_table.get_unchecked(l1,l2), 
            rhs_table.get_unchecked(r1,r2))
    {
      (lhs_value, rhs_value) => {
        match (lhs_value, rhs_value) {
          (Value::Number(x), Value::Number(y)) => {
            match x.add(y) {
              Ok(result) => {
                let function_result = Value::from_quantity(result);
                unsafe {
                  (*out_table).set_unchecked(o1, o2, function_result);
                }
              }
              Err(_) => (), // TODO Handle error here
            }
          }
          _ => (),
        }
      }
      _ => (),
    }
    if i >= lhs_table.rows {
      break;
    }
    i += 1;
  }
}

pub extern "C" fn math_subtract(arguments: &Vec<(u64, TableId, Index, Index)>, out: &(TableId, Index, Index), block_tables: &mut HashMap<u64, Table>, database: &Rc<RefCell<Database>>) {
  // TODO test argument count is 2
  let (_, lhs_table_id, lhs_rows, lhs_columns) = &arguments[0];
  let (_, rhs_table_id, rhs_rows, rhs_columns) = &arguments[1];
  let (out_table_id, out_rows, out_columns) = out;
  let mut db = database.borrow_mut();

  let mut out_table = match out_table_id {
    TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
  };

  let lhs_table = match lhs_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };
  let rhs_table = match rhs_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };
  let store = &db.store;

  // Figure out dimensions
  let equal_dimensions = if lhs_table.rows == rhs_table.rows
  { true } else { false };
  let lhs_scalar = if lhs_table.rows == 1 && lhs_table.columns == 1 
  { true } else { false };
  let rhs_scalar = if rhs_table.rows == 1 && rhs_table.columns == 1
  { true } else { false };

  let out_rows_count = unsafe{(*out_table).rows};

  let (mut lrix, mut lcix, mut rrix, mut rcix, mut out_rix, mut out_cix) = if rhs_scalar && lhs_scalar {
    (
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),               
    )
  } else if equal_dimensions {
    (
      IndexIterator::Range(1..=lhs_table.rows),
      IndexIterator::Constant(*lhs_columns),
      IndexIterator::Range(1..=rhs_table.rows),
      IndexIterator::Constant(*rhs_columns),
      IndexIterator::Range(1..=out_rows_count),
      IndexIterator::Constant(*out_columns),
    )
  } else if rhs_scalar {
    (
      IndexIterator::Range(1..=lhs_table.rows),
      IndexIterator::Constant(*lhs_columns),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Range(1..=out_rows_count),
      IndexIterator::Constant(*out_columns),
    )
  } else {
    (
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Constant(Index::Index(1)),
      IndexIterator::Range(1..=rhs_table.rows),
      IndexIterator::Constant(*rhs_columns),
      IndexIterator::Range(1..=out_rows_count),
      IndexIterator::Constant(*out_columns),
    )
  };

  let mut i = 1;

  loop {
    let l1 = lrix.next().unwrap().unwrap();
    let l2 = lcix.next().unwrap().unwrap();
    let r1 = rrix.next().unwrap().unwrap();
    let r2 = rcix.next().unwrap().unwrap();
    let o1 = out_rix.next().unwrap().unwrap();
    let o2 = out_cix.next().unwrap().unwrap();
    match (lhs_table.get_unchecked(l1,l2), 
            rhs_table.get_unchecked(r1,r2))
    {
      (lhs_value, rhs_value) => {
        match (lhs_value, rhs_value) {
          (Value::Number(x), Value::Number(y)) => {
            match x.sub(y) {
              Ok(result) => {
                let function_result = Value::from_quantity(result);
                unsafe {
                  (*out_table).set_unchecked(o1, o2, function_result);
                }
              }
              Err(_) => (), // TODO Handle error here
            }
          }
          _ => (),
        }
      }
      _ => (),
    }
    if i >= lhs_table.rows {
      break;
    }
    i += 1;
  }
}









/*

/*
Queries are compiled down to a Plan, which is a sequence of Operations that 
work on the supplied data.
*/

// ## Parameters

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Parameter {
  TableId (TableId),
  Index (Index),
  All,
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
      // TODO Test for the right amount of inputs
      let (_, lhs_rc) = &input[0];
      let (_, rhs_rc) = &input[1];
      let lhs = lhs_rc.borrow();
      let rhs = rhs_rc.borrow();

      // Get the math dimensions
      let lhs_width  = lhs.columns;
      let rhs_width  = rhs.columns;
      let lhs_height = lhs.rows;
      let rhs_height = rhs.rows;

      let lhs_is_scalar = lhs_width == 1 && lhs_height == 1;
      let rhs_is_scalar = rhs_width == 1 && rhs_height == 1;

      let mut out = output.borrow_mut();

      // The tables are the same size
      if lhs_width == rhs_width && lhs_height == rhs_height {
        out.grow_to_fit(lhs_height,lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&*lhs.data[i][j], &*rhs.data[i][j]) {
              (Value::Number(x), Value::Number(y)) => {
                match x.$op(*y) {
                  Ok(op_result) => {
                    let value = Value::from_quantity(op_result);
                    let value_rc = Rc::new(value);
                    out.data[i][j] = value_rc;
                  },
                  //Err(error) => errors.push(error), // TODO Throw an error here
                  _ => (),
                }
              },
              (Value::String(x), Value::String(y)) => {
                out.data[i][j] = Rc::new(Value::Bool(lhs.data[i][j].$op(&rhs.data[i][j]).unwrap()));
              },
              _ => (),
            }
          }
        }
      // Operate with scalar on the left
      } else if lhs_is_scalar {
        out.grow_to_fit(rhs_height,rhs_width);
        for i in 0..rhs_width as usize {
          for j in 0..rhs_height as usize {
            match (&*lhs.data[0][0], &*rhs.data[i][j]) {
              (Value::Number(x), Value::Number(y)) => {
                match x.$op(*y) {
                  Ok(op_result) => {
                    let value = Value::from_quantity(op_result);
                    let value_rc = Rc::new(value);
                    out.data[i][j] = value_rc;
                  },
                  //Err(error) => errors.push(error), // TODO Throw an error here
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
      // Operate with scalar on the right
      } else if rhs_is_scalar {
        out.grow_to_fit(lhs_height,lhs_width);
        for i in 0..lhs_width as usize {
          for j in 0..lhs_height as usize {
            match (&*lhs.data[i][j], &*rhs.data[0][0]) {
              (Value::Number(x), Value::Number(y)) => {
                match x.$op(*y) {
                  Ok(op_result) => {
                    let value = Value::from_quantity(op_result);
                    let value_rc = Rc::new(value);
                    out.data[i][j] = value_rc;
                  },
                  //Err(error) => errors.push(error), // TODO Throw an error here
                  _ => (),
                }
              },
              _ => (),
            }
          }
        }
      } else {
        // TODO: Throw an error here
      }
    }
  )
}

binary_infix!{math_add, add}
binary_infix!{math_subtract, sub}
binary_infix!{math_multiply, multiply}
binary_infix!{math_divide, divide}
// FIXME this isn't actually right at all. ^ is not power in Rust
//binary_math!{math_power, add}
/*
binary_infix!{compare_not_equal, not_equal}
binary_infix!{compare_equal, equal}
binary_infix!{compare_less_than_equal, less_than_equal}
binary_infix!{compare_greater_than_equal, greater_than_equal}
binary_infix!{compare_greater_than, greater_than}
binary_infix!{compare_less_than, less_than}

#[no_mangle]
pub extern "C" fn stats_sum(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
  let (argument, table_ref_rc) = &input[0];
  let table_ref = table_ref_rc.borrow();

  let mut out = output.borrow_mut();

  if argument == "row" {
    out.grow_to_fit(table_ref.rows, 1);
    for i in 0..table_ref.rows as usize {
      let mut total = 0.to_quantity();
      for j in 0..table_ref.columns as usize {
        match table_ref.data[j][i] {
          Value::Number(x) => {
            total = total.add(x).unwrap();
          }
          _ => (),
        }
      }
      out.data[0][i] = Value::Number(total);
    }
  } else if argument == "column" {
    out.grow_to_fit(1, table_ref.columns);
    for i in 0..table_ref.columns as usize {
      let mut total = 0.to_quantity();
      for j in 0..table_ref.rows as usize {
        match table_ref.data[i][j] {
          Value::Number(x) => {
            total = total.add(x).unwrap();
          }
          _ => (),
        }
      }
      out.data[i][0] = Value::Number(total);
    }
  } else if argument == "table" {
    out.grow_to_fit(1,1);
    let mut total = 0.to_quantity();
    for i in 0..table_ref.columns as usize {
      for j in 0..table_ref.rows as usize {
        match table_ref.data[i][j] {
          Value::Number(x) => {
            total = total.add(x).unwrap();
          }
          _ => (),
        }
      }
      out.data[0][0] = Value::Number(total);
    }
  } else {
    // TODO Throw error
  }
}
*/
pub extern "C" fn table_range(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
  let (_, lhs_rc) = &input[0];
  let (_, rhs_rc) = &input[1];
  let lhs = lhs_rc.borrow();
  let rhs = rhs_rc.borrow();

  let mut out = output.borrow_mut();

  let start = lhs.data[0][0].as_i64().unwrap();
  let end = rhs.data[0][0].as_i64().unwrap();
  let steps = (end - start) as usize + 1;
  out.grow_to_fit(steps as u64, 1);
  for i in 0..steps {
    out.data[0][i] = Rc::new(Value::Number((start + i as i64).to_quantity()))
  }
}
/*
pub extern "C" fn set_any(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
  
  let (field, table_ref_rc) = &input[0];
  let table_ref = table_ref_rc.borrow();

  let mut out = output.borrow_mut();
  out.grow_to_fit(1,1);

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
  } else {
    // TODO Throw error
  }
}
*/
pub extern "C" fn table_horizontal_concatenate(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
  
  let mut cat_table = output.borrow_mut();

  for (_, scanned_rc) in input {
    let scanned = scanned_rc.borrow();

    // Do all the work here:
    if cat_table.rows == 0 {
      cat_table.grow_to_fit(scanned.rows,scanned.columns);
      cat_table.data = scanned.data.clone();
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
      let rows = cat_table.rows;
      let columns = cat_table.columns + scanned.columns;
      cat_table.grow_to_fit(rows, columns);
      for i in 0..scanned.columns as usize {
        for j in 0..cat_table.rows as usize {
          cat_table.data[cols+i][j] = scanned.data[i][j].clone();
        }
      }
    } else {
      // TODO Throw size error
    }
  }
}

pub extern "C" fn table_vertical_concatenate(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
  let mut cat_table = output.borrow_mut();

  for (_, scanned_rc) in input {
    let scanned = scanned_rc.borrow();

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
      let rows = cat_table.rows + scanned.rows;
      let columns = cat_table.columns;
      cat_table.grow_to_fit(rows, columns);
    } else {
      // TODO Throw size error
    }
  }
}

// ## Logic
/*
#[macro_export]
macro_rules! logic {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(input: Vec<(String, Rc<RefCell<Table>>)>, output: Rc<RefCell<Table>>) {
      // TODO Test for the right amount of inputs
      let (_, lhs_rc) = &input[0];
      let (_, rhs_rc) = &input[1];
      let lhs = lhs_rc.borrow();
      let rhs = rhs_rc.borrow();

      // Get the math dimensions
      let lhs_width  = lhs.columns;
      let rhs_width  = rhs.columns;
      let lhs_height = lhs.rows;
      let rhs_height = rhs.rows;

      let lhs_is_scalar = lhs_width == 1 && lhs_height == 1;
      let rhs_is_scalar = rhs_width == 1 && rhs_height == 1;

      let mut out = output.borrow_mut();

      // The tables are the same size
      if lhs_width == rhs_width && lhs_height == rhs_height {
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
      } else {
        // TODO Throw error
      };
    }
  )
}

logic!{logic_and, &&}
logic!{logic_or, ||}*/
*/