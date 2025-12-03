use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Expm1 -----------------------------------------------------------------------

use libm::{expm1, expm1f};
macro_rules! expm1_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = expm1((*$arg));}
  };}

macro_rules! expm1_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = expm1(((&(*$arg))[i]));
      }}};}

macro_rules! expm1f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = expm1f((*$arg));}
  };}

macro_rules! expm1f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = expm1f(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f64")]      
impl_math_unop!(MathExpm1, f64, expm1, FeatureFlag::Custom(hash_str("math/expm1")));
#[cfg(feature = "f32")]
impl_math_unop!(MathExpm1, f32, expm1f, FeatureFlag::Custom(hash_str("math/expm1")));

fn impl_expm1_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathExpm1,
    lhs_value,
    F32 => MatrixF32, F32, f32::default(), "f32";
    F64 => MatrixF64, F64, f64::default(), "f64";
  )
}

pub struct MathExpm1 {}

impl NativeFunctionCompiler for MathExpm1 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_expm1_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match input {
          Value::MutableReference(input) => impl_expm1_fxn(input.borrow().clone()),
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/expm1".to_string() },
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
    name: "math/expm1",
    ptr: &MathExpm1{},
  }
}