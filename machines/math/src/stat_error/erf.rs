use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Erf ------------------------------------------------------------------------

use libm::{erf,erff};
macro_rules! erf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = erf((*$arg).0);}
  };}

macro_rules! erf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = erf(((&(*$arg))[i]).0);
      }}};}

macro_rules! erff_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = erff((*$arg).0);}
  };}  

macro_rules! erff_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = erff(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathErf, F32, erff, FeatureFlag::Custom(hash_str("math/erf")));
#[cfg(feature = "f64")]
impl_math_unop!(MathErf, F64, erf, FeatureFlag::Custom(hash_str("math/erf")));

fn impl_erf_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathErf,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathErf {}

impl NativeFunctionCompiler for MathErf {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_erf_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_erf_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.clone(), fxn_name: "math/erf".to_string() },
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
    name: "math/erf",
    ptr: &MathErf{},
  }
}