use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Cot ------------------------------------------------------------------------

use libm::{tan, tanf};
macro_rules! cot_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = 1.0 / tan((*$arg));}
  };}

macro_rules! cot_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = 1.0 / tan(((&(*$arg))[i]));
      }}};}

macro_rules! cotf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = 1.0 / tanf((*$arg));}
  };}  

macro_rules! cotf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = 1.0 / tanf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathCot, f32, cotf, FeatureFlag::Custom(hash_str("math/cot")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCot, f64, cot, FeatureFlag::Custom(hash_str("math/cot")));

fn impl_cot_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathCot,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathCot {}

impl NativeFunctionCompiler for MathCot {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_cot_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cot_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/cot".to_string() },
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
    name: "math/cot",
    ptr: &MathCot{},
  }
}