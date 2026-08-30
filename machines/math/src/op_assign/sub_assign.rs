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
