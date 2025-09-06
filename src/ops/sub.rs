#[macro_use]
use crate::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Sub ------------------------------------------------------------------------

macro_rules! sub_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { 
      *$out = *$lhs - *$rhs; 
    }
  };
}

macro_rules! sub_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { 
      (&*$lhs).sub_to(&*$rhs, &mut *$out); 
    }
  };
}

macro_rules! sub_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        (&mut *$out)[i] = (&*$lhs)[i] - *$rhs;
      }
    }
  };
}

macro_rules! sub_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$rhs).len() {
        (&mut *$out)[i] = *$lhs - (&*$rhs)[i];
      }
    }
  };
}

macro_rules! sub_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i] - rhs_deref[i];
        }
      }
    }
  };}
    
macro_rules! sub_vec_mat_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
    let mut out_deref = &mut (*$out);
    let lhs_deref = &(*$lhs);
    let rhs_deref = &(*$rhs);
    for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
      for i in 0..col.len() {
      col[i] = lhs_deref[i] - rhs_col[i];
      }
    }
  }
};}

macro_rules! sub_mat_row_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
    let mut out_deref = &mut (*$out);
    let lhs_deref = &(*$lhs);
    let rhs_deref = &(*$rhs);
    for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        for i in 0..row.len() {
        row[i] = lhs_row[i] - rhs_deref[i];
        }
    }
  }
};}

macro_rules! sub_row_mat_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
    let mut out_deref = &mut (*$out);
    let lhs_deref = &(*$lhs);
    let rhs_deref = &(*$rhs);
    for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
      for i in 0..row.len() {
        row[i] = lhs_deref[i] - rhs_row[i];
      }
    }
  }
};}  

impl_math_fxns!(Sub);

fn impl_sub_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    Sub,
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
    ComplexNumber, ComplexNumber, "complex";
  )
}

impl_mech_binop_fxn!(MathSub,impl_sub_fxn);