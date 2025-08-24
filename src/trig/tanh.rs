use crate::*;
use mech_core::*;
use libm::{tanh, tanhf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Tanh ------------------------------------------------------------------------
macro_rules! tanh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = tanh((*$arg).0); }
  };
}

macro_rules! tanh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = tanh(((&(*$arg))[i]).0);
      }
    }
  };
}

macro_rules! tanhf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = tanhf((*$arg).0); }
  };
}

macro_rules! tanhf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = tanhf(((&(*$arg))[i]).0);
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTanh, F32, tanhf);
#[cfg(feature = "f64")]
impl_math_unop!(MathTanh, F64, tanh);

fn impl_tanh_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathTanh,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathTanh {}

impl NativeFunctionCompiler for MathTanh {
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
    match impl_tanh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_tanh_fxn(input.borrow().clone()),
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
