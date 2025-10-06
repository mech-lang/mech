use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Trunc ------------------------------------------------------------------------

use libm::{trunc,truncf};
macro_rules! trunc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = trunc((*$arg).0);}
  };}

macro_rules! trunc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = trunc(((&(*$arg))[i]).0);
      }}};}

macro_rules! truncf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = truncf((*$arg).0);}
  };}  

macro_rules! truncf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = truncf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathTrunc, F32, truncf, FeatureFlag::Custom(hash_str("math/trunc")));
#[cfg(feature = "f64")]
impl_math_unop!(MathTrunc, F64, trunc, FeatureFlag::Custom(hash_str("math/trunc")));

fn impl_trunc_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathTrunc,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathTrunc {}

impl NativeFunctionCompiler for MathTrunc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_trunc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_trunc_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompiler {
    name: "math/trunc",
    ptr: &MathTrunc{},
  }
}