use mech_core::{
    ExtentEvolution, FunctionCatalogBuilder, MResult, RuntimeFunctionContract,
    RuntimeOutputAliasPolicy, SchemaBody, ValueCell, function_shape_contract_violation,
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
    let declaration = mech_core::maintained_source_type_declaration(canonical_name)?;
    let operation =
        builder.insert_canonical_specializer(canonical_name, declaration, Arc::new(compiler))?;
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
        .map(ValueCell::snapshot)
        .collect::<MResult<Vec<_>>>()?;
    let data = values
        .iter()
        .map(|value| value.data().clone())
        .collect::<Vec<_>>();
    mech_core::canonical_value_range_size(&data, inclusive, incremented).map_err(|_| {
        function_shape_contract_violation(
            contract,
            "range values must have one numeric representation, finite endpoints, a nonzero step, a nonempty direction, and a representable element count",
        )
    })
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

macro_rules! range_operation {
    (exclusive) => {
        "range/exclusive"
    };
    (exclusive_increment) => {
        "range/exclusive-increment"
    };
    (inclusive) => {
        "range/inclusive"
    };
    (inclusive_increment) => {
        "range/inclusive-increment"
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
                operations: [mech_core::OperationId::from_name(range_operation!($module))],
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
