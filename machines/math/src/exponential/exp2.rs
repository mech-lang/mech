use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Exp2 ------------------------------------------------------------------------

use libm::{exp2, exp2f};
macro_rules! exp2_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = exp2((*$arg).0);}
  };}

macro_rules! exp2_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = exp2(((&(*$arg))[i]).0);
      }}};}

macro_rules! exp2f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = exp2f((*$arg).0);}
  };}

macro_rules! exp2f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = exp2f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f64")]      
impl_math_unop!(MathExp2, F64, exp2, FeatureFlag::Custom(hash_str("math/exp2")));
#[cfg(feature = "f32")]
impl_math_unop!(MathExp2, F32, exp2f, FeatureFlag::Custom(hash_str("math/exp2")));

fn impl_exp2_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathExp2,
    lhs_value,
    F32 => MatrixF32, F32, F32::default(), "f32";
    F64 => MatrixF64, F64, F64::default(), "f64";
  )
}

pub struct MathExp2 {}

impl NativeFunctionCompiler for MathExp2 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_exp2_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match input {
          Value::MutableReference(input) => impl_exp2_fxn(input.borrow().clone()),
          x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/exp2",
    ptr: &MathExp2{},
  }
}