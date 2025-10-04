use crate::*;
use mech_core::*;
use libm::{acosh, acoshf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Acosh Macros
macro_rules! acosh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = acosh((*$arg).0); }
  };
}

macro_rules! acosh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = acosh(((&(*$arg))[i]).0);
      }
    }
  };
}

macro_rules! acoshf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = acoshf((*$arg).0); }
  };
}

macro_rules! acoshf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = acoshf(((&(*$arg))[i]).0);
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcosh, F32, acoshf, FeatureFlag::Custom(hash_str("math/acosh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAcosh, F64, acosh, FeatureFlag::Custom(hash_str("math/acosh")));

fn impl_acosh_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAcosh,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathAcosh {}

impl NativeFunctionCompiler for MathAcosh {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError {
        file: file!().to_string(),
        tokens: vec![],
        msg: "".to_string(),
        id: line!(),
        kind: MechErrorKind::IncorrectNumberOfArguments,
      });
    }
    let input = arguments[0].clone();
    match impl_acosh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_acosh_fxn(input.borrow().clone()),
        _ => Err(MechError {
          file: file!().to_string(),
          tokens: vec![],
          msg: "".to_string(),
          id: line!(),
          kind: MechErrorKind::UnhandledFunctionArgumentKind,
        }),
      },
    }
  }
}
