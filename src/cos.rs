use crate::*;
use mech_core::*;

// Cos ------------------------------------------------------------------------

use libm::{cos,cosf};
macro_rules! cos_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cos((*$arg).0);}
  };}

macro_rules! cos_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = cos(((*$arg)[i]).0);
      }}};}

macro_rules! cosf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = cosf((*$arg).0);}
  };}  

macro_rules! cosf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = cosf(((*$arg)[i]).0);
      }}};}

impl_math_urop!(MathCos, F32, cosf);
impl_math_urop!(MathCos, F64, cos);


fn impl_cos_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathCos,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
  )
}

pub struct MathCos {}

impl NativeFunctionCompiler for MathCos {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_cos_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_cos_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}