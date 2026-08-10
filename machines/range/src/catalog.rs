use mech_core::{
    FunctionArgs, FunctionArgumentRole, FunctionCatalogBuilder, MResult, MechFunctionFactory,
    RuntimeFunctionContract, RuntimeOutputAliasPolicy, LegacyValue, function_shape_contract_violation,
};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(all(not(feature = "matrix1"), feature = "matrixd"))]
use nalgebra::DMatrix;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;

#[cfg(feature = "source")]
fn install_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
    exposure: FunctionExposure,
) -> MResult<()>
where
    T: FunctionSpecializer + 'static,
{
    let operation = builder.insert_specializer(canonical_name, Arc::new(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: None,
        item: None,
        exposure,
    })
}

#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "exclusive")]
    install_operation(
        builder,
        "range/exclusive",
        crate::exclusive::RangeExclusive {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "exclusive_increment")]
    install_operation(
        builder,
        "range/exclusive-increment",
        crate::exclusive_increment::RangeIncrementExclusive {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "inclusive")]
    install_operation(
        builder,
        "range/inclusive",
        crate::inclusive::RangeInclusive {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "inclusive_increment")]
    install_operation(
        builder,
        "range/inclusive-increment",
        crate::inclusive_increment::RangeIncrementInclusive {},
        FunctionExposure::Internal,
    )?;
    Ok(())
}

macro_rules! install_range_factory {
    ($builder:expr, $module:ident, $factory:ident, $scalar:ty, $scalar_name:literal, $shape:ident) => {
        $builder.insert_runtime_factory::<crate::$module::$factory<$scalar, $shape<$scalar>>>(
            concat!(
                stringify!($factory),
                "<",
                $scalar_name,
                stringify!($shape),
                ">"
            ),
            RuntimeFunctionContract::custom(
                "range_construction",
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                range_contract_validator!($module),
            ),
        )?;
    };
}

#[derive(Clone, Copy, Debug)]
enum RangeContractNumber {
    Unsigned(u128),
    Signed(i128),
    Float(f64),
}

fn range_numeric_value(value: &LegacyValue) -> Option<RangeContractNumber> {
    match value {
        #[cfg(feature = "u8")]
        LegacyValue::U8(value) => Some(RangeContractNumber::Unsigned(*value.borrow() as u128)),
        #[cfg(feature = "u16")]
        LegacyValue::U16(value) => Some(RangeContractNumber::Unsigned(*value.borrow() as u128)),
        #[cfg(feature = "u32")]
        LegacyValue::U32(value) => Some(RangeContractNumber::Unsigned(*value.borrow() as u128)),
        #[cfg(feature = "u64")]
        LegacyValue::U64(value) => Some(RangeContractNumber::Unsigned(*value.borrow() as u128)),
        #[cfg(feature = "u128")]
        LegacyValue::U128(value) => Some(RangeContractNumber::Unsigned(*value.borrow())),
        #[cfg(feature = "i8")]
        LegacyValue::I8(value) => Some(RangeContractNumber::Signed(*value.borrow() as i128)),
        #[cfg(feature = "i16")]
        LegacyValue::I16(value) => Some(RangeContractNumber::Signed(*value.borrow() as i128)),
        #[cfg(feature = "i32")]
        LegacyValue::I32(value) => Some(RangeContractNumber::Signed(*value.borrow() as i128)),
        #[cfg(feature = "i64")]
        LegacyValue::I64(value) => Some(RangeContractNumber::Signed(*value.borrow() as i128)),
        #[cfg(feature = "i128")]
        LegacyValue::I128(value) => Some(RangeContractNumber::Signed(*value.borrow())),
        #[cfg(feature = "f32")]
        LegacyValue::F32(value) => Some(RangeContractNumber::Float(*value.borrow() as f64)),
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => Some(RangeContractNumber::Float(*value.borrow())),
        _ => None,
    }
}

fn integer_range_size(magnitude: u128, step: u128, inclusive: bool) -> Option<usize> {
    let size = if inclusive {
        magnitude.checked_div(step)?.checked_add(1)?
    } else {
        let quotient = magnitude.checked_div(step)?;
        quotient.checked_add(u128::from(magnitude % step != 0))?
    };
    usize::try_from(size).ok()
}

fn float_range_size(from: f64, step: f64, to: f64, inclusive: bool) -> Option<usize> {
    if !from.is_finite() || !step.is_finite() || !to.is_finite() || step == 0.0 {
        return None;
    }
    let diff = to - from;
    let size = if diff == 0.0 {
        if inclusive { 1.0 } else { 0.0 }
    } else if (diff > 0.0 && step > 0.0) || (diff < 0.0 && step < 0.0) {
        let quotient = diff / step;
        if inclusive {
            quotient.floor() + 1.0
        } else {
            quotient.ceil()
        }
    } else {
        0.0
    };
    if !size.is_finite() || size < 0.0 || size >= usize::MAX as f64 {
        return None;
    }
    Some(size as usize)
}

#[cfg(all(test, feature = "u64", feature = "u128", feature = "matrixd"))]
mod exact_range_contract_tests {
    use super::*;
    use mech_core::{Ref, matrix::Matrix};

    fn u64_output(columns: usize) -> LegacyValue {
        LegacyValue::MatrixU64(Matrix::from_vec(vec![0; columns], 1, columns))
    }

    fn u128_output(columns: usize) -> LegacyValue {
        LegacyValue::MatrixU128(Matrix::from_vec(vec![0; columns], 1, columns))
    }

    #[test]
    fn large_unsigned_ranges_preserve_exact_cardinality() {
        let from = 1_u64 << 60;
        validate_range_exclusive(&FunctionArgs::Binary(
            u64_output(2),
            LegacyValue::U64(Ref::new(from)),
            LegacyValue::U64(Ref::new(from + 2)),
        ))
        .unwrap();

        validate_range_inclusive(&FunctionArgs::Binary(
            u128_output(2),
            LegacyValue::U128(Ref::new(u128::MAX - 1)),
            LegacyValue::U128(Ref::new(u128::MAX)),
        ))
        .unwrap();
    }

    #[test]
    fn large_incremented_ranges_preserve_exact_cardinality() {
        let from = 1_u64 << 60;
        validate_range_increment_exclusive(&FunctionArgs::Ternary(
            u64_output(2),
            LegacyValue::U64(Ref::new(from)),
            LegacyValue::U64(Ref::new(2)),
            LegacyValue::U64(Ref::new(from + 4)),
        ))
        .unwrap();
        validate_range_increment_inclusive(&FunctionArgs::Ternary(
            u64_output(3),
            LegacyValue::U64(Ref::new(from)),
            LegacyValue::U64(Ref::new(2)),
            LegacyValue::U64(Ref::new(from + 4)),
        ))
        .unwrap();

        let error = validate_range_increment_exclusive(&FunctionArgs::Ternary(
            u64_output(1),
            LegacyValue::U64(Ref::new(from)),
            LegacyValue::U64(Ref::new(2)),
            LegacyValue::U64(Ref::new(from + 4)),
        ))
        .unwrap_err();
        assert!(error.kind_message().contains("range requires 2"));
    }
}

fn range_contract_size(
    values: &[RangeContractNumber],
    inclusive: bool,
    incremented: bool,
) -> Option<usize> {
    match (values, incremented) {
        ([RangeContractNumber::Unsigned(from), RangeContractNumber::Unsigned(to)], false) => {
            let magnitude = to.checked_sub(*from)?;
            integer_range_size(magnitude, 1, inclusive)
        }
        ([RangeContractNumber::Signed(from), RangeContractNumber::Signed(to)], false) => {
            if to < from {
                Some(0)
            } else {
                integer_range_size(to.abs_diff(*from), 1, inclusive)
            }
        }
        ([RangeContractNumber::Float(from), RangeContractNumber::Float(to)], false) => {
            float_range_size(*from, 1.0, *to, inclusive)
        }
        (
            [
                RangeContractNumber::Unsigned(from),
                RangeContractNumber::Unsigned(step),
                RangeContractNumber::Unsigned(to),
            ],
            true,
        ) => {
            if *step == 0 {
                None
            } else if to < from {
                Some(0)
            } else {
                integer_range_size(to - from, *step, inclusive)
            }
        }
        (
            [
                RangeContractNumber::Signed(from),
                RangeContractNumber::Signed(step),
                RangeContractNumber::Signed(to),
            ],
            true,
        ) => {
            if *step == 0 {
                return None;
            }
            if from == to {
                return Some(usize::from(inclusive));
            }
            if (to > from && *step < 0) || (to < from && *step > 0) {
                return Some(0);
            }
            integer_range_size(to.abs_diff(*from), step.unsigned_abs(), inclusive)
        }
        (
            [
                RangeContractNumber::Float(from),
                RangeContractNumber::Float(step),
                RangeContractNumber::Float(to),
            ],
            true,
        ) => float_range_size(*from, *step, *to, inclusive),
        _ => None,
    }
}

fn validate_range_contract(args: &FunctionArgs, inclusive: bool, incremented: bool) -> MResult<()> {
    let contract = "range_construction";
    let output = args
        .output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "output must be matrix-backed")
        })?;
    if output.rows != 1 {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output must be a row, found {}x{}",
                output.rows, output.cols
            ),
        ));
    }
    let expected_inputs = if incremented { 3 } else { 2 };
    if args.input_count() != expected_inputs {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "expected {expected_inputs} inputs, found {}",
                args.input_count()
            ),
        ));
    }
    let mut values = Vec::with_capacity(expected_inputs);
    for index in 0..expected_inputs {
        let input = args
            .input_value(index)
            .and_then(range_numeric_value)
            .ok_or_else(|| {
                function_shape_contract_violation(
                    contract,
                    format!("input {index} must be a numeric scalar"),
                )
            })?;
        values.push(input);
    }
    let size = range_contract_size(&values, inclusive, incremented).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "range values must have one numeric representation, finite endpoints, a nonzero step, and a representable element count",
        )
    })?;
    if size == 0 || output.cols != size {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output has {} elements, range requires {size}",
                output.rows.saturating_mul(output.cols),
            ),
        ));
    }
    Ok(())
}

pub(crate) fn validate_range_exclusive(args: &FunctionArgs) -> MResult<()> {
    validate_range_contract(args, false, false)
}

pub(crate) fn validate_range_inclusive(args: &FunctionArgs) -> MResult<()> {
    validate_range_contract(args, true, false)
}

pub(crate) fn validate_range_increment_exclusive(args: &FunctionArgs) -> MResult<()> {
    validate_range_contract(args, false, true)
}

pub(crate) fn validate_range_increment_inclusive(args: &FunctionArgs) -> MResult<()> {
    validate_range_contract(args, true, true)
}

macro_rules! range_contract_validator {
    (exclusive) => {
        validate_range_exclusive
    };
    (inclusive) => {
        validate_range_inclusive
    };
    (exclusive_increment) => {
        validate_range_increment_exclusive
    };
    (inclusive_increment) => {
        validate_range_increment_inclusive
    };
}

macro_rules! install_range_factories_for_type {
    ($builder:expr, $module:ident, $factory:ident, $scalar:ty, $scalar_name:literal) => {{
        #[cfg(feature = "matrix1")]
        install_range_factory!($builder, $module, $factory, $scalar, $scalar_name, Matrix1);
        #[cfg(all(not(feature = "matrix1"), feature = "matrixd"))]
        install_range_factory!($builder, $module, $factory, $scalar, $scalar_name, DMatrix);
        #[cfg(feature = "row_vector2")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowVector2
        );
        #[cfg(feature = "row_vector3")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowVector3
        );
        #[cfg(feature = "row_vector4")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowVector4
        );
        #[cfg(feature = "row_vectord")]
        install_range_factory!(
            $builder,
            $module,
            $factory,
            $scalar,
            $scalar_name,
            RowDVector
        );
    }};
}

macro_rules! install_range_operation_runtime {
    ($builder:expr, $module:ident, $factory:ident) => {{
        #[cfg(feature = "f32")]
        install_range_factories_for_type!($builder, $module, $factory, f32, "f32");
        #[cfg(feature = "f64")]
        install_range_factories_for_type!($builder, $module, $factory, f64, "f64");
        #[cfg(feature = "i8")]
        install_range_factories_for_type!($builder, $module, $factory, i8, "i8");
        #[cfg(feature = "i16")]
        install_range_factories_for_type!($builder, $module, $factory, i16, "i16");
        #[cfg(feature = "i32")]
        install_range_factories_for_type!($builder, $module, $factory, i32, "i32");
        #[cfg(feature = "i64")]
        install_range_factories_for_type!($builder, $module, $factory, i64, "i64");
        #[cfg(feature = "i128")]
        install_range_factories_for_type!($builder, $module, $factory, i128, "i128");
        #[cfg(feature = "u8")]
        install_range_factories_for_type!($builder, $module, $factory, u8, "u8");
        #[cfg(feature = "u16")]
        install_range_factories_for_type!($builder, $module, $factory, u16, "u16");
        #[cfg(feature = "u32")]
        install_range_factories_for_type!($builder, $module, $factory, u32, "u32");
        #[cfg(feature = "u64")]
        install_range_factories_for_type!($builder, $module, $factory, u64, "u64");
        #[cfg(feature = "u128")]
        install_range_factories_for_type!($builder, $module, $factory, u128, "u128");
    }};
}

/// Legacy direct-registration implementation retained while the native
/// declaration traversal below owns the active runtime path.
fn install_legacy_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "exclusive")]
    {
        install_range_operation_runtime!(builder, exclusive, RangeExclusiveScalar);
        // The legacy module gate compiles increment factories with `exclusive`,
        // even when the named source operation is not exported.
        install_range_operation_runtime!(
            builder,
            exclusive_increment,
            RangeIncrementExclusiveScalar
        );
    }
    #[cfg(feature = "inclusive")]
    {
        install_range_operation_runtime!(builder, inclusive, RangeInclusiveScalar);
        // Preserve the matching legacy module-gate quirk for parity.
        install_range_operation_runtime!(
            builder,
            inclusive_increment,
            RangeIncrementInclusiveScalar
        );
    }
    Ok(())
}

macro_rules! for_each_range_scalar {
    ($callback:ident, $context:tt; [$cfg:meta]; $module:ident; $factory:ident; $operation_feature:literal; $shape:ident; [$($shape_feature:literal),* $(,)?]) => {
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "f32"; "f32"; f32; f32);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "f64"; "f64"; f64; f64);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i8"; "i8"; i8; i8);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i16"; "i16"; i16; i16);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i32"; "i32"; i32; i32);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i64"; "i64"; i64; i64);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "i128"; "i128"; i128; i128);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u8"; "u8"; u8; u8);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u16"; "u16"; u16; u16);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u32"; "u32"; u32; u32);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u64"; "u64"; u64; u64);
        $callback!($context; [$cfg]; $module; $factory; $operation_feature; $shape; [$($shape_feature),*]; feature = "u128"; "u128"; u128; u128);
    };
}

macro_rules! for_each_range_shape {
    ($callback:ident, $context:tt; $module:ident; $factory:ident; $operation_feature:literal) => {
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "matrix1")]; $module; $factory; $operation_feature; Matrix1; ["matrix1"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, not(feature = "matrix1"), feature = "matrixd")]; $module; $factory; $operation_feature; DMatrix; ["matrixd"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vector2")]; $module; $factory; $operation_feature; RowVector2; ["row_vector2"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vector3")]; $module; $factory; $operation_feature; RowVector3; ["row_vector3"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vector4")]; $module; $factory; $operation_feature; RowVector4; ["row_vector4"]);
        for_each_range_scalar!($callback, $context; [all(feature = $operation_feature, feature = "row_vectord")]; $module; $factory; $operation_feature; RowDVector; ["row_vectord"]);
    };
}

macro_rules! for_each_range_family_with_context {
    ($callback:ident, $context:tt) => {
        for_each_range_shape!($callback, $context; exclusive; RangeExclusiveScalar; "exclusive");
        for_each_range_shape!($callback, $context; exclusive_increment; RangeIncrementExclusiveScalar; "exclusive");
        for_each_range_shape!($callback, $context; inclusive; RangeInclusiveScalar; "inclusive");
        for_each_range_shape!($callback, $context; inclusive_increment; RangeIncrementInclusiveScalar; "inclusive");
    };
}

macro_rules! for_each_range_family {
    ($callback:ident) => {
        for_each_range_family_with_context!($callback, ());
    };
    ($callback:ident, $context:tt) => {
        for_each_range_family_with_context!($callback, $context);
    };
}

macro_rules! declare_range_runtime_factory {
    ($_context:tt; [$cfg:meta]; $module:ident; $factory:ident; $operation_feature:literal; $shape:ident; [$($shape_feature:literal),* $(,)?]; $scalar_cfg:meta; $scalar_feature:literal; $scalar:ty; $scalar_token:ident) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all($cfg, $scalar_cfg),
                registration: [<register_ $factory:snake _ $scalar_token _ $shape:snake>],
                installer: [<install_ $factory:snake _ $scalar_token _ $shape:snake>],
                name: concat!(stringify!($factory), "<", $scalar_feature, stringify!($shape), ">"),
                factory_type: crate::$module::$factory<$scalar, $shape<$scalar>>,
                contract: RuntimeFunctionContract::custom(
                    "range_construction",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    range_contract_validator!($module),
                ),
                package: "mech-range", crate_name: "mech_range",
                installer_path: concat!("mech_range::__mech_native::", stringify!([<install_ $factory:snake _ $scalar_token _ $shape:snake>])),
                extra_cargo_features: [$operation_feature],
            }
        }
    };
}

for_each_range_family!(declare_range_runtime_factory);

/// Installs every concrete runtime factory declared by the range family traversal.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register_range_runtime_factory {
        (($builder:ident); [$cfg:meta]; $_module:ident; $factory:ident; $_operation_feature:literal; $shape:ident; [$($_shape_feature:literal),* $(,)?]; $scalar_cfg:meta; $_scalar_feature:literal; $_scalar:ty; $scalar_token:ident) => {
            #[cfg(all($cfg, $scalar_cfg))]
            mech_core::paste::paste! { [<register_ $factory:snake _ $scalar_token _ $shape:snake>]($builder)?; }
        };
    }
    for_each_range_family!(register_range_runtime_factory, (builder));
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_range_runtime_factory {
        ($_context:tt; [$cfg:meta]; $_module:ident; $factory:ident; $_operation_feature:literal; $shape:ident; [$($_shape_feature:literal),* $(,)?]; $scalar_cfg:meta; $_scalar_feature:literal; $_scalar:ty; $scalar_token:ident) => {
            #[cfg(all($cfg, $scalar_cfg))]
            mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $scalar_token _ $shape:snake>]; }
        };
    }
    for_each_range_family!(export_range_runtime_factory);
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    fn expected_operations() -> Vec<(&'static str, FunctionExposure)> {
        let mut expected = Vec::new();
        #[cfg(feature = "exclusive")]
        expected.push(("range/exclusive", FunctionExposure::Prelude));
        #[cfg(feature = "exclusive_increment")]
        expected.push(("range/exclusive-increment", FunctionExposure::Internal));
        #[cfg(feature = "inclusive")]
        expected.push(("range/inclusive", FunctionExposure::Prelude));
        #[cfg(feature = "inclusive_increment")]
        expected.push(("range/inclusive-increment", FunctionExposure::Internal));
        expected
    }

    #[test]
    fn source_catalog_matches_the_frozen_range_surface() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let expected = expected_operations();

        #[cfg(all(
            feature = "exclusive",
            feature = "exclusive_increment",
            feature = "inclusive",
            feature = "inclusive_increment",
        ))]
        assert_eq!(expected.len(), 4);
        assert_eq!(catalog.specializer_count(), expected.len());
        assert_eq!(catalog.runtime_factory_count(), 0);
        for (name, exposure) in expected {
            let operation = OperationId::from_name(name);
            assert_eq!(catalog.specializer(operation).unwrap().canonical_name, name);
            assert_eq!(
                catalog.exports_for_operation(operation),
                &[FunctionExport {
                    operation,
                    canonical_name: name.to_string(),
                    module: None,
                    item: None,
                    exposure,
                }],
            );
        }
    }
}
