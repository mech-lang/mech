use crate::*;
use num_traits::*;
fn checked_runtime_add<T: RuntimeCheckedArithmetic>(lhs: T, rhs: T) -> MResult<T> {
    lhs.runtime_checked_add(rhs)
        .ok_or_else(|| arithmetic_overflow::<T>("addition"))
}

// Add ------------------------------------------------------------------------

macro_rules! add_op {
    (@managed $lhs:expr, $rhs:expr) => {
        checked_runtime_add($lhs, $rhs)
    };
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_runtime_add(*$lhs, *$rhs)?;
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
            add_op,
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

#[cfg(all(test, feature = "u8", feature = "source"))]
mod checked_arithmetic_tests {
    use super::*;

    #[test]
    fn integer_addition_rejects_reactive_overflow_and_retains_output() {
        let lhs = ValueCell::from_exact(40_u8).unwrap();
        let rhs = ValueCell::from_exact(1_u8).unwrap();
        let function = specialize_add(lhs, rhs.clone());
        function.instance().solve_result().unwrap();
        assert_eq!(u8_output(&function), 41);

        let overflow = rhs.rebuild_data_draft(ValueDataDraft::U8(u8::MAX)).unwrap();
        rhs.replace(&overflow).unwrap();
        let error = function.instance().solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(u8_output(&function), 41);
    }

    #[test]
    fn owned_inputs_share_one_session_and_remain_updatable_after_call_binding() {
        let session = MemoryDomain::new().unwrap();
        let lhs = ValueCell::from_exact_in(&session, 40_u8).unwrap();
        let rhs = ValueCell::from_exact_in(&session, 1_u8).unwrap();
        let lhs_clone = lhs.clone();
        let function = specialize_add(lhs.clone(), rhs.clone());
        function.instance().solve_result().unwrap();
        assert_eq!(u8_output(&function), 41);

        lhs.replace(&lhs.rebuild_data_draft(ValueDataDraft::U8(50)).unwrap())
            .unwrap();
        rhs.replace(&rhs.rebuild_data_draft(ValueDataDraft::U8(2)).unwrap())
            .unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(u8_output(&function), 52);
        assert!(matches!(
            lhs_clone.snapshot().unwrap().data(),
            ValueData::U8(50)
        ));
        assert!(lhs_clone.same_logical_cell(&lhs));

        // Candidate issuance does not revoke either owned value or the
        // already bound consumer.
        session.issue_plan_revision().unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(u8_output(&function), 52);
    }

    #[cfg(feature = "matrixd")]
    #[test]
    fn matrix_addition_preserves_publication_after_first_middle_and_last_failures() {
        let lhs =
            ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &[10_u8, 20, 30, 40, 50, 60]))
                .unwrap();
        let rhs =
            ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &[1_u8, 2, 3, 4, 5, 6])).unwrap();
        let function = specialize_add(lhs, rhs.clone());
        let output_clone = function.output().clone();
        function.instance().solve_result().unwrap();
        assert_eq!(
            matrix_output(function.output()),
            vec![11, 22, 33, 44, 55, 66]
        );
        for bad_index in [0, 2, 5] {
            let before = matrix_output(function.output());
            let before_version = function.output().published_version();
            let mut replacements = [1_u8; 6];
            replacements[bad_index] = u8::MAX;
            rhs.replace(
                &ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &replacements))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
            let error = function.instance().solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathArithmeticOverflow");
            assert_eq!(matrix_output(function.output()), before);
            assert_eq!(matrix_output(&output_clone), before);
            assert_eq!(function.output().published_version(), before_version);
            rhs.replace(
                &ValueCell::from_exact(DMatrix::from_element(2, 3, 1_u8))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
            function.instance().solve_result().unwrap();
            assert_eq!(
                matrix_output(function.output()),
                vec![11, 21, 31, 41, 51, 61]
            );
        }
        assert!(function.output().same_cell(&output_clone));
    }

    #[cfg(feature = "matrixd")]
    fn matrix_output(cell: &ValueCell) -> Vec<u8> {
        let value = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("expected matrix")
        };
        let mech_core::snapshot::SequenceView::U8(values) = matrix.elements() else {
            panic!("expected U8 elements")
        };
        values.to_vec()
    }

    fn specialize_add(lhs: ValueCell, rhs: ValueCell) -> SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_runtime(&mut builder).unwrap();
        install_math_add_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        crate::catalog::specialize_test_operation(&catalog, "math/add", vec![lhs, rhs])
    }

    fn u8_output(function: &SpecializedFunction) -> u8 {
        let snapshot = function.output().snapshot().unwrap();
        let ValueData::U8(value) = snapshot.data() else {
            panic!("expected U8 add output")
        };
        *value
    }
}

#[cfg(all(
    test,
    feature = "source",
    feature = "f64",
    feature = "matrix2",
    feature = "matrixd"
))]
mod state_port_tests {
    use super::*;

    #[test]
    fn fixed_and_dynamic_add_outputs_restore_through_typed_state_ports() {
        let fixed_rhs = ValueCell::from_exact(Matrix2::from_element(2.0_f64)).unwrap();
        let fixed = specialize_add(
            ValueCell::from_exact(Matrix2::from_element(1.0_f64)).unwrap(),
            fixed_rhs.clone(),
        );
        fixed.instance().solve_result().unwrap();
        let fixed_before = fixed.output().snapshot().unwrap();

        let dynamic_rhs = ValueCell::from_exact(DMatrix::from_element(1, 2, 4.0_f64)).unwrap();
        let dynamic = specialize_add(
            ValueCell::from_exact(DMatrix::from_element(1, 2, 3.0_f64)).unwrap(),
            dynamic_rhs.clone(),
        );
        dynamic.instance().solve_result().unwrap();
        let dynamic_before = dynamic.output().snapshot().unwrap();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_instance(fixed.instance())?;
            participant.capture_function_instance(dynamic.instance())?;
            fixed_rhs
                .replace(&ValueCell::from_exact(Matrix2::from_element(8.0_f64))?.snapshot()?)?;
            dynamic_rhs.replace(
                &ValueCell::from_exact(DMatrix::from_element(1, 2, 6.0_f64))?.snapshot()?,
            )?;
            fixed.instance().solve_result()?;
            dynamic.instance().solve_result()?;
            assert!(!same_snapshot(fixed.output(), &fixed_before));
            assert!(!same_snapshot(dynamic.output(), &dynamic_before));
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(same_snapshot(fixed.output(), &fixed_before));
        assert!(same_snapshot(dynamic.output(), &dynamic_before));
    }

    fn same_snapshot(cell: &ValueCell, before: &Value) -> bool {
        let current = cell.snapshot().unwrap();
        current
            .snapshot_eq(
                &current.schemas().unwrap(),
                before,
                &before.schemas().unwrap(),
            )
            .unwrap()
    }

    fn specialize_add(lhs: ValueCell, rhs: ValueCell) -> SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_runtime(&mut builder).unwrap();
        install_math_add_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        crate::catalog::specialize_test_operation(&catalog, "math/add", vec![lhs, rhs])
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

#[cfg(all(test, feature = "source", feature = "f64"))]
mod managed_source_tests {
    use super::*;

    #[test]
    fn scalar_add_executes_through_the_bound_managed_function_instance() {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_runtime(&mut builder).unwrap();
        install_math_add_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let specialized = crate::catalog::specialize_test_operation(
            &catalog,
            "math/add",
            vec![
                ValueCell::from_exact(2.0_f64).unwrap(),
                ValueCell::from_exact(3.5_f64).unwrap(),
            ],
        );

        specialized.instance().solve_result().unwrap();
        let output = specialized.output().snapshot().unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("managed scalar add must publish F64")
        };
        assert_eq!(value.to_f64(), 5.5);
    }
}

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
