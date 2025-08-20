use crate::*;
use mech_core::*;
use num_traits::*;

// Cot ------------------------------------------------------------------------

use libm::{tan, tanf};
macro_rules! cot_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / tan((*$arg).0);}
  };}

macro_rules! cot_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / tan(((&(*$arg))[i]).0);
      }}};}

macro_rules! cotf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / tanf((*$arg).0);}
  };}  

macro_rules! cotf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / tanf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_urop!(MathCot, F32, cotf);
#[cfg(feature = "f64")]
impl_math_urop!(MathCot, F64, cot);

fn impl_cot_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathCot,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCot {}

impl NativeFunctionCompiler for MathCot {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_cot_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cot_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
