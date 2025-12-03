use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Log1p ------------------------------------------------------------------------

use libm::{log1p,log1pf};
macro_rules! log1p_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = log1p((*$arg));}
  };}

macro_rules! log1p_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = log1p(((&(*$arg))[i]));
      }}};}

macro_rules! log1pf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = log1pf((*$arg));}
  };}  

macro_rules! log1pf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = log1pf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog1p, f32, log1pf, FeatureFlag::Custom(hash_str("math/log1p")));
#[cfg(feature = "f64")]
impl_math_unop!(MathLog1p, f64, log1p, FeatureFlag::Custom(hash_str("math/log1p")));

fn impl_log1p_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathLog1p,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathLog1p {}

impl NativeFunctionCompiler for MathLog1p {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_log1p_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_log1p_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/log1p".to_string() },
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
    name: "math/log1p",
    ptr: &MathLog1p{},
  }
}