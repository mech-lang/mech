use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Ilogb ------------------------------------------------------------------------

use libm::{ilogb,ilogbf};
macro_rules! ilogb_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = ilogb((*$arg));}
  };}

macro_rules! ilogb_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = ilogb(((&(*$arg))[i]));
      }}};}

macro_rules! ilogbf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = ilogbf((*$arg));}
  };}  

macro_rules! ilogbf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = ilogbf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathIlogb, F32, ilogbf, FeatureFlag::Custom(hash_str("math/ilogb")));
#[cfg(feature = "f64")]
impl_math_unop!(MathIlogb, F64, ilogb, FeatureFlag::Custom(hash_str("math/ilogb")));

fn impl_ilogb_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathIlogb,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathIlogb {}

impl NativeFunctionCompiler for MathIlogb {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_ilogb_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_ilogb_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompiler {
    name: "math/ilogb",
    ptr: &MathIlogb{},
  }
}