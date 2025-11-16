use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sincos ------------------------------------------------------------------------

use libm::{sincos,sincosf};
macro_rules! sincos_op {
  ($arg:expr, $out1:expr, $out2:expr) => {
    unsafe{(*$out1, *$out2) = sincos((*$arg).0);}
  };}

macro_rules! sincos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sincos(((&(*$arg))[i]).0);
      }}};}

macro_rules! sincosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sincosf((*$arg).0);}
  };}  

macro_rules! sincosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sincosf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]      
impl_math_unop!(MathSincos, F32, sincosf, FeatureFlag::Custom(hash_str("math/sincos")));
#[cfg(feature = "f64")]
impl_math_unop!(MathSincos, F64, sincos, FeatureFlag::Custom(hash_str("math/sincos")));

fn impl_sincos_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathSincos,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathSincos {}

impl NativeFunctionCompiler for MathSincos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_sincos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sincos_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/sincos",
    ptr: &MathSincos{},
  }
}