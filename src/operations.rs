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
use hashbrown::HashMap;
use ::{hash_string};


lazy_static! {
  static ref ROW: u64 = hash_string("row");
  static ref COLUMN: u64 = hash_string("column");
  static ref TABLE: u64 = hash_string("table");
}

pub fn resolve_subscript(
  table_id: TableId,
  row_index: TableIndex,
  column_index: TableIndex,
  block_tables: &mut HashMap<u64, Table>,
  database: &Arc<RefCell<Database>>) -> ValueIterator {

  let mut db = database.borrow_mut();
  let mut table_id = table_id;

  let mut table = match table_id {
    TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
    TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
  };

  unsafe{
    if (*table).rows == 1 && (*table).columns == 1 {
      match (row_index, column_index) {
        (TableIndex::All, TableIndex::All) => (),
        (_, _) => {
          let (reference, _) = (*table).get_unchecked(1,1);
          match reference.as_reference() {
            Some(table_reference) => {
              match db.tables.get_mut(&table_reference) {
                Some(dbtable) => table = dbtable as *mut Table,
                None => (),
              }
            }
            _ => (),
          }
        }
      }
    }
    table_id = TableId::Global((*table).id);
  }

  let row_iter = unsafe { match row_index {
    TableIndex::Index(ix) => IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(ix))),
    TableIndex::All => {
      match (*table).rows {
        0 => IndexIterator::None,
        r => IndexIterator::Range(1..=r),
      }
    },
    TableIndex::Table(table_id) => {
      let row_table = match table_id {
        TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
        TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
      };
      IndexIterator::Table(TableIterator::new(row_table))
    }
    TableIndex::Alias(alias) => IndexIterator::Alias(AliasIterator::new(alias, table_id, db.store.clone())),
    _ => IndexIterator::Range(1..=(*table).rows),
  }};

  let column_iter = unsafe { match column_index {
    TableIndex::Index(ix) => IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(ix))),
    TableIndex::All => {
      match (*table).columns {
        0 => IndexIterator::None,
        c => IndexIterator::Range(1..=c),
      }
    }
    TableIndex::Table(table_id) => {
      let col_table = match table_id {
        TableId::Global(id) => db.tables.get_mut(&id).unwrap() as *mut Table,
        TableId::Local(id) => block_tables.get_mut(&id).unwrap() as *mut Table,
      };
      IndexIterator::Table(TableIterator::new(col_table))
    }
    TableIndex::Alias(alias) => IndexIterator::Alias(AliasIterator::new(alias, table_id, db.store.clone())),
    TableIndex::None => IndexIterator::None,
    //_ => IndexIterator::Range(1..=(*table).columns),
  }};

  ValueIterator{
    scope: table_id,
    table,
    row_index,
    column_index,
    row_iter: IndexRepeater::new(row_iter,1,1),
    column_iter: IndexRepeater::new(column_iter,1,1),
  }
}

pub type MechFunction = extern "C" fn(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator);

pub extern "C" fn set_any(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  /*
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let rows = vi.rows();
  let cols = match vi.column_iter {
    IndexIterator::Constant{..} => 1,
    _ => vi.columns(),
  };

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
  };*/
}

pub extern "C" fn stats_sum(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  /*
  // TODO test argument count is 1
  let (in_arg_name, vi) = &arguments[0];

  let rows = vi.rows();
  let cols = match vi.column_iter {
    IndexIterator::Constant{..} |
    IndexIterator::Alias{..} => 1,
    _ => vi.columns(),
  };

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
  }*/
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
  /*
  let _row = 0;
  let mut column = 0;
  let mut out_rows = 0;
  let mut out_columns = 0;

  // Get the size of the output table
  for (_, vi) in arguments {
    let _vi_rows = match &vi.row_iter {
      IndexIterator::None => 0,
      IndexIterator::Range(_) => vi.rows(),
      IndexIterator::Constant(_) => 1,
      IndexIterator::Alias(_) => 1,
      IndexIterator::Table(iter) => iter.len(),
    };
    let vi_columns = match &vi.column_iter {
      IndexIterator::None => 0,
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
        let value = vi.get(&i,&j);
        let n = out_row_iter.next();
        let m = out.column_iter.next();
        match (n, m, value) {
          (_, Some(TableIndex::None), _) |
          (Some(TableIndex::None), _, _) => {
            continue;
          }
          (Some(out_row), Some(out_col), Some(value)) => {
            out.set(&out_row, &out_col, value);
          }
          _ => continue,
        }
      }
    }
    column += width;
  }*/
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

pub extern "C" fn table_vertical_concatenate(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  /*let mut row = 0;
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

  out.resize(out_rows, out_columns);
  
  for (_, vi) in arguments {
    for (i,k) in (1..=out_columns).zip(vi.column_iter.clone()) {
      // Add alias to column if it's there
      unsafe {
        let id = (*vi.table).id;
        match k {
          TableIndex::Index(ix) => {
            match (*vi.table).store.column_index_to_alias.get(&(id,ix)) {
              Some(alias) => {
                let out_id = (*out.table).id;
                let store = &mut *Arc::get_mut_unchecked(&mut (*out.table).store);
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
        let value = vi.get(&TableIndex::Index(j),&k).unwrap();
        out.set(&TableIndex::Index(row + j), &TableIndex::Index(i), value);
      }
    }
    row += 1;
  }*/
}

pub extern "C" fn table_range(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
  /*// TODO test argument count is 2 or 3
  // 2 -> start, end
  // 3 -> start, increment, end
  let (_, start_vi) = &arguments[0];
  let (_, end_vi) = &arguments[1];

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
  */
}

#[macro_export]
macro_rules! binary_infix {
  ($func_name:ident, $op:tt) => (
    pub extern "C" fn $func_name(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) {
      /*
      // TODO test argument count is 2
      let (_, lhs_vi) = &arguments[0];
      let (_, rhs_vi) = &arguments[1];

      // Figure out dimensions
      let lhs_rows_count = match lhs_vi.row_index {
        TableIndex::All => lhs_vi.rows(),
        _ => 1,
      };
      let lhs_columns_count = match lhs_vi.column_index {
        TableIndex::All => lhs_vi.columns(),
        _ => 1,
      };
      let rhs_rows_count = match rhs_vi.row_index {
        TableIndex::All => rhs_vi.rows(),
        _ => 1,
      };
      let rhs_columns_count = match rhs_vi.column_index {
        TableIndex::All => rhs_vi.columns(),
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
          IndexRepeater::new(lhs_vi.row_iter.clone(),1,1),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1,1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),1,1),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1,1),
          IndexRepeater::new(IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(1))),1,1),
          IndexRepeater::new(IndexIterator::Constant(ConstantIterator::new(TableIndex::Index(1))),1,1),
        )
      } else if equal_dimensions {
        out.resize(lhs_rows_count, lhs_columns_count);
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),lhs_vi.columns(),1),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),rhs_vi.columns(),1),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1,1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count,1),
          IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1,1),
        )
      } else if rhs_scalar {
        out.resize(lhs_rows_count, lhs_columns_count);
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),lhs_columns_count,1),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1,1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),1,1),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1,1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count,1),
          match out.column_index {
            TableIndex::All => IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1,1),
            _ => IndexRepeater::new(IndexIterator::Constant(ConstantIterator::new(out.column_index)),1,1),
          },
        )
      } else {
        out.resize(rhs_rows_count, rhs_columns_count);
        (
          IndexRepeater::new(lhs_vi.row_iter.clone(),1),
          IndexRepeater::new(lhs_vi.column_iter.clone(),1),
          IndexRepeater::new(rhs_vi.row_iter.clone(),rhs_columns_count),
          IndexRepeater::new(rhs_vi.column_iter.clone(),1),
          IndexRepeater::new(IndexIterator::Range(1..=out_rows_count),out_columns_count,1),
          match out.column_index {
            TableIndex::All => IndexRepeater::new(IndexIterator::Range(1..=out_columns_count),1,1),
            _ => IndexRepeater::new(IndexIterator::Constant(ConstantIterator::new(out.column_index)),1,1),
          },
        )
      };

      let mut i = 1;
      let out_elements = out.rows() * out.columns();

      loop {
        let l1 = lrix.next().unwrap().unwrap();
        let l2 = lcix.next().unwrap().unwrap();
        let r1 = rrix.next().unwrap().unwrap();
        let r2 = rcix.next().unwrap().unwrap();
        let o1 = out_rix.next().unwrap().unwrap();
        let o2 = out_cix.next().unwrap().unwrap();
        let (lhs_value, lhs_changed) = if l2 == 0 {
          lhs_vi.get_unchecked_linear(l1)
        } else {
          lhs_vi.get_unchecked(l1,l2)
        };
        let (rhs_value, rhs_changed) = if r2 == 0 {
          rhs_vi.get_unchecked_linear(r1)
        } else {
          rhs_vi.get_unchecked(r1,r2)
        };
        match (lhs_value, rhs_value, lhs_changed, rhs_changed)
        {
          (lhs_value, rhs_value, true, true) => {
            match lhs_value.$op(rhs_value) {
              Ok(result) => {
                out.set_unchecked(o1, o2, result);
              }
              Err(_) => (), // TODO Handle error here
            }
          }
          // If either operand is not changed but the output is cell is empty, then we can do the operation
          (lhs_value, rhs_value, false, _) |
          (lhs_value, rhs_value, _, false) => {
            let (out_value, _) = out.get_unchecked(o1, o2);
            if out_value.is_empty() {
              match lhs_value.$op(rhs_value) {
                Ok(result) => {
                  out.set_unchecked(o1, o2, result);
                }
                Err(_) => (), // TODO Handle error here
              }
            }
          }
        }
        if i >= out_elements {
          break;
        }
        i += 1;
      }*/
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