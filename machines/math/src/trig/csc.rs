use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Csc ------------------------------------------------------------------------

use libm::{sin, sinf};
macro_rules! csc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / sin((*$arg).0);}
  };}

macro_rules! csc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / sin(((&(*$arg))[i]).0);
      }}};}

macro_rules! cscf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / sinf((*$arg).0);}
  };}  

macro_rules! cscf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / sinf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathCsc, F32, cscf, FeatureFlag::Custom(hash_str("math/csc")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCsc, F64, csc, FeatureFlag::Custom(hash_str("math/csc")));

fn impl_csc_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathCsc,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCsc {}

impl NativeFunctionCompiler for MathCsc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_csc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_csc_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x, fxn_name: "math/csc".to_string() },
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
    name: "math/csc",
    ptr: &MathCsc{},
  }
}