use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Atan ------------------------------------------------------------------------

use libm::{atan,atanf};
macro_rules! atan_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = atan((*$arg).0);}
  };}

macro_rules! atan_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = atan(((&(*$arg))[i]).0);
      }}};}

macro_rules! atanf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = atanf((*$arg).0);}
  };}  

macro_rules! atanf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = atanf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathAtan, F32, atanf, FeatureFlag::Custom(hash_str("math/atan")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAtan, F64, atan, FeatureFlag::Custom(hash_str("math/atan")));

fn impl_atan_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathAtan,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAtan {}

impl NativeFunctionCompiler for MathAtan {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() == 1 {
      let input = arguments[0].clone();
      match impl_atan_fxn(input.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (input) {
            (Value::MutableReference(input)) => {impl_atan_fxn(input.borrow().clone())}
            x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    } else if arguments.len() == 2 {
      let arg1 = arguments[0].clone();
      let arg2 = arguments[1].clone();
      match impl_atan2_fxn(arg1.clone(), arg2.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => {
          match (arg1,arg2) {
            (Value::MutableReference(arg1),Value::MutableReference(arg2)) => {impl_atan2_fxn(arg1.borrow().clone(),arg2.borrow().clone())}
            (Value::MutableReference(arg1),arg2) => {impl_atan2_fxn(arg1.borrow().clone(),arg2.clone())}
            (arg1,Value::MutableReference(arg2)) => {impl_atan2_fxn(arg1.clone(),arg2.borrow().clone())}
            x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
          }
        }
      }
    } else {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/atan",
    ptr: &MathAtan{},
  }
}