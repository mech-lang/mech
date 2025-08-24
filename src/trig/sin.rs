use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sin ------------------------------------------------------------------------

use libm::{sin,sinf};
macro_rules! sin_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sin((*$arg).0);}
  };}

macro_rules! sin_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sin(((&(*$arg))[i]).0);
      }}};}

macro_rules! sinf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sinf((*$arg).0);}
  };}  

macro_rules! sinf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sinf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]      
impl_math_unop!(MathSin, F32, sinf);
#[cfg(feature = "f64")]
impl_math_unop!(MathSin, F64, sin);

fn impl_sin_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathSin,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathSin {}

impl NativeFunctionCompiler for MathSin {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_sin_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sin_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}