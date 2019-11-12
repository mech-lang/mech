extern crate mech_core;
extern crate mech_utilities;
extern crate libm;
use mech_core::{Interner, Transaction};
use mech_core::Value;
use mech_utilities::Watcher;
use mech_core::{Quantity, ToQuantity, QuantityMath, make_quantity};
use libm::{sin, cos, fmod, round, floor};

static PI: f64 = 3.141592653589793238462643383279502884197169399375105820974944592307816406286;

#[no_mangle]
pub extern "C" fn math_sin(x: Value) -> Value {
  let result = match fmod(x.as_float().unwrap(), 360.0) {
    0.0 => 0.0,
    90.0 => 1.0,
    180.0 => 0.0,
    270.0 => -1.0,
    _ => sin(x.as_float().unwrap() * PI / 180.0),
  };
  Value::from_quantity(result.to_quantity())
}

#[no_mangle]
pub extern "C" fn math_cos(x: Value) -> Value {
  let result = match fmod(x.as_float().unwrap(), 360.0) {
    0.0 => 1.0,
    90.0 => 0.0,
    180.0 => -1.0,
    270.0 => 0.0,
    _ => cos(x.as_float().unwrap() * PI / 180.0),
  };
  Value::from_quantity(result.to_quantity())
}

#[no_mangle]
pub extern "C" fn math_round(x: Value) -> Value {
  match x {
    Value::Number(n) => Value::from_quantity(round(n.to_float()).to_quantity()),
    _ => Value::Empty
  }
}

#[no_mangle]
pub extern "C" fn math_floor(x: Value) -> Value {
  match x {
    Value::Number(n) => Value::from_quantity(floor(n.to_float()).to_quantity()),
    _ => Value::Empty
  }
}