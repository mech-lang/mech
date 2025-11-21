use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Trunc ------------------------------------------------------------------------

use libm::{trunc,truncf};
macro_rules! trunc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = trunc((*$arg));}
  };}

macro_rules! trunc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = trunc(((&(*$arg))[i]));
      }}};}

macro_rules! truncf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = truncf((*$arg));}
  };}  

macro_rules! truncf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = truncf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathTrunc, f32, truncf, FeatureFlag::Custom(hash_str("math/trunc")));
#[cfg(feature = "f64")]
impl_math_unop!(MathTrunc, f64, trunc, FeatureFlag::Custom(hash_str("math/trunc")));

fn impl_trunc_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathTrunc,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathTrunc {}

impl NativeFunctionCompiler for MathTrunc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_trunc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_trunc_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/trunc".to_string() },
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
    name: "math/trunc",
    ptr: &MathTrunc{},
  }
}