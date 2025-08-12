use crate::*;
use mech_core::*;

// Less Than Equal ---------------------------------------------------------------

macro_rules! lte_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        (&mut *$out)[i] = (&*$lhs)[i] <= *$rhs;
      }
    }
  };
}

macro_rules! lte_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$rhs).len() {
        (&mut *$out)[i] = *$lhs <= (&*$rhs)[i];
      }
    }
  };
}

macro_rules! lte_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        (&mut *$out)[i] = (&*$lhs)[i] <= (&*$rhs)[i];
      }
    }
  };
}

macro_rules! lte_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      (*$out) = (*$lhs) <= (*$rhs);
    }};}

macro_rules! lte_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i] <= rhs_deref[i];
        }
      }
    }
  };}   
      
macro_rules! lte_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
        let mut out_deref = &mut (*$out);
        let lhs_deref = &(*$lhs);
        let rhs_deref = &(*$rhs);
        for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
          for i in 0..col.len() {
            col[i] = lhs_deref[i] <= rhs_col[i];
          }
        }
      }
  };}
  
macro_rules! lte_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
          for i in 0..row.len() {
          row[i] = lhs_row[i] <= rhs_deref[i];
          }
        }
      }
  };}

macro_rules! lte_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
          for i in 0..row.len() {
          row[i] = lhs_deref[i] <= rhs_row[i];
          }
        }
      }
  };}    


impl_compare_fxns!(LTE);

fn impl_lte_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    LTE,
    (lhs_value, rhs_value),
    I8,   bool, "I8";
    I16,  bool, "I16";
    I32,  bool, "I32";
    I64,  bool, "I64";
    I128, bool, "I128";
    U8,   bool, "U8";
    U16,  bool, "U16";
    U32,  bool, "U32";
    U64,  bool, "U64";
    U128, bool, "U128";
    F32,  bool, "F32";
    F64,  bool, "F64";
    RationalNumber, bool, "RationalNumber";
    ComplexNumber, bool, "ComplexNumber";
  )
}

impl_mech_binop_fxn!(CompareLessThanEqual,impl_lte_fxn);
  