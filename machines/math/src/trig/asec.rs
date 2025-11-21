use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Asec ------------------------------------------------------------------------

use libm::{acos, acosf};
macro_rules! asec_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = acos(1.0 / (*$arg));}
  };}

macro_rules! asec_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = acos(1.0 / ((&(*$arg))[i]));
      }}};}

macro_rules! asecf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = acosf(1.0 / (*$arg));}
  };}  

macro_rules! asecf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = acosf(1.0 / ((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathAsec, f32, asecf, FeatureFlag::Custom(hash_str("math/asec")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAsec, f64, asec, FeatureFlag::Custom(hash_str("math/asec")));

fn impl_asec_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathAsec,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathAsec {}

impl NativeFunctionCompiler for MathAsec {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_asec_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_asec_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/asec".to_string() },
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
    name: "math/asec",
    ptr: &MathAsec{},
  }
}