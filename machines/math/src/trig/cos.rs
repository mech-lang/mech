use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Cos ------------------------------------------------------------------------

use libm::{cos,cosf};
macro_rules! cos_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cos((*$arg).0);}
  };}

macro_rules! cos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = cos(((&(*$arg))[i]).0);
      }}};}

macro_rules! cosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cosf((*$arg).0);}
  };}  

macro_rules! cosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = cosf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathCos, F32, cosf, FeatureFlag::Custom(hash_str("math/cos")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCos, F64, cos, FeatureFlag::Custom(hash_str("math/cos")));

fn impl_cos_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathCos,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCos {}

impl NativeFunctionCompiler for MathCos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_cos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cos_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/cos".to_string() },
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
    name: "math/cos",
    ptr: &MathCos{},
  }
}