use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Y0 ------------------------------------------------------------------------

use libm::{y0,y0f};
macro_rules! y0_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = y0((*$arg));}
  };}

macro_rules! y0_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = y0(((&(*$arg))[i]));
      }}};}

macro_rules! y0f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = y0f((*$arg));}
  };}  

macro_rules! y0f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = y0f(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathY0, f32, y0f, FeatureFlag::Custom(hash_str("math/y0")));
#[cfg(feature = "f64")]
impl_math_unop!(MathY0, f64, y0, FeatureFlag::Custom(hash_str("math/y0")));

fn impl_y0_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathY0,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathY0 {}

impl NativeFunctionCompiler for MathY0 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_y0_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_y0_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/bessel/y0".to_string() },
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
    name: "math/bessel/y0",
    ptr: &MathY0{},
  }
}