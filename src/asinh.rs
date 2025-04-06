use crate::*;
use mech_core::*;
use libm::{asinh, asinhf};

// Asinh Macros
macro_rules! asinh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = asinh((*$arg).0); }
  };
}

macro_rules! asinh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = asinh(((*$arg)[i]).0);
      }
    }
  };
}

macro_rules! asinhf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = asinhf((*$arg).0); }
  };
}

macro_rules! asinhf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((*$out)[i]).0 = asinhf(((*$arg)[i]).0);
      }
    }
  };
}

impl_math_urop!(MathAsinh, F32, asinhf);
impl_math_urop!(MathAsinh, F64, asinh);

fn impl_asinh_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathAsinh,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "F32";
    F64 => MatrixF64, F64, F64::zero(), "F64";
  )
}

pub struct MathAsinh {}

impl NativeFunctionCompiler for MathAsinh {
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
    match impl_asinh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_asinh_fxn(input.borrow().clone()),
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
