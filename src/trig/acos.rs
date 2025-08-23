use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Acos ------------------------------------------------------------------------

use libm::{acos,acosf};
macro_rules! acos_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = acos((*$arg).0);}
  };}

macro_rules! acos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = acos(((&(*$arg))[i]).0);
      }}};}

macro_rules! acosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = acosf((*$arg).0);}
  };}  

macro_rules! acosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = acosf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_urop!(MathAcos, F32, acosf);
#[cfg(feature = "f64")]
impl_math_urop!(MathAcos, F64, acos);

fn impl_acos_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAcos,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAcos {}

impl NativeFunctionCompiler for MathAcos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_acos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_acos_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}