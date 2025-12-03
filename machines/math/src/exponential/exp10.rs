use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Exp10 -----------------------------------------------------------------------

use libm::{exp10, exp10f};
macro_rules! exp10_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = exp10((*$arg));}
  };}

macro_rules! exp10_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = exp10(((&(*$arg))[i]));
      }}};}

macro_rules! exp10f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = exp10f((*$arg));}
  };}

macro_rules! exp10f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = exp10f(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f64")]      
impl_math_unop!(MathExp10, f64, exp10, FeatureFlag::Custom(hash_str("math/exp10")));
#[cfg(feature = "f32")]
impl_math_unop!(MathExp10, f32, exp10f, FeatureFlag::Custom(hash_str("math/exp10")));

fn impl_exp10_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathExp10,
    lhs_value,
    F32 => MatrixF32, F32, f32::default(), "f32";
    F64 => MatrixF64, F64, f64::default(), "f64";
  )
}

pub struct MathExp10 {}

impl NativeFunctionCompiler for MathExp10 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_exp10_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match input {
          Value::MutableReference(input) => impl_exp10_fxn(input.borrow().clone()),
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/exp10".to_string() },
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
    name: "math/exp10",
    ptr: &MathExp10{},
  }
}