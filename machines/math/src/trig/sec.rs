use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sec ------------------------------------------------------------------------

use libm::{cos, cosf};
macro_rules! sec_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = 1.0 / cos((*$arg));}
  };}

macro_rules! sec_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = 1.0 / cos(((&(*$arg))[i]));
      }}};}

macro_rules! secf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = 1.0 / cosf((*$arg));}
  };}  

macro_rules! secf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = 1.0 / cosf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathSec, f32, secf, FeatureFlag::Custom(hash_str("math/sec")));
#[cfg(feature = "f64")]
impl_math_unop!(MathSec, f64, sec, FeatureFlag::Custom(hash_str("math/sec")));

fn impl_sec_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathSec,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathSec {}

impl NativeFunctionCompiler for MathSec {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_sec_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sec_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/sec".to_string() },
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
    name: "math/sec",
    ptr: &MathSec{},
  }
}