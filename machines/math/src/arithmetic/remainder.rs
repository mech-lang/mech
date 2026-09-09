use crate::*;
use libm::{remainder, remainderf};

// Remainder ------------------------------------------------------------------------

macro_rules! remainder_op {
    ($arg1:expr, $arg2:expr) => {
        remainder($arg1, $arg2)
    };
}
macro_rules! remainder_vec_op {
    ($arg1:expr, $arg2:expr) => {
        remainder($arg1, $arg2)
    };
}
macro_rules! remainderf_op {
    ($arg1:expr, $arg2:expr) => {
        remainderf($arg1, $arg2)
    };
}
macro_rules! remainderf_vec_op {
    ($arg1:expr, $arg2:expr) => {
        remainderf($arg1, $arg2)
    };
}

macro_rules! impl_two_arg_fxn {
    ($struct_name:ident, $element:ty, $kind1:ty, $kind2:ty, $out_kind:ty, $op:ident) => {
        impl_managed_math_binary_full_write!(
            $struct_name,
            $element,
            $kind1,
            $kind2,
            $out_kind,
            $op,
            "math/remainder"
        );
    };
}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(
    RemainderM1F32,
    f32,
    Matrix1<f32>,
    Matrix1<f32>,
    Matrix1<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(
    RemainderM2F32,
    f32,
    Matrix2<f32>,
    Matrix2<f32>,
    Matrix2<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    RemainderM3F32,
    f32,
    Matrix3<f32>,
    Matrix3<f32>,
    Matrix3<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    RemainderM2x3F32,
    f32,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3x2"))]
impl_two_arg_fxn!(
    RemainderM3x2F32,
    f32,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(
    RemainderM4F32,
    f32,
    Matrix4<f32>,
    Matrix4<f32>,
    Matrix4<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(
    RemainderV2F32,
    f32,
    Vector2<f32>,
    Vector2<f32>,
    Vector2<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(
    RemainderV3F32,
    f32,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(
    RemainderV4F32,
    f32,
    Vector4<f32>,
    Vector4<f32>,
    Vector4<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(
    RemainderR2F32,
    f32,
    RowVector2<f32>,
    RowVector2<f32>,
    RowVector2<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(
    RemainderR3F32,
    f32,
    RowVector3<f32>,
    RowVector3<f32>,
    RowVector3<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(
    RemainderR4F32,
    f32,
    RowVector4<f32>,
    RowVector4<f32>,
    RowVector4<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(
    RemainderRDF32,
    f32,
    RowDVector<f32>,
    RowDVector<f32>,
    RowDVector<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(
    RemainderVDF32,
    f32,
    DVector<f32>,
    DVector<f32>,
    DVector<f32>,
    remainderf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(
    RemainderMDF32,
    f32,
    DMatrix<f32>,
    DMatrix<f32>,
    DMatrix<f32>,
    remainderf_vec_op
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(RemainderF32, f32, f32, f32, f32, remainderf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(
    RemainderM1F64,
    f64,
    Matrix1<f64>,
    Matrix1<f64>,
    Matrix1<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(
    RemainderM2F64,
    f64,
    Matrix2<f64>,
    Matrix2<f64>,
    Matrix2<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    RemainderM3F64,
    f64,
    Matrix3<f64>,
    Matrix3<f64>,
    Matrix3<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    RemainderM2x3F64,
    f64,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3x2"))]
impl_two_arg_fxn!(
    RemainderM3x2F64,
    f64,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(
    RemainderM4F64,
    f64,
    Matrix4<f64>,
    Matrix4<f64>,
    Matrix4<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(
    RemainderV2F64,
    f64,
    Vector2<f64>,
    Vector2<f64>,
    Vector2<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(
    RemainderV3F64,
    f64,
    Vector3<f64>,
    Vector3<f64>,
    Vector3<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(
    RemainderV4F64,
    f64,
    Vector4<f64>,
    Vector4<f64>,
    Vector4<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(
    RemainderR2F64,
    f64,
    RowVector2<f64>,
    RowVector2<f64>,
    RowVector2<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(
    RemainderR3F64,
    f64,
    RowVector3<f64>,
    RowVector3<f64>,
    RowVector3<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(
    RemainderR4F64,
    f64,
    RowVector4<f64>,
    RowVector4<f64>,
    RowVector4<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(
    RemainderRDF64,
    f64,
    RowDVector<f64>,
    RowDVector<f64>,
    RowDVector<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(
    RemainderVDF64,
    f64,
    DVector<f64>,
    DVector<f64>,
    DVector<f64>,
    remainder_vec_op
);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(
    RemainderMDF64,
    f64,
    DMatrix<f64>,
    DMatrix<f64>,
    DMatrix<f64>,
    remainder_vec_op
);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(RemainderF64, f64, f64, f64, f64, remainder_op);

impl_canonical_math_same_type_binop_specializer!(MathRemainder, Remainder, "math/remainder");
