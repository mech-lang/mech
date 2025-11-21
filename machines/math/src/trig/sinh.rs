use crate::*;
use mech_core::*;
use libm::{sinh, sinhf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sinh ------------------------------------------------------------------------
macro_rules! sinh_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = sinh((*$arg)); }
  };
}

macro_rules! sinh_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = sinh(((&(*$arg))[i]));
      }}};}

macro_rules! sinhf_op {
  ($arg:expr, $out:expr) => {
    unsafe { (*$out) = sinhf((*$arg)); }
  };
}

macro_rules! sinhf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = sinhf(((&(*$arg))[i]));
      }
    }
  };
}

#[cfg(feature = "f32")]
impl_math_unop!(MathSinh, f32, sinhf, FeatureFlag::Custom(hash_str("math/sinh")));
#[cfg(feature = "f64")]
impl_math_unop!(MathSinh, f64, sinh, FeatureFlag::Custom(hash_str("math/sinh")));

fn impl_sinh_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathSinh,
    (lhs_value),
    F32 => MatrixF32, F32, f32::zero(), "f32";
    F64 => MatrixF64, F64, f64::zero(), "f64";
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