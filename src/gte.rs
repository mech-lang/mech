use crate::*;
use mech_core::*;

// Greater Than Equal ---------------------------------------------------------------

macro_rules! gte_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] >= (*$rhs);
      }}};}

macro_rules! gte_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) >= (*$rhs)[i];
      }}};}

macro_rules! gte_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] >= (*$rhs)[i];
      }}};}

macro_rules! gte_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) >= (*$rhs);
    }};}

impl_compare_fxns!(GTE);

fn impl_gte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    GTE,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false, "I8";
    I16,  I16  => MatrixI16,  i16,  false, "I16";
    I32,  I32  => MatrixI32,  i32,  false, "I32";
    I64,  I64  => MatrixI64,  i64,  false, "I64";
    I128, I128 => MatrixI128, i128, false, "I128";
    U8,   U8   => MatrixU8,   u8,   false, "U8";
    U16,  U16  => MatrixU16,  u16,  false, "U16";
    U32,  U32  => MatrixU32,  u32,  false, "U32";
    U64,  U64  => MatrixU64,  u64,  false, "U64";
    U128, U128 => MatrixU128, u128, false, "U128";
    F32,  F32  => MatrixF32,  F32,  false, "F32";
    F64,  F64  => MatrixF64,  F64,  false, "F64";
  )
}

impl_mech_binop_fxn!(CompareGreaterThanEqual,impl_gte_fxn);
  