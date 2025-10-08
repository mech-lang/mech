use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Acot ------------------------------------------------------------------------

use libm::{atan, atanf};
macro_rules! acot_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = atan(1.0 / (*$arg).0);}
  };}

macro_rules! acot_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = atan(1.0 / ((&(*$arg))[i]).0);
      }}};}

macro_rules! acotf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = atanf(1.0 / (*$arg).0);}
  };}  

macro_rules! acotf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = atanf(1.0 / ((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcot, F32, acotf, FeatureFlag::Custom(hash_str("math/acot")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAcot, F64, acot, FeatureFlag::Custom(hash_str("math/acot")));

fn impl_acot_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAcot,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAcot {}

impl NativeFunctionCompiler for MathAcot {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_acot_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_acot_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/acot",
    ptr: &MathAcot{},
  }
}