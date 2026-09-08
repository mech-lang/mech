use crate::*;
use libm::{fdim, fdimf};

// Fdim ------------------------------------------------------------------------

macro_rules! fdim_op {
    ($arg1:expr, $arg2:expr) => {
        fdim($arg1, $arg2)
    };
}

macro_rules! fdim_vec_op {
    ($arg1:expr, $arg2:expr) => {
        fdim($arg1, $arg2)
    };
}

macro_rules! fdimf_op {
    ($arg1:expr, $arg2:expr) => {
        fdimf($arg1, $arg2)
    };
}

macro_rules! fdimf_vec_op {
    ($arg1:expr, $arg2:expr) => {
        fdimf($arg1, $arg2)
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
            "math/fdim"
        );
    };
}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(
    FdimM1F32,
    f32,
    Matrix1<f32>,
    Matrix1<f32>,
    Matrix1<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(
    FdimM2F32,
    f32,
    Matrix2<f32>,
    Matrix2<f32>,
    Matrix2<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    FdimM3F32,
    f32,
    Matrix3<f32>,
    Matrix3<f32>,
    Matrix3<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    FdimM2x3F32,
    f32,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    FdimM3x2F32,
    f32,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(
    FdimM4F32,
    f32,
    Matrix4<f32>,
    Matrix4<f32>,
    Matrix4<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(
    FdimV2F32,
    f32,
    Vector2<f32>,
    Vector2<f32>,
    Vector2<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(
    FdimV3F32,
    f32,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(
    FdimV4F32,
    f32,
    Vector4<f32>,
    Vector4<f32>,
    Vector4<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(
    FdimR2F32,
    f32,
    RowVector2<f32>,
    RowVector2<f32>,
    RowVector2<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(
    FdimR3F32,
    f32,
    RowVector3<f32>,
    RowVector3<f32>,
    RowVector3<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(
    FdimR4F32,
    f32,
    RowVector4<f32>,
    RowVector4<f32>,
    RowVector4<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(
    FdimRDF32,
    f32,
    RowDVector<f32>,
    RowDVector<f32>,
    RowDVector<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(
    FdimVDF32,
    f32,
    DVector<f32>,
    DVector<f32>,
    DVector<f32>,
    fdimf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(
    FdimMDF32,
    f32,
    DMatrix<f32>,
    DMatrix<f32>,
    DMatrix<f32>,
    fdimf_vec_op
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(FdimF32, f32, f32, f32, f32, fdimf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(
    FdimM1F64,
    f64,
    Matrix1<f64>,
    Matrix1<f64>,
    Matrix1<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(
    FdimM2F64,
    f64,
    Matrix2<f64>,
    Matrix2<f64>,
    Matrix2<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    FdimM3F64,
    f64,
    Matrix3<f64>,
    Matrix3<f64>,
    Matrix3<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    FdimM2x3F64,
    f64,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    FdimM3x2F64,
    f64,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(
    FdimM4F64,
    f64,
    Matrix4<f64>,
    Matrix4<f64>,
    Matrix4<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(
    FdimV2F64,
    f64,
    Vector2<f64>,
    Vector2<f64>,
    Vector2<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(
    FdimV3F64,
    f64,
    Vector3<f64>,
    Vector3<f64>,
    Vector3<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(
    FdimV4F64,
    f64,
    Vector4<f64>,
    Vector4<f64>,
    Vector4<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(
    FdimR2F64,
    f64,
    RowVector2<f64>,
    RowVector2<f64>,
    RowVector2<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(
    FdimR3F64,
    f64,
    RowVector3<f64>,
    RowVector3<f64>,
    RowVector3<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(
    FdimR4F64,
    f64,
    RowVector4<f64>,
    RowVector4<f64>,
    RowVector4<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(
    FdimRDF64,
    f64,
    RowDVector<f64>,
    RowDVector<f64>,
    RowDVector<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(
    FdimVDF64,
    f64,
    DVector<f64>,
    DVector<f64>,
    DVector<f64>,
    fdim_vec_op
);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(
    FdimMDF64,
    f64,
    DMatrix<f64>,
    DMatrix<f64>,
    DMatrix<f64>,
    fdim_vec_op
);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(FdimF64, f64, f64, f64, f64, fdim_op);

impl_canonical_math_same_type_binop_specializer!(MathFdim, Fdim, "math/fdim");
