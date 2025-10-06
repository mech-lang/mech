use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Ceil ------------------------------------------------------------------------

use libm::{ceil,ceilf};
macro_rules! ceil_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = ceil((*$arg).0);}
  };}

macro_rules! ceil_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = ceil(((&(*$arg))[i]).0);
      }}};}

macro_rules! ceilf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = ceilf((*$arg).0);}
  };}  

macro_rules! ceilf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = ceilf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathCeil, F32, ceilf, FeatureFlag::Custom(hash_str("math/ceil")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCeil, F64, ceil, FeatureFlag::Custom(hash_str("math/ceil")));

fn impl_ceil_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathCeil,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCeil {}

impl NativeFunctionCompiler for MathCeil {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_ceil_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_ceil_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompilerDescriptor {
    name: "math/ceil",
    ptr: &MathCeil{},
  }
}