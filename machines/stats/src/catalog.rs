use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult, NativeFunctionCompiler,
    legacy_source_specializer,
};

#[cfg(feature = "sum")]
use crate::{StatsSumColumn, StatsSumRow};

macro_rules! install_numeric_runtime_factories {
    ($builder:expr, $module:ident, $factory:ident) => {{
        #[cfg(feature = "u8")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u8>"),
            <crate::$module::$factory<u8> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u16")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u16>"),
            <crate::$module::$factory<u16> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u32")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u32>"),
            <crate::$module::$factory<u32> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u64>"),
            <crate::$module::$factory<u64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u128")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u128>"),
            <crate::$module::$factory<u128> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i8")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i8>"),
            <crate::$module::$factory<i8> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i16")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i16>"),
            <crate::$module::$factory<i16> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i32")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i32>"),
            <crate::$module::$factory<i32> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i64>"),
            <crate::$module::$factory<i64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i128")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i128>"),
            <crate::$module::$factory<i128> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "f32")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<f32>"),
            <crate::$module::$factory<f32> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "f64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<f64>"),
            <crate::$module::$factory<f64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "complex")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<c64>"),
            <crate::$module::$factory<mech_core::C64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "rational")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<r64>"),
            <crate::$module::$factory<mech_core::R64> as mech_core::MechFunctionFactory>::new,
        )?;

        Ok::<(), mech_core::MechError>(())
    }};
}

fn install_module_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &'static str,
    item: &'static str,
    compiler: T,
) -> MResult<()>
where
    T: NativeFunctionCompiler + 'static,
{
    let operation =
        builder.insert_specializer(canonical_name, legacy_source_specializer(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: Some("stats".to_string()),
        item: Some(item.to_string()),
        exposure: FunctionExposure::ModuleOnly,
    })
}

/// Installs the frozen named source-specializer surface for the statistics machine.
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "sum")]
    {
        install_module_operation(builder, "stats/sum/column", "sum/column", StatsSumColumn {})?;
        install_module_operation(builder, "stats/sum/row", "sum/row", StatsSumRow {})?;
    }

    Ok(())
}

/// Installs the concrete runtime factories for every enabled statistics shape.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
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

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod runtime_tests {
    use super::*;
    use mech_core::FunctionDescriptor;
    use std::collections::BTreeMap;

    #[test]
    fn explicit_runtime_factories_match_the_linked_stats_inventory() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        let mut legacy = BTreeMap::new();
        for descriptor in inventory::iter::<FunctionDescriptor>
            .into_iter()
            .filter(|descriptor| {
                descriptor.name.starts_with("StatsSumColumn")
                    || descriptor.name.starts_with("StatsSumRow")
            })
        {
            assert!(legacy.insert(descriptor.name, descriptor.ptr).is_none());
        }

        assert_eq!(catalog.runtime_factory_count(), legacy.len());
        for entry in catalog.runtime_entries() {
            let legacy_factory = legacy
                .remove(entry.name.as_str())
                .unwrap_or_else(|| panic!("missing legacy stats factory {}", entry.name));
            assert_eq!(
                entry.factory as usize, legacy_factory as usize,
                "{}",
                entry.name
            );
        }
        assert!(
            legacy.is_empty(),
            "unmigrated legacy stats factories: {legacy:?}"
        );
    }
}

#[cfg(all(test, feature = "sum"))]
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
