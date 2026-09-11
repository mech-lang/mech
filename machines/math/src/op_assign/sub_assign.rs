use super::*;
use num_traits::*;

// Sub Assign -----------------------------------------------------------------

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_sub_assign_match_arms {
    ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
        paste! {
          [<impl_set_ $macro_name _match_arms>]!(
            $fxn_name,
            $arg,
            U8, "u8";
            U16, "u16";
            U32, "u32";
            U64, "u64";
            U128, "u128";
            I8, "i8";
            I16, "i16";
            I32, "i32";
            I64, "i64";
            U128, "u128";
            F32, "f32";
            F64, "f64" ;
            C64, "complex";
            R64, "rational";
          )
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_sub_assign_range_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_s!($struct_name, $op, $ix);
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_sub_assign_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        impl_op_assign_range_fxn_v!($struct_name, $op, $ix);
    };
}

// x = 1 ----------------------------------------------------------------------

impl_assign_scalar_scalar!(Sub, checked_sub_assign);
impl_assign_vector_vector!(Sub, checked_sub_assign);
impl_assign_vector_scalar!(Sub, checked_sub_assign);

// x[1..3] -= 1 ----------------------------------------------------------------

macro_rules! sub_assign_1d_range {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_index_scalar_positions($source, $ix, $sink, checked_sub_assign)
    };
}

macro_rules! sub_assign_1d_range_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_index_scalar_mask($source, $ix, $sink, checked_sub_assign)
    };
}

macro_rules! sub_assign_1d_range_vec {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_index_vector_positions($source, $ix, $sink, checked_sub_assign)
    };
}

macro_rules! sub_assign_1d_range_vec_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_index_vector_mask($source, $ix, $sink, checked_sub_assign)
    };
}

#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign1DRS, sub_assign_1d_range, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign1DRB, sub_assign_1d_range_b, bool);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign1DRV, sub_assign_1d_range_vec, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign1DRVB, sub_assign_1d_range_vec_b, bool);

// x[1..3,:] -= 1 ------------------------------------------------------------------

macro_rules! sub_assign_2d_vector_all {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_rows_scalar_positions($source, $ix, $sink, checked_sub_assign)
    };
}

macro_rules! sub_assign_2d_vector_all_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_rows_scalar_mask($source, $ix, $sink, checked_sub_assign)
    };
}

macro_rules! sub_assign_2d_vector_all_mat {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_rows_matrix_positions($source, $ix, $sink, checked_sub_assign)
    };
}

macro_rules! sub_assign_2d_vector_all_mat_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        apply_rows_matrix_mask($source, $ix, $sink, checked_sub_assign)
    };
}

#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign2DRAS, sub_assign_2d_vector_all, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_s!(SubAssign2DRASB, sub_assign_2d_vector_all_b, bool);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign2DRAV, sub_assign_2d_vector_all_mat, usize);
#[cfg(feature = "matrix")]
impl_sub_assign_range_fxn_v!(SubAssign2DRAVB, sub_assign_2d_vector_all_mat_b, bool);

#[cfg(feature = "source")]
crate::impl_canonical_op_assign_specializers!(
    SubAssignValue,
    SubAssignRange,
    SubAssignRangeAll,
    Sub,
    "SubAssign",
    "SubAssign1DR",
    "SubAssign2DRA"
);
