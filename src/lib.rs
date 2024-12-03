#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use paste::paste;
use mech_core::matrix::Matrix;
use std::fmt::Debug;

pub mod gt;
pub mod lt;
pub mod lte;
pub mod gte;
pub mod eq;
pub mod neq;

pub use self::gt::*;
pub use self::lt::*;
pub use self::lte::*;
pub use self::gte::*;
pub use self::eq::*;
pub use self::neq::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_compare_binop {
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
      PartialEq + PartialOrd,
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
macro_rules! impl_compare_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
  }
}

#[macro_export]
macro_rules! impl_compare_fxns_bool {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
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