use crate::*;
use mech_core::*;
use libm::{acosh, acoshf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Acosh Macros
macro_rules! acosh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = acosh((*$arg)); }
  };
}

macro_rules! acosh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = acosh(((&(*$arg))[i]));
      }
    }
  };
}

macro_rules! acoshf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = acoshf((*$arg)); }
  };
}

macro_rules! acoshf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = acoshf(((&(*$arg))[i]));
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAcosh, f32, acoshf, FeatureFlag::Custom(hash_str("math/acosh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAcosh, f64, acosh, FeatureFlag::Custom(hash_str("math/acosh")));

fn impl_acosh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathAcosh,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathAcosh {}

impl NativeFunctionCompiler for MathAcosh {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments { expected: 1, found: arguments.len() },
        None
      ).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_acosh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_acosh_fxn(input.borrow().clone()),
        _ => Err(MechError2::new(
            UnhandledFunctionArgumentKind1 { arg: input.kind(), fxn_name: "math/acosh".to_string() },
            None
          ).with_compiler_loc()
        ),
      },
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/acosh",
    ptr: &MathAcosh{},
  }
}
