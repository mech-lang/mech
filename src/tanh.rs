use crate::*;
use mech_core::*;
use libm::{tanh, tanhf};

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
        ((*$out)[i]).0 = tanh(((&(*$arg))[i]).0);
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
        ((*$out)[i]).0 = tanhf(((&(*$arg))[i]).0);
      }
    }
  };
}

impl_math_urop!(MathTanh, F32, tanhf);
impl_math_urop!(MathTanh, F64, tanh);

fn impl_tanh_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathTanh,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
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
