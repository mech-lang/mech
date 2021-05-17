extern crate mech_core;
extern crate mech_utilities;
#[macro_use]
extern crate lazy_static;
use mech_core::{Transaction, ValueIterator, ValueMethods};
use mech_core::{Value, Table, TableIndex};
use mech_core::{Quantity, ToQuantity, QuantityMath, hash_string, Argument};
use std::cell::RefCell;
use std::rc::Rc;

lazy_static! {
  static ref ROW: u64 = hash_string("row");
  static ref COLUMN: u64 = hash_string("column");
  static ref TABLE: u64 = hash_string("table");
}

#[no_mangle]
pub extern "C" fn stats_average(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  // TODO test argument count is 1
  let arg = arguments[0].borrow();
  let in_arg_name = arg.name;
  let vi = arg.iterator.clone();
  let mut out = arguments.last().unwrap().borrow().iterator.clone();

  let mut in_rows = vi.rows();
  let mut in_columns = vi.columns();

  if in_arg_name == *ROW {
    out.resize(in_rows, 1);
    for i in 1..=in_rows {
      let mut sum: Value = Value::from_u32(0);
      for j in 1..=in_columns {
        match vi.get(&TableIndex::Index(i),&TableIndex::Index(j)) {
          Some((value,_)) => {
            sum = sum.add(value)
          }
          _ => ()
        }
      }
      out.set_unchecked(i, 1, Value::from_f32(sum.as_f32().unwrap() / vi.columns() as f32));
    }
  } else if in_arg_name == *COLUMN {
    out.resize(1, in_columns);
    for (i,m) in (1..=in_columns).zip(vi.column_iter.clone()) {
      let mut sum: Value = Value::from_u32(0);
      for (j,k) in (1..=in_rows).zip(vi.row_iter.clone()) {
        match vi.get(&k,&m) {
          Some((value,_)) => {
            sum = sum.add(value)
          }
          _ => ()
        }
      }
      out.set_unchecked(1, i, Value::from_f32(sum.as_f32().unwrap() / vi.rows() as f32));
    }      
  } else if in_arg_name == *TABLE {
    out.resize(1, 1);
    let mut sum: Value = Value::from_u32(0);
    for (value,_) in vi.clone() {
      sum = sum.add(value)
    }
    out.set_unchecked(1, 1, Value::from_f32(sum.as_f32().unwrap() / (vi.rows() * vi.columns()) as f32));
  } else {
    // TODO Warn about unknown argument
  }
}