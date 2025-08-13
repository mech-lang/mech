use crate::*;
use mech_core::*;

// Acsc ------------------------------------------------------------------------

use libm::{asin, asinf};
macro_rules! acsc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = asin(1.0 / (*$arg).0);}
  };}

macro_rules! acsc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = asin(1.0 / ((&(*$arg))[i]).0);
      }}};}

macro_rules! acscf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = asinf(1.0 / (*$arg).0);}
  };}  

macro_rules! acscf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = asinf(1.0 / ((&(*$arg))[i]).0);
      }}};}

impl_math_urop!(MathAcsc, F32, acscf);
impl_math_urop!(MathAcsc, F64, acsc);

fn impl_acsc_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAcsc,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAcsc {}

impl NativeFunctionCompiler for MathAcsc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_acsc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_acsc_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
