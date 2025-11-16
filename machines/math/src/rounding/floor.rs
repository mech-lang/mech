use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Floor ------------------------------------------------------------------------

use libm::{floor,floorf};
macro_rules! floor_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = floor((*$arg).0);}
  };}

macro_rules! floor_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = floor(((&(*$arg))[i]).0);
      }}};}

macro_rules! floorf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = floorf((*$arg).0);}
  };}  

macro_rules! floorf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = floorf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathFloor, F32, floorf, FeatureFlag::Custom(hash_str("math/floor")));
#[cfg(feature = "f64")]
impl_math_unop!(MathFloor, F64, floor, FeatureFlag::Custom(hash_str("math/floor")));

fn impl_floor_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathFloor,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathFloor {}

impl NativeFunctionCompiler for MathFloor {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_floor_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_floor_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.clone(), fxn_name: "math/floor".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/floor",
    ptr: &MathFloor{},
  }
}