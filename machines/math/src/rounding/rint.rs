use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Rint ------------------------------------------------------------------------

use libm::{rint,rintf};
macro_rules! rint_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = rint((*$arg).0);}
  };}

macro_rules! rint_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = rint(((&(*$arg))[i]).0);
      }}};}

macro_rules! rintf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = rintf((*$arg).0);}
  };}  

macro_rules! rintf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = rintf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathRint, F32, rintf, FeatureFlag::Custom(hash_str("math/rint")));
#[cfg(feature = "f64")]
impl_math_unop!(MathRint, F64, rint, FeatureFlag::Custom(hash_str("math/rint")));

fn impl_rint_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathRint,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathRint {}

impl NativeFunctionCompiler for MathRint {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_rint_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_rint_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/rint".to_string() },
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
    name: "math/rint",
    ptr: &MathRint{},
  }
}