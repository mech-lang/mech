
use crate::*;
use num_traits::*;

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Add ------------------------------------------------------------------------

macro_rules! add_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe { *$out = *$lhs + *$rhs; }
  };}

macro_rules! add_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).add_to(&*$rhs,&mut *$out) }
  };}

macro_rules! add_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        lhs_col.add_to(&rhs_deref,&mut col);
      }
    }
  };}

macro_rules! add_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
        lhs_deref.add_to(&rhs_col,&mut col);
      }
    }
  };}

macro_rules! add_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        lhs_row.add_to(&rhs_deref,&mut row);
      }
    }
  };}

macro_rules! add_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        lhs_deref.add_to(&rhs_row,&mut row);
      }
    }
  };}  

macro_rules! add_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).add_scalar(*$rhs); }
  };}

macro_rules! add_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$rhs).add_scalar(*$lhs); }
  };}

impl_math_fxns!(Add);

fn impl_add_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_binop_match_arms!(
    Add,
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
    F32,  f32,  "f32";
    F64,  f64,  "f64";
    R64, R64, "rational";
    C64, C64, "complex";
  )
}

impl_mech_binop_fxn!(MathAdd,impl_add_fxn,"math/add");