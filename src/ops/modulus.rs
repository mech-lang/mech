#[macro_use]
use crate::*;

// Mod ------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_binop2 {
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
      Rem<Output = T> + RemAssign +
      Zero + One,
      Ref<$out_type>: ToValue
      {
      fn solve(&self) {
          let lhs_ptr = self.lhs.as_ptr();
          let rhs_ptr = self.rhs.as_ptr();
          let out_ptr = self.out.as_mut_ptr();
          $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
      }};}

#[macro_export]
macro_rules! impl_math_fxns2 {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_binop2);
  }}



macro_rules! mod_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs % *$rhs; }};}
  
macro_rules! mod_vec_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
    let mut out_deref = &mut (*$out);
    let lhs_deref = &(*$lhs);
    let rhs_deref = &(*$rhs);
    for (o,(l,r)) in out_deref.iter_mut().zip(lhs_deref.iter().zip(rhs_deref.iter())) {
      *o = *l % *r;
    }
  }};}

macro_rules! mod_scalar_lhs_op {
($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { 
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = (*$rhs);
      for (o,l) in out_deref.iter_mut().zip(lhs_deref.iter()) {
        *o = *l % rhs_deref;
      }
    }};}

macro_rules! mod_scalar_rhs_op {
($lhs:expr, $rhs:expr, $out:expr) => {
  unsafe {
    let mut out_deref = &mut (*$out);
    let lhs_deref = (*$lhs);
    let rhs_deref = &(*$rhs);
    for (o,r) in out_deref.iter_mut().zip(rhs_deref.iter()) {
      *o = lhs_deref % *r;
    }
  }};}

macro_rules! mod_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i] % rhs_deref[i];
        }
      }
    }
  };}

macro_rules! mod_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_deref[i] % rhs_col[i];
        }
      }
    }
  };}

macro_rules! mod_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_row[i] % rhs_deref[i];
        }
      }
    }
  };}

macro_rules! mod_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_deref[i] % rhs_row[i];
        }
      }
    }
  };}  

impl_math_fxns2!(Mod);

fn impl_mod_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    Mod,
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
  )
}

impl_mech_binop_fxn!(MathMod,impl_mod_fxn);