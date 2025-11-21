use crate::*;
use mech_core::*;
use libm::{asinh, asinhf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Asinh Macros
macro_rules! asinh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = asinh((*$arg)); }
  };
}

macro_rules! asinh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = asinh(((&(*$arg))[i]));
      }
    }
  };
}

macro_rules! asinhf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = asinhf((*$arg)); }
  };
}

macro_rules! asinhf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = asinhf(((&(*$arg))[i]));
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathAsinh, f32, asinhf, FeatureFlag::Custom(hash_str("math/asinh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAsinh, f64, asinh, FeatureFlag::Custom(hash_str("math/asinh")));

fn impl_asinh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathAsinh,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathAsinh {}

impl NativeFunctionCompiler for MathAsinh {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments { expected: 1, found: arguments.len() },
        None
      ).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_asinh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_asinh_fxn(input.borrow().clone()),
        _ => Err(MechError2::new(
            UnhandledFunctionArgumentKind1 { arg: input.kind(), fxn_name: "math/asinh".to_string() },
            None
          ).with_compiler_loc()
        ),
      },
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/asinh",
    ptr: &MathAsinh{},
  }
}