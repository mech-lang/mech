use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Erfc ------------------------------------------------------------------------

use libm::{erfc,erfcf};
macro_rules! erfc_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = erfc((*$arg).0);}
  };}

macro_rules! erfc_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = erfc(((&(*$arg))[i]).0);
      }}};}

macro_rules! erfcf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out).0 = erfcf((*$arg).0);}
  };}  

macro_rules! erfcf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]).0 = erfcf(((&(*$arg))[i]).0);
      }}};}

#[cfg(feature = "f32")]
impl_math_unop!(MathErfc, F32, erfcf, FeatureFlag::Custom(hash_str("math/erfc")));
#[cfg(feature = "f64")]
impl_math_unop!(MathErfc, F64, erfc, FeatureFlag::Custom(hash_str("math/erfc")));

fn impl_erfc_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathErfc,
    (lhs_value),
    F32 => MatrixF32, F32, F32::zero(), "f32";
    F64 => MatrixF64, F64, F64::zero(), "f64";
  )
}

pub struct MathErfc {}

impl NativeFunctionCompiler for MathErfc {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_erfc_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_erfc_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.clone(), fxn_name: "math/erfc".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/erfc",
    ptr: &MathErfc{},
  }
}