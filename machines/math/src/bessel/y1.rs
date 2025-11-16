use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Y1 ------------------------------------------------------------------------

use libm::{y1,y1f};
macro_rules! y1_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = y1((*$arg).0);}
  };}

macro_rules! y1_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = y1(((&(*$arg))[i]).0);
      }}};}

macro_rules! y1f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = y1f((*$arg).0);}
  };}  

macro_rules! y1f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = y1f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathY1, F32, y1f, FeatureFlag::Custom(hash_str("math/y1")));
#[cfg(feature = "f64")]
impl_math_unop!(MathY1, F64, y1, FeatureFlag::Custom(hash_str("math/y1")));

fn impl_y1_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathY1,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathY1 {}

impl NativeFunctionCompiler for MathY1 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_y1_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_y1_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x, fxn_name: "math/bessel/y1".to_string() },
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
    name: "math/bessel/y1",
    ptr: &MathY1{},
  }
}