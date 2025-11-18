use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Acsc ------------------------------------------------------------------------

use libm::{asin, asinf};
macro_rules! acsc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = asin(1.0 / (*$arg).0);}
  };}

macro_rules! acsc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = asin(1.0 / ((&(*$arg))[i]).0);
      }}};}

macro_rules! acscf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = asinf(1.0 / (*$arg).0);}
  };}  

macro_rules! acscf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = asinf(1.0 / ((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]      
impl_math_unop!(MathAcsc, F32, acscf, FeatureFlag::Custom(hash_str("math/acsc")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAcsc, F64, acsc, FeatureFlag::Custom(hash_str("math/acsc")));

fn impl_acsc_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathAcsc,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAcsc {}

impl NativeFunctionCompiler for MathAcsc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_acsc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_acsc_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/acsc".to_string() },
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
    name: "math/acsc",
    ptr: &MathAcsc{},
  }
}