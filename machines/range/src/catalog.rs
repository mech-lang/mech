use mech_core::{
    ExtentEvolution, FunctionCatalogBuilder, MResult, RuntimeFunctionContract,
    RuntimeOutputAliasPolicy, SchemaBody, ValueCell, ValueData, function_shape_contract_violation,
};
#[cfg(feature = "source")]
use mech_core::{CanonicalFunctionSpecializer, FunctionExport, FunctionExposure};
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
    T: CanonicalFunctionSpecializer + 'static,
{
    let operation = builder.insert_canonical_specializer(canonical_name, Arc::new(compiler))?;
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

#[derive(Clone, Copy, Debug)]
enum RangeContractNumber {
    #[cfg(any(
        feature = "u8",
        feature = "u16",
        feature = "u32",
        feature = "u64",
        feature = "u128"
    ))]
    Unsigned(u128),
    #[cfg(any(
        feature = "i8",
        feature = "i16",
        feature = "i32",
        feature = "i64",
        feature = "i128"
    ))]
    Signed(i128),
    #[cfg(any(feature = "f32", feature = "f64"))]
    Float(f64),
}

fn range_numeric_cell(value: &ValueCell) -> MResult<Option<RangeContractNumber>> {
    Ok(match value.snapshot()?.data() {
        #[cfg(feature = "u8")]
        ValueData::U8(value) => Some(RangeContractNumber::Unsigned(*value as u128)),
        #[cfg(feature = "u16")]
        ValueData::U16(value) => Some(RangeContractNumber::Unsigned(*value as u128)),
        #[cfg(feature = "u32")]
        ValueData::U32(value) => Some(RangeContractNumber::Unsigned(*value as u128)),
        #[cfg(feature = "u64")]
        ValueData::U64(value) => Some(RangeContractNumber::Unsigned(*value as u128)),
        #[cfg(feature = "u128")]
        ValueData::U128(value) => Some(RangeContractNumber::Unsigned(*value)),
        #[cfg(feature = "i8")]
        ValueData::I8(value) => Some(RangeContractNumber::Signed(*value as i128)),
        #[cfg(feature = "i16")]
        ValueData::I16(value) => Some(RangeContractNumber::Signed(*value as i128)),
        #[cfg(feature = "i32")]
        ValueData::I32(value) => Some(RangeContractNumber::Signed(*value as i128)),
        #[cfg(feature = "i64")]
        ValueData::I64(value) => Some(RangeContractNumber::Signed(*value as i128)),
        #[cfg(feature = "i128")]
        ValueData::I128(value) => Some(RangeContractNumber::Signed(*value)),
        #[cfg(feature = "f32")]
        ValueData::F32(value) => Some(RangeContractNumber::Float(value.to_f32() as f64)),
        #[cfg(feature = "f64")]
        ValueData::F64(value) => Some(RangeContractNumber::Float(value.to_f64())),
        _ => None,
    })
}

#[cfg(any(
    feature = "u8",
    feature = "u16",
    feature = "u32",
    feature = "u64",
    feature = "u128",
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128"
))]
fn integer_range_size(magnitude: u128, step: u128, inclusive: bool) -> Option<usize> {
    let size = if inclusive {
        magnitude.checked_div(step)?.checked_add(1)?
    } else {
        let quotient = magnitude.checked_div(step)?;
        quotient.checked_add(u128::from(magnitude % step != 0))?
    };
    usize::try_from(size).ok()
}

#[cfg(any(feature = "f32", feature = "f64"))]
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

fn range_contract_size(
    values: &[RangeContractNumber],
    inclusive: bool,
    incremented: bool,
) -> Option<usize> {
    match (values, incremented) {
        #[cfg(any(
            feature = "u8",
            feature = "u16",
            feature = "u32",
            feature = "u64",
            feature = "u128"
        ))]
        ([RangeContractNumber::Unsigned(from), RangeContractNumber::Unsigned(to)], false) => {
            let magnitude = to.checked_sub(*from)?;
            integer_range_size(magnitude, 1, inclusive)
        }
        #[cfg(any(
            feature = "i8",
            feature = "i16",
            feature = "i32",
            feature = "i64",
            feature = "i128"
        ))]
        ([RangeContractNumber::Signed(from), RangeContractNumber::Signed(to)], false) => {
            if to < from {
                Some(0)
            } else {
                integer_range_size(to.abs_diff(*from), 1, inclusive)
            }
        }
        #[cfg(any(feature = "f32", feature = "f64"))]
        ([RangeContractNumber::Float(from), RangeContractNumber::Float(to)], false) => {
            float_range_size(*from, 1.0, *to, inclusive)
        }
        #[cfg(any(
            feature = "u8",
            feature = "u16",
            feature = "u32",
            feature = "u64",
            feature = "u128"
        ))]
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
        #[cfg(any(
            feature = "i8",
            feature = "i16",
            feature = "i32",
            feature = "i64",
            feature = "i128"
        ))]
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
        #[cfg(any(feature = "f32", feature = "f64"))]
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

pub(crate) fn canonical_range_size(
    inputs: &[ValueCell],
    inclusive: bool,
    incremented: bool,
) -> MResult<usize> {
    let contract = "range_construction";
    let expected_inputs = if incremented { 3 } else { 2 };
    if inputs.len() != expected_inputs {
        return Err(function_shape_contract_violation(
            contract,
            format!("expected {expected_inputs} inputs, found {}", inputs.len()),
        ));
    }
    let values = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            range_numeric_cell(input)?.ok_or_else(|| {
                function_shape_contract_violation(
                    contract,
                    format!("input {index} must be a numeric scalar"),
                )
            })
        })
        .collect::<MResult<Vec<_>>>()?;
    let size = range_contract_size(&values, inclusive, incremented).ok_or_else(|| {
        function_shape_contract_violation(
            contract,
            "range values must have one numeric representation, finite endpoints, a nonzero step, and a representable element count",
        )
    })?;
    if size == 0 {
        return Err(function_shape_contract_violation(
            contract,
            "range output must contain at least one element",
        ));
    }
    Ok(size)
}

fn validate_canonical_range_contract(
    output: &ValueCell,
    inputs: &[ValueCell],
    inclusive: bool,
    incremented: bool,
) -> MResult<()> {
    let contract = "range_construction";
    let SchemaBody::Matrix { dimensions, .. } = output.closed_schema_body()? else {
        return Err(function_shape_contract_violation(
            contract,
            "output must be matrix-backed",
        ));
    };
    let [mech_core::DimensionExpr::Constant(rows), mech_core::DimensionExpr::Constant(columns)] =
        dimensions.as_ref()
    else {
        return Err(function_shape_contract_violation(
            contract,
            "output matrix dimensions must be resolved",
        ));
    };
    if *rows != 1 {
        return Err(function_shape_contract_violation(
            contract,
            format!("output must be a row, found {rows}x{columns}"),
        ));
    }
    let expected_inputs = if incremented { 3 } else { 2 };
    if inputs.len() != expected_inputs {
        return Err(function_shape_contract_violation(
            contract,
            format!("expected {expected_inputs} inputs, found {}", inputs.len()),
        ));
    }
    let size = canonical_range_size(inputs, inclusive, incremented)?;
    let columns = usize::try_from(*columns).unwrap_or(usize::MAX);
    if matches!(
        output.extent_evolution(),
        ExtentEvolution::Fixed | ExtentEvolution::ActivationFixed
    ) && columns != size
    {
        return Err(function_shape_contract_violation(
            contract,
            format!("output has {columns} elements, range requires {size}"),
        ));
    }
    Ok(())
}

#[cfg(feature = "exclusive")]
pub(crate) fn validate_canonical_range_exclusive(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    validate_canonical_range_contract(output, inputs, false, false)
}

#[cfg(feature = "inclusive")]
pub(crate) fn validate_canonical_range_inclusive(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    validate_canonical_range_contract(output, inputs, true, false)
}

#[cfg(feature = "exclusive")]
pub(crate) fn validate_canonical_range_increment_exclusive(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    validate_canonical_range_contract(output, inputs, false, true)
}

#[cfg(feature = "inclusive")]
pub(crate) fn validate_canonical_range_increment_inclusive(
    output: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    validate_canonical_range_contract(output, inputs, true, true)
}

macro_rules! canonical_range_contract_validator {
    (exclusive) => {
        validate_canonical_range_exclusive
    };
    (inclusive) => {
        validate_canonical_range_inclusive
    };
    (exclusive_increment) => {
        validate_canonical_range_increment_exclusive
    };
    (inclusive_increment) => {
        validate_canonical_range_increment_inclusive
    };
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
                contract: RuntimeFunctionContract::canonical_custom(
                    "range_construction",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    canonical_range_contract_validator!($module),
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
