use crate::*;
use mech_core::*;
use libm::{sinh, sinhf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sinh ------------------------------------------------------------------------
macro_rules! sinh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = sinh((*$arg).0); }
  };
}

macro_rules! sinh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sinh(((&(*$arg))[i]).0);
      }}};}

macro_rules! sinhf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out).0 = sinhf((*$arg).0); }
  };
}

macro_rules! sinhf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = sinhf(((&(*$arg))[i]).0);
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSinh, F32, sinhf, FeatureFlag::Custom(hash_str("math/sinh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathSinh, F64, sinh, FeatureFlag::Custom(hash_str("math/sinh")));

fn impl_sinh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathSinh,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathSinh {}

impl NativeFunctionCompiler for MathSinh {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments { expected: 1, found: arguments.len() },
        None
      ).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_sinh_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => match input {
        Value::MutableReference(input) => impl_sinh_fxn(input.borrow().clone()),
        _ => Err(MechError2::new(
            UnhandledFunctionArgumentKind1 { arg: input.kind(), fxn_name: "math/sinh".to_string() },
            None
          ).with_compiler_loc()
        ),
      },
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/sinh",
    ptr: &MathSinh{},
  }
}