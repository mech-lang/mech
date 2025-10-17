use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// J1 ------------------------------------------------------------------------

use libm::{j1,j1f};
macro_rules! j1_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = j1((*$arg).0);}
  };}

macro_rules! j1_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = j1(((&(*$arg))[i]).0);
      }}};}

macro_rules! j1f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = j1f((*$arg).0);}
  };}  

macro_rules! j1f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = j1f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathJ1, F32, j1f, FeatureFlag::Custom(hash_str("math/j1")));
#[cfg(feature = "f64")]
impl_math_unop!(MathJ1, F64, j1, FeatureFlag::Custom(hash_str("math/j1")));

fn impl_j1_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathJ1,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathJ1 {}

impl NativeFunctionCompiler for MathJ1 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_j1_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_j1_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/bessel/j1",
    ptr: &MathJ1{},
  }
}