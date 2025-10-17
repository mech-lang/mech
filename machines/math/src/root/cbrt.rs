use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Cbrt ------------------------------------------------------------------------

use libm::{cbrt,cbrtf};
macro_rules! cbrt_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cbrt((*$arg).0);}
  };}

macro_rules! cbrt_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = cbrt(((&(*$arg))[i]).0);
      }}};}

macro_rules! cbrtf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cbrtf((*$arg).0);}
  };}  

macro_rules! cbrtf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = cbrtf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathCbrt, F32, cbrtf, FeatureFlag::Custom(hash_str("math/cbrt")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCbrt, F64, cbrt, FeatureFlag::Custom(hash_str("math/cbrt")));

fn impl_cbrt_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathCbrt,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCbrt {}

impl NativeFunctionCompiler for MathCbrt {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_cbrt_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cbrt_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/cbrt",
    ptr: &MathCbrt{},
  }
}