// # Operations

// ## Prelude

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use table::{Table, Value, ValueMethods, TableId, Index};
use runtime::Runtime;
use block::{Block, IndexIterator, TableIterator, AliasIterator, ValueIterator, IndexRepeater};
use database::Database;
use errors::ErrorType;
use std::rc::Rc;
use std::cell::RefCell;
use hashbrown::HashMap;


const ROW: u64 = 0x6a1e3f1182ea4d9d;
const COLUMN: u64 = 0x3b71b9e91df03940;
const TABLE: u64 = 0x7764ae06e4bbf825;

pub fn resolve_subscript(
  table_id: TableId, 
  row_index: Index, 
  column_index: Index,
  block_tables: &mut HashMap<u64, Table>, 
  database: &Rc<RefCell<Database>>) -> ValueIterator {

  let mut db = database.borrow_mut();

  let mut table = match table_id {
    TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
  };

  let row_iter = unsafe { match row_index {
    Index::Index(ix) => IndexIterator::Constant(Index::Index(ix)),
    Index::All => IndexIterator::Range(1..=(*table).rows),
    Index::Table(table_id) => {
      let mut row_table = match table_id {
        TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
        TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
      };
      IndexIterator::Table(TableIterator::new(row_table))
    }
    Index::Alias(alias) => IndexIterator::Alias(AliasIterator::new(alias, table_id, db.store.clone())),
    _ => IndexIterator::Range(1..=(*table).rows),
  }};

  let column_iter = unsafe { match column_index {
    Index::Index(ix) => IndexIterator::Constant(Index::Index(ix)),
    Index::All => IndexIterator::Range(1..=(*table).columns),
    Index::Table(table_id) => {
      let mut col_table = match table_id {
        TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
        TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
      };
      IndexIterator::Table(TableIterator::new(col_table))
    }
    Index::Alias(alias) => IndexIterator::Alias(AliasIterator::new(alias, table_id, db.store.clone())),
    _ => IndexIterator::Range(1..=(*table).columns),
  }};
  
  ValueIterator{
    table,
    row_index,
    column_index,
    row_iter,
    column_iter,
  }

}


pub type MechFunction = extern "C" fn(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator);


pub extern "C" fn set_any(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator) {                                        

  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let mut rows = vi.rows();
  let mut cols = match vi.column_iter {
    IndexIterator::Constant{..} => 1,
    _ => vi.columns(),
  };

  match in_arg_name {
    &ROW => {
      unsafe {
        (*out.table).rows = vi.rows();
        (*out.table).columns = 1;
        (*out.table).data.resize(vi.rows(), 0);
      }
      for i in 1..=rows {
        let mut flag: bool = false;
        for j in 1..=cols {
          let value = unsafe{(*vi.table).get(&Index::Index(i),&Index::Index(j)).unwrap()};
          match value.as_bool() {
            Some(true) => flag = true,
            _ => (), // TODO Alert user that there was an error
          }
        }
        unsafe {
          (*out.table).set_unchecked(i, 1, Value::from_bool(flag));
        }
      }
    }
    &COLUMN => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = cols;
        (*out.table).data.resize(cols, 0);
      }
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        let mut flag: bool = false;
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
          match value.as_bool() {
            Some(true) => flag = true,
            _ => (), // TODO Alert user that there was an error
          }
        }
        unsafe {
          (*out.table).set_unchecked(1, i, Value::from_bool(flag));
        }
      }      
    }
    &TABLE => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = 1;
        (*out.table).data.resize(1, 0);
      }
      let mut flag: bool = false;
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
          match value.as_bool() {
            Some(true) => flag = true,
            _ => (), // TODO Alert user that there was an error
          }
        }
      }  
      unsafe {
        (*out.table).set_unchecked(1, 1, Value::from_bool(flag));
      }    
    }
    _ => (), // TODO alert user that argument is unknown
  }
}

pub extern "C" fn stats_sum(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator) {                                        

  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let mut rows = vi.rows();
  let mut cols = match vi.column_iter {
    IndexIterator::Constant{..} => 1,
    _ => vi.columns(),
  };

  match in_arg_name {
    &ROW => {
      unsafe {
        (*out.table).rows = vi.rows();
        (*out.table).columns = 1;
        (*out.table).data.resize(vi.rows(), 0);
      }
      for i in 1..=rows {
        let mut sum: Value = Value::from_u64(0);
        for j in 1..=cols {
          let value = unsafe{(*vi.table).get(&Index::Index(i),&Index::Index(j)).unwrap()};
          match sum.add(value) {
            Ok(result) => sum = result,
            _ => (), // TODO Alert user that there was an error
          }
        }
        unsafe {
          (*out.table).set_unchecked(i, 1, sum);
        }
      }
    }
    &COLUMN => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = cols;
        (*out.table).data.resize(cols, 0);
      }
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        let mut sum: Value = Value::from_u64(0);
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
          match sum.add(value) {
            Ok(result) => sum = result,
            _ => (), // TODO Alert user that there was an error
          }
        }
        unsafe {
          (*out.table).set_unchecked(1, i, sum);
        }
      }      
    }
    &TABLE => {
      unsafe {
        (*out.table).rows = 1;
        (*out.table).columns = 1;
        (*out.table).data.resize(1, 0);
      }
      let mut sum: Value = Value::from_u64(0);
      for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
        for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
          let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
          match sum.add(value) {
            Ok(result) => sum = result,
            _ => (), // TODO Alert user that there was an error
          }
        }
      }  
      unsafe {
        (*out.table).set_unchecked(1, 1, sum);
      }    
    }
    _ => (), // TODO alert user that argument is unknown
  }
}
        

pub extern "C" fn table_horizontal_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator) {

  let mut row = 0;
  let mut column = 0;
  let mut out_rows = 0;
  let mut out_columns = 0;

  // Get the size of the output table
  for (_, vi) in arguments {
    let vi_rows = match &vi.row_iter {
      IndexIterator::Range(_) => vi.rows(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_rows = if out_rows == 0 {
      vi_rows
    } else if vi_rows > out_rows && vi_rows == 1 {
      vi_rows
    } else if vi_rows == out_rows {
      vi_rows
    } else {
      // TODO Throw a size error here
      0
    };

    let vi_columns = match &vi.column_iter {
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_columns += vi_columns;

  }

  unsafe {
    (*out.table).rows = out_rows;
    (*out.table).columns = out_columns;
    (*out.table).data.resize(out_rows * out_columns, 0);
  }

  for (_, vi) in arguments {
    let width = match &vi.column_iter {
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    for (c,j) in (1..=width).zip(vi.column_iter.clone()) {
      for (k,i) in (1..=out_rows).zip(vi.row_iter.clone()) {
        let value = unsafe{(*vi.table).get(&i,&j).unwrap()};
        unsafe {
          (*out.table).set(&Index::Index(k), &Index::Index(column+c), value);
        }
      }
    }
    column += width;
    
  }
}

pub extern "C" fn table_vertical_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator) {

  let mut row = 0;
  let mut out_columns = 0;
  let mut out_rows = 0;
  // First pass, make sure the dimensions work out
  for (_, vi) in arguments {
    if out_columns == 0 {
      out_columns = vi.columns();
    } else if vi.columns() != 1 && out_columns != vi.columns() {
      // TODO Throw an error here
    } else if vi.columns() > out_columns && out_columns == 1 {
      out_columns = vi.columns()
    }
    out_rows += vi.rows();
  }

  unsafe {
    (*out.table).rows = out_rows;
    (*out.table).columns = out_columns;
    (*out.table).data.resize(out_rows * out_columns, 0);
  }
  for (_, vi) in arguments {
    for (i,k) in (1..=out_columns).zip(vi.column_iter.clone()) {
      for j in 1..=vi.rows() {
        let value = unsafe{(*vi.table).get(&Index::Index(j),&k).unwrap()};
        unsafe {
          (*out.table).set(&Index::Index(row + j), &Index::Index(i), value);
        }
      }
    }
    row += 1;
  }
}

pub extern "C" fn table_range(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator) {
  // TODO test argument count is 2 or 3
  // 2 -> start, end
  // 3 -> start, increment, end
  let (_, start_vi) = &arguments[0];
  let (_, end_vi) = &arguments[1];

  let start_value = unsafe{(*start_vi.table).get(&Index::Index(1),&Index::Index(1)).unwrap()};
  let end_value = unsafe{(*end_vi.table).get(&Index::Index(1),&Index::Index(1)).unwrap()};
  let start = start_value.as_u64().unwrap() as usize;
  let end = end_value.as_u64().unwrap() as usize;
  let range = end - start;
  
  unsafe{
    (*out.table).rows = range+1;
    (*out.table).columns = 1;
    (*out.table).data.resize(range+1, 0);
    let mut j = 1;
    for i in start..=end {
      (*out.table).set(&Index::Index(j), &Index::Index(1), Value::from_u64(i as u64));
      j += 1;
    }
  }
  
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(arguments: &Vec<(u64, ValueIterator)>, out: &ValueIterator) {
      // TODO test argument count is 2
      let (_, lhs_vi) = &arguments[0];
      let (_, rhs_vi) = &arguments[1];

      // Figure out dimensions
      let lhs_rows_count = match lhs_vi.row_index {
        Index::All => lhs_vi.rows(),
        _ => 1,
      };
      let lhs_columns_count = match lhs_vi.column_index {
        Index::All => lhs_vi.columns(),
        _ => 1,
      };
      let rhs_rows_count = match rhs_vi.row_index {
        Index::All => rhs_vi.rows(),
        _ => 1,
      };
      let rhs_columns_count = match rhs_vi.column_index {
        Index::All => rhs_vi.columns(),
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
          IndexRepeater::new(lhs_vi.row_iter.clone(),1),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),1),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1),
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
          IndexRepeater::new(IndexIterator::Constant(Index::Index(1)),1),
        )
      } else if equal_dimensions {
        unsafe {
          (*out.table).rows = lhs_rows_count;
          (*out.table).columns = lhs_columns_count;
          (*out.table).data.resize(lhs_rows_count * lhs_columns_count, 0);
        }
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),lhs_vi.columns()),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),rhs_vi.columns()),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
        )
      } else if rhs_scalar {
        unsafe {
          (*out.table).rows = lhs_rows_count;
          (*out.table).columns = lhs_columns_count;
          (*out.table).data.resize(lhs_rows_count * lhs_columns_count, 0);
        }
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),lhs_columns_count),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),1),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          match out.column_index {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
            _ => IndexRepeater::new(IndexIterator::Constant(out.column_index),1),
          },
        )
      } else {
        unsafe {
          (*out.table).rows = rhs_rows_count;
          (*out.table).columns = rhs_columns_count;
          (*out.table).data.resize(rhs_rows_count * rhs_columns_count, 0);
        }
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),1),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),rhs_columns_count),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          match out.column_index {
            Index::All => IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
            _ => IndexRepeater::new(IndexIterator::Constant(out.column_index),1),
          },
        )
      };

      let mut i = 1;
      
      let out_elements = out.rows() * out.columns();
      unsafe{
        loop {
          let l1 = lrix.next().unwrap().unwrap();
          let l2 = lcix.next().unwrap().unwrap();
          let r1 = rrix.next().unwrap().unwrap();
          let r2 = rcix.next().unwrap().unwrap();
          let o1 = out_rix.next().unwrap().unwrap();
          let o2 = out_cix.next().unwrap().unwrap();
          match ((*lhs_vi.table).get_unchecked(l1,l2), 
                (*rhs_vi.table).get_unchecked(r1,r2))
          {
            (lhs_value, rhs_value) => {
              match lhs_value.$op(rhs_value) {
                Ok(result) => {
                  (*out.table).set_unchecked(o1, o2, result);
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
    }
  )
}

binary_infix!{math_add, add}
binary_infix!{math_subtract, sub}
binary_infix!{math_multiply, multiply}
binary_infix!{math_divide, divide}
binary_infix!{compare_greater_than_equal, greater_than_equal}
binary_infix!{compare_greater_than, greater_than}
binary_infix!{compare_less_than_equal, less_than_equal}
binary_infix!{compare_less_than, less_than}
binary_infix!{compare_equal, equal}
binary_infix!{compare_not_equal, not_equal}
binary_infix!{logic_and, and}
binary_infix!{logic_or, or}






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