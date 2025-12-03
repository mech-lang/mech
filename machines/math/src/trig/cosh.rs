use crate::*;
use mech_core::*;
use libm::{cosh, coshf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Cosh ------------------------------------------------------------------------
macro_rules! cosh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = cosh((*$arg)); }
  };
}

macro_rules! cosh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = cosh(((&(*$arg))[i]));
      }
    }
  };
}

macro_rules! coshf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = coshf((*$arg)); }
  };
}

macro_rules! coshf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = coshf(((&(*$arg))[i]));
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathCosh, f32, coshf, FeatureFlag::Custom(hash_str("math/cosh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathCosh, f64, cosh, FeatureFlag::Custom(hash_str("math/cosh")));

fn impl_cosh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathCosh,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathCosh {}

impl NativeFunctionCompiler for MathCosh {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments { expected: 1, found: arguments.len() },
        None
      ).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_cosh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_cosh_fxn(input.borrow().clone()),
        _ => Err(MechError2::new(
            UnhandledFunctionArgumentKind1 { arg: input.kind(), fxn_name: "math/cosh".to_string() },
            None
          ).with_compiler_loc()
        ),
      },
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/cosh",
    ptr: &MathCosh{},
  }
}