use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Abs ------------------------------------------------------------------------

use libm::{fabs,fabsf};


macro_rules! abs_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = (*$arg).abs();}
  };}

macro_rules! abs_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        (&mut (*$out))[i] =  (&(*$arg))[i].abs();
      }}};}

macro_rules! fabs_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = fabs((*$arg).0);}
  };}

macro_rules! fabs_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = fabs(((&(*$arg))[i]).0);
      }}};}

macro_rules! fabsf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = fabsf((*$arg).0);}
  };}  

macro_rules! fabsf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = fabsf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "i64")]
impl_math_unop!(MathAbs, i64, abs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "f32")]
impl_math_unop!(MathAbs, F32, fabsf, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAbs, F64, fabs, FeatureFlag::Custom(hash_str("math/abs")));

fn impl_abs_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAbs,
    (lhs_value),
    I64 => MatrixI64, i64, i64::zero(), "i64";
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAbs {}

impl NativeFunctionCompiler for MathAbs {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_abs_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_abs_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompiler {
    name: "math/abs",
    ptr: &MathAbs{},
  }
}