use crate::*;
use mech_core::*;

// Tan ------------------------------------------------------------------------

use libm::{tan,tanf};
macro_rules! tan_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = tan((*$arg).0);}
  };}

macro_rules! tan_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = tan(((&(*$arg))[i]).0);
      }}};}

macro_rules! tanf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = tanf((*$arg).0);}
  };}  

macro_rules! tanf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = tanf(((&(*$arg))[i]).0);
      }}};}

impl_math_urop!(MathTan, F32, tanf);
impl_math_urop!(MathTan, F64, tan);

fn impl_tan_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathTan,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathTan {}

impl NativeFunctionCompiler for MathTan {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_tan_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_tan_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}