use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Cbrt ------------------------------------------------------------------------

use libm::{cbrt,cbrtf};
macro_rules! cbrt_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = cbrt((*$arg));}
  };}

macro_rules! cbrt_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = cbrt(((&(*$arg))[i]));
      }}};}

macro_rules! cbrtf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = cbrtf((*$arg));}
  };}  

macro_rules! cbrtf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = cbrtf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathCbrt, f32, cbrtf, FeatureFlag::Custom(hash_str("math/cbrt")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCbrt, f64, cbrt, FeatureFlag::Custom(hash_str("math/cbrt")));

fn impl_cbrt_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathCbrt,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathCbrt {}

impl NativeFunctionCompiler for MathCbrt {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_cbrt_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cbrt_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/cbrt".to_string() },
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
    name: "math/cbrt",
    ptr: &MathCbrt{},
  }
}