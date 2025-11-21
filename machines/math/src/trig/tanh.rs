use crate::*;
use mech_core::*;
use libm::{tanh, tanhf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Tanh ------------------------------------------------------------------------
macro_rules! tanh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = tanh((*$arg)); }
  };
}

macro_rules! tanh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = tanh(((&(*$arg))[i]));
      }
    }
  };
}

macro_rules! tanhf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = tanhf((*$arg)); }
  };
}

macro_rules! tanhf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = tanhf(((&(*$arg))[i]));
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathTanh, f32, tanhf, FeatureFlag::Custom(hash_str("math/tanh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathTanh, f64, tanh, FeatureFlag::Custom(hash_str("math/tanh")));

fn impl_tanh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathTanh,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
  )
}

pub struct MathTanh {}

impl NativeFunctionCompiler for MathTanh {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() },None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_tanh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_tanh_fxn(input.borrow().clone()),
        _ => Err(MechError2::new(
            UnhandledFunctionArgumentKind1 { arg: input.kind(), fxn_name: "math/tanh".to_string() },
            None
          ).with_compiler_loc()
        ),
      },
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/tanh",
    ptr: &MathTanh{},
  }
}