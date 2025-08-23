use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sec ------------------------------------------------------------------------

use libm::{cos, cosf};
macro_rules! sec_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / cos((*$arg).0);}
  };}

macro_rules! sec_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / cos(((&(*$arg))[i]).0);
      }}};}

macro_rules! secf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / cosf((*$arg).0);}
  };}  

macro_rules! secf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / cosf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_urop!(MathSec, F32, secf);
#[cfg(feature = "f64")]
impl_math_urop!(MathSec, F64, sec);

fn impl_sec_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathSec,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathSec {}

impl NativeFunctionCompiler for MathSec {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_sec_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sec_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
