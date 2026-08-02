#[cfg(feature = "matrix")]
use super::matrix::*;
use super::*;
use mech_core::{FunctionCatalogBuilder, MResult, MechFunctionFactory, RuntimeFunctionFactory};
#[cfg(feature = "matrix")]
use paste::paste;
use std::collections::BTreeSet;

macro_rules! install_scalar_assign {
    ($install:expr, $scalar:ty, $scalar_name:literal) => {
        ($install)(
            concat!("Assign<", $scalar_name, ">"),
            <Assign<$scalar> as MechFunctionFactory>::new,
        )?;
    };
}

macro_rules! install_legacy_assign {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_s {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<usize>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr_b {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<bool>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr_bu {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<usize>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr_ub {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<bool>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr_b2 {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<bool>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr_bu2 {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<usize>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr_ub2 {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<bool>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_srr2 {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<usize>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_s1 {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), ">"), $fxn_name::<$scalar,$row1<$scalar>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_s2 {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_b {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>>::new)?;
        }
    };
}

macro_rules! install_legacy_assign_s_b {
    ($install:expr, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        paste! {
            ($install)(concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), ">"), $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>>::new)?;
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, $shape);
        }
    };
}

macro_rules! install_legacy_impl_assign_all_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, $shape);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_scalar_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, $shape);
            #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
            install_legacy_assign_s!($install, [<$fxn_name MD>], $value_kind, $value_string, $shape, DMatrix);
        }
    };
}

macro_rules! install_legacy_impl_set_range_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_set_range_all_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_scalar_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_range_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_srr!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix2x3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix3x2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix2x3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix3x2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "matrix2"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_all_range_arms {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($install, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($install, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_assign_all_scalar_arms {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowDVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowVector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowVector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowVector4);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2x3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3x2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_all_arms {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowDVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowVector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowVector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s1!($install, [<$fxn_name S>], $value_kind, $value_string, RowVector4);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix2x3"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix3x2"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($install, [<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
        }
    };
}

macro_rules! install_legacy_impl_set_all_range_arms_b {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_set_range_all_arms_b {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_set_range_arms_b {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_arms_b {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($install, [<$fxn_name B>], $value_kind, $value_string, $shape);
            #[cfg(feature = $value_string)]
            install_legacy_assign_s2!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, $shape);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_scalar_arms_b {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_range_arms_b {
    ($install:expr, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($install, [<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($install, [<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms_b {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, RowDVector,Matrix1,DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Vector2,Vector2,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Vector3,Vector3,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Vector4,Vector4,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DVector,DVector,Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Matrix2,Vector2,Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Matrix2,Matrix3,Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Matrix4,Vector4,Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Matrix2x3,Vector2,Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, Matrix3x2,Vector3,Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_b!($install, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, RowVector2, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, RowVector3, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, RowVector4, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, RowDVector, RowDVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Vector2, Vector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Vector3, Vector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Vector4, Vector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DVector, DVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_b2!($install, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms_bu {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_bu!($install, [<$fxn_name BU>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_bu2!($install, [<$fxn_name VBU>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms_ub {
    ($install:expr, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        paste! {
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_ub!($install, [<$fxn_name UB>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_ub2!($install, [<$fxn_name VUB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_sink_shapes {
    ($install:expr, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        #[cfg(feature = "row_vector2")]
        $arm!($install, $fxn_name, RowVector2, $value_kind, $value_string);
        #[cfg(feature = "row_vector3")]
        $arm!($install, $fxn_name, RowVector3, $value_kind, $value_string);
        #[cfg(feature = "row_vector4")]
        $arm!($install, $fxn_name, RowVector4, $value_kind, $value_string);
        #[cfg(feature = "vector2")]
        $arm!($install, $fxn_name, Vector2, $value_kind, $value_string);
        #[cfg(feature = "vector3")]
        $arm!($install, $fxn_name, Vector3, $value_kind, $value_string);
        #[cfg(feature = "vector4")]
        $arm!($install, $fxn_name, Vector4, $value_kind, $value_string);
        #[cfg(feature = "matrix1")]
        $arm!($install, $fxn_name, Matrix1, $value_kind, $value_string);
        #[cfg(feature = "matrix2")]
        $arm!($install, $fxn_name, Matrix2, $value_kind, $value_string);
        #[cfg(feature = "matrix3")]
        $arm!($install, $fxn_name, Matrix3, $value_kind, $value_string);
        #[cfg(feature = "matrix4")]
        $arm!($install, $fxn_name, Matrix4, $value_kind, $value_string);
        #[cfg(feature = "matrix2x3")]
        $arm!($install, $fxn_name, Matrix2x3, $value_kind, $value_string);
        #[cfg(feature = "matrix3x2")]
        $arm!($install, $fxn_name, Matrix3x2, $value_kind, $value_string);
        #[cfg(feature = "matrixd")]
        $arm!($install, $fxn_name, DMatrix, $value_kind, $value_string);
        #[cfg(feature = "row_vectord")]
        $arm!($install, $fxn_name, RowDVector, $value_kind, $value_string);
        #[cfg(feature = "vectord")]
        $arm!($install, $fxn_name, DVector, $value_kind, $value_string);
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_type {
    (
        $driver:ident,
        $install:expr,
        $arm:ident,
        $fxn_name:ident,
        $value_kind:ident,
        $value_string:tt
    ) => {{
        #[inline(never)]
        fn install_type(
            install: &mut impl FnMut(&'static str, RuntimeFunctionFactory) -> MResult<()>,
        ) -> MResult<()> {
            $driver!(install, $arm, $fxn_name, $value_kind, $value_string);
            Ok(())
        }

        install_type($install)?;
    }};
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_all_types {
    ($driver:ident, $install:expr, $arm:ident, $fxn_name:ident) => {
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, u8, "u8");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, u16, "u16");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, u32, "u32");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, u64, "u64");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, u128, "u128");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, i8, "i8");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, i16, "i16");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, i32, "i32");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, i64, "i64");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, i128, "i128");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, f32, "f32");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, f64, "f64");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, R64, "rational");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, C64, "complex");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, bool, "bool");
        install_legacy_for_type!($driver, $install, $arm, $fxn_name, String, "string");
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_direct {
    ($install:expr, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        $arm!($install, $fxn_name, $value_kind, $value_string);
    };
}

#[cfg(feature = "matrix")]
fn install_matrix_runtime(
    install: &mut impl FnMut(&'static str, RuntimeFunctionFactory) -> MResult<()>,
) -> MResult<()> {
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_scalar_arms,
        Assign1D
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_scalar_arms_b,
        Assign1D
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_arms,
        Assign1DR
    );
    install_legacy_for_all_types!(
        install_legacy_direct,
        install,
        install_legacy_impl_set_range_arms_b,
        Assign1DR
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_all_arms,
        Set1DA
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_scalar_scalar_arms,
        Assign2DSS
    );
    install_legacy_for_all_types!(
        install_legacy_direct,
        install,
        install_legacy_impl_assign_all_scalar_arms,
        Assign2DAS
    );
    install_legacy_for_all_types!(
        install_legacy_direct,
        install,
        install_legacy_impl_assign_scalar_all_arms,
        Assign2DSA
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_range_scalar_arms,
        Assign2DRS
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_range_scalar_arms_b,
        Assign2DRS
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_scalar_range_arms,
        Assign2DSR
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_scalar_range_arms_b,
        Assign2DSR
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_range_range_arms,
        Assign2DRR
    );
    install_legacy_for_all_types!(
        install_legacy_direct,
        install,
        install_legacy_impl_assign_range_range_arms_b,
        Assign2DRR
    );
    install_legacy_direct!(
        install,
        install_legacy_impl_assign_range_range_arms_bu,
        Assign2DRR,
        f64,
        "f64"
    );
    install_legacy_direct!(
        install,
        install_legacy_impl_assign_range_range_arms_ub,
        Assign2DRR,
        f64,
        "f64"
    );
    install_legacy_for_all_types!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_assign_all_range_arms,
        Set2DAR
    );
    install_legacy_for_all_types!(
        install_legacy_direct,
        install,
        install_legacy_impl_set_all_range_arms_b,
        Set2DAR
    );

    // Preserve the legacy omission of i128 from the non-boolean range/all path.
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        u8,
        "u8"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        u16,
        "u16"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        u32,
        "u32"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        u64,
        "u64"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        u128,
        "u128"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        i8,
        "i8"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        i16,
        "i16"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        i32,
        "i32"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        i64,
        "i64"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        f32,
        "f32"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        f64,
        "f64"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        R64,
        "rational"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        C64,
        "complex"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        bool,
        "bool"
    );
    install_legacy_for_type!(
        install_legacy_for_sink_shapes,
        install,
        install_legacy_impl_set_range_all_arms,
        Set2DRA,
        String,
        "string"
    );
    install_legacy_for_all_types!(
        install_legacy_direct,
        install,
        install_legacy_impl_set_range_all_arms_b,
        Set2DRA
    );
    Ok(())
}

/// Installs every enabled bytecode factory owned by stable assignment.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let mut installed = BTreeSet::new();
    let mut install = |name: &'static str, factory: RuntimeFunctionFactory| -> MResult<()> {
        if installed.insert(name) {
            builder.insert_runtime_factory(name, factory)
        } else {
            Ok(())
        }
    };

    #[cfg(feature = "u8")]
    install_scalar_assign!(install, u8, "u8");
    #[cfg(feature = "u16")]
    install_scalar_assign!(install, u16, "u16");
    #[cfg(feature = "u32")]
    install_scalar_assign!(install, u32, "u32");
    #[cfg(feature = "u64")]
    install_scalar_assign!(install, u64, "u64");
    #[cfg(feature = "u128")]
    install_scalar_assign!(install, u128, "u128");
    #[cfg(feature = "i8")]
    install_scalar_assign!(install, i8, "i8");
    #[cfg(feature = "i16")]
    install_scalar_assign!(install, i16, "i16");
    #[cfg(feature = "i32")]
    install_scalar_assign!(install, i32, "i32");
    #[cfg(feature = "i64")]
    install_scalar_assign!(install, i64, "i64");
    #[cfg(feature = "i128")]
    install_scalar_assign!(install, i128, "i128");
    #[cfg(feature = "f32")]
    install_scalar_assign!(install, f32, "f32");
    #[cfg(feature = "f64")]
    install_scalar_assign!(install, f64, "f64");
    #[cfg(feature = "bool")]
    install_scalar_assign!(install, bool, "bool");
    #[cfg(feature = "string")]
    install_scalar_assign!(install, String, "string");
    #[cfg(feature = "r64")]
    install_scalar_assign!(install, R64, "r64");
    #[cfg(feature = "c64")]
    install_scalar_assign!(install, C64, "c64");
    install_scalar_assign!(install, usize, "index");

    #[cfg(feature = "matrix")]
    install_matrix_runtime(&mut install)?;
    Ok(())
}
