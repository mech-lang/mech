use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Round ------------------------------------------------------------------------

use libm::{round,roundf};
macro_rules! round_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = round((*$arg).0);}
  };}

macro_rules! round_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = round(((&(*$arg))[i]).0);
      }}};}

macro_rules! roundf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = roundf((*$arg).0);}
  };}  

macro_rules! roundf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = roundf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathRound, F32, roundf, FeatureFlag::Custom(hash_str("math/round")));
#[cfg(feature = "f64")]
impl_math_unop!(MathRound, F64, round, FeatureFlag::Custom(hash_str("math/round")));

fn impl_round_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathRound,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathRound {}

impl NativeFunctionCompiler for MathRound {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_round_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_round_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompiler {
    name: "math/round",
    ptr: &MathRound{},
  }
}