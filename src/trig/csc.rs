use crate::*;
use mech_core::*;
use num_traits::*;

// Csc ------------------------------------------------------------------------

use libm::{sin, sinf};
macro_rules! csc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / sin((*$arg).0);}
  };}

macro_rules! csc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / sin(((&(*$arg))[i]).0);
      }}};}

macro_rules! cscf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = 1.0 / sinf((*$arg).0);}
  };}  

macro_rules! cscf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = 1.0 / sinf(((&(*$arg))[i]).0);
      }}};}

impl_math_urop!(MathCsc, F32, cscf);
impl_math_urop!(MathCsc, F64, csc);

fn impl_csc_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathCsc,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCsc {}

impl NativeFunctionCompiler for MathCsc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_csc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_csc_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
