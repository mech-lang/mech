use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Exponential -----------------------------------------------------------------

use libm::{exp, expf};
macro_rules! exponential_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = exp((*$arg).0);}
  };}

macro_rules! exponential_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = exp(((&(*$arg))[i]).0);
      }}};}

macro_rules! exponentialf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = expf((*$arg).0);}
  };}

macro_rules! exponentialf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = expf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f64")]
impl_math_unop!(MathExponential, F64, exponential, FeatureFlag::Custom(hash_str("math/exponential")));
#[cfg(feature = "f32")]
impl_math_unop!(MathExponential, F32, exponentialf, FeatureFlag::Custom(hash_str("math/exponential")));

fn impl_exponential_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathExponential,
    lhs_value,
    F32 => MatrixF32, F32, F32::default(), "f32";
    F64 => MatrixF64, F64, F64::default(), "f64";
  )
}

pub struct MathExponential {}

impl NativeFunctionCompiler for MathExponential {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_exponential_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match input {
          Value::MutableReference(input) => impl_exponential_fxn(input.borrow().clone()),
          x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/exponential",
    ptr: &MathExponential{},
  }
}