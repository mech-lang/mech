use crate::*;
use num_traits::*;
fn checked_runtime_add<T: RuntimeCheckedArithmetic>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_add(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("addition"))
}

// Add ------------------------------------------------------------------------

macro_rules! add_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_add(*$lhs, *$rhs)?;
            *$out = next;
        }
    };
}

#[cfg(any(
    feature = "matrix1",
    feature = "matrix2",
    feature = "matrix3",
    feature = "matrix4",
    feature = "matrix2x3",
    feature = "matrix3x2",
    feature = "matrixd",
    feature = "row_vector2",
    feature = "row_vector3",
    feature = "row_vector4",
    feature = "row_vectord",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord"
))]
macro_rules! add_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for (output, (lhs, rhs)) in next.iter_mut().zip((*$lhs).iter().zip((*$rhs).iter())) {
                *output = checked_runtime_add(*lhs, *rhs)?;
            }
            *$out = next;
        }
    };
}

// A dynamic row-vector x dynamic vector has a feature-invariant Matrix1
// result. Preserve that representation while still allowing it to compose
// reactively with a source-level 1x1 dynamic matrix. The output remains
// dynamic because the other operand owns the broader storage contract.
#[cfg(all(
    feature = "matrixd",
    any(feature = "matrix1", feature = "matrix1_interop")
))]
macro_rules! add_m1_md_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_add((&*$lhs)[(0, 0)], (&*$rhs)[(0, 0)])?;
            (&mut *$out)[(0, 0)] = next;
        }
    };
}

#[cfg(all(
    feature = "matrixd",
    any(feature = "matrix1", feature = "matrix1_interop")
))]
macro_rules! add_md_m1_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_add((&*$lhs)[(0, 0)], (&*$rhs)[(0, 0)])?;
            (&mut *$out)[(0, 0)] = next;
        }
    };
}

#[cfg(any(
    all(feature = "matrix2", feature = "vector2"),
    all(feature = "matrix3", feature = "vector3"),
    all(feature = "matrix4", feature = "vector4"),
    all(feature = "matrix2x3", feature = "vector2"),
    all(feature = "matrix3x2", feature = "vector3"),
    all(feature = "matrixd", feature = "vectord"),
    all(feature = "matrixd", feature = "vector2"),
    all(feature = "matrixd", feature = "vector3"),
    all(feature = "matrixd", feature = "vector4")
))]
macro_rules! add_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in next.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_add(lhs_col[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

#[cfg(any(
    all(feature = "vector2", feature = "matrix2"),
    all(feature = "vector3", feature = "matrix3"),
    all(feature = "vector4", feature = "matrix4"),
    all(feature = "vector2", feature = "matrix2x3"),
    all(feature = "vector3", feature = "matrix3x2"),
    all(feature = "vectord", feature = "matrixd"),
    all(feature = "vector2", feature = "matrixd"),
    all(feature = "vector3", feature = "matrixd"),
    all(feature = "vector4", feature = "matrixd")
))]
macro_rules! add_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in next.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = checked_runtime_add(lhs_deref[i], rhs_col[i])?;
                }
            }
            *$out = next;
        }
    };
}

#[cfg(any(
    all(feature = "matrix2", feature = "row_vector2"),
    all(feature = "matrix3", feature = "row_vector3"),
    all(feature = "matrix4", feature = "row_vector4"),
    all(feature = "matrix2x3", feature = "row_vector3"),
    all(feature = "matrix3x2", feature = "row_vector2"),
    all(feature = "matrixd", feature = "row_vectord"),
    all(feature = "matrixd", feature = "row_vector2"),
    all(feature = "matrixd", feature = "row_vector3"),
    all(feature = "matrixd", feature = "row_vector4")
))]
macro_rules! add_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in next.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_add(lhs_row[i], rhs_deref[i])?;
                }
            }
            *$out = next;
        }
    };
}

#[cfg(any(
    all(feature = "row_vector2", feature = "matrix2"),
    all(feature = "row_vector3", feature = "matrix3"),
    all(feature = "row_vector4", feature = "matrix4"),
    all(feature = "row_vector3", feature = "matrix2x3"),
    all(feature = "row_vector2", feature = "matrix3x2"),
    all(feature = "row_vectord", feature = "matrixd"),
    all(feature = "row_vector2", feature = "matrixd"),
    all(feature = "row_vector3", feature = "matrixd"),
    all(feature = "row_vector4", feature = "matrixd")
))]
macro_rules! add_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in next.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = checked_runtime_add(lhs_deref[i], rhs_row[i])?;
                }
            }
            *$out = next;
        }
    };
}

#[cfg(any(
    feature = "matrix1",
    feature = "matrix2",
    feature = "matrix3",
    feature = "matrix4",
    feature = "matrix2x3",
    feature = "matrix3x2",
    feature = "matrixd",
    feature = "row_vector2",
    feature = "row_vector3",
    feature = "row_vector4",
    feature = "row_vectord",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord"
))]
macro_rules! add_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for (output, lhs) in next.iter_mut().zip((*$lhs).iter()) {
                *output = checked_runtime_add(*lhs, *$rhs)?;
            }
            *$out = next;
        }
    };
}

#[cfg(any(
    feature = "matrix1",
    feature = "matrix2",
    feature = "matrix3",
    feature = "matrix4",
    feature = "matrix2x3",
    feature = "matrix3x2",
    feature = "matrixd",
    feature = "row_vector2",
    feature = "row_vector3",
    feature = "row_vector4",
    feature = "row_vectord",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord"
))]
macro_rules! add_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut next = (*$out).clone();
            for (output, rhs) in next.iter_mut().zip((*$rhs).iter()) {
                *output = checked_runtime_add(*$lhs, *rhs)?;
            }
            *$out = next;
        }
    };
}

macro_rules! impl_checked_add_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_checked_arithmetic_binop!(
            $struct_name,
            $arg1_type,
            $arg2_type,
            $out_type,
            $op,
            crate::ops::arithmetic_full_write_contract
        );
    };
}

impl_fxns!(Add, T, T, impl_checked_add_binop);

#[cfg(all(
    feature = "matrixd",
    any(feature = "matrix1", feature = "matrix1_interop")
))]
impl_checked_add_binop!(AddM1MD, Matrix1<T>, DMatrix<T>, DMatrix<T>, add_m1_md_op);
#[cfg(all(
    feature = "matrixd",
    any(feature = "matrix1", feature = "matrix1_interop")
))]
impl_checked_add_binop!(AddMDM1, DMatrix<T>, Matrix1<T>, DMatrix<T>, add_md_m1_op);

#[cfg(all(test, feature = "u8"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_addition_rejects_reactive_overflow_and_retains_output() {
        let rhs = Ref::new(1_u8);
        let out = Ref::new(17_u8);
        let function = AddSS {
            lhs: Ref::new(40_u8),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 41);
        assert_eq!(
            function.reactive_output_cell_ids(),
            vec![out.reactive_cell_id()],
        );

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&function)?;
            *rhs.borrow_mut() = u8::MAX;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathArithmeticOverflow");
            assert_eq!(*out.borrow(), 41);
            *out.borrow_mut() = 99;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), 41);
    }
}

#[cfg(all(test, feature = "f64", feature = "matrix2", feature = "matrixd"))]
mod state_port_tests {
    use super::*;

    #[test]
    fn fixed_and_dynamic_add_outputs_restore_through_typed_state_ports() {
        let fixed_out = Ref::new(Matrix2::from_element(0.0_f64));
        let fixed = AddM2M2 {
            lhs: Ref::new(Matrix2::from_element(1.0)),
            rhs: Ref::new(Matrix2::from_element(2.0)),
            out: fixed_out.clone(),
        };
        fixed.solve_result().unwrap();

        let dynamic_out = Ref::new(DMatrix::from_element(1, 2, 0.0_f64));
        let dynamic = AddMDMD {
            lhs: Ref::new(DMatrix::from_element(1, 2, 3.0)),
            rhs: Ref::new(DMatrix::from_element(1, 2, 4.0)),
            out: dynamic_out.clone(),
        };
        dynamic.solve_result().unwrap();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&fixed)?;
            participant.capture_function_state(&dynamic)?;
            *fixed_out.borrow_mut() = Matrix2::from_element(9.0);
            *dynamic_out.borrow_mut() = DMatrix::from_element(2, 1, 9.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert_eq!(*fixed_out.borrow(), Matrix2::from_element(3.0));
        assert_eq!(*dynamic_out.borrow(), DMatrix::from_element(1, 2, 7.0));
    }
}

macro_rules! declare_add_matrix1_dynamic_native_factories {
    ($scalar_feature:literal, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "add",
                    feature = $scalar_feature,
                    feature = "matrixd",
                    any(feature = "matrix1", feature = "matrix1_interop")
                ),
                registration: [<register_add_m1_md_ $scalar_token>],
                installer: [<install_add_m1_md_ $scalar_token>],
                name: concat!("AddM1MD<", $scalar_name, ">"),
                factory_type: AddM1MD<$scalar>,
                contract: RuntimeFunctionContract::same_shape(
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
                operations: [mech_core::OperationId::from_name("math/add")],
                package: "mech-math",
                crate_name: "mech_math",
                installer_path: concat!(
                    "mech_math::__mech_native::",
                    stringify!([<install_add_m1_md_ $scalar_token>])
                ),
                extra_cargo_features: ["add"],
            }
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "add",
                    feature = $scalar_feature,
                    feature = "matrixd",
                    any(feature = "matrix1", feature = "matrix1_interop")
                ),
                registration: [<register_add_md_m1_ $scalar_token>],
                installer: [<install_add_md_m1_ $scalar_token>],
                name: concat!("AddMDM1<", $scalar_name, ">"),
                factory_type: AddMDM1<$scalar>,
                contract: RuntimeFunctionContract::same_shape(
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
                operations: [mech_core::OperationId::from_name("math/add")],
                package: "mech-math",
                crate_name: "mech_math",
                installer_path: concat!(
                    "mech_math::__mech_native::",
                    stringify!([<install_add_md_m1_ $scalar_token>])
                ),
                extra_cargo_features: ["add"],
            }
        }
    };
}

declare_add_matrix1_dynamic_native_factories!("i8", i8, "i8", i8);
declare_add_matrix1_dynamic_native_factories!("i16", i16, "i16", i16);
declare_add_matrix1_dynamic_native_factories!("i32", i32, "i32", i32);
declare_add_matrix1_dynamic_native_factories!("i64", i64, "i64", i64);
declare_add_matrix1_dynamic_native_factories!("i128", i128, "i128", i128);
declare_add_matrix1_dynamic_native_factories!("u8", u8, "u8", u8);
declare_add_matrix1_dynamic_native_factories!("u16", u16, "u16", u16);
declare_add_matrix1_dynamic_native_factories!("u32", u32, "u32", u32);
declare_add_matrix1_dynamic_native_factories!("u64", u64, "u64", u64);
declare_add_matrix1_dynamic_native_factories!("u128", u128, "u128", u128);
declare_add_matrix1_dynamic_native_factories!("f32", f32, "f32", f32);
declare_add_matrix1_dynamic_native_factories!("f64", f64, "f64", f64);
declare_add_matrix1_dynamic_native_factories!("rational", R64, "rational", r64);
declare_add_matrix1_dynamic_native_factories!("complex", C64, "complex", c64);

#[cfg(feature = "f64")]
macro_rules! declare_add_f64_native_runtime_factory {
    ($_context:tt, $lib:ident, $suffix:ident, $_shape_feature:tt, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "add", feature = "f64"),

                registration: [<register_add_ $suffix:lower _f64>],
                installer: [<install_add_ $suffix:lower _f64>],

                name: concat!("Add", stringify!($suffix), "<", $scalar_name, ">"),
                factory_type: [<Add $suffix>]<$scalar>,
                contract: mech_core::__mech_elementwise_binop_contract!($suffix),
                operations: [mech_core::OperationId::from_name("math/add")],

                package: "mech-math",
                crate_name: "mech_math",
                installer_path: concat!(
                    "mech_math::__mech_native::",
                    stringify!([<install_add_ $suffix:lower _f64>])
                ),

                extra_cargo_features: ["add"],
            }
        }
    };
}

#[cfg(feature = "f64")]
macro_rules! register_add_f64_native_runtime_factory {
    ($builder:ident, $lib:ident, $suffix:ident, $_shape_feature:tt, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        paste::paste! {
            [<register_add_ $suffix:lower _f64>]($builder)?;
        }
    };
}

#[cfg(feature = "f64")]
mech_core::__mech_for_each_binop_runtime_factory_for_type!(
    declare_add_f64_native_runtime_factory,
    (),
    Add,
    f64,
    "f64",
    f64
);

mech_core::declare_native_binop_runtime_factories! {
    package: "mech-math",
    crate_name: "mech_math",
    operation: Add,
    canonical_operation: "math/add",
    operation_feature: "add",
    additional_features: [],
    scalars:
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("rational", R64, "r64", r64),
        ("complex", C64, "c64", c64),
}

#[cfg(all(
    feature = "native-plan",
    feature = "matrixd",
    any(feature = "matrix1", feature = "matrix1_interop")
))]
macro_rules! register_add_matrix1_dynamic_native_factories {
    ($builder:expr; $scalar_feature:literal, $scalar_token:ident) => {
        #[cfg(all(
                    feature = $scalar_feature,
                    feature = "matrixd",
                    any(feature = "matrix1", feature = "matrix1_interop")
                ))]
        paste! {
            [<register_add_m1_md_ $scalar_token>]($builder)?;
            [<register_add_md_m1_ $scalar_token>]($builder)?;
        }
    };
}

impl_canonical_registered_math_binop_specializer!(MathAdd, "Add");

#[cfg(feature = "f64")]
fn install_add_f64_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    mech_core::__mech_for_each_binop_runtime_factory_for_type!(
        register_add_f64_native_runtime_factory,
        builder,
        Add,
        f64,
        "f64",
        f64
    );
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "add",
        operation: Add;
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("f64", f64, "f64", f64),
        ("rational", R64, "r64", r64),
        ("complex", C64, "c64", c64),
    }

    macro_rules! export_add_matrix1_dynamic_native_factories {
        ($scalar_feature:literal, $scalar_token:ident) => {
            #[cfg(all(
                            feature = $scalar_feature,
                            feature = "matrixd",
                            any(feature = "matrix1", feature = "matrix1_interop")
                        ))]
            mech_core::paste::paste! {
                pub use super::[<install_add_m1_md_ $scalar_token>];
                pub use super::[<install_add_md_m1_ $scalar_token>];
            }
        };
    }

    export_add_matrix1_dynamic_native_factories!("i8", i8);
    export_add_matrix1_dynamic_native_factories!("i16", i16);
    export_add_matrix1_dynamic_native_factories!("i32", i32);
    export_add_matrix1_dynamic_native_factories!("i64", i64);
    export_add_matrix1_dynamic_native_factories!("i128", i128);
    export_add_matrix1_dynamic_native_factories!("u8", u8);
    export_add_matrix1_dynamic_native_factories!("u16", u16);
    export_add_matrix1_dynamic_native_factories!("u32", u32);
    export_add_matrix1_dynamic_native_factories!("u64", u64);
    export_add_matrix1_dynamic_native_factories!("u128", u128);
    export_add_matrix1_dynamic_native_factories!("f32", f32);
    export_add_matrix1_dynamic_native_factories!("f64", f64);
    export_add_matrix1_dynamic_native_factories!("rational", r64);
    export_add_matrix1_dynamic_native_factories!("complex", c64);
}

pub fn install_math_add_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Add;
        ("i8", i8, "i8", i8),
        ("i16", i16, "i16", i16),
        ("i32", i32, "i32", i32),
        ("i64", i64, "i64", i64),
        ("i128", i128, "i128", i128),
        ("u8", u8, "u8", u8),
        ("u16", u16, "u16", u16),
        ("u32", u32, "u32", u32),
        ("u64", u64, "u64", u64),
        ("u128", u128, "u128", u128),
        ("f32", f32, "f32", f32),
        ("rational", R64, "r64", r64),
        ("complex", C64, "c64", c64),
    )?;
    #[cfg(feature = "f64")]
    install_add_f64_runtime(builder)?;
    Ok(())
}

#[cfg(all(
    feature = "native-plan",
    feature = "matrixd",
    any(feature = "matrix1", feature = "matrix1_interop")
))]
pub fn install_math_add_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_add_matrix1_dynamic_native_factories!(builder; "i8", i8);
    register_add_matrix1_dynamic_native_factories!(builder; "i16", i16);
    register_add_matrix1_dynamic_native_factories!(builder; "i32", i32);
    register_add_matrix1_dynamic_native_factories!(builder; "i64", i64);
    register_add_matrix1_dynamic_native_factories!(builder; "i128", i128);
    register_add_matrix1_dynamic_native_factories!(builder; "u8", u8);
    register_add_matrix1_dynamic_native_factories!(builder; "u16", u16);
    register_add_matrix1_dynamic_native_factories!(builder; "u32", u32);
    register_add_matrix1_dynamic_native_factories!(builder; "u64", u64);
    register_add_matrix1_dynamic_native_factories!(builder; "u128", u128);
    register_add_matrix1_dynamic_native_factories!(builder; "f32", f32);
    register_add_matrix1_dynamic_native_factories!(builder; "f64", f64);
    register_add_matrix1_dynamic_native_factories!(builder; "rational", r64);
    register_add_matrix1_dynamic_native_factories!(builder; "complex", c64);
    Ok(())
}

#[cfg(feature = "source")]
pub fn install_math_add_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::catalog::install_canonical_source_specializer(
        builder,
        "math/add",
        None,
        None,
        FunctionExposure::Prelude,
        MathAdd {},
    )
}
