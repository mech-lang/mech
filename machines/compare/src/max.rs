use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Max ------------------------------------------------------------------------

macro_rules! max_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        let a = (&*$lhs)[i].clone();
        let b = (*$rhs).clone();
        (&mut *$out)[i] =
          if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) {
            a
          } else {
            b
          };
      }
    }
  };
}

macro_rules! max_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$rhs).len() {
        let a = (*$lhs).clone();
        let b = (&*$rhs)[i].clone();
        (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
      }
    }
  };
}

macro_rules! max_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        let a = (&*$lhs)[i].clone();
        let b = (&*$rhs)[i].clone();
        (&mut *$out)[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
      }
    }
  };
}

macro_rules! max_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let a = (*$lhs).clone();
      let b = (*$rhs).clone();
      (*$out) = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
    }
  };
}

macro_rules! max_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          let a = lhs_col[i].clone();
          let b = rhs_deref[i].clone();
          col[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
        }
      }
    }
  };
}

macro_rules! max_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
        for i in 0..col.len() {
          let a = lhs_deref[i].clone();
          let b = rhs_col[i].clone();
          col[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
        }
      }
    }
  };
}

macro_rules! max_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        for i in 0..row.len() {
          let a = lhs_row[i].clone();
          let b = rhs_deref[i].clone();
          row[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
        }
      }
    }
  };
}

macro_rules! max_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        for i in 0..row.len() {
          let a = lhs_deref[i].clone();
          let b = rhs_row[i].clone();
          row[i] = if a.partial_cmp(&b) != Some(std::cmp::Ordering::Less) { a } else { b };
        }
      }
    }
  };
}

impl_compare_fxns2!(Max);

fn impl_max_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_binop_match_arms!(
    Max,
    register_fxn_descriptor_inner,
    (lhs_value, rhs_value),
    I8,   i8, "i8";
    I16,  i16, "i16";
    I32,  i32, "i32";
    I64,  i64, "i64";
    I128, i128, "i128";
    U8,   u8, "u8";
    U16,  u16, "u16";
    U32,  u32, "u32";
    U64,  u64, "u64";
    U128, u128, "u128";
    F32,  f32, "f32";
    F64,  f64, "f64";
    R64, R64, "rational";
    C64, C64, "complex";
  )
}

impl_mech_binop_fxn!(CompareMax,impl_max_fxn,"compare/max");
