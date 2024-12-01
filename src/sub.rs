use crate::*;
use mech_core::*;

// Sub ------------------------------------------------------------------------

macro_rules! sub_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs - *$rhs; }
    };}
  
macro_rules! sub_vec_op {
($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).sub_to(&*$rhs,&mut *$out) }
    };}

macro_rules! sub_scalar_lhs_op {
($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
    for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] - (*$rhs);
    }}};}

macro_rules! sub_scalar_rhs_op {
($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
    for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) - (*$rhs)[i];
    }}};}

impl_math_fxns!(Sub);

fn impl_sub_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
impl_binop_match_arms!(
    Sub,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   i8::zero(), "I8";
    I16,  I16  => MatrixI16,  i16,  i16::zero(), "I16";
    I32,  I32  => MatrixI32,  i32,  i32::zero(), "I32";
    I64,  I64  => MatrixI64,  i64,  i64::zero(), "I64";
    I128, I128 => MatrixI128, i128, i128::zero(), "I128";
    U8,   U8   => MatrixU8,   u8,   u8::zero(), "U8";
    U16,  U16  => MatrixU16,  u16,  u16::zero(), "U16";
    U32,  U32  => MatrixU32,  u32,  u32::zero(), "U32";
    U64,  U64  => MatrixU64,  u64,  u64::zero(), "U64";
    U128, U128 => MatrixU128, u128, u128::zero(), "U128";
    F32,  F32  => MatrixF32,  F32,  F32::zero(), "F32";
    F64,  F64  => MatrixF64,  F64,  F64::zero(), "F64";
)
}

impl_mech_binop_fxn!(MathSub,impl_sub_fxn);