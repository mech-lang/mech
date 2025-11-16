use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Lgamma ------------------------------------------------------------------------

use libm::{lgamma,lgammaf};
macro_rules! lgamma_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = lgamma((*$arg).0);}
  };}

macro_rules! lgamma_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = lgamma(((&(*$arg))[i]).0);
      }}};}

macro_rules! lgammaf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = lgammaf((*$arg).0);}
  };}  

macro_rules! lgammaf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = lgammaf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathLgamma, F32, lgammaf, FeatureFlag::Custom(hash_str("math/lgamma")));
#[cfg(feature = "f64")]
impl_math_unop!(MathLgamma, F64, lgamma, FeatureFlag::Custom(hash_str("math/lgamma")));

fn impl_lgamma_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathLgamma,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathLgamma {}

impl NativeFunctionCompiler for MathLgamma {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_lgamma_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_lgamma_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/lgamma",
    ptr: &MathLgamma{},
  }
}