use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Round ------------------------------------------------------------------------

use libm::{round,roundf};
macro_rules! round_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = round((*$arg));}
  };}

macro_rules! round_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = round(((&(*$arg))[i]));
      }}};}

macro_rules! roundf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = roundf((*$arg));}
  };}  

macro_rules! roundf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = roundf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathRound, f32, roundf, FeatureFlag::Custom(hash_str("math/round")));
#[cfg(feature = "f64")]
impl_math_unop!(MathRound, f64, round, FeatureFlag::Custom(hash_str("math/round")));

fn impl_round_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathRound,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathRound {}

impl NativeFunctionCompiler for MathRound {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_round_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_round_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/round".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/round",
    ptr: &MathRound{},
  }
}