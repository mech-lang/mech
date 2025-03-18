use crate::*;
use mech_core::*;

// Sincos ------------------------------------------------------------------------

use libm::{sincos,sincosf};
macro_rules! sincos_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sincos((*$arg).0);}
  };}

macro_rules! sincos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = sincos(((*$arg)[i]).0);
      }}};}

macro_rules! sincosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = sincosf((*$arg).0);}
  };}  

macro_rules! sincosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = sincosf(((*$arg)[i]).0);
      }}};}

impl_math_urop!(MathSincos, F32, sincosf);
impl_math_urop!(MathSincos, F64, sincos);


fn impl_sincos_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathSincos,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
  )
}

pub struct MathSincos {}

impl NativeFunctionCompiler for MathSincos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_sincos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_sincos_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}