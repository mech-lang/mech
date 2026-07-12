use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Greater Than ---------------------------------------------------------------

macro_rules! concat_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (&mut (*$out))[i] = (&(*$lhs))[i].concat(&(*$rhs));
      }}};}

macro_rules! concat_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$rhs).len() {
        (&mut (*$out))[i] = (*$lhs).concat(&(&(*$rhs))[i]);
      }}};}

macro_rules! concat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(*$lhs).len() {
        (&mut (*$out))[i] = (&(*$lhs))[i].concat(&(&(*$rhs))[i]);
      }}};}

macro_rules! concat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs).concat(&(*$rhs));
    }};}

macro_rules! concat_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i].concat(&rhs_deref[i]);
        }
      }
    }
  };}   
      
macro_rules! concat_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        let mut out_deref = &mut (*$out);
        let lhs_deref = &(*$lhs);
        let rhs_deref = &(*$rhs);
        for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
          for i in 0..col.len() {
            col[i] = lhs_deref[i].concat(&rhs_col[i]);
          }
        }
      }
  };}
  
macro_rules! concat_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
          for i in 0..row.len() {
          row[i] = lhs_row[i].concat(&rhs_deref[i]);
          }
      }
      }
  };}

macro_rules! concat_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_deref[i].concat(&rhs_row[i]);
        }
      }
      }
  };}    

impl_string_fxns!(Concat);

fn impl_concat_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_binop_match_arms!(
    Concat,
    register_fxn_descriptor_inner,
    (lhs_value, rhs_value),
    String, String, "string";
  )
}

impl_mech_binop_fxn!(StringConcat,impl_concat_fxn,"string/concat");