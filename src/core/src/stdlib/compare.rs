#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_compare_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_binop);
  }
}

#[macro_export]
macro_rules! impl_compare_fxns_bool {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_bool_binop);
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

impl_compare_fxns!(GT);

fn impl_gt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    GT,
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

impl_mech_binop_fxn!(CompareGreaterThan,impl_gt_fxn);

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

impl_compare_fxns!(LT);

fn impl_lt_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    LT,
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

impl_mech_binop_fxn!(CompareLessThan,impl_lt_fxn);

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

impl_compare_fxns!(LTE);

fn impl_lte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    LTE,
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

impl_mech_binop_fxn!(CompareLessThanEqual,impl_lte_fxn);

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

impl_compare_fxns_bool!(EQ);

fn impl_eq_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    EQ,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false, "Bool";
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

impl_mech_binop_fxn!(CompareEqual,impl_eq_fxn);

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

impl_compare_fxns_bool!(NEQ);

fn impl_neq_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    NEQ,
    (lhs_value, rhs_value),
    Bool, Bool => MatrixBool, bool, false, "Bool";
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

impl_mech_binop_fxn!(CompareNotEqual,impl_neq_fxn);