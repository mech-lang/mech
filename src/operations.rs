// # Operations

// ## Prelude

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use table::{Table, Value, ValueMethods, TableId, Index};
use runtime::Runtime;
use block::{Block, IndexIterator, TableIterator, IndexRepeater};
use database::Database;
use errors::ErrorType;
use quantities::{Quantity, QuantityMath, ToQuantity};
use std::rc::Rc;
use std::cell::RefCell;
use hashbrown::HashMap;


pub type MechFunction = extern "C" fn(arguments: &Vec<(u64, TableId, Index, Index)>, 
                                      out: &(TableId, Index, Index), 
                                      block_tables: &mut HashMap<u64, Table>, 
                                      database: &Rc<RefCell<Database>>);


pub extern "C" fn stats_sum(arguments: &Vec<(u64, TableId, Index, Index)>, 
                              out: &(TableId, Index, Index), 
                              block_tables: &mut HashMap<u64, Table>, 
                              database: &Rc<RefCell<Database>>) {                               
  // TODO test argument count is 1
  let (in_arg_name, in_table_id, in_rows, in_columns) = &arguments[0];
  let (out_table_id, out_rows, out_columns) = out;
  let mut db = database.borrow_mut();
  
  let mut out_table = match out_table_id {
    TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
  };
  
  let in_table = match in_table_id {
    TableId::Global(id) => db.tables.get(id).unwrap(),
    TableId::Local(id) => block_tables.get(id).unwrap(),
  };

  let rows = in_table.rows;
  let cols = in_table.columns;

  match in_arg_name {
    // rows
    0x6a1e3f1182ea4d9d => {
      unsafe {
        (*out_table).rows = in_table.rows;
        (*out_table).columns = 1;
        (*out_table).data.resize(in_table.rows, 0);
      }
      for i in 1..=rows {
        let mut sum: Value = Value::from_u64(0);
        for j in 1..=cols {
          let value = in_table.get(&Index::Index(i),&Index::Index(j)).unwrap();
          match sum.add(value) {
            Ok(result) => sum = result,
            _ => (), // TODO Alert user that there was an error
          }
        }
        unsafe {
          (*out_table).set_unchecked(i, 1, sum);
        }
      }
    }
    // columns
    0x3b71b9e91df03940 => {
      unsafe {
        (*out_table).rows = 1;
        (*out_table).columns = in_table.columns;
        (*out_table).data.resize(in_table.columns, 0);
      }
      for i in 1..=cols {
        let mut sum: Value = Value::from_u64(0);
        for j in 1..=rows {
          let value = in_table.get(&Index::Index(i),&Index::Index(j)).unwrap();
          match sum.add(value) {
            Ok(result) => sum = result,
            _ => (), // TODO Alert user that there was an error
          }
        }
        unsafe {
          (*out_table).set_unchecked(i, 1, sum);
        }
      }      
    }
    _ => (), // TODO alert user that argument is unknown
  }
}
        

pub extern "C" fn table_horizontal_concatenate(arguments: &Vec<(u64, TableId, Index, Index)>, 
                                               out: &(TableId, Index, Index), 
                                               block_tables: &mut HashMap<u64, Table>, 
                                               database: &Rc<RefCell<Database>>) {
  let (out_table_id, out_rows, out_columns) = out;
  let mut db = database.borrow_mut();
  let mut column = 0;
  let mut out_rows = 0;
  let mut out_columns = 0;
  // First pass, make sure the dimensions work out
  for (_, table_id, rows, columns) in arguments {
    let table = match table_id {
      TableId::Global(id) => db.tables.get(id).unwrap(),
      TableId::Local(id) => block_tables.get(id).unwrap(),
    };
    if out_rows == 0 {
      match rows {
        Index::Index(ix) => out_rows = 1,
        Index::Table(table_id) => {
          let row_table = match table_id {
            TableId::Global(id) => db.tables.get(id).unwrap(),
            TableId::Local(id) => block_tables.get(id).unwrap(),
          };
          out_rows = row_table.rows;
        },
        _ => out_rows = table.rows,
      }
    } else if table.rows != 1 && out_rows != table.rows {
      // TODO Throw an error here
    } else if table.rows > out_rows && out_rows == 1 {
      match rows {
        Index::Index(ix) => out_rows = 1,
        _ => out_rows = table.rows,
      }
    }
    out_columns += match columns {
      Index::All => table.columns,
      _ => 1,
    };
  }
  let mut out_table = match out_table_id {
    TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
  };
  unsafe {
    (*out_table).rows = out_rows;
    (*out_table).columns = out_columns;
    (*out_table).data.resize(out_rows * out_columns, 0);
  }
  for (_, table_id, rows, columns) in arguments {
    let mut table = match table_id {
      TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
      TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
    };
    let rows_iter = match rows {
      Index::Index(ix) => IndexIterator::Constant(Index::Index(*ix)),
      Index::Table(table_id) => {
        let mut row_table = match table_id {
          TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
          TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
        };
        IndexIterator::Table(TableIterator::new(row_table))
      }
      _ => IndexIterator::Range(1..=unsafe{(*table).rows}),
    };
    
    for (i,k) in (1..=out_rows).zip(rows_iter) {
      let columns_iter = match columns {
        Index::Index(ix) => IndexIterator::Constant(Index::Index(*ix)),
        _ => IndexIterator::Range(1..=unsafe{(*table).columns}),
      };
      let out_cols = match columns {
        Index::All => unsafe{(*table).columns},
        _ => 1,
      };
      for (m,j) in (1..=out_cols).zip(columns_iter) {
        let value = unsafe{(*table).get(&k,&j).unwrap()};
        unsafe {
          (*out_table).set(&Index::Index(i), &Index::Index(column+m), value);
        }
      }
    }
    column += 1;
  }
}

pub extern "C" fn table_vertical_concatenate(arguments: &Vec<(u64, TableId, Index, Index)>, 
                                             out: &(TableId, Index, Index), 
                                             block_tables: &mut HashMap<u64, Table>, 
                                             database: &Rc<RefCell<Database>>) {
  let (out_table_id, out_rows, out_columns) = out;
  let mut db = database.borrow_mut();
  let mut row = 0;
  let mut out_columns = 0;
  let mut out_rows = 0;
  // First pass, make sure the dimensions work out
  for (_, table_id, rows, columns) in arguments {
    let table = match table_id {
      TableId::Global(id) => db.tables.get(id).unwrap(),
      TableId::Local(id) => block_tables.get(id).unwrap(),
    };
    if out_columns == 0 {
      out_columns = table.columns;
    } else if table.columns != 1 && out_columns != table.columns {
      // TODO Throw an error here
    } else if table.columns > out_columns && out_columns == 1 {
      out_columns = table.columns
    }
    out_rows += table.rows;
  }
  let mut out_table = match out_table_id {
    TableId::Global(id) => db.tables.get_mut(id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(id).unwrap() as *mut Table,
  };
  unsafe {
    (*out_table).rows = out_rows;
    (*out_table).columns = out_columns;
    (*out_table).data.resize(out_rows * out_columns, 0);
  }
  for (_, table_id, rows, columns) in arguments {
    let table = match table_id {
      TableId::Global(id) => db.tables.get(id).unwrap(),
      TableId::Local(id) => block_tables.get(id).unwrap(),
    };
    let columns_iter = if table.columns == 1 {
      IndexIterator::Constant(Index::Index(1))
    } else {
      match columns {
        Index::Index(ix) => IndexIterator::Constant(Index::Index(*ix)),
        _ => IndexIterator::Range(1..=table.columns),
      }      
    };
    for (i,k) in (1..=out_columns).zip(columns_iter) {
      for j in 1..=table.rows {
        let value = table.get(&Index::Index(j),&k).unwrap();
        unsafe {
          (*out_table).set(&Index::Index(row + j), &Index::Index(i), value);
        }
      }
    }
    row += 1;
  }
}

pub extern "C" fn table_range(arguments: &Vec<(u64, TableId, Index, Index)>, 
                              out: &(TableId, Index, Index), 
                              block_tables: &mut HashMap<u64, Table>, 
                              database: &Rc<RefCell<Database>>) {
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
  let start = start_value.as_u64().unwrap() as usize;
  let end = end_value.as_u64().unwrap() as usize;
  let range = end - start;
  match out_table_id {
    TableId::Local(id) => {
      let mut out_table = block_tables.get_mut(id).unwrap();
      out_table.rows = range+1;
      out_table.columns = 1;
      out_table.data.resize(range+1, 0);
      let mut j = 1;
      for i in start..=end {
        out_table.set(&Index::Index(j), &Index::Index(1), Value::from_u64(i as u64));
        j += 1;
      }
    }
    TableId::Global(id) => {

    }
  }
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(arguments: &Vec<(u64, TableId, Index, Index)>, 
                                 out: &(TableId, Index, Index), 
                                 block_tables: &mut HashMap<u64, Table>, 
                                 database: &Rc<RefCell<Database>>) {
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
      let lhs_rows_count = match lhs_rows {
        Index::All => lhs_table.rows,
        _ => 1,
      };
      let lhs_columns_count = match lhs_columns {
        Index::All => lhs_table.columns,
        _ => 1,
      };
      let rhs_rows_count = match rhs_rows {
        Index::All => rhs_table.rows,
        _ => 1,
      };
      let rhs_columns_count = match rhs_columns {
        Index::All => rhs_table.columns,
        _ => 1,
      };


      let equal_dimensions = if lhs_rows_count == rhs_rows_count && lhs_columns_count == rhs_columns_count
      { true } else { false };
      let lhs_scalar = if lhs_rows_count == 1 && lhs_columns_count == 1 
      { true } else { false };
      let rhs_scalar = if rhs_rows_count == 1 && rhs_columns_count == 1
      { true } else { false };

      let out_rows_count = if lhs_rows_count > rhs_rows_count {
        lhs_rows_count
      } else {
        rhs_rows_count
      };
      let out_columns_count = if lhs_columns_count > rhs_columns_count {
        lhs_columns_count
      } else {
        rhs_columns_count
      };

      let (mut lrix, mut lcix, mut rrix, mut rcix, mut out_rix, mut out_cix) = if rhs_scalar && lhs_scalar {
        (
          match lhs_rows {
            Index::All => IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*lhs_rows),1),
          },
          match lhs_columns {
            Index::All => IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*lhs_columns),1),
          },
          match rhs_rows {
            Index::All => IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*rhs_rows),1),
          },
          match rhs_columns {
            Index::All => IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*rhs_columns),1),
          },
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
        )
      } else if equal_dimensions {
        unsafe {
          (*out_table).rows = lhs_rows_count;
          (*out_table).columns = lhs_columns_count;
          (*out_table).data.resize(lhs_rows_count * lhs_columns_count, 0);
        }
        (
          IndexRepeater::new(IndexIterator::Range(1..=lhs_table.rows),lhs_table.columns),
          match lhs_columns {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=lhs_table.columns),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*lhs_columns),1),
          },
          IndexRepeater::new(IndexIterator::Range(1..=rhs_table.rows),rhs_table.columns),
          match rhs_columns {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=rhs_table.columns),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*rhs_columns),1),
          },
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
        )
      } else if rhs_scalar {
        unsafe {
          (*out_table).rows = lhs_rows_count;
          (*out_table).columns = lhs_columns_count;
          (*out_table).data.resize(lhs_rows_count * lhs_columns_count, 0);
        }
        (
          IndexRepeater::new(IndexIterator::Range(1..=lhs_table.rows),lhs_table.columns),
          match lhs_columns {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=lhs_table.columns),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*lhs_columns),1),
          },
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          match out_columns {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*out_columns),1),
          },
        )
      } else {
        unsafe {
          (*out_table).rows = rhs_rows_count;
          (*out_table).columns = rhs_columns_count;
          (*out_table).data.resize(rhs_rows_count * rhs_columns_count, 0);
        }
        (
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
          IndexRepeater::new(IndexIterator::Range(1..=rhs_table.rows),rhs_table.columns),
          match rhs_columns {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=rhs_table.columns),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*rhs_columns),1),
          },
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          match out_columns {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
            _ => IndexRepeater::new(IndexIterator::Constant(*out_columns),1),
          },
        )
      };

      let mut i = 1;
      
      let out_elements = unsafe { (*out_table).rows * (*out_table).columns };

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
            match lhs_value.$op(rhs_value) {
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
        if i >= out_elements {
          break;
        }
        i += 1;
      }
    }
  )
}

binary_infix!{math_add, add}
binary_infix!{math_subtract, sub}
binary_infix!{math_multiply, multiply}
binary_infix!{math_divide, divide}






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