#[macro_use]
use crate::stdlib::*;

// Exp ------------------------------------------------------------------------

macro_rules! exp_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {*$out = (*$lhs).pow(*$rhs);}
  };}
  
macro_rules! exp_vec_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
  for i in 0..(*$lhs).len() {
    (*$out)[i] = (*$lhs)[i].pow((*$rhs)[i]);
  }}};}

macro_rules! exp_scalar_lhs_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
  for i in 0..(*$lhs).len() {
    (*$out)[i] = (*$lhs)[i].pow((*$rhs));
  }}};}

macro_rules! exp_scalar_rhs_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
  for i in 0..(*$rhs).len() {
    (*$out)[i] = (*$lhs).pow((*$rhs)[i]);
  }}};}

macro_rules! exp_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i].pow(rhs_deref[i]);
        }
      }
    }
  };}

macro_rules! exp_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_deref[i].pow(rhs_col[i]);
        }
      }
    }
  };}

macro_rules! exp_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_row[i].pow(rhs_deref[i]);
        }
      }
    }
  };}

macro_rules! exp_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_deref[i].pow(rhs_row[i]);
        }
      }
    }
  };} 
  
#[macro_export]
macro_rules! impl_expop {
($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
  #[derive(Debug)]
  struct $struct_name<T> {
    lhs: Ref<$arg1_type>,
    rhs: Ref<$arg2_type>,
    out: Ref<$out_type>,
  }
  impl<T> MechFunction for $struct_name<T>
  where
    T: Copy + Debug + Clone + Sync + Send + 'static + 
    PartialEq + PartialOrd +
    Add<Output = T> + AddAssign +
    Sub<Output = T> + SubAssign +
    Mul<Output = T> + MulAssign +
    Div<Output = T> + DivAssign +
    Pow<T, Output = T> +
    Zero + One,
    Ref<$out_type>: ToValue
  {
    fn solve(&self) {
      let lhs_ptr = self.lhs.as_ptr();
      let rhs_ptr = self.rhs.as_ptr();
      let out_ptr = self.out.as_ptr();
      $op!(lhs_ptr,rhs_ptr,out_ptr);
    }
    fn out(&self) -> Value { self.out.to_value() }
    fn to_string(&self) -> String { format!("{:?}", self) }
  }};}

#[macro_export]
macro_rules! impl_math_fxns_exp {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_expop);
  }}

impl_math_fxns_exp!(Exp);

fn impl_exp_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
      Exp,
      (lhs_value, rhs_value),
      U8,   U8   => MatrixU8,  u8,  u8::zero(), "U8";
      U16,  U16  => MatrixU16, u16, u16::zero(), "U16";
      U32,  U32  => MatrixU32, u32, u32::zero(), "U32";
      F32,  F32  => MatrixF32,  F32,  F32::zero(), "F32";
      F64,  F64  => MatrixF64,  F64,  F64::zero(), "F64";
  )}

impl_mech_binop_fxn!(MathExp,impl_exp_fxn);