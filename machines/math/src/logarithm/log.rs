use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Log ------------------------------------------------------------------------

use libm::{log,logf};
macro_rules! log_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = log((*$arg).0);}
  };}

macro_rules! log_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = log(((&(*$arg))[i]).0);
      }}};}

macro_rules! logf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = logf((*$arg).0);}
  };}  

macro_rules! logf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = logf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathLog, F32, logf, FeatureFlag::Custom(hash_str("math/log")));
#[cfg(feature = "f64")]
impl_math_unop!(MathLog, F64, log, FeatureFlag::Custom(hash_str("math/log")));

fn impl_log_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathLog,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathLog {}

impl NativeFunctionCompiler for MathLog {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_log_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_log_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompilerDescriptor {
    name: "math/log",
    ptr: &MathLog{},
  }
}