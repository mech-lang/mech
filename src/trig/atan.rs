use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Atan ------------------------------------------------------------------------

use libm::{atan,atanf};
macro_rules! atan_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = atan((*$arg).0);}
  };}

macro_rules! atan_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = atan(((&(*$arg))[i]).0);
      }}};}

macro_rules! atanf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = atanf((*$arg).0);}
  };}  

macro_rules! atanf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = atanf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtan, F32, atanf, FeatureFlag::Custom(hash_str("math/atan")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAtan, F64, atan, FeatureFlag::Custom(hash_str("math/atan")));

fn impl_atan_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAtan,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAtan {}

impl NativeFunctionCompiler for MathAtan {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_atan_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_atan_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}