use crate::*;
use libm::{jn, jnf};

// Jn ------------------------------------------------------------------------

macro_rules! jn_op {
    ($arg1:expr, $arg2:expr) => {
        jn($arg1 as i32, $arg2)
    };
}
macro_rules! jn_vec_op {
    ($arg1:expr, $arg2:expr) => {
        jn($arg1 as i32, $arg2)
    };
}
macro_rules! jnf_op {
    ($arg1:expr, $arg2:expr) => {
        jnf($arg1 as i32, $arg2)
    };
}
macro_rules! jnf_vec_op {
    ($arg1:expr, $arg2:expr) => {
        jnf($arg1 as i32, $arg2)
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
            "math/jn"
        );
    };
}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(
    JnM1F32,
    f32,
    Matrix1<f32>,
    Matrix1<f32>,
    Matrix1<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(
    JnM2F32,
    f32,
    Matrix2<f32>,
    Matrix2<f32>,
    Matrix2<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    JnM3F32,
    f32,
    Matrix3<f32>,
    Matrix3<f32>,
    Matrix3<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    JnM2x3F32,
    f32,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    JnM3x2F32,
    f32,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(
    JnM4F32,
    f32,
    Matrix4<f32>,
    Matrix4<f32>,
    Matrix4<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(
    JnV2F32,
    f32,
    Vector2<f32>,
    Vector2<f32>,
    Vector2<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(
    JnV3F32,
    f32,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(
    JnV4F32,
    f32,
    Vector4<f32>,
    Vector4<f32>,
    Vector4<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(
    JnR2F32,
    f32,
    RowVector2<f32>,
    RowVector2<f32>,
    RowVector2<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(
    JnR3F32,
    f32,
    RowVector3<f32>,
    RowVector3<f32>,
    RowVector3<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(
    JnR4F32,
    f32,
    RowVector4<f32>,
    RowVector4<f32>,
    RowVector4<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(
    JnRDF32,
    f32,
    RowDVector<f32>,
    RowDVector<f32>,
    RowDVector<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(
    JnVDF32,
    f32,
    DVector<f32>,
    DVector<f32>,
    DVector<f32>,
    jnf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(
    JnMDF32,
    f32,
    DMatrix<f32>,
    DMatrix<f32>,
    DMatrix<f32>,
    jnf_vec_op
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(JnF32, f32, f32, f32, f32, jnf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(
    JnM1F64,
    f64,
    Matrix1<f64>,
    Matrix1<f64>,
    Matrix1<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(
    JnM2F64,
    f64,
    Matrix2<f64>,
    Matrix2<f64>,
    Matrix2<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    JnM3F64,
    f64,
    Matrix3<f64>,
    Matrix3<f64>,
    Matrix3<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    JnM2x3F64,
    f64,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    JnM3x2F64,
    f64,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(
    JnM4F64,
    f64,
    Matrix4<f64>,
    Matrix4<f64>,
    Matrix4<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(
    JnV2F64,
    f64,
    Vector2<f64>,
    Vector2<f64>,
    Vector2<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(
    JnV3F64,
    f64,
    Vector3<f64>,
    Vector3<f64>,
    Vector3<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(
    JnV4F64,
    f64,
    Vector4<f64>,
    Vector4<f64>,
    Vector4<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(
    JnR2F64,
    f64,
    RowVector2<f64>,
    RowVector2<f64>,
    RowVector2<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(
    JnR3F64,
    f64,
    RowVector3<f64>,
    RowVector3<f64>,
    RowVector3<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(
    JnR4F64,
    f64,
    RowVector4<f64>,
    RowVector4<f64>,
    RowVector4<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(
    JnRDF64,
    f64,
    RowDVector<f64>,
    RowDVector<f64>,
    RowDVector<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(
    JnVDF64,
    f64,
    DVector<f64>,
    DVector<f64>,
    DVector<f64>,
    jn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(
    JnMDF64,
    f64,
    DMatrix<f64>,
    DMatrix<f64>,
    DMatrix<f64>,
    jn_vec_op
);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(JnF64, f64, f64, f64, f64, jn_op);

impl_canonical_math_same_type_binop_specializer!(MathJn, Jn, "math/bessel/jn");
