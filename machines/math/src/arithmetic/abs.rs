use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Abs ------------------------------------------------------------------------

use libm::{fabs,fabsf};

macro_rules! uabs_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = (*$arg).clone();}
  };}

macro_rules! uabs_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        (&mut (*$out))[i] =  (&(*$arg))[i].clone();
      }}};}

macro_rules! abs_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = (*$arg).abs();}
  };}

macro_rules! abs_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        (&mut (*$out))[i] =  (&(*$arg))[i].abs();
      }}};}

macro_rules! fabs_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = fabs((*$arg));}
  };}

macro_rules! fabs_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = fabs(((&(*$arg))[i]));
      }}};}

macro_rules! fabsf_op {
  ($arg:expr, $out:expr) => {
    unsafe{(*$out) = fabsf((*$arg));}
  };}  

macro_rules! fabsf_vec_op {
  ($arg:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$arg).len() {
        ((&mut (*$out))[i]) = fabsf(((&(*$arg))[i]));
      }}};}

#[cfg(feature = "u8")]
impl_math_unop!(MathAbs, u8, uabs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "u16")]
impl_math_unop!(MathAbs, u16, uabs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "u32")]
impl_math_unop!(MathAbs, u32, uabs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "u64")]
impl_math_unop!(MathAbs, u64, uabs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "u128")]
impl_math_unop!(MathAbs, u128, uabs, FeatureFlag::Custom(hash_str("math/abs")));

#[cfg(feature = "i8")]
impl_math_unop!(MathAbs, i8, abs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "i16")]
impl_math_unop!(MathAbs, i16, abs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "i32")]
impl_math_unop!(MathAbs, i32, abs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "i64")]
impl_math_unop!(MathAbs, i64, abs, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "i128")]
impl_math_unop!(MathAbs, i128, abs, FeatureFlag::Custom(hash_str("math/abs")));

#[cfg(feature = "f32")]
impl_math_unop!(MathAbs, f32, fabsf, FeatureFlag::Custom(hash_str("math/abs")));
#[cfg(feature = "f64")]
impl_math_unop!(MathAbs, f64, fabs, FeatureFlag::Custom(hash_str("math/abs")));

#[cfg(feature = "c64")]
impl_math_unop!(MathAbs, C64, abs, FeatureFlag::Custom(hash_str("math/abs")));

#[cfg(feature = "r64")]
impl_math_unop!(MathAbs, R64, abs, FeatureFlag::Custom(hash_str("math/abs")));

fn impl_abs_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_urnop_match_arms2!(
    MathAbs,
    (lhs_value),
    U8 => MatrixU8, u8, u8::zero(), "u8";
    U16 => MatrixU16, u16, u16::zero(), "u16";
    U32 => MatrixU32, u32, u32::zero(), "u32";
    U64 => MatrixU64, u64, u64::zero(), "u64";
    U128 => MatrixU128, u128, u128::zero(), "u128";
    I8 => MatrixI8, i8, i8::zero(), "i8";
    I16 => MatrixI16, i16, i16::zero(), "i16";
    I32 => MatrixI32, i32, i32::zero(), "i32";
    I64 => MatrixI64, i64, i64::zero(), "i64";
    I128 => MatrixI128, i128, i128::zero(), "i128";
    F32 => MatrixF32, f32, f32::zero(), "f32";
    F64 => MatrixF64, f64, f64::zero(), "f64";
    C64 => MatrixC64, C64, C64::default(), "c64";
    R64 => MatrixR64, R64, R64::zero(), "r64";
  )
}

pub struct MathAbs {}

impl NativeFunctionCompiler for MathAbs {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 1 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let input = arguments[0].clone();
    match impl_abs_fxn(input.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (input) {
          (Value::MutableReference(input)) => {impl_abs_fxn(input.borrow().clone())}
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "math/abs".to_string() },
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
    name: "math/abs",
    ptr: &MathAbs{},
  }
}