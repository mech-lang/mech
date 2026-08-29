//! Compatibility conversions from exact matrix storage into the retired value ABI.

use crate::matrix::Matrix;
use crate::*;

macro_rules! impl_to_value_for_matrix {
    ($t:ty, $variant:ident) => {
        impl ToValue for Matrix<$t> {
            fn to_value(&self) -> LegacyValue {
                LegacyValue::$variant(self.clone())
            }
        }
    };
}

impl_to_value_for_matrix!(LegacyValue, MatrixValue);
impl_to_value_for_matrix!(usize, MatrixIndex);
#[cfg(feature = "matrixd")]
impl ToValue for Ref<nalgebra::DMatrix<LegacyValue>> {
    fn to_value(&self) -> LegacyValue {
        Matrix::DMatrix(self.clone()).to_value()
    }
}
#[cfg(feature = "f64")]
impl_to_value_for_matrix!(f64, MatrixF64);
#[cfg(feature = "f32")]
impl_to_value_for_matrix!(f32, MatrixF32);
#[cfg(feature = "i8")]
impl_to_value_for_matrix!(i8, MatrixI8);
#[cfg(feature = "i16")]
impl_to_value_for_matrix!(i16, MatrixI16);
#[cfg(feature = "i32")]
impl_to_value_for_matrix!(i32, MatrixI32);
#[cfg(feature = "i64")]
impl_to_value_for_matrix!(i64, MatrixI64);
#[cfg(feature = "i128")]
impl_to_value_for_matrix!(i128, MatrixI128);
#[cfg(feature = "u8")]
impl_to_value_for_matrix!(u8, MatrixU8);
#[cfg(feature = "u16")]
impl_to_value_for_matrix!(u16, MatrixU16);
#[cfg(feature = "u32")]
impl_to_value_for_matrix!(u32, MatrixU32);
#[cfg(feature = "u64")]
impl_to_value_for_matrix!(u64, MatrixU64);
#[cfg(feature = "u128")]
impl_to_value_for_matrix!(u128, MatrixU128);
#[cfg(feature = "bool")]
impl_to_value_for_matrix!(bool, MatrixBool);
#[cfg(feature = "string")]
impl_to_value_for_matrix!(String, MatrixString);
#[cfg(feature = "complex")]
impl_to_value_for_matrix!(C64, MatrixC64);
#[cfg(feature = "rational")]
impl_to_value_for_matrix!(R64, MatrixR64);

macro_rules! to_value_ndmatrix {
    ($($nd_matrix_kind:ident, $matrix_kind:ident, $base_type:ty, $type_string:tt),+ $(,)?) => {
        $(
            #[cfg(all(feature = "matrix", feature = $type_string))]
            impl ToValue for Ref<nalgebra::$nd_matrix_kind<$base_type>> {
                fn to_value(&self) -> LegacyValue {
                    LegacyValue::$matrix_kind(Matrix::<$base_type>::$nd_matrix_kind(self.clone()))
                }
            }
        )+
    };
}

macro_rules! impl_to_value_matrix {
    ($matrix_kind:ident) => {
        to_value_ndmatrix!(
            $matrix_kind,
            MatrixIndex,
            usize,
            "matrix",
            $matrix_kind,
            MatrixBool,
            bool,
            "bool",
            $matrix_kind,
            MatrixI8,
            i8,
            "i8",
            $matrix_kind,
            MatrixI16,
            i16,
            "i16",
            $matrix_kind,
            MatrixI32,
            i32,
            "i32",
            $matrix_kind,
            MatrixI64,
            i64,
            "i64",
            $matrix_kind,
            MatrixI128,
            i128,
            "i128",
            $matrix_kind,
            MatrixU8,
            u8,
            "u8",
            $matrix_kind,
            MatrixU16,
            u16,
            "u16",
            $matrix_kind,
            MatrixU32,
            u32,
            "u32",
            $matrix_kind,
            MatrixU64,
            u64,
            "u64",
            $matrix_kind,
            MatrixU128,
            u128,
            "u128",
            $matrix_kind,
            MatrixF32,
            f32,
            "f32",
            $matrix_kind,
            MatrixF64,
            f64,
            "f64",
            $matrix_kind,
            MatrixString,
            String,
            "string",
            $matrix_kind,
            MatrixR64,
            R64,
            "rational",
            $matrix_kind,
            MatrixC64,
            C64,
            "complex",
        );
    };
}

#[cfg(feature = "matrix2x3")]
impl_to_value_matrix!(Matrix2x3);
#[cfg(feature = "matrix3x2")]
impl_to_value_matrix!(Matrix3x2);
#[cfg(feature = "matrix1")]
impl_to_value_matrix!(Matrix1);
#[cfg(feature = "matrix2")]
impl_to_value_matrix!(Matrix2);
#[cfg(feature = "matrix3")]
impl_to_value_matrix!(Matrix3);
#[cfg(feature = "matrix4")]
impl_to_value_matrix!(Matrix4);
#[cfg(feature = "vector2")]
impl_to_value_matrix!(Vector2);
#[cfg(feature = "vector3")]
impl_to_value_matrix!(Vector3);
#[cfg(feature = "vector4")]
impl_to_value_matrix!(Vector4);
#[cfg(feature = "row_vector2")]
impl_to_value_matrix!(RowVector2);
#[cfg(feature = "row_vector3")]
impl_to_value_matrix!(RowVector3);
#[cfg(feature = "row_vector4")]
impl_to_value_matrix!(RowVector4);
#[cfg(feature = "row_vectord")]
impl_to_value_matrix!(RowDVector);
#[cfg(feature = "vectord")]
impl_to_value_matrix!(DVector);
#[cfg(feature = "matrixd")]
impl_to_value_matrix!(DMatrix);
