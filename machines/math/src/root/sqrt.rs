use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sqrt ------------------------------------------------------------------------

use libm::{sqrt,sqrtf};
macro_rules! sqrt_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sqrt((*$arg).0);}
  };}

macro_rules! sqrt_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sqrt(((&(*$arg))[i]).0);
      }}};}

macro_rules! sqrtf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sqrtf((*$arg).0);}
  };}  

macro_rules! sqrtf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sqrtf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathSqrt, F32, sqrtf, FeatureFlag::Custom(hash_str("math/sqrt")));
#[cfg(feature = "f64")]
impl_math_unop!(MathSqrt, F64, sqrt, FeatureFlag::Custom(hash_str("math/sqrt")));

fn impl_sqrt_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathSqrt,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathSqrt {}

impl NativeFunctionCompiler for MathSqrt {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_sqrt_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sqrt_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.clone(), fxn_name: "math/sqrt".to_string() },
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
    name: "math/sqrt",
    ptr: &MathSqrt{},
  }
}