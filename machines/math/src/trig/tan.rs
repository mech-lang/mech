use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Tan ------------------------------------------------------------------------

use libm::{tan,tanf};
macro_rules! tan_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = tan((*$arg));}
  };}

macro_rules! tan_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = tan(((&(*$arg))[i]));
      }}};}

macro_rules! tanf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = tanf((*$arg));}
  };}  

macro_rules! tanf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = tanf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]      
impl_math_unop!(MathTan, f32, tanf, FeatureFlag::Custom(hash_str("math/tan")));
#[cfg(feature = "f64")]
impl_math_unop!(MathTan, f64, tan, FeatureFlag::Custom(hash_str("math/tan")));

fn impl_tan_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathTan,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathTan {}

impl NativeFunctionCompiler for MathTan {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_tan_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_tan_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/tan".to_string() },
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
    name: "math/tan",
    ptr: &MathTan{},
  }
}