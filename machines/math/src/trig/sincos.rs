use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sincos ------------------------------------------------------------------------

use libm::{sincos,sincosf};
macro_rules! sincos_op {
  ($arg:expr, $out1:expr, $out2:expr) => {
    unsafe{(*$out1, *$out2) = sincos((*$arg));}
  };}

macro_rules! sincos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = sincos(((&(*$arg))[i]));
      }}};}

macro_rules! sincosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = sincosf((*$arg));}
  };}  

macro_rules! sincosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = sincosf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]      
impl_math_unop!(MathSincos, f32, sincosf, FeatureFlag::Custom(hash_str("math/sincos")));
#[cfg(feature = "f64")]
impl_math_unop!(MathSincos, f64, sincos, FeatureFlag::Custom(hash_str("math/sincos")));

fn impl_sincos_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathSincos,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathSincos {}

impl NativeFunctionCompiler for MathSincos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_sincos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sincos_fxn(input.borrow().clone())}
          (arg1,arg2) => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: "math/sincos".to_string() },
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
    name: "math/sincos",
    ptr: &MathSincos{},
  }
}