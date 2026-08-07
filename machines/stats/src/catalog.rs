use mech_core::{
    FunctionArgs, FunctionArgumentRole, FunctionCatalogBuilder, MResult, RuntimeFunctionContract,
    RuntimeOutputAliasPolicy, function_shape_contract_violation,
};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(all(feature = "source", feature = "sum"))]
use crate::{StatsSumColumn, StatsSumRow};

macro_rules! install_numeric_runtime_factories {
    ($builder:expr, $module:ident, $factory:ident) => {{
        #[cfg(feature = "u8")]
        install_numeric_runtime_factory!($builder, $module, $factory, u8, "u8");
        #[cfg(feature = "u16")]
        install_numeric_runtime_factory!($builder, $module, $factory, u16, "u16");
        #[cfg(feature = "u32")]
        install_numeric_runtime_factory!($builder, $module, $factory, u32, "u32");
        #[cfg(feature = "u64")]
        install_numeric_runtime_factory!($builder, $module, $factory, u64, "u64");
        #[cfg(feature = "u128")]
        install_numeric_runtime_factory!($builder, $module, $factory, u128, "u128");
        #[cfg(feature = "i8")]
        install_numeric_runtime_factory!($builder, $module, $factory, i8, "i8");
        #[cfg(feature = "i16")]
        install_numeric_runtime_factory!($builder, $module, $factory, i16, "i16");
        #[cfg(feature = "i32")]
        install_numeric_runtime_factory!($builder, $module, $factory, i32, "i32");
        #[cfg(feature = "i64")]
        install_numeric_runtime_factory!($builder, $module, $factory, i64, "i64");
        #[cfg(feature = "i128")]
        install_numeric_runtime_factory!($builder, $module, $factory, i128, "i128");
        #[cfg(feature = "f32")]
        install_numeric_runtime_factory!($builder, $module, $factory, f32, "f32");
        #[cfg(feature = "f64")]
        install_numeric_runtime_factory!($builder, $module, $factory, f64, "f64");
        #[cfg(feature = "complex")]
        install_numeric_runtime_factory!($builder, $module, $factory, mech_core::C64, "c64");
        #[cfg(feature = "rational")]
        install_numeric_runtime_factory!($builder, $module, $factory, mech_core::R64, "r64");

        Ok::<(), mech_core::MechError>(())
    }};
}

macro_rules! install_numeric_runtime_factory {
    ($builder:expr, $module:ident, $factory:ident, $scalar:ty, $name:literal) => {
        $builder.insert_runtime_factory::<crate::$module::$factory<$scalar>>(
            concat!(stringify!($factory), "<", $name, ">"),
            RuntimeFunctionContract::custom(
                "statistical_reduction",
                RuntimeOutputAliasPolicy::DisallowInputAlias,
                statistical_reduction_contract!($module),
            ),
        )?;
    };
}

fn validate_sum_column(args: &FunctionArgs) -> MResult<()> {
    validate_statistical_reduction(args, true)
}

fn validate_sum_row(args: &FunctionArgs) -> MResult<()> {
    validate_statistical_reduction(args, false)
}

fn validate_statistical_reduction(args: &FunctionArgs, column: bool) -> MResult<()> {
    let contract = "statistical_reduction";
    let output = args
        .output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "output must be matrix-backed")
        })?;
    let input = args
        .input_value(0)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input"))?
        .function_matrix_descriptor(FunctionArgumentRole::Input(0))?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "input must be matrix-backed")
        })?;
    let expected = if column {
        (input.rows, 1)
    } else {
        (1, input.cols)
    };
    if (output.rows, output.cols) != expected {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "output is {}x{}, expected {}x{} for input {}x{}",
                output.rows, output.cols, expected.0, expected.1, input.rows, input.cols,
            ),
        ));
    }
    Ok(())
}

macro_rules! statistical_reduction_contract {
    (sum_column) => {
        validate_sum_column
    };
    (sum_row) => {
        validate_sum_row
    };
}

#[cfg(feature = "source")]
fn install_module_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &'static str,
    item: &'static str,
    compiler: T,
) -> MResult<()>
where
    T: FunctionSpecializer + 'static,
{
    let operation = builder.insert_specializer(canonical_name, Arc::new(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: Some("stats".to_string()),
        item: Some(item.to_string()),
        exposure: FunctionExposure::ModuleOnly,
    })
}

/// Installs the frozen named source-specializer surface for the statistics machine.
#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "sum")]
    {
        install_module_operation(builder, "stats/sum/column", "sum/column", StatsSumColumn {})?;
        install_module_operation(builder, "stats/sum/row", "sum/row", StatsSumRow {})?;
    }

    Ok(())
}

/// Legacy direct-registration implementation retained while the native
/// declaration traversal below takes ownership of the active runtime path.
fn install_legacy_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "sum")]
    {
        #[cfg(feature = "matrix1")]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnM1)?;
        #[cfg(all(feature = "matrix2", feature = "vector2"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnM2)?;
        #[cfg(all(feature = "matrix3", feature = "vector3"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnM3)?;
        #[cfg(all(feature = "matrix4", feature = "vector4"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnM4)?;
        #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnM2x3)?;
        #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnM3x2)?;
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnMD)?;
        #[cfg(feature = "vector2")]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnV2)?;
        #[cfg(feature = "vector3")]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnV3)?;
        #[cfg(feature = "vector4")]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnV4)?;
        #[cfg(feature = "vectord")]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnVD)?;
        #[cfg(all(feature = "row_vector2", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnR2)?;
        #[cfg(all(feature = "row_vector3", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnR3)?;
        #[cfg(all(feature = "row_vector4", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnR4)?;
        #[cfg(all(feature = "row_vectord", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnRD)?;
        #[cfg(all(feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
        install_numeric_runtime_factories!(builder, sum_column, StatsSumColumnRD2)?;

        #[cfg(feature = "matrix1")]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowM1)?;
        #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowM2)?;
        #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowM3)?;
        #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowM4)?;
        #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowM2x3)?;
        #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowM3x2)?;
        #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowMD)?;
        #[cfg(all(feature = "vector2", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowV2)?;
        #[cfg(all(feature = "vector3", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowV3)?;
        #[cfg(all(feature = "vector4", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowV4)?;
        #[cfg(all(feature = "vectord", feature = "matrix1"))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowVD)?;
        #[cfg(all(feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowVDMD)?;
        #[cfg(feature = "row_vector2")]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowR2)?;
        #[cfg(feature = "row_vector3")]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowR3)?;
        #[cfg(feature = "row_vector4")]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowR4)?;
        #[cfg(feature = "row_vectord")]
        install_numeric_runtime_factories!(builder, sum_row, StatsSumRowRD)?;
    }

    Ok(())
}

macro_rules! for_each_stats_scalar {
    ($callback:ident, $context:tt; [$cfg:meta]; $module:ident; $factory:ident; [$($shape_feature:literal),* $(,)?]) => {
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "u8"; u8; "u8"; u8);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "u16"; u16; "u16"; u16);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "u32"; u32; "u32"; u32);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "u64"; u64; "u64"; u64);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "u128"; u128; "u128"; u128);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "i8"; i8; "i8"; i8);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "i16"; i16; "i16"; i16);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "i32"; i32; "i32"; i32);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "i64"; i64; "i64"; i64);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "i128"; i128; "i128"; i128);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "f32"; f32; "f32"; f32);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "f64"; f64; "f64"; f64);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "complex"; mech_core::C64; "c64"; c64);
        $callback!($context; [$cfg]; $module; $factory; [$($shape_feature),*]; feature = "rational"; mech_core::R64; "r64"; r64);
    };
}

macro_rules! for_each_stats_family_with_context {
    ($callback:ident, $context:tt) => {
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix1")]; sum_column; StatsSumColumnM1; ["matrix1"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix2", feature = "vector2")]; sum_column; StatsSumColumnM2; ["matrix2", "vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix3", feature = "vector3")]; sum_column; StatsSumColumnM3; ["matrix3", "vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix4", feature = "vector4")]; sum_column; StatsSumColumnM4; ["matrix4", "vector4"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix2x3", feature = "vector2")]; sum_column; StatsSumColumnM2x3; ["matrix2x3", "vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix3x2", feature = "vector3")]; sum_column; StatsSumColumnM3x2; ["matrix3x2", "vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrixd", feature = "vectord")]; sum_column; StatsSumColumnMD; ["matrixd", "vectord"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vector2")]; sum_column; StatsSumColumnV2; ["vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vector3")]; sum_column; StatsSumColumnV3; ["vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vector4")]; sum_column; StatsSumColumnV4; ["vector4"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vectord")]; sum_column; StatsSumColumnVD; ["vectord"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vector2", feature = "matrix1")]; sum_column; StatsSumColumnR2; ["matrix1", "row_vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vector3", feature = "matrix1")]; sum_column; StatsSumColumnR3; ["matrix1", "row_vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vector4", feature = "matrix1")]; sum_column; StatsSumColumnR4; ["matrix1", "row_vector4"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vectord", feature = "matrix1")]; sum_column; StatsSumColumnRD; ["matrix1", "row_vectord"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vectord", feature = "matrixd", not(feature = "matrix1"))]; sum_column; StatsSumColumnRD2; ["matrixd", "row_vectord"]);

        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix1")]; sum_row; StatsSumRowM1; ["matrix1"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix2", feature = "row_vector2")]; sum_row; StatsSumRowM2; ["matrix2", "row_vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix3", feature = "row_vector3")]; sum_row; StatsSumRowM3; ["matrix3", "row_vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix4", feature = "row_vector4")]; sum_row; StatsSumRowM4; ["matrix4", "row_vector4"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix2x3", feature = "row_vector3")]; sum_row; StatsSumRowM2x3; ["matrix2x3", "row_vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrix3x2", feature = "row_vector2")]; sum_row; StatsSumRowM3x2; ["matrix3x2", "row_vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "matrixd", feature = "row_vectord")]; sum_row; StatsSumRowMD; ["matrixd", "row_vectord"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vector2", feature = "matrix1")]; sum_row; StatsSumRowV2; ["matrix1", "vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vector3", feature = "matrix1")]; sum_row; StatsSumRowV3; ["matrix1", "vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vector4", feature = "matrix1")]; sum_row; StatsSumRowV4; ["matrix1", "vector4"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vectord", feature = "matrix1")]; sum_row; StatsSumRowVD; ["matrix1", "vectord"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "vectord", feature = "matrixd", not(feature = "matrix1"))]; sum_row; StatsSumRowVDMD; ["matrixd", "vectord"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vector2")]; sum_row; StatsSumRowR2; ["row_vector2"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vector3")]; sum_row; StatsSumRowR3; ["row_vector3"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vector4")]; sum_row; StatsSumRowR4; ["row_vector4"]);
        for_each_stats_scalar!($callback, $context; [all(feature = "sum", feature = "row_vectord")]; sum_row; StatsSumRowRD; ["row_vectord"]);
    };
}

macro_rules! for_each_stats_family {
    ($callback:ident) => {
        for_each_stats_family_with_context!($callback, ());
    };
    ($callback:ident, $context:tt) => {
        for_each_stats_family_with_context!($callback, $context);
    };
}

macro_rules! declare_stats_runtime_factory {
    ($_context:tt; [$cfg:meta]; $module:ident; $factory:ident; [$($shape_feature:literal),* $(,)?]; $scalar_feature:meta; $scalar:ty; $scalar_name:literal; $scalar_token:ident) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all($cfg, $scalar_feature),
                registration: [<register_ $factory:snake _ $scalar_token>],
                installer: [<install_ $factory:snake _ $scalar_token>],
                name: concat!(stringify!($factory), "<", $scalar_name, ">"),
                factory_type: crate::$module::$factory<$scalar>,
                contract: RuntimeFunctionContract::custom(
                    "statistical_reduction",
                    RuntimeOutputAliasPolicy::DisallowInputAlias,
                    statistical_reduction_contract!($module),
                ),
                package: "mech-stats", crate_name: "mech_stats",
                installer_path: concat!("mech_stats::__mech_native::", stringify!([<install_ $factory:snake _ $scalar_token>])),
                extra_cargo_features: ["sum"],
            }
        }
    };
}

for_each_stats_family!(declare_stats_runtime_factory);

/// Installs every concrete runtime factory declared by the statistics family traversal.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register_stats_runtime_factory {
        (($builder:ident); [$cfg:meta]; $_module:ident; $factory:ident; [$($_shape_feature:literal),* $(,)?]; $scalar_feature:meta; $_scalar:ty; $_scalar_name:literal; $scalar_token:ident) => {
            #[cfg(all($cfg, $scalar_feature))]
            mech_core::paste::paste! { [<register_ $factory:snake _ $scalar_token>]($builder)?; }
        };
    }
    for_each_stats_family!(register_stats_runtime_factory, (builder));
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_stats_runtime_factory {
        ($_context:tt; [$cfg:meta]; $_module:ident; $factory:ident; [$($_shape_feature:literal),* $(,)?]; $scalar_feature:meta; $_scalar:ty; $_scalar_name:literal; $scalar_token:ident) => {
            #[cfg(all($cfg, $scalar_feature))]
            mech_core::paste::paste! { pub use super::[<install_ $factory:snake _ $scalar_token>]; }
        };
    }
    for_each_stats_family!(export_stats_runtime_factory);
}

#[cfg(all(test, feature = "source", feature = "sum"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    #[test]
    fn sum_operations_are_nested_module_only_exports() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        assert_eq!(catalog.specializer_count(), 2);
        for (canonical_name, item) in [
            ("stats/sum/column", "sum/column"),
            ("stats/sum/row", "sum/row"),
        ] {
            let operation = OperationId::from_name(canonical_name);
            let export = catalog.module_export("stats", item).unwrap();
            assert_eq!(export.operation, operation);
            assert_eq!(export.canonical_name, canonical_name);
            assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
            assert_eq!(catalog.exports_for_operation(operation), [export.clone()]);
        }
    }
}
