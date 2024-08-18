#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! generate_compare_fxns {
  ($lib:ident) => {
    generate_fxns!($lib,T,bool,impl_binop);
  }
}

#[macro_export]
macro_rules! generate_compare_fxns_bool {
  ($lib:ident) => {
    generate_fxns!($lib,T,bool,impl_bool_binop);
  }
}

// Greater Than ---------------------------------------------------------------

macro_rules! gt_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] > (*$rhs);
      }}};}

macro_rules! gt_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) > (*$rhs)[i];
      }}};}

macro_rules! gt_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] > (*$rhs)[i];
      }}};}

macro_rules! gt_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) > (*$rhs);
    }};}

generate_compare_fxns!(GT);

fn generate_gt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    GT,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

impl_mech_binop_fxn!(CompareGreaterThan,generate_gt_fxn);

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

generate_compare_fxns!(GTE);

fn generate_gte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    GTE,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

impl_mech_binop_fxn!(CompareGreaterThanEqual,generate_gte_fxn);

// Less Than ------------------------------------------------------------------

macro_rules! lt_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] < (*$rhs);
      }}};}

macro_rules! lt_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) < (*$rhs)[i];
      }}};}

macro_rules! lt_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] < (*$rhs)[i];
      }}};}

macro_rules! lt_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) < (*$rhs);
    }};}

generate_compare_fxns!(LT);

fn generate_lt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    LT,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

impl_mech_binop_fxn!(CompareLessThan,generate_lt_fxn);

// Less Than Equal ---------------------------------------------------------------

macro_rules! lte_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] <= (*$rhs);
      }}};}

macro_rules! lte_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) <= (*$rhs)[i];
      }}};}

macro_rules! lte_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] <= (*$rhs)[i];
      }}};}

macro_rules! lte_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) <= (*$rhs);
    }};}

generate_compare_fxns!(LTE);

fn generate_lte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    LTE,
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

impl_mech_binop_fxn!(CompareLessThanEqual,generate_lte_fxn);

// Equal ---------------------------------------------------------------

macro_rules! eq_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] == (*$rhs);
      }}};}

macro_rules! eq_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) == (*$rhs)[i];
      }}};}


macro_rules! eq_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] == (*$rhs)[i];
      }}};}

macro_rules! eq_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) == (*$rhs);
    }};}

generate_compare_fxns_bool!(EQ);

fn generate_eq_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    EQ,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

impl_mech_binop_fxn!(CompareEqual,generate_eq_fxn);

// Not Equal ---------------------------------------------------------------

macro_rules! neq_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] != (*$rhs);
      }}};}

macro_rules! neq_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (*$out)[i] = (*$lhs) != (*$rhs)[i];
      }}};}

macro_rules! neq_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (*$out)[i] = (*$lhs)[i] != (*$rhs)[i];
      }}};}

macro_rules! neq_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) != (*$rhs);
    }};}

generate_compare_fxns_bool!(NEQ);

fn generate_neq_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_binop_match_arms!(
    NEQ,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false;
    I8,   I8   => MatrixI8,   i8,   false;
    I16,  I16  => MatrixI16,  i16,  false;
    I32,  I32  => MatrixI32,  i32,  false;
    I64,  I64  => MatrixI64,  i64,  false;
    I128, I128 => MatrixI128, i128, false;
    U8,   U8   => MatrixU8,   u8,   false;
    U16,  U16  => MatrixU16,  u16,  false;
    U32,  U32  => MatrixU32,  u32,  false;
    U64,  U64  => MatrixU64,  u64,  false;
    U128, U128 => MatrixU128, u128, false;
    F32,  F32  => MatrixF32,  F32,  false;
    F64,  F64  => MatrixF64,  F64,  false;
  )
}

impl_mech_binop_fxn!(CompareNotEqual,generate_neq_fxn);