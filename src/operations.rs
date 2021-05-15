// # Operations

// ## Prelude

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
use table::{Table, TableId, TableIndex};
use value::{Value, ValueMethods};
use index::{IndexIterator, TableIterator, AliasIterator, ValueIterator, IndexRepeater, CycleIterator, ConstantIterator};
use database::Database;
//use errors::ErrorType;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::Cell;
use hashbrown::{HashMap, HashSet};
use ::{hash_string};


lazy_static! {
  static ref ROW: u64 = hash_string("row");
  static ref COLUMN: u64 = hash_string("column");
  static ref TABLE: u64 = hash_string("table");
}

pub type MechFunction = extern "C" fn(arguments: &mut Vec<Rc<RefCell<Argument>>>);

pub struct Argument {
  pub name: u64,
  pub iterator: ValueIterator,
}

/*
#[no_mangle]
pub extern "C" fn set_all(arguments: &mut Vec<(u64, ValueIterator)>) {
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];
  let (_, mut out) = arguments[1].clone();

  let rows = vi.rows();
  let cols = vi.columns();

  if *in_arg_name == *ROW {
    out.resize(vi.rows(), 1);
    for i in 1..=rows {
      let mut flag: bool = true;
      for j in 1..=cols {
        let (value,_) = vi.get(&TableIndex::Index(i),&TableIndex::Index(j)).unwrap();
        match value.as_bool() {
          Some(false) => flag = false,
          _ => (), // TODO Alert user that there was an error
        }
      }
      out.set_unchecked(i, 1, Value::from_bool(flag));
    }
  } else if *in_arg_name == *COLUMN {
    out.resize(1, cols);
    for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      let mut flag: bool = true;
      for (_j,k) in (1..=rows).zip(vi.row_iter.clone()) {
        let (value,_) = vi.get(&k,&m).unwrap();
        match value.as_bool() {
          Some(false) => flag = false,
          _ => (), // TODO Alert user that there was an error
        }
      }
      out.set_unchecked(1, i, Value::from_bool(flag));
    }
  } else if *in_arg_name == *TABLE {
    out.resize(1, 1);
    let mut flag: bool = true;
    for (value, _) in vi.clone() {
      match value.as_bool() {
        Some(false) => flag = false,
        _ => (), // TODO Alert user that there was an error
      }
    }
    out.set_unchecked(1, 1, Value::from_bool(flag));
  } else {
    () // TODO alert user that argument is unknown
  };
}

#[no_mangle]
pub extern "C" fn set_any(arguments:  &mut Vec<(u64, ValueIterator)>) {
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];
  let (_, mut out) = arguments[1].clone();

  let rows = vi.rows();
  let cols = vi.columns();

  if *in_arg_name == *ROW {
    out.resize(vi.rows(), 1);
    for i in 1..=rows {
      let mut flag: bool = false;
      for j in 1..=cols {
        let (value,_) = vi.get(&TableIndex::Index(i),&TableIndex::Index(j)).unwrap();
        match value.as_bool() {
          Some(true) => flag = true,
          _ => (), // TODO Alert user that there was an error
        }
      }
      out.set_unchecked(i, 1, Value::from_bool(flag));
    }
  } else if *in_arg_name == *COLUMN {
    out.resize(1, cols);
    for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      let mut flag: bool = false;
      for (_j,k) in (1..=rows).zip(vi.row_iter.clone()) {
        let (value,_) = vi.get(&k,&m).unwrap();
        match value.as_bool() {
          Some(true) => flag = true,
          _ => (), // TODO Alert user that there was an error
        }
      }
      out.set_unchecked(1, i, Value::from_bool(flag));
    }
  } else if *in_arg_name == *TABLE {
    out.resize(1, 1);
    let mut flag: bool = false;
    for (value, _) in vi.clone() {
      match value.as_bool() {
        Some(true) => flag = true,
        _ => (), // TODO Alert user that there was an error
      }
    }
    out.set_unchecked(1, 1, Value::from_bool(flag));
  } else {
    () // TODO alert user that argument is unknown
  };
}

#[no_mangle]
pub extern "C" fn stats_sum(arguments: &mut Vec<(u64, ValueIterator)>) {
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];
  let (_, mut out) = arguments[1].clone();

  let rows = vi.rows();
  let cols = vi.columns();

  if *in_arg_name == *ROW {
    out.resize(vi.rows(), 1);
    for i in 1..=rows {
      let mut sum: Value = Value::from_u64(0);
      for j in 1..=cols {
        match vi.get(&TableIndex::Index(i),&TableIndex::Index(j)) {
          Some((value,_)) => {
            match sum.add(value) {
              Ok(result) => sum = result,
              _ => (), // TODO Alert user that there was an error
            }
          }
          _ => ()
        }
      }
      out.set_unchecked(i, 1, sum);
    }
  } else if *in_arg_name == *COLUMN {
    out.resize(1, cols);
    for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      let mut sum: Value = Value::from_u64(0);
      for (_j,k) in (1..=rows).zip(vi.row_iter.clone()) {
        match vi.get(&k,&m) {
          Some((value,_)) => {
            match sum.add(value) {
              Ok(result) => sum = result,
              _ => (), // TODO Alert user that there was an error
            }
          }
          _ => ()
        }
      }
      out.set_unchecked(1, i, sum);
    }
  } else if *in_arg_name == *TABLE {
    out.resize(1, 1);
    let mut sum: Value = Value::from_u64(0);
    for (value, _) in vi.clone() {
      match sum.add(value) {
        Ok(result) => sum = result,
        _ => (), // TODO Alert user that there was an error
      }
    }
    out.set_unchecked(1, 1, sum);
  } else {
    () // TODO alert user that argument is unknown
  }
}

#[no_mangle]
pub extern "C" fn table_append__row(arguments: &mut Vec<(u64, ValueIterator)>) {
  //TODO There needs to be some checking of dimensions here
  let (_, mut vi) = arguments[0].clone();
  let (_, mut out) = arguments[1].clone();
  let out_rows = out.rows();
  let out_columns = if out.columns() == 0 {vi.columns()} else {out.columns()};
  let in_rows = vi.rows();

  if in_rows == 0 {
    return;
  }

  if vi.column_index != TableIndex::None {
    out.resize(out_rows + in_rows, out_columns);
    for (row_index, column_index) in vi.index_iterator() {
      let (value,_) = vi.get(&row_index,&column_index).unwrap();
      // If the column has an alias, let's use it instead
      let out_column = match vi.get_column_alias(column_index.unwrap()) {
        Some(alias) => alias,
        None => column_index,
      };
      out.set(&TableIndex::Index(out_rows + row_index.unwrap()), &out_column, value);
    }
  } else {
    out.resize(out_rows + 1, out_columns);
    for index in vi.linear_index_iterator() {
      let (value,_)= vi.get_unchecked_linear(index);
      out.set(&TableIndex::Index(out_rows + 1), &TableIndex::Index(index), value);
    }    
  }
}*/

#[no_mangle]
pub extern "C" fn table_set(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let mut input = &mut arguments[0].borrow_mut();
  let mut out = &mut arguments[1].borrow_mut();

  if out.iterator.table_rows() == 0 || out.iterator.table_columns() == 0 {
    out.iterator.resize(input.iterator.rows(), input.iterator.columns());
  }

  if input.iterator.is_scalar() {
    input.iterator.inf_cycle();
  } else {
    // If we're indexing on a table, then match the input iter with the output iter
    match (out.iterator.row_index, input.iterator.row_index) {
      (TableIndex::Table(_), TableIndex::All) => {
        input.iterator.row_index = out.iterator.row_index.clone();
        input.iterator.raw_row_iter = out.iterator.raw_row_iter.clone();
        input.iterator.row_iter = out.iterator.row_iter.clone();
      }
      _ => (),
    }
    match (out.iterator.column_index, input.iterator.column_index) {
      (TableIndex::Table(_), TableIndex::All) => {
        input.iterator.column_index = out.iterator.column_index.clone();
        input.iterator.raw_column_iter = out.iterator.raw_column_iter.clone();
        input.iterator.column_iter = out.iterator.column_iter.clone();
      }
      _ => (),
    }
  }

  let mut out_iterator = out.iterator.linear_index_iterator();
  loop {
    let in_value = input.iterator.next();
    let out_ix = out_iterator.next();
    match (in_value, out_ix) {
      (Some((value,_)),Some(out_ix)) => out.iterator.set_unchecked_linear(out_ix, value),
      _ => {
        input.iterator.reset();
        break
      },
    }
  }
}

/*#[no_mangle]
pub extern "C" fn table_copy(arguments:  &mut Vec<(u64, ValueIterator)>) {
  let (_, vi) = &arguments[0];
  let (_, mut out) = arguments[1].clone();

  out.resize(vi.rows(), vi.columns());
  for j in 1..=vi.columns() {
    for i in 1..=vi.rows() {
      let (value, _) = vi.get_unchecked(i,j);
      out.set_unchecked(i, j, value);
    }
  }
}*/

#[no_mangle]
pub extern "C" fn table_horizontal__concatenate(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let mut out = &mut arguments.last().unwrap().borrow_mut();

  // Get the size of the output table we will create, and resize the out table
  let out_rows: usize = arguments.iter().take(arguments.len()-1).map(|vi| vi.borrow().iterator.rows()).max().unwrap();
  let out_columns: usize = arguments.iter().take(arguments.len()-1).map(|vi| vi.borrow().iterator.columns()).sum();
  out.iterator.resize(out_rows, out_columns);

  // Iterate through the input table and insert values into output table
  let mut column = 0;
  for vi_rc in arguments.iter().take(arguments.len()-1) {
    let vi = vi_rc.borrow().iterator.clone();
    let width = vi.columns();
    let mut out_row_iter = IndexRepeater::new(IndexIterator::Range(1..=out_rows),width,1);
    let mut out_column_iter = IndexRepeater::new(IndexIterator::Range(column+1..=column+width),1,out_rows as u64);
    for (((value, _), out_row_ix), out_column_ix) in vi.cycle().zip(out_row_iter).zip(out_column_iter) {
      out.iterator.set_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap(), value);
    }
    column += width;
  }
}

#[no_mangle]
pub extern "C" fn table_vertical__concatenate(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let mut out = &mut arguments.last().unwrap().borrow_mut();

  // Do all of the arguments have a compatible width?
  if arguments.iter().take(arguments.len()-1).map(|vi| vi.borrow().iterator.columns()).collect::<HashSet<usize>>().len() != 1 {
    // TODO Warn that one or more arguments is the wrong height
    return;
  }
  
  // Get the size of the output table we will create, and resize the out table
  let out_columns: usize = arguments.iter().take(arguments.len()-1).map(|vi| vi.borrow().iterator.columns()).max().unwrap();
  let out_rows: usize = arguments.iter().take(arguments.len()-1).map(|vi| vi.borrow().iterator.rows()).sum();
  out.iterator.resize(out_rows, out_columns);

  // Iterate through the input table and insert values into output table
  let mut row = 0;
  for vi_rc in arguments.iter().take(arguments.len()-1) {
    let vi = vi_rc.borrow().iterator.clone();
    let height = vi.rows();
    if height != 0 {
      let mut out_row_iter = IndexRepeater::new(IndexIterator::Range(row+1..=row+height),out_columns,1);
      let mut out_column_iter= IndexRepeater::new(IndexIterator::Range(1..=out_columns),1, height as u64);
      for (((value, _), out_row_ix), out_column_ix) in vi.zip(out_row_iter).zip(out_column_iter) {
        out.iterator.set_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap(), value);
      }
      row += height;
    }
  }
}

#[no_mangle]
pub extern "C" fn table_range(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  // TODO test argument count is 2 or 3
  // 2 -> start, end
  // 3 -> start, increment, end
  let start_vi = &arguments[0].borrow();
  let end_vi = &arguments[1].borrow();
  let mut out = arguments[arguments.len()-1].borrow_mut();
  // TODO add increment argument if there are three

  // TODO We have to test to see if all of these things are valid
  let (start_value,_) = start_vi.iterator.get(&TableIndex::Index(1),&TableIndex::Index(1)).unwrap();
  let (end_value,_) = end_vi.iterator.get(&TableIndex::Index(1),&TableIndex::Index(1)).unwrap();
  let start = start_value.as_u64().unwrap() as usize;
  let end = end_value.as_u64().unwrap() as usize;
  let range = end - start;
  
  out.iterator.resize(range+1, 1);
  let mut j = 1;
  for i in start..=end {
    out.iterator.set(&TableIndex::Index(j), &TableIndex::Index(1), Value::from_u64(i as u64));
    j += 1;
  }
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    #[no_mangle]
    pub extern "C" fn $func_name(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
      // TODO test argument count is 3
      let mut lhs = &mut arguments[0].borrow_mut();
      let mut rhs = &mut arguments[1].borrow_mut();
      let mut out = &mut arguments[2].borrow_mut();

      let (mut out_rows, mut out_columns) = 
      // Equal dimensions
      if lhs.iterator.rows() == rhs.iterator.rows() && lhs.iterator.columns() == rhs.iterator.columns() {
        (lhs.iterator.rows(), lhs.iterator.columns())
      // LHS scalar
      } else if lhs.iterator.is_scalar() {
        lhs.iterator.inf_cycle();
        (rhs.iterator.rows(), rhs.iterator.columns())
      // RHS scalar
      } else if rhs.iterator.is_scalar() {
        rhs.iterator.inf_cycle();
        (lhs.iterator.rows(), lhs.iterator.columns())      
      } else {
        // TODO Warn of mismatch of dimensions
        return;
      };

      out.iterator.resize(out_rows, out_columns);

      let mut out_row_iter = IndexRepeater::new(IndexIterator::Range(1..=out_rows), out_columns, 1);
      let mut out_column_iter= IndexRepeater::new(IndexIterator::Range(1..=out_columns), 1, out_rows as u64);

      loop {
        let lhs_value = lhs.iterator.next();
        let rhs_value = rhs.iterator.next();
        let out_row_ix = out_row_iter.next();
        let out_column_ix = out_column_iter.next();
        match (lhs_value, rhs_value) {
          (Some((lhs_value,_)), Some((rhs_value,_))) => {
            match lhs_value.$op(rhs_value) {
              Ok(result) => {
                out.iterator.set_unchecked(out_row_ix.unwrap().unwrap(), out_column_ix.unwrap().unwrap(), result);
              }
              Err(_) => (), // TODO Handle error here
            }
          }
          _ => {
            lhs.iterator.reset();
            rhs.iterator.reset();
            break
          },
        }        
      }
    }
  )
}

binary_infix!{math_add, add}
/*binary_infix!{math_subtract, sub}
binary_infix!{math_multiply, multiply}
binary_infix!{math_divide, divide}
binary_infix!{math_exponent, power}
binary_infix!{compare_greater__than__equal, greater_than_equal}
binary_infix!{compare_greater__than, greater_than}
binary_infix!{compare_less__than__equal, less_than_equal}
binary_infix!{compare_less__than, less_than}
binary_infix!{compare_equal, equal}
binary_infix!{compare_not__equal, not_equal}
binary_infix!{logic_and, and}
binary_infix!{logic_or, or}
binary_infix!{logic_xor, xor}*/