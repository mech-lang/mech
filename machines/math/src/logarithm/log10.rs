use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Log10 ------------------------------------------------------------------------

use libm::{log10,log10f};
macro_rules! log10_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log10((*$arg).0);}
  };}

macro_rules! log10_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log10(((&(*$arg))[i]).0);
      }}};}

macro_rules! log10f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log10f((*$arg).0);}
  };}  

macro_rules! log10f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log10f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog10, F32, log10f, FeatureFlag::Custom(hash_str("math/log10")));
#[cfg(feature = "f64")]
impl_math_unop!(MathLog10, F64, log10, FeatureFlag::Custom(hash_str("math/log10")));

fn impl_log10_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathLog10,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathLog10 {}

impl NativeFunctionCompiler for MathLog10 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_log10_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_log10_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/log10".to_string() },
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
    name: "math/log10",
    ptr: &MathLog10{},
  }
}