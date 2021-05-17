extern crate mech_core;
extern crate mech_utilities;
extern crate libm;
#[macro_use]
extern crate lazy_static;
use mech_core::{Transaction};
use mech_core::{Value, ValueMethods, IndexIterator, Table, ValueIterator};
use mech_core::{Quantity, ToQuantity, QuantityMath, hash_string, Argument};
use libm::{sinf, cosf, fmodf, roundf, floorf};
use std::cell::RefCell;
use std::rc::Rc;

static PI: f32 = 3.141592653589793238462643383279502884197169399375105820974944592307816406286;

lazy_static! {
  static ref ANGLE: u64 = hash_string("angle");
  static ref TABLE: u64 = hash_string("table");
}

#[no_mangle]
pub extern "C" fn math_sin(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let arg = arguments[0].borrow();
  let in_arg_name = arg.name;
  let vi = arg.iterator.clone();
  let mut out = arguments.last().unwrap().borrow().iterator.clone();
  if in_arg_name == *ANGLE {
    out.resize(vi.rows(),vi.columns());
    let mut flag: bool = false;
    for ((value, changed), out_ix) in vi.clone().zip(out.linear_index_iterator()) {
      match value.as_quantity() {
        Some(x) => {
          let result = match fmodf(x.as_f32().unwrap(), 360.0) {
            0.0 => 0.0,
            90.0 => 1.0,
            180.0 => 0.0,
            270.0 => -1.0,
            _ => sinf(x.as_f32().unwrap() * PI / 180.0),
          };
          out.set_unchecked_linear(out_ix, Value::from_f32(result));
        },
        _ => (), // TODO Alert user that there was an error
      }
    }
  } else {
    // TODO Warn about unknown argument
  }
}

#[no_mangle]
pub extern "C" fn math_cos(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let arg = arguments[0].borrow();
  let in_arg_name = arg.name;
  let vi = arg.iterator.clone();
  let mut out = arguments.last().unwrap().borrow().iterator.clone();
  if in_arg_name == *ANGLE {
    out.resize(vi.rows(),vi.columns());
    let mut flag: bool = false;
    for ((value, changed), out_ix) in vi.clone().zip(out.linear_index_iterator()) {
      match value.as_quantity() {
        Some(x) => {
          let result = match fmodf(x.as_f32().unwrap(), 360.0) {
            0.0 => 1.0,
            90.0 => 0.0,
            180.0 => -1.0,
            270.0 => 0.0,
            _ => cosf(x.as_f32().unwrap() * PI / 180.0),
          };
          out.set_unchecked_linear(out_ix, Value::from_f32(result));
        },
        _ => (), // TODO Alert user that there was an error
      }
    }
  } else {
    // TODO Warn about unknown argument
  }
}


#[no_mangle]
pub extern "C" fn math_round(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let arg = arguments[0].borrow();
  let in_arg_name = arg.name;
  let vi = arg.iterator.clone();
  let mut out = arguments.last().unwrap().borrow().iterator.clone();
  if in_arg_name == *TABLE {
    out.resize(vi.rows(),vi.columns());
    let mut flag: bool = false;
    for ((value, changed), out_ix) in vi.clone().zip(out.linear_index_iterator()) {
      match value.as_f32() {
        Some(x) => {
          out.set_unchecked_linear(out_ix, Value::from_f32(roundf(x)));
        },
        _ => (), // TODO Alert user that there was an error
      }
    }
  } else {
    // TODO Warn about unknown argument
  }
}

#[no_mangle]
pub extern "C" fn math_floor(arguments:  &mut Vec<Rc<RefCell<Argument>>>) {
  let arg = arguments[0].borrow();
  let in_arg_name = arg.name;
  let vi = arg.iterator.clone();
  let mut out = arguments.last().unwrap().borrow().iterator.clone();
  if in_arg_name == *TABLE {
    out.resize(vi.rows(),vi.columns());
    let mut flag: bool = false;
    for ((value, changed), out_ix) in vi.clone().zip(out.linear_index_iterator()) {
      match value.as_f32() {
        Some(x) => {
          out.set_unchecked_linear(out_ix, Value::from_f32(floorf(x)));
        },
        _ => (), // TODO Alert user that there was an error
      }
    }
  } else {
    // TODO Warn about unknown argument
  }
}