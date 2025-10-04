use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Log1p ------------------------------------------------------------------------

use libm::{log1p,log1pf};
macro_rules! log1p_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log1p((*$arg).0);}
  };}

macro_rules! log1p_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log1p(((&(*$arg))[i]).0);
      }}};}

macro_rules! log1pf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log1pf((*$arg).0);}
  };}  

macro_rules! log1pf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log1pf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog1p, F32, log1pf, FeatureFlag::Custom(hash_str("math/log1p")));
#[cfg(feature = "f64")]
impl_math_unop!(MathLog1p, F64, log1p, FeatureFlag::Custom(hash_str("math/log1p")));

fn impl_log1p_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathLog1p,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathLog1p {}

impl NativeFunctionCompiler for MathLog1p {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_log1p_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_log1p_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}