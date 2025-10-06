use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Log2 ------------------------------------------------------------------------

use libm::{log2,log2f};
macro_rules! log2_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log2((*$arg).0);}
  };}

macro_rules! log2_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log2(((&(*$arg))[i]).0);
      }}};}

macro_rules! log2f_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log2f((*$arg).0);}
  };}  

macro_rules! log2f_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log2f(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog2, F32, log2f, FeatureFlag::Custom(hash_str("math/log2")));
#[cfg(feature = "f64")]
impl_math_unop!(MathLog2, F64, log2, FeatureFlag::Custom(hash_str("math/log2")));

fn impl_log2_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathLog2,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathLog2 {}

impl NativeFunctionCompiler for MathLog2 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_log2_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_log2_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompiler {
    name: "math/log2",
    ptr: &MathLog2{},
  }
}