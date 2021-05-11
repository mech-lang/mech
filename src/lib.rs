extern crate mech_core;
extern crate mech_utilities;
#[macro_use]
extern crate lazy_static;
use mech_core::{Transaction};
use mech_core::{Value, ValueMethods, IndexIterator, Table, TableIndex, ValueIterator};
use mech_core::{Quantity, ToQuantity, QuantityMath, hash_string};

lazy_static! {
  static ref TABLE: u64 = hash_string("table");
  static ref ROW: u64 = hash_string("row");
  static ref COLUMN: u64 = hash_string("column");
}

#[no_mangle]
pub extern "C" fn set_none(arguments: &Vec<(u64, ValueIterator)>) {
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
          Some(true) => flag = false,
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
          Some(true) => flag = false,
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
        Some(true) => flag = false,
        _ => (), // TODO Alert user that there was an error
      }
    }
    out.set_unchecked(1, 1, Value::from_bool(flag));
  } else {
    () // TODO alert user that argument is unknown
  };
}