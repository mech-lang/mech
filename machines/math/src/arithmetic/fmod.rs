use crate::*;
use libm::{fmod, fmodf};

// Fmod ------------------------------------------------------------------------

macro_rules! fmod_op {
    ($arg1:expr, $arg2:expr) => {
        fmod($arg1, $arg2)
    };
}
macro_rules! fmod_vec_op {
    ($arg1:expr, $arg2:expr) => {
        fmod($arg1, $arg2)
    };
}
macro_rules! fmodf_op {
    ($arg1:expr, $arg2:expr) => {
        fmodf($arg1, $arg2)
    };
}
macro_rules! fmodf_vec_op {
    ($arg1:expr, $arg2:expr) => {
        fmodf($arg1, $arg2)
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
            "math/fmod"
        );
    };
}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(
    FmodM1F32,
    f32,
    Matrix1<f32>,
    Matrix1<f32>,
    Matrix1<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(
    FmodM2F32,
    f32,
    Matrix2<f32>,
    Matrix2<f32>,
    Matrix2<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    FmodM3F32,
    f32,
    Matrix3<f32>,
    Matrix3<f32>,
    Matrix3<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    FmodM2x3F32,
    f32,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3x2"))]
impl_two_arg_fxn!(
    FmodM3x2F32,
    f32,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(
    FmodM4F32,
    f32,
    Matrix4<f32>,
    Matrix4<f32>,
    Matrix4<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(
    FmodV2F32,
    f32,
    Vector2<f32>,
    Vector2<f32>,
    Vector2<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(
    FmodV3F32,
    f32,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(
    FmodV4F32,
    f32,
    Vector4<f32>,
    Vector4<f32>,
    Vector4<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(
    FmodR2F32,
    f32,
    RowVector2<f32>,
    RowVector2<f32>,
    RowVector2<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(
    FmodR3F32,
    f32,
    RowVector3<f32>,
    RowVector3<f32>,
    RowVector3<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(
    FmodR4F32,
    f32,
    RowVector4<f32>,
    RowVector4<f32>,
    RowVector4<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(
    FmodRDF32,
    f32,
    RowDVector<f32>,
    RowDVector<f32>,
    RowDVector<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(
    FmodVDF32,
    f32,
    DVector<f32>,
    DVector<f32>,
    DVector<f32>,
    fmodf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(
    FmodMDF32,
    f32,
    DMatrix<f32>,
    DMatrix<f32>,
    DMatrix<f32>,
    fmodf_vec_op
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(FmodF32, f32, f32, f32, f32, fmodf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(
    FmodM1F64,
    f64,
    Matrix1<f64>,
    Matrix1<f64>,
    Matrix1<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(
    FmodM2F64,
    f64,
    Matrix2<f64>,
    Matrix2<f64>,
    Matrix2<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    FmodM3F64,
    f64,
    Matrix3<f64>,
    Matrix3<f64>,
    Matrix3<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    FmodM2x3F64,
    f64,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3x2"))]
impl_two_arg_fxn!(
    FmodM3x2F64,
    f64,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(
    FmodM4F64,
    f64,
    Matrix4<f64>,
    Matrix4<f64>,
    Matrix4<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(
    FmodV2F64,
    f64,
    Vector2<f64>,
    Vector2<f64>,
    Vector2<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(
    FmodV3F64,
    f64,
    Vector3<f64>,
    Vector3<f64>,
    Vector3<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(
    FmodV4F64,
    f64,
    Vector4<f64>,
    Vector4<f64>,
    Vector4<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(
    FmodR2F64,
    f64,
    RowVector2<f64>,
    RowVector2<f64>,
    RowVector2<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(
    FmodR3F64,
    f64,
    RowVector3<f64>,
    RowVector3<f64>,
    RowVector3<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(
    FmodR4F64,
    f64,
    RowVector4<f64>,
    RowVector4<f64>,
    RowVector4<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(
    FmodRDF64,
    f64,
    RowDVector<f64>,
    RowDVector<f64>,
    RowDVector<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(
    FmodVDF64,
    f64,
    DVector<f64>,
    DVector<f64>,
    DVector<f64>,
    fmod_vec_op
);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(
    FmodMDF64,
    f64,
    DMatrix<f64>,
    DMatrix<f64>,
    DMatrix<f64>,
    fmod_vec_op
);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(FmodF64, f64, f64, f64, f64, fmod_op);

impl_canonical_math_same_type_binop_specializer!(MathFmod, Fmod, "math/fmod");
