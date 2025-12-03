use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Greater Than ---------------------------------------------------------------

macro_rules! gt_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (&mut (*$out))[i] = (&(*$lhs))[i] > (*$rhs);
      }}};}

macro_rules! gt_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (&mut (*$out))[i] = (*$lhs) > (&(*$rhs))[i];
      }}};}

macro_rules! gt_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (&mut (*$out))[i] = (&(*$lhs))[i] > (&(*$rhs))[i];
      }}};}

macro_rules! gt_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) > (*$rhs);
    }};}

macro_rules! gt_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i] > rhs_deref[i];
        }
      }
    }
  };}   
      
macro_rules! gt_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        let mut out_deref = &mut (*$out);
        let lhs_deref = &(*$lhs);
        let rhs_deref = &(*$rhs);
        for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
          for i in 0..col.len() {
            col[i] = lhs_deref[i] > rhs_col[i];
          }
        }
      }
  };}
  
macro_rules! gt_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
          for i in 0..row.len() {
          row[i] = lhs_row[i] > rhs_deref[i];
          }
      }
      }
  };}

macro_rules! gt_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
          for i in 0..row.len() {
          row[i] = lhs_deref[i] > rhs_row[i];
          }
      }
      }
  };}    


impl_compare_fxns!(GT);

fn impl_gt_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_binop_match_arms!(
    GT,
    register_fxn_descriptor_inner,
    (lhs_value, rhs_value),
    I8,   bool, "i8";
    I16,  bool, "i16";
    I32,  bool, "i32";
    I64,  bool, "i64";
    I128, bool, "i128";
    U8,   bool, "u8";
    U16,  bool, "u16";
    U32,  bool, "u32";
    U64,  bool, "u64";
    U128, bool, "u128";
    F32,  bool, "f32";
    F64,  bool, "f64";
    R64, bool, "rational";
    C64, bool, "complex";
  )
}

impl_mech_binop_fxn!(CompareGreaterThan,impl_gt_fxn,"compare/gt");  