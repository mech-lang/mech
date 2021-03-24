// # Operations

// ## Prelude

#[cfg(feature = "no-std")] use alloc::vec::Vec;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(not(feature = "no-std"))] use rust_core::fmt;
use table::{Table, TableId, Index};
use value::{Value, ValueMethods};
use runtime::Runtime;
use block::{Block};
use index::{IndexIterator, TableIterator, AliasIterator, ValueIterator, IndexRepeater};
use database::Database;
use errors::ErrorType;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use hashbrown::HashMap;
use ::{humanize, hash_string};


lazy_static! {
  static ref ROW: u64 = hash_string("row");
  static ref COLUMN: u64 = hash_string("column");
  static ref TABLE: u64 = hash_string("table");
}

pub fn resolve_subscript(
  table_id: TableId, 
  row_index: Index, 
  column_index: Index,
  block_tables: &mut HashMap<u64, Table>, 
  database: &Arc<RefCell<Database>>) -> ValueIterator {

  let mut db = database.borrow_mut();

  let mut table = match table_id {
    TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
  };

  unsafe{
    if (*table).rows == 1 && (*table).columns == 1 {
      match (row_index, column_index) {
        (Index::All, Index::All) => (),
        _ => {
          let (reference, _) = (*table).get_unchecked(1,1);
          match reference.as_reference() {
            Some(table_reference) => {
              table = db.tables.get_mut(&table_reference).unwrap() as *mut Table;
            }
            _ => (),
          }
          
        }
      }
    }
  }

  let row_iter = unsafe { match row_index {
    Index::Index(ix) => IndexIterator::Constant(Index::Index(ix)),
    Index::All => {
      let r = if (*table).rows == 0 {
        1
      } else {
        (*table).rows
      };
      IndexIterator::Range(1..=r)
    },
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
    Index::None => IndexIterator::Constant(Index::Index(0)),
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


pub type MechFunction = extern "C" fn(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator);


pub extern "C" fn set_any(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {                                        

  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let mut rows = vi.rows();
  let mut cols = match vi.column_iter {
    IndexIterator::Constant{..} => 1,
    _ => vi.columns(),
  };

  if *in_arg_name == *ROW {
    unsafe { (*out.table).resize(vi.rows(), 1); }
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
  } else if *in_arg_name == *COLUMN {
    unsafe { (*out.table).resize(1, cols); }
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
  } else if *in_arg_name == *TABLE {
    unsafe { (*out.table).resize(1, 1); }
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
  } else { 
    () // TODO alert user that argument is unknown
  };
}

pub extern "C" fn stats_sum(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {                                        

  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let mut rows = vi.rows();
  let mut cols = match vi.column_iter {
    IndexIterator::Constant{..} |
    IndexIterator::Alias{..} => 1,
    _ => vi.columns(),
  };

  if *in_arg_name == *ROW {
    unsafe { (*out.table).resize(vi.rows(), 1); }
    for i in 1..=rows {
      let mut sum: Value = Value::from_u64(0);
      for j in 1..=cols {
        match vi.get(&Index::Index(i),&Index::Index(j)) {
          Some(value) => {
            match sum.add(value) {
              Ok(result) => sum = result,
              _ => (), // TODO Alert user that there was an error
            }
          }
          _ => ()
        }
      }
      unsafe {
        (*out.table).set_unchecked(i, 1, sum);
      }
    }
  } else if *in_arg_name == *COLUMN {
    unsafe { (*out.table).resize(1, cols); }
    for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      let mut sum: Value = Value::from_u64(0);
      for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
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
      unsafe {
        (*out.table).set_unchecked(1, i, sum);
      }
    }      
  } else if *in_arg_name == *TABLE {
    unsafe { (*out.table).resize(1, 1); }
    let mut sum: Value = Value::from_u64(0);
    for (i,m) in (1..=cols).zip(vi.column_iter.clone()) {
      for (j,k) in (1..=rows).zip(vi.row_iter.clone()) {
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
    unsafe {
      (*out.table).set_unchecked(1, 1, sum);
    }    
  } else {
    () // TODO alert user that argument is unknown
  } 
}
    
pub extern "C" fn table_add_row(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {

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
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_columns += vi_columns;

  }

  let base_rows = out.rows();
  
  unsafe { (*out.table).resize(out_rows + out.rows(), out_columns); }

  for (_, vi) in arguments {
    let width = match &vi.column_iter {
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    let mut out_row_iter = match out.row_iter {
      IndexIterator::Table(_) => IndexRepeater::new(out.row_iter.clone(),1),
      _ => IndexRepeater::new(out.row_iter.clone(), out_columns),
    };
    for (c,j) in (1..=width).zip(vi.column_iter.clone()) {
      let row_iter = if vi.rows() == 1 {
        (1..=out_rows).zip(CycleIterator::Cycle(vi.row_iter.clone().cycle()))
      } else {
        (1..=out_rows).zip(CycleIterator::Index(vi.row_iter.clone()))
      };
      for (k,i) in row_iter {
        let value = vi.get(&i,&j).unwrap();
        let n = out_row_iter.next();
        let m = out.column_iter.next();
        match (n, m) {
          (_, Some(Index::None)) |
          (Some(Index::None), _) => continue,
          (Some(out_row), Some(out_col)) => {
            unsafe {
              (*out.table).set_unchecked(out_row.unwrap() + base_rows, out_col.unwrap(), value);
            }
          }
          _ => continue,
        }
      }
    }
    column += width;
    
  }
}

pub extern "C" fn table_set(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {

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
    let vi_columns = match &vi.column_iter {
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_columns += vi_columns;
    out_rows = out.rows();
  }

  for (_, vi) in arguments {
    let width = match &vi.column_iter {
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };

    let mut out_row_iter = match out.row_iter {
      IndexIterator::Table(_) => IndexRepeater::new(out.row_iter.clone(),1),
      _ => IndexRepeater::new(out.row_iter.clone(), out_columns),
    };

    for (c,j) in (1..=width).zip(vi.column_iter.clone()) {
      let row_iter = if vi.rows() == 1 {
        (1..=out_rows).zip(CycleIterator::Cycle(vi.row_iter.clone().cycle()))
      } else {
        (1..=out_rows).zip(CycleIterator::Index(vi.row_iter.clone()))
      };
      for (k,i) in row_iter {
        let value = vi.get(&i,&j).unwrap();
        let n = out_row_iter.next();
        let m = out.column_iter.next();
        match (n, m) {
          (_, Some(Index::None)) |
          (Some(Index::None), _) => continue,
          (Some(out_row), Some(out_col)) => {
            unsafe {
              (*out.table).set(&out_row, &out_col, value);
            }
          }
          _ => continue,
        }
      }
    }
    column += width;
  }
}

pub extern "C" fn table_horizontal_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
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
      IndexIterator::Range(_) => vi.columns(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    out_columns += vi_columns;

  }
  unsafe { (*out.table).resize(out_rows, out_columns); }
  for (_, vi) in arguments {
    let width = match &vi.column_iter {
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
          Index::Index(ix) => {
            match (*vi.table).store.column_index_to_alias.get(&(id,ix)) {
              Some(alias) => {
                let out_id = (*out.table).id;
                let store = unsafe{&mut *Arc::get_mut_unchecked(&mut (*out.table).store)};
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
        while i == Some(Index::None) {
          i = row_iter.next();
          if i == None {
            break;
          }
        }
        match vi.get(&i.unwrap(),&j) {
          Some(value) => {
            unsafe {
              (*out.table).set_unchecked(k, column+c, value);
            }
          }
          _ => (),
        }
      }
    }
    column += width;
  }
}

enum CycleIterator {
  Cycle(std::iter::Cycle<IndexIterator>),
  Index(IndexIterator),
}

impl Iterator for CycleIterator {
  type Item = Index;
  
  fn next(&mut self) -> Option<Index> {
    match self {
      CycleIterator::Cycle(itr) => itr.next(),
      CycleIterator::Index(itr) => itr.next(),
    }
  }  
}

pub extern "C" fn table_vertical_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
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
  unsafe { (*out.table).resize(out_rows, out_columns); }
  for (_, vi) in arguments {
    for (i,k) in (1..=out_columns).zip(vi.column_iter.clone()) {
      // Add alias to column if it's there
      unsafe {
        let id = (*vi.table).id;
        match k {
          Index::Index(ix) => {
            match (*vi.table).store.column_index_to_alias.get(&(id,ix)) {
              Some(alias) => {
                let out_id = (*out.table).id;
                let store = unsafe{&mut *Arc::get_mut_unchecked(&mut (*out.table).store)};
                store.column_index_to_alias.entry((out_id,i)).or_insert(*alias);
                store.column_alias_to_index.entry((out_id,*alias)).or_insert(ix);
              }
              _ => (),
            }
          },
          _ => (),
        }
      }
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

pub extern "C" fn table_range(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
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
    (*out.table).resize(range+1, 1);
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
    pub extern "C" fn $func_name(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
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
        unsafe { (*out.table).resize(lhs_rows_count, lhs_columns_count); }
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),lhs_vi.columns()),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),rhs_vi.columns()),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count),
          IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1),
        )
      } else if rhs_scalar {
        unsafe { (*out.table).resize(lhs_rows_count, lhs_columns_count); }
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
        unsafe { (*out.table).resize(rhs_rows_count, rhs_columns_count); }
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
          let (lhs_value, lhs_changed) = if l2 == 0 {
            (*lhs_vi.table).get_unchecked_linear(l1)
          } else {
            (*lhs_vi.table).get_unchecked(l1,l2)
          };
          let (rhs_value, rhs_changed) = if r2 == 0 {
            (*rhs_vi.table).get_unchecked_linear(r1)
          } else {
            (*rhs_vi.table).get_unchecked(r1,r2)
          };
          println!("{:?} * {:?}", lhs_value, rhs_value);
          match (lhs_value, rhs_value, lhs_changed, rhs_changed)
          {
            (lhs_value, rhs_value, true, true) => {
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