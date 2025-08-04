use crate::*;
use mech_core::*;

// Asec ------------------------------------------------------------------------

use libm::{acos, acosf};
macro_rules! asec_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = acos(1.0 / (*$arg).0);}
  };}

macro_rules! asec_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = acos(1.0 / ((&(*$arg))[i]).0);
      }}};}

macro_rules! asecf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = acosf(1.0 / (*$arg).0);}
  };}  

macro_rules! asecf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = acosf(1.0 / ((&(*$arg))[i]).0);
      }}};}

impl_math_urop!(MathAsec, F32, asecf);
impl_math_urop!(MathAsec, F64, asec);

fn impl_asec_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAsec,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
  )
}

pub struct MathAsec {}

impl NativeFunctionCompiler for MathAsec {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_asec_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_asec_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
