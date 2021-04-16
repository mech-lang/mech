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
use std::cell::RefCell;
use hashbrown::{HashMap, HashSet};
use ::{hash_string};


lazy_static! {
  static ref ROW: u64 = hash_string("row");
  static ref COLUMN: u64 = hash_string("column");
  static ref TABLE: u64 = hash_string("table");
}

pub type MechFunction = extern "C" fn(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator);

pub extern "C" fn set_any(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let rows = vi.rows();
  let cols = vi.columns();

  if *in_arg_name == *ROW {
    out.resize(vi.rows(), 1);
    for i in 1..=rows {
      let mut flag: bool = false;
      for j in 1..=cols {
        let value = vi.get(&TableIndex::Index(i),&TableIndex::Index(j)).unwrap();
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
        let value = vi.get(&k,&m).unwrap();
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
    for (_i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      for (_j,k) in (1..=rows).zip(vi.row_iter.clone()) {
        let value = vi.get(&k,&m).unwrap();
        match value.as_bool() {
          Some(true) => flag = true,
          _ => (), // TODO Alert user that there was an error
        }
      }
    }
    out.set_unchecked(1, 1, Value::from_bool(flag));
  } else {
    () // TODO alert user that argument is unknown
  };
}

pub extern "C" fn stats_sum(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let rows = vi.rows();
  let cols = vi.columns();

  if *in_arg_name == *ROW {
    out.resize(vi.rows(), 1);
    for i in 1..=rows {
      let mut sum: Value = Value::from_u64(0);
      for j in 1..=cols {
        match vi.get(&TableIndex::Index(i),&TableIndex::Index(j)) {
          Some(value) => {
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
          Some(value) => {
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
    for (_i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      for (_j,k) in (1..=rows).zip(vi.row_iter.clone()) {
        match vi.get(&k,&m) {
          Some(value) => {
            match sum.add(value) {
              Ok(result) => sum = result,
              _ => (), // TODO Alert user that there was an error
            }
          }
          _ => ()
        }
      }
    }
    out.set_unchecked(1, 1, sum);
  } else {
    () // TODO alert user that argument is unknown
  }
}

pub extern "C" fn table_add_row(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  /*
  let _row = 0;
  let mut column = 0;
  let mut out_rows = 0;
  let mut out_columns = 0;

  // Get the size of the output table
  for (_, vi) in arguments {
    let vi_rows = match &vi.row_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.rows(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_rows = if out_rows == 0 {
      vi_rows
    } else if vi_rows > out_rows && out_rows == 1 {
      vi_rows
    } else if vi_rows == out_rows {
      vi_rows
    } else if vi_rows == 1 {
      out_rows
    } else {
      // TODO Throw a size error here
      0
    };

    let vi_columns = match &vi.column_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_columns += vi_columns;

  }

  let base_rows = out.rows();

  // If the table is already bigger than what we need, don't resize
  out_columns = if out.columns() > out_columns {
    out.columns()
  } else {
    out_columns
  };

  out.resize(out_rows + out.rows(), out_columns);

  for (_, vi) in arguments {
    let width = match &vi.column_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    let mut out_row_iter = match out.row_iter {
      IndexIterator::Table(_) => IndexRepeater::new(out.row_iter.clone(),1,1),
      _ => IndexRepeater::new(out.row_iter.clone(), out_columns,1),
    };
    for (_c,j) in (1..=width).zip(vi.column_iter.clone()) {
      let row_iter = if vi.rows() == 1 {
        (1..=out_rows).zip(CycleIterator::Cycle(vi.row_iter.clone().cycle()))
      } else {
        (1..=out_rows).zip(CycleIterator::Index(vi.row_iter.clone()))
      };
      for (_k,i) in row_iter {
        let value = vi.get(&i,&j).unwrap();
        let n = out_row_iter.next();
        let column_alias = unsafe{(*vi.table).get_column_alias(j.unwrap())};
        // If the column has an alias, let's use it instead
        let m = match column_alias {
          Some(alias) => Some(alias),
          None => out.column_iter.next(),
        };
        match (n, m) {
          (_, Some(TableIndex::None)) |
          (Some(TableIndex::None), _) => continue,
          (Some(out_row), Some(out_col)) => {
            out.set(&TableIndex::Index(out_row.unwrap() + base_rows), &out_col, value);
          }
          _ => continue,
        }
      }
    }
    column += width;
  }*/
}

pub extern "C" fn table_set(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  let (_ , mut input) = arguments[0].clone();

  if input.is_scalar() {
    input.inf_cycle();
  }

  for ((out_row, out_column), (value, _)) in out.index_iterator().zip(input) {
    out.set(&out_row, &out_column, value);
  }
}

pub extern "C" fn table_index(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  /*
  let _row = 0;
  let mut column = 0;
  let mut out_rows = 0;
  let mut out_columns = 0;

  // Get the size of the output table
  for (_, vi) in arguments {
    let vi_rows = match &vi.row_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.rows(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_rows = if out_rows == 0 {
      vi_rows
    } else if vi_rows > out_rows && out_rows == 1 {
      vi_rows
    } else if vi_rows == out_rows {
      vi_rows
    } else if vi_rows == 1 {
      out_rows
    } else {
      // TODO Throw a size error here
      0
    };

    let vi_columns = match &vi.column_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_columns += vi_columns;

  }

  out.resize(out_rows, out_columns);

  for (_, vi) in arguments {
    let width = match &vi.column_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    for (c,j) in (1..=width).zip(vi.column_iter.clone()) {
      // Add alias to column if it's there
      unsafe {
        let id = (*vi.table).id;
        match j {
          TableIndex::Index(ix) => {
            match (*vi.table).store.column_index_to_alias.get(&(id,ix)) {
              Some(alias) => {
                let out_id = (*out.table).id;
                let store = &mut *Arc::get_mut_unchecked(&mut (*out.table).store);
                store.column_index_to_alias.entry((out_id,c)).or_insert(*alias);
                store.column_alias_to_index.entry((out_id,*alias)).or_insert(ix);
              }
              _ => (),
            }
          },
          _ => (),
        }
      }
      let mut row_iter = if vi.rows() == 1 {
        CycleIterator::Cycle(vi.row_iter.clone().cycle())
      } else {
        CycleIterator::Index(vi.row_iter.clone())
      };
      for k in 1..=out_rows {
        // Fast forward to the next true value
        let mut i = row_iter.next();
        while i == Some(TableIndex::None) {
          i = row_iter.next();
          if i == None {
            break;
          }
        }
        match vi.get(&i.unwrap(),&j) {
          Some(value) => {
            out.set_unchecked(k, column+c, value);
          }
          _ => (),
        }
      }
    }
    column += width;
  }*/
}

pub extern "C" fn table_copy(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  let (_, vi) = &arguments[0];
  out.resize(vi.rows(), vi.columns());
  for j in 1..=vi.columns() {
    for i in 1..=vi.rows() {
      let (value, _) = vi.get_unchecked(i,j);
      out.set_unchecked(i, j, value);
    }
  }
}

pub extern "C" fn table_horizontal_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  // Do all of the arguments have a compatible height?
  if arguments.iter().map(|(_, vi)| vi.rows()).collect::<HashSet<usize>>().len() != 1 {
    // TODO Warn that one or more arguments is the wrong height
    return;
  }

  // Get the size of the output table we will create, and resize the out table
  let (_, vi) = &arguments[0];
  let out_rows = vi.rows();
  let out_columns: usize = arguments.iter().map(|(_, vi)| vi.columns()).sum();
  out.resize(out_rows, out_columns);

  // Iterate through the input table and insert values into output table
  let mut column = 0;
  for (_, vi) in arguments {
    let width = vi.columns();
    let mut out_row_iter = IndexRepeater::new(IndexIterator::Range(1..=out_rows),width,1);
    let mut out_column_iter = IndexRepeater::new(IndexIterator::Range(column+1..=column+width),1,out_rows as u64);
    for (((value, _), out_row_ix), out_column_ix) in vi.clone().zip(out_row_iter).zip(out_column_iter) {
      out.set_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap(), value);
    }
    column += width;
  }
}

pub extern "C" fn table_vertical_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  // Do all of the arguments have a compatible height?
  if arguments.iter().map(|(_, vi)| vi.columns()).collect::<HashSet<usize>>().len() != 1 {
    // TODO Warn that one or more arguments is the wrong height
    return;
  }
  
  // Get the size of the output table we will create, and resize the out table
  let (_, vi) = &arguments[0];
  let out_columns = vi.columns();
  let out_rows: usize = arguments.iter().map(|(_, vi)| vi.rows()).sum();
  out.resize(out_rows, out_columns);

  // Iterate through the input table and insert values into output table
  let mut row = 0;
  for (_, vi) in arguments {
    let height = vi.rows();
    let mut out_row_iter = IndexRepeater::new(IndexIterator::Range(row+1..=row+height),out_columns,1);
    let mut out_column_iter= IndexRepeater::new(IndexIterator::Range(1..=out_columns),1, height as u64);
    for (((value, _), out_row_ix), out_column_ix) in vi.clone().zip(out_row_iter).zip(out_column_iter) {
      out.set_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap(), value);
    }
    row += height;
  }
}

pub extern "C" fn table_range(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  // TODO test argument count is 2 or 3
  // 2 -> start, end
  // 3 -> start, increment, end
  let (_, start_vi) = &arguments[0];
  let (_, end_vi) = &arguments[1];
  // TODO add increment argument if there are three

  // TODO We have to test to see if all of these things are valid
  let start_value = start_vi.get(&TableIndex::Index(1),&TableIndex::Index(1)).unwrap();
  let end_value = end_vi.get(&TableIndex::Index(1),&TableIndex::Index(1)).unwrap();
  let start = start_value.as_u64().unwrap() as usize;
  let end = end_value.as_u64().unwrap() as usize;
  let range = end - start;
  
  out.resize(range+1, 1);
  let mut j = 1;
  for i in start..=end {
    out.set(&TableIndex::Index(j), &TableIndex::Index(1), Value::from_u64(i as u64));
    j += 1;
  }
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
      // TODO test argument count is 2
      let (_, lhs) = &arguments[0];
      let (_, rhs) = &arguments[1];

      let (out_rows, out_columns, lhs_iter, rhs_iter) = 
      // Equal dimensions
      if lhs.rows() == rhs.rows() && lhs.columns() == rhs.columns() {
        (lhs.rows(), lhs.columns(), lhs.clone(), rhs.clone())
      // LHS scalar
      } else if lhs.is_scalar() {
        let mut lhs = lhs.clone();
        lhs.inf_cycle();
        (rhs.rows(), rhs.columns(), lhs, rhs.clone())
      // RHS scalar
      } else if rhs.is_scalar() {
        let mut rhs = rhs.clone();
        rhs.inf_cycle();
        (lhs.rows(), lhs.columns(), lhs.clone(), rhs)      
      } else {
        // TODO Warn of mismatch of dimensions
        return;
      };

      out.resize(out_rows, out_columns);

      let mut out_row_iter = IndexRepeater::new(IndexIterator::Range(1..=out_rows), out_columns, 1);
      let mut out_column_iter= IndexRepeater::new(IndexIterator::Range(1..=out_columns), 1, out_rows as u64);
      for ((((lhs_value, lhs_changed), (rhs_value, rhs_changed)), out_row_ix), out_column_ix) in 
              lhs_iter.zip(rhs_iter).zip(out_row_iter).zip(out_column_iter) {
        match (lhs_value, rhs_value, lhs_changed, rhs_changed)
        {
          (lhs_value, rhs_value, true, true) => {
            match lhs_value.$op(rhs_value) {
              Ok(result) => {
                out.set_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap(), result);
              }
              Err(_) => (), // TODO Handle error here
            }
          }
          // If either operand is not changed but the output is cell is empty, then we can do the operation
          (lhs_value, rhs_value, false, _) |
          (lhs_value, rhs_value, _, false) => {
            let (out_value, _) = out.get_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap());
            if out_value.is_empty() {
              match lhs_value.$op(rhs_value) {
                Ok(result) => {
                  out.set_unchecked(out_row_ix.unwrap(), out_column_ix.unwrap(), result);
                }
                Err(_) => (), // TODO Handle error here
              }
            }
          }
        }        
      }
    }
  )
}

binary_infix!{math_add, add}
binary_infix!{math_subtract, sub}
binary_infix!{math_multiply, multiply}
binary_infix!{math_divide, divide}
binary_infix!{math_exponent, power}
binary_infix!{compare_greater_than_equal, greater_than_equal}
binary_infix!{compare_greater_than, greater_than}
binary_infix!{compare_less_than_equal, less_than_equal}
binary_infix!{compare_less_than, less_than}
binary_infix!{compare_equal, equal}
binary_infix!{compare_not_equal, not_equal}
binary_infix!{logic_and, and}
binary_infix!{logic_or, or}