use crate::*;
use libm::{yn, ynf};

// Yn ------------------------------------------------------------------------

macro_rules! yn_op {
    ($arg1:expr, $arg2:expr) => {
        yn($arg1 as i32, $arg2)
    };
}
macro_rules! yn_vec_op {
    ($arg1:expr, $arg2:expr) => {
        yn($arg1 as i32, $arg2)
    };
}
macro_rules! ynf_op {
    ($arg1:expr, $arg2:expr) => {
        ynf($arg1 as i32, $arg2)
    };
}
macro_rules! ynf_vec_op {
    ($arg1:expr, $arg2:expr) => {
        ynf($arg1 as i32, $arg2)
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
            "math/yn"
        );
    };
}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(
    YnM1F32,
    f32,
    Matrix1<f32>,
    Matrix1<f32>,
    Matrix1<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(
    YnM2F32,
    f32,
    Matrix2<f32>,
    Matrix2<f32>,
    Matrix2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    YnM3F32,
    f32,
    Matrix3<f32>,
    Matrix3<f32>,
    Matrix3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    YnM2x3F32,
    f32,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3x2"))]
impl_two_arg_fxn!(
    YnM3x2F32,
    f32,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(
    YnM4F32,
    f32,
    Matrix4<f32>,
    Matrix4<f32>,
    Matrix4<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(
    YnV2F32,
    f32,
    Vector2<f32>,
    Vector2<f32>,
    Vector2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(
    YnV3F32,
    f32,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(
    YnV4F32,
    f32,
    Vector4<f32>,
    Vector4<f32>,
    Vector4<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(
    YnR2F32,
    f32,
    RowVector2<f32>,
    RowVector2<f32>,
    RowVector2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(
    YnR3F32,
    f32,
    RowVector3<f32>,
    RowVector3<f32>,
    RowVector3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(
    YnR4F32,
    f32,
    RowVector4<f32>,
    RowVector4<f32>,
    RowVector4<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(
    YnRDF32,
    f32,
    RowDVector<f32>,
    RowDVector<f32>,
    RowDVector<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(
    YnVDF32,
    f32,
    DVector<f32>,
    DVector<f32>,
    DVector<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(
    YnMDF32,
    f32,
    DMatrix<f32>,
    DMatrix<f32>,
    DMatrix<f32>,
    ynf_vec_op
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(YnF32, f32, f32, f32, f32, ynf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(
    YnM1F64,
    f64,
    Matrix1<f64>,
    Matrix1<f64>,
    Matrix1<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(
    YnM2F64,
    f64,
    Matrix2<f64>,
    Matrix2<f64>,
    Matrix2<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    YnM3F64,
    f64,
    Matrix3<f64>,
    Matrix3<f64>,
    Matrix3<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    YnM2x3F64,
    f64,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3x2"))]
impl_two_arg_fxn!(
    YnM3x2F64,
    f64,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(
    YnM4F64,
    f64,
    Matrix4<f64>,
    Matrix4<f64>,
    Matrix4<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(
    YnV2F64,
    f64,
    Vector2<f64>,
    Vector2<f64>,
    Vector2<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(
    YnV3F64,
    f64,
    Vector3<f64>,
    Vector3<f64>,
    Vector3<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(
    YnV4F64,
    f64,
    Vector4<f64>,
    Vector4<f64>,
    Vector4<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(
    YnR2F64,
    f64,
    RowVector2<f64>,
    RowVector2<f64>,
    RowVector2<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(
    YnR3F64,
    f64,
    RowVector3<f64>,
    RowVector3<f64>,
    RowVector3<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(
    YnR4F64,
    f64,
    RowVector4<f64>,
    RowVector4<f64>,
    RowVector4<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(
    YnRDF64,
    f64,
    RowDVector<f64>,
    RowDVector<f64>,
    RowDVector<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(
    YnVDF64,
    f64,
    DVector<f64>,
    DVector<f64>,
    DVector<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(
    YnMDF64,
    f64,
    DMatrix<f64>,
    DMatrix<f64>,
    DMatrix<f64>,
    yn_vec_op
);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(YnF64, f64, f64, f64, f64, yn_op);

impl_canonical_math_same_type_binop_specializer!(MathYn, Yn, "math/bessel/yn");
