//! Runtime element capabilities shared by interpreted and compiled execution.
//!
//! Bytecode encoding is intentionally not part of this module. Official
//! bytecode codecs live under `program::bytecode`; this marker carries no
//! byte-oriented API into runtime-only builds.

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(not(feature = "no_std"))]
use std::string::String;

#[cfg(feature = "matrix")]
use core::fmt::Debug;

use crate::{LegacyValue, ValueKind};

/// Identifies values supported as scalar or aggregate runtime elements.
///
/// This is a capability marker, not a serialization contract. Keeping it
/// available without the compiler lets runtime machinery express its element
/// requirements without exposing bytecode codec methods.
pub trait ConstElem: 'static {}

macro_rules! impl_const_elem_scalar {
    ($feature:literal, $type:ty) => {
        #[cfg(feature = $feature)]
        impl ConstElem for $type {}
    };
}

impl_const_elem_scalar!("bool", bool);
impl_const_elem_scalar!("u8", u8);
impl_const_elem_scalar!("u16", u16);
impl_const_elem_scalar!("u32", u32);
impl_const_elem_scalar!("u64", u64);
impl_const_elem_scalar!("u128", u128);
impl_const_elem_scalar!("i8", i8);
impl_const_elem_scalar!("i16", i16);
impl_const_elem_scalar!("i32", i32);
impl_const_elem_scalar!("i64", i64);
impl_const_elem_scalar!("i128", i128);
impl_const_elem_scalar!("f32", f32);
impl_const_elem_scalar!("f64", f64);

#[cfg(feature = "rational")]
impl ConstElem for crate::R64 {}

#[cfg(feature = "complex")]
impl ConstElem for crate::C64 {}

impl ConstElem for String {}
impl ConstElem for usize {}
impl ConstElem for LegacyValue {}
impl ConstElem for ValueKind {}

macro_rules! impl_const_elem_matrix {
    ($feature:literal, $matrix_type:ty) => {
        #[cfg(feature = $feature)]
        impl<T> ConstElem for $matrix_type
        where
            T: ConstElem + Debug + Clone + PartialEq + 'static,
        {
        }
    };
}

impl_const_elem_matrix!("matrixd", nalgebra::DMatrix<T>);
impl_const_elem_matrix!("vectord", nalgebra::DVector<T>);
impl_const_elem_matrix!("row_vectord", nalgebra::RowDVector<T>);
impl_const_elem_matrix!("matrix1", nalgebra::Matrix1<T>);
impl_const_elem_matrix!("matrix2", nalgebra::Matrix2<T>);
impl_const_elem_matrix!("matrix3", nalgebra::Matrix3<T>);
impl_const_elem_matrix!("matrix4", nalgebra::Matrix4<T>);
impl_const_elem_matrix!("matrix2x3", nalgebra::Matrix2x3<T>);
impl_const_elem_matrix!("matrix3x2", nalgebra::Matrix3x2<T>);
impl_const_elem_matrix!("row_vector2", nalgebra::RowVector2<T>);
impl_const_elem_matrix!("row_vector3", nalgebra::RowVector3<T>);
impl_const_elem_matrix!("row_vector4", nalgebra::RowVector4<T>);
impl_const_elem_matrix!("vector2", nalgebra::Vector2<T>);
impl_const_elem_matrix!("vector3", nalgebra::Vector3<T>);
impl_const_elem_matrix!("vector4", nalgebra::Vector4<T>);

#[cfg(feature = "matrix")]
impl<T> ConstElem for crate::structures::Matrix<T> where
    T: ConstElem + Debug + Clone + PartialEq + 'static
{
}

#[cfg(feature = "enum")]
impl ConstElem for crate::MechEnum {}

#[cfg(feature = "table")]
impl ConstElem for crate::MechTable {}

#[cfg(feature = "set")]
impl ConstElem for crate::MechSet {}

#[cfg(feature = "tuple")]
impl ConstElem for crate::MechTuple {}
