use crate::*;
use mech_core::*;

// Asin ------------------------------------------------------------------------

use libm::{asin,asinf};
macro_rules! asin_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = asin((*$arg).0);}
  };}

macro_rules! asin_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = asin(((*$arg)[i]).0);
      }}};}

macro_rules! asinf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = asinf((*$arg).0);}
  };}  

macro_rules! asinf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = asinf(((*$arg)[i]).0);
      }}};}

impl_math_urop!(MathAsin, F32, asinf);
impl_math_urop!(MathAsin, F64, asin);

fn impl_asin_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAsin,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
  )
}

pub struct MathAsin {}

impl NativeFunctionCompiler for MathAsin {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_asin_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_asin_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}