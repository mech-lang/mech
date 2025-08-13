use crate::*;
use mech_core::*;
use libm::{cosh, coshf};

// Cosh ------------------------------------------------------------------------
macro_rules! cosh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = cosh((*$arg).0); }
  };
}

macro_rules! cosh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = cosh(((&(*$arg))[i]).0);
      }
    }
  };
}

macro_rules! coshf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = coshf((*$arg).0); }
  };
}

macro_rules! coshf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = coshf(((&(*$arg))[i]).0);
      }
    }
  };
}

impl_math_urop!(MathCosh, F32, coshf);
impl_math_urop!(MathCosh, F64, cosh);

fn impl_cosh_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_urnop_match_arms2!(
    MathCosh,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathCosh {}

impl NativeFunctionCompiler for MathCosh {
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
    match impl_cosh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_cosh_fxn(input.borrow().clone()),
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
