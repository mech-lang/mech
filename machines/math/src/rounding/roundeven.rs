use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Roundeven ------------------------------------------------------------------------

use libm::{roundeven,roundevenf};
macro_rules! roundeven_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = roundeven((*$arg).0);}
  };}

macro_rules! roundeven_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = roundeven(((&(*$arg))[i]).0);
      }}};}

macro_rules! roundevenf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = roundevenf((*$arg).0);}
  };}  

macro_rules! roundevenf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = roundevenf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathRoundeven, F32, roundevenf, FeatureFlag::Custom(hash_str("math/roundeven")));
#[cfg(feature = "f64")]
impl_math_unop!(MathRoundeven, F64, roundeven, FeatureFlag::Custom(hash_str("math/roundeven")));

fn impl_roundeven_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathRoundeven,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathRoundeven {}

impl NativeFunctionCompiler for MathRoundeven {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let input = arguments[0].clone();
    match impl_roundeven_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_roundeven_fxn(input.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/roundeven",
    ptr: &MathRoundeven{},
  }
}