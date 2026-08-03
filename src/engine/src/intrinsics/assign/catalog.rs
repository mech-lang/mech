#[cfg(feature = "matrix")]
use super::matrix::*;
use super::*;
use mech_core::{FunctionCatalogBuilder, MResult, MechFunctionFactory};

// Assignment's scalar factories deliberately keep the Rust type, emitted
// runtime spelling, Cargo value feature, and installer token independent.
// `r64` and `c64` are the two cases where those spellings differ.
macro_rules! for_each_assign_scalar_factory {
    ($callback:ident, $context:tt) => {
        $callback!($context; u8; u8; "u8"; "u8");
        $callback!($context; u16; u16; "u16"; "u16");
        $callback!($context; u32; u32; "u32"; "u32");
        $callback!($context; u64; u64; "u64"; "u64");
        $callback!($context; u128; u128; "u128"; "u128");
        $callback!($context; i8; i8; "i8"; "i8");
        $callback!($context; i16; i16; "i16"; "i16");
        $callback!($context; i32; i32; "i32"; "i32");
        $callback!($context; i64; i64; "i64"; "i64");
        $callback!($context; i128; i128; "i128"; "i128");
        $callback!($context; f32; f32; "f32"; "f32");
        $callback!($context; f64; f64; "f64"; "f64");
        $callback!($context; bool; bool; "bool"; "bool");
        $callback!($context; string; String; "string"; "string");
        $callback!($context; r64; R64; "r64"; "rational");
        $callback!($context; c64; C64; "c64"; "complex");
    };
}

macro_rules! declare_assign_scalar_factory {
    ($_context:tt; $installer_token:ident; $scalar:ty; $runtime_name:literal; $cargo_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "assign", feature = $cargo_feature),
                registration: [<register_assign_ $installer_token>],
                installer: [<install_assign_ $installer_token>],
                name: concat!("Assign<", $runtime_name, ">"),
                factory: <Assign<$scalar> as MechFunctionFactory>::new,
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!("mech_engine::__mech_native::", stringify!([<install_assign_ $installer_token>])),
                cargo_features: ["assign", $cargo_feature, "native-link", "runtime"],
            }
        }
    };
}

for_each_assign_scalar_factory!(declare_assign_scalar_factory, ());

mech_core::declare_native_runtime_factory! {
    cfg: feature = "assign",
    registration: register_assign_index,
    installer: install_assign_index,
    name: "Assign<index>",
    factory: <Assign<usize> as MechFunctionFactory>::new,
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_assign_index",
    cargo_features: ["assign", "native-link", "runtime"],
}

macro_rules! register_assign_scalar_factory {
    (($builder:ident); $installer_token:ident; $_scalar:ty; $_runtime_name:literal; $cargo_feature:literal) => {
        #[cfg(all(feature = "assign", feature = $cargo_feature))]
        mech_core::paste::paste! { [<register_assign_ $installer_token>]($builder)?; }
    };
}

macro_rules! export_assign_scalar_factory {
    ($_context:tt; $installer_token:ident; $_scalar:ty; $_runtime_name:literal; $cargo_feature:literal) => {
        #[cfg(all(feature = "assign", feature = $cargo_feature))]
        mech_core::paste::paste! { pub use super::[<install_assign_ $installer_token>]; }
    };
}

// Keep the concrete storage spellings and their Cargo switches in one place.
// The matrix traversal below supplies the exact type list; these mappings only
// translate those already-selected concrete storage types into plan metadata.
macro_rules! assign_matrix_feature {
    (Matrix1) => {
        "matrix1"
    };
    (Matrix2) => {
        "matrix2"
    };
    (Matrix3) => {
        "matrix3"
    };
    (Matrix4) => {
        "matrix4"
    };
    (Matrix2x3) => {
        "matrix2x3"
    };
    (Matrix3x2) => {
        "matrix3x2"
    };
    (RowVector2) => {
        "row_vector2"
    };
    (RowVector3) => {
        "row_vector3"
    };
    (RowVector4) => {
        "row_vector4"
    };
    (Vector2) => {
        "vector2"
    };
    (Vector3) => {
        "vector3"
    };
    (Vector4) => {
        "vector4"
    };
    (RowDVector) => {
        "row_vectord"
    };
    (DVector) => {
        "vectord"
    };
    (DMatrix) => {
        "matrixd"
    };
}

// All three consumers below are fed by the same concrete-factory traversal:
// declarations for native plans, direct runtime registrations, and hidden
// generated-application exports.  This deliberately replaces the historic
// aggregate assignment installer, whose single path could not describe an
// individual factory exactly.
macro_rules! declare_matrix_assign_factory {
    (
        $_context:tt;
        $fxn_name:ident, $scalar:ident, $scalar_name:literal, $scalar_feature:literal,
        [$($shape:ident),+], [$($extra_feature:literal),*], $factory:expr
    ) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "assign", feature = "matrix", feature = $scalar_feature),
                registration: [<register_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>],
                installer: [<install_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>],
                name: concat!(stringify!($fxn_name), "<", $scalar_name, $(stringify!($shape)),*, ">"),
                factory: $factory,
                package: "mech-engine", crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::",
                    stringify!([<install_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>]),
                ),
                cargo_features: [
                    "assign",
                    $scalar_feature,
                    $(assign_matrix_feature!($shape)),*,
                    $($extra_feature,)*
                    "native-link",
                    "runtime",
                ],
            }
        }
    };
}

macro_rules! export_matrix_assign_factory {
    (
        $_context:tt;
        $fxn_name:ident, $scalar:ident, $scalar_name:literal, $scalar_feature:literal,
        [$($shape:ident),+], [$($extra_feature:literal),*], $factory:expr
    ) => {
        mech_core::paste::paste! {
            pub use super::[<install_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>];
        }
    };
}

macro_rules! register_matrix_assign_factory {
    (
        $builder:ident;
        $fxn_name:ident, $scalar:ident, $scalar_name:literal, $scalar_feature:literal,
        [$($shape:ident),+], [$($extra_feature:literal),*], $factory:expr
    ) => {
        mech_core::paste::paste! {
            [<register_assign_ $fxn_name:lower _ $scalar:lower $( _ $shape:lower )*>]($builder)?;
        }
    };
}

macro_rules! install_legacy_assign {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>>::new);
        }
    };
}

macro_rules! install_legacy_assign_s {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<usize>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr_b {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<bool>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr_bu {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<usize>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr_ub {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<bool>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr_b2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<bool>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr_bu2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<usize>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr_ub2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<bool>>::new);
        }
    };
}

macro_rules! install_legacy_assign_srr2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3, $row4], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<usize>>::new);
        }
    };
}

macro_rules! install_legacy_assign_s1 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1], [], $fxn_name::<$scalar,$row1<$scalar>>::new);
        }
    };
}

macro_rules! install_legacy_assign_s2 {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2], [], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>>::new);
        }
    };
}

macro_rules! install_legacy_assign_b {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2, $row3], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>>::new);
        }
    };
}

macro_rules! install_legacy_assign_s_b {
    ($emit:ident, $context:tt, $fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
        mech_core::paste::paste! {
            $emit!($context; $fxn_name, $scalar, $scalar_string, $scalar_string, [$row1, $row2], ["bool"], $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>>::new);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape);
        }
    };
}

macro_rules! install_legacy_impl_assign_all_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape);
            #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name MD>], $value_kind, $value_string, $shape, DMatrix);
        }
    };
}

macro_rules! install_legacy_impl_set_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_set_range_all_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_srr!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix2x3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix3x2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix2x3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix3x2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "matrix2"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, DVector);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord"))]
            install_legacy_assign_srr2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_all_range_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_assign_all_scalar_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowDVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector4);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_all_arms {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DMatrix);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowDVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name S>], $value_kind, $value_string, RowVector4);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector3);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix2x3"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector2);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix3x2"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
        }
    };
}

macro_rules! install_legacy_impl_set_all_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_set_range_all_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_set_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(feature = $value_string)]
            install_legacy_assign_s1!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape);
            #[cfg(feature = $value_string)]
            install_legacy_assign_s2!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, $shape);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_scalar_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_scalar_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $shape:tt, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_s_b!($emit, $context, [<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            install_legacy_assign_b!($emit, $context, [<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms_b {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, RowDVector,Matrix1,DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Vector2,Vector2,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Vector3,Vector3,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Vector4,Vector4,Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DVector,DVector,Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix2,Vector2,Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix2,Matrix3,Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix4,Vector4,Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix2x3,Vector2,Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, Matrix3x2,Vector3,Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_b!($emit, $context, [<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1, Matrix1);
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowVector2, RowVector2, Matrix1, Vector2);
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowVector3, RowVector3, Matrix1, Vector3);
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowVector4, RowVector4, Matrix1, Vector4);
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, RowDVector, RowDVector, Matrix1, DVector);
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Vector2, Vector2, Vector2, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Vector3, Vector3, Vector3, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Vector4, Vector4, Vector4, Matrix1);
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DVector, DVector, DVector, Matrix1);
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector4, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector2, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector2);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector3);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector4);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_b2!($emit, $context, [<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms_bu {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_bu!($emit, $context, [<$fxn_name BU>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_bu2!($emit, $context, [<$fxn_name VBU>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

macro_rules! install_legacy_impl_assign_range_range_arms_ub {
    ($emit:ident, $context:tt, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        mech_core::paste::paste! {
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_ub!($emit, $context, [<$fxn_name UB>], $value_kind, $value_string, DMatrix, DVector, DVector);
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
            install_legacy_assign_srr_ub2!($emit, $context, [<$fxn_name VUB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_sink_shapes {
    ($emit:ident, $context:tt, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        #[cfg(feature = "row_vector2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowVector2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "row_vector3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowVector3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "row_vector4")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowVector4,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vector2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Vector2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vector3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Vector3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vector4")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Vector4,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix1")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix1,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix4")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix4,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix2x3")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix2x3,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrix3x2")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            Matrix3x2,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "matrixd")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            DMatrix,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "row_vectord")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            RowDVector,
            $value_kind,
            $value_string
        );
        #[cfg(feature = "vectord")]
        $arm!(
            $emit,
            $context,
            $fxn_name,
            DVector,
            $value_kind,
            $value_string
        );
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_type {
    (
        $driver:ident,
        $emit:ident,
        $context:tt,
        $arm:ident,
        $fxn_name:ident,
        $value_kind:ident,
        $value_string:tt
    ) => {
        $driver!($emit, $context, $arm, $fxn_name, $value_kind, $value_string);
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_all_types {
    ($driver:ident, $emit:ident, $context:tt, $arm:ident, $fxn_name:ident) => {
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u8, "u8");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u16, "u16");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u32, "u32");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u64, "u64");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, u128, "u128");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i8, "i8");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i16, "i16");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i32, "i32");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i64, "i64");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, i128, "i128");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, f32, "f32");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, f64, "f64");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, R64, "rational");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, C64, "complex");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, bool, "bool");
        install_legacy_for_type!($driver, $emit, $context, $arm, $fxn_name, String, "string");
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_direct {
    ($emit:ident, $context:tt, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {
        $arm!($emit, $context, $fxn_name, $value_kind, $value_string);
    };
}

#[cfg(feature = "matrix")]
macro_rules! for_each_matrix_assignment_factory {
    ($all_types:ident, $one_type:ident, $direct:ident, $emit:ident, $context:tt) => {
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_scalar_arms,
            Assign1D
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_scalar_arms_b,
            Assign1D
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_arms,
            Assign1DR
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            $context,
            install_legacy_impl_set_range_arms_b,
            Assign1DR
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_all_arms,
            Set1DA
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_scalar_scalar_arms,
            Assign2DSS
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            $context,
            install_legacy_impl_assign_all_scalar_arms,
            Assign2DAS
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            $context,
            install_legacy_impl_assign_scalar_all_arms,
            Assign2DSA
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_range_scalar_arms,
            Assign2DRS
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_range_scalar_arms_b,
            Assign2DRS
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_scalar_range_arms,
            Assign2DSR
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_scalar_range_arms_b,
            Assign2DSR
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_range_range_arms,
            Assign2DRR
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            $context,
            install_legacy_impl_assign_range_range_arms_b,
            Assign2DRR
        );
        $direct!(
            $emit,
            $context,
            install_legacy_impl_assign_range_range_arms_bu,
            Assign2DRR,
            f64,
            "f64"
        );
        $direct!(
            $emit,
            $context,
            install_legacy_impl_assign_range_range_arms_ub,
            Assign2DRR,
            f64,
            "f64"
        );
        $all_types!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_assign_all_range_arms,
            Set2DAR
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            $context,
            install_legacy_impl_set_all_range_arms_b,
            Set2DAR
        );

        // Preserve the legacy omission of i128 from the non-boolean range/all path.
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u8,
            "u8"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u16,
            "u16"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u32,
            "u32"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u64,
            "u64"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            u128,
            "u128"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i8,
            "i8"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i16,
            "i16"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i32,
            "i32"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            i64,
            "i64"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            f32,
            "f32"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            f64,
            "f64"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            R64,
            "rational"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            C64,
            "complex"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            bool,
            "bool"
        );
        $one_type!(
            install_legacy_for_sink_shapes,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms,
            Set2DRA,
            String,
            "string"
        );
        $all_types!(
            install_legacy_direct,
            $emit,
            $context,
            install_legacy_impl_set_range_all_arms_b,
            Set2DRA
        );
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_type_runtime {
    (
        $driver:ident,
        $emit:ident,
        $builder:ident,
        $arm:ident,
        $fxn_name:ident,
        $value_kind:ident,
        $value_string:tt
    ) => {{
        #[inline(never)]
        fn install_type(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
            $driver!($emit, builder, $arm, $fxn_name, $value_kind, $value_string);
            Ok(())
        }

        install_type($builder)?;
    }};
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_for_all_types_runtime {
    ($driver:ident, $emit:ident, $builder:ident, $arm:ident, $fxn_name:ident) => {
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, u8, "u8");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, u16, "u16");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, u32, "u32");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, u64, "u64");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, u128, "u128");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, i8, "i8");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, i16, "i16");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, i32, "i32");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, i64, "i64");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, i128, "i128");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, f32, "f32");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, f64, "f64");
        install_legacy_for_type_runtime!(
            $driver, $emit, $builder, $arm, $fxn_name, R64, "rational"
        );
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, C64, "complex");
        install_legacy_for_type_runtime!($driver, $emit, $builder, $arm, $fxn_name, bool, "bool");
        install_legacy_for_type_runtime!(
            $driver, $emit, $builder, $arm, $fxn_name, String, "string"
        );
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_legacy_direct_runtime {
    ($emit:ident, $builder:ident, $arm:ident, $fxn_name:ident, $value_kind:ident, $value_string:tt) => {{
        #[inline(never)]
        fn install_type(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
            $arm!($emit, builder, $fxn_name, $value_kind, $value_string);
            Ok(())
        }

        install_type($builder)?;
    }};
}

#[cfg(feature = "matrix")]
for_each_matrix_assignment_factory!(
    install_legacy_for_all_types,
    install_legacy_for_type,
    install_legacy_direct,
    declare_matrix_assign_factory,
    ()
);

#[cfg(feature = "matrix")]
fn install_matrix_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for_each_matrix_assignment_factory!(
        install_legacy_for_all_types_runtime,
        install_legacy_for_type_runtime,
        install_legacy_direct_runtime,
        register_matrix_assign_factory,
        builder
    );
    Ok(())
}

/// Installs every enabled bytecode factory owned by stable assignment.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    for_each_assign_scalar_factory!(register_assign_scalar_factory, (builder));
    #[cfg(feature = "assign")]
    register_assign_index(builder)?;

    #[cfg(feature = "matrix")]
    install_matrix_runtime(builder)?;
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    for_each_assign_scalar_factory!(export_assign_scalar_factory, ());

    #[cfg(feature = "assign")]
    pub use super::install_assign_index;
    #[cfg(feature = "matrix")]
    for_each_matrix_assignment_factory!(
        install_legacy_for_all_types,
        install_legacy_for_type,
        install_legacy_direct,
        export_matrix_assign_factory,
        ()
    );
}
