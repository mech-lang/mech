use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Exp10 -----------------------------------------------------------------------

use libm::{exp10, exp10f};
macro_rules! exp10_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = exp10((*$arg).0);}
  };}

macro_rules! exp10_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = exp10(((&(*$arg))[i]).0);
      }}};}

macro_rules! exp10f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = exp10f((*$arg).0);}
  };}

macro_rules! exp10f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = exp10f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f64")]      
impl_math_unop!(MathExp10, F64, exp10, FeatureFlag::Custom(hash_str("math/exp10")));
#[cfg(feature = "f32")]
impl_math_unop!(MathExp10, F32, exp10f, FeatureFlag::Custom(hash_str("math/exp10")));

fn impl_exp10_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathExp10,
    lhs_value,
    F32 => MatrixF32, F32, F32::default(), "f32";
    F64 => MatrixF64, F64, F64::default(), "f64";
  )
}

pub struct MathExp10 {}

impl NativeFunctionCompiler for MathExp10 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_exp10_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match input {
          Value::MutableReference(input) => impl_exp10_fxn(input.borrow().clone()),
          x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
    }
  }
}