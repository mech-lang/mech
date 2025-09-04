use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Y0 ------------------------------------------------------------------------

use libm::{y0,y0f};
macro_rules! y0_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = y0((*$arg).0);}
  };}

macro_rules! y0_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = y0(((&(*$arg))[i]).0);
      }}};}

macro_rules! y0f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = y0f((*$arg).0);}
  };}  

macro_rules! y0f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = y0f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathY0, F32, y0f, FeatureFlag::Custom(hash_str("math/y0")));
#[cfg(feature = "f64")]
impl_math_unop!(MathY0, F64, y0, FeatureFlag::Custom(hash_str("math/y0")));

fn impl_y0_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathY0,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathY0 {}

impl NativeFunctionCompiler for MathY0 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_y0_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_y0_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}