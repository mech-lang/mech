use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// J0 ------------------------------------------------------------------------

use libm::{j0,j0f};
macro_rules! j0_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = j0((*$arg).0);}
  };}

macro_rules! j0_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = j0(((&(*$arg))[i]).0);
      }}};}

macro_rules! j0f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = j0f((*$arg).0);}
  };}  

macro_rules! j0f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = j0f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathJ0, F32, j0f, FeatureFlag::Custom(hash_str("math/j0")));
#[cfg(feature = "f64")]
impl_math_unop!(MathJ0, F64, j0, FeatureFlag::Custom(hash_str("math/j0")));

fn impl_j0_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathJ0,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathJ0 {}

impl NativeFunctionCompiler for MathJ0 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_j0_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_j0_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/bessel/j0",
    ptr: &MathJ0{},
  }
}