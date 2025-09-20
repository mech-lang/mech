
use crate::*;
use num_traits::*;

use mech_core::matrix::Matrix;

#[cfg(feature = "set")]
use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Add ------------------------------------------------------------------------

macro_rules! union_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe { let new_set = (*$lhs).set.union((*$rhs).set);
    *$out = {(*$lhs).kind, new_set.len(), new_set}; }
  };}

impl_set_fxns!(Union);

fn impl_union_fxn(lhs_value: MechSet, rhs_value: MechSet) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    Union,
    register_fxn_descriptor_inner,
    (lhs_value, rhs_value),
    I8,   i8,   "i8";
    I16,  i16,  "i16";
    I32,  i32,  "i32";
    I64,  i64,  "i64";
    I128, i128, "i128";
    U8,   u8,   "u8";
    U16,  u16,  "u16";
    U32,  u32,  "u32";
    U64,  u64,  "u64";
    U128, u128, "u128";
    F32,  F32,  "f32";
    F64,  F64,  "f64";
    R64, R64, "rational";
    C64, C64, "complex";
  )
}

impl_mech_binop_fxn!(SetUnion,impl_union_fxn);