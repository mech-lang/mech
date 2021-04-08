extern crate mech_core;
extern crate mech_utilities;
extern crate libm;
#[macro_use]
extern crate lazy_static;
use mech_core::{Transaction};
use mech_core::{Value, ValueMethods, IndexIterator, Table, ValueIterator};
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity, hash_string};
use libm::{sin, cos, fmod, round, floor};

static PI: f64 = 3.141592653589793238462643383279502884197169399375105820974944592307816406286;

lazy_static! {
  static ref ANGLE: u64 = hash_string("angle");
  static ref TABLE: u64 = hash_string("table");
}

#[no_mangle]
pub extern "C" fn math_sin(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  let (in_arg_name, vi) = &arguments[0];
  let mut rows = vi.rows();
  let mut cols = vi.columns();
  if *in_arg_name == *ANGLE {
    out.resize(rows*cols,1);
    let mut flag: bool = false;
    for (i,k) in (1..=rows).zip(vi.row_iter.clone()) {
      for (j,m) in (1..=cols).zip(vi.column_iter.clone()) {
        let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
        match value.as_quantity() {
          Some(x) => {
            let result = match fmod(x.as_float().unwrap(), 360.0) {
              0.0 => 0.0,
              90.0 => 1.0,
              180.0 => 0.0,
              270.0 => -1.0,
              _ => sin(x.as_float().unwrap() * PI / 180.0),
            };
            unsafe {
              (*out.table).set_unchecked(i, j, Value::from_f64(result));
            }
          },
          _ => (), // TODO Alert user that there was an error
        }
      }
    }  
  } else {
    // TODO Warn about unknown argument
  }
}

#[no_mangle]
pub extern "C" fn math_cos(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  let (in_arg_name, vi) = &arguments[0];
  let mut rows = vi.rows();
  let mut cols = vi.columns();
  if *in_arg_name == *ANGLE {
    out.resize(rows*cols,1);
    let mut flag: bool = false;
    for (i,k) in (1..=rows).zip(vi.row_iter.clone()) {
      for (j,m) in (1..=cols).zip(vi.column_iter.clone()) {
        let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
        match value.as_quantity() {
          Some(x) => {
            let result = match fmod(x.as_float().unwrap(), 360.0) {
              0.0 => 1.0,
              90.0 => 0.0,
              180.0 => -1.0,
              270.0 => 0.0,
              _ => cos(x.as_float().unwrap() * PI / 180.0),
            };
            unsafe {
              (*out.table).set_unchecked(i, j, Value::from_f64(result));
            }
          },
          _ => (), // TODO Alert user that there was an error
        }
      }
    }  
  } else {
    // TODO Warn about unknown argument
  }
}


#[no_mangle]
pub extern "C" fn math_round(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  let (in_arg_name, vi) = &arguments[0];
  let mut rows = vi.rows();
  let mut cols = vi.columns();
  if *in_arg_name == *TABLE {
      out.resize(rows*cols,1);
      let mut flag: bool = false;
      for (i,k) in (1..=rows).zip(vi.row_iter.clone()) {
        for (j,m) in (1..=cols).zip(vi.column_iter.clone()) {
          let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
          match value.as_float() {
            Some(x) => {
              unsafe {
                (*out.table).set_unchecked(i, j, Value::from_f64(round(x)));
              }
            },
            _ => (), // TODO Alert user that there was an error
          }
        }
      }  
  } else {
    // TODO Warn about unknown argument
  }
}

#[no_mangle]
pub extern "C" fn math_floor(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator) { 
  let (in_arg_name, vi) = &arguments[0];
  let mut rows = vi.rows();
  let mut cols = vi.columns();
  if *in_arg_name == *TABLE {
    out.resize(rows*cols,1);
    let mut flag: bool = false;
    for (i,k) in (1..=rows).zip(vi.row_iter.clone()) {
      for (j,m) in (1..=cols).zip(vi.column_iter.clone()) {
        let value = unsafe{(*vi.table).get(&k,&m).unwrap()};
        match value.as_float() {
          Some(x) => {
            unsafe {
              (*out.table).set_unchecked(i, j, Value::from_f64(floor(x)));
            }
          },
          _ => (), // TODO Alert user that there was an error
        }
      }
    }  
  } else {
    // TODO Warn about unknown argument
  }
}