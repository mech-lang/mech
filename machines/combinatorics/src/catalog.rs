use mech_core::{FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult};
use std::sync::Arc;

#[cfg(feature = "n_choose_k")]
use crate::CombinatoricsNChooseK;

macro_rules! install_numeric_runtime_factories {
    ($builder:expr, $factory:ident) => {{
        #[cfg(feature = "u8")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u8>"),
            <crate::n_choose_k::$factory<u8> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i8")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i8>"),
            <crate::n_choose_k::$factory<i8> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u16")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u16>"),
            <crate::n_choose_k::$factory<u16> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i16")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i16>"),
            <crate::n_choose_k::$factory<i16> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u32")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u32>"),
            <crate::n_choose_k::$factory<u32> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i32")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i32>"),
            <crate::n_choose_k::$factory<i32> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u64>"),
            <crate::n_choose_k::$factory<u64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i64>"),
            <crate::n_choose_k::$factory<i64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "u128")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<u128>"),
            <crate::n_choose_k::$factory<u128> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "i128")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<i128>"),
            <crate::n_choose_k::$factory<i128> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "f32")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<f32>"),
            <crate::n_choose_k::$factory<f32> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "f64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<f64>"),
            <crate::n_choose_k::$factory<f64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "r64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<r64>"),
            <crate::n_choose_k::$factory<mech_core::R64> as mech_core::MechFunctionFactory>::new,
        )?;
        #[cfg(feature = "c64")]
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<c64>"),
            <crate::n_choose_k::$factory<mech_core::C64> as mech_core::MechFunctionFactory>::new,
        )?;

        Ok::<(), mech_core::MechError>(())
    }};
}

/// Installs the frozen named source-specializer surface for the combinatorics machine.
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "n_choose_k")]
    {
        let canonical_name = "combinatorics/n-choose-k";
        let operation =
            builder.insert_specializer(canonical_name, Arc::new(CombinatoricsNChooseK {}))?;
        builder.insert_export(FunctionExport {
            operation,
            canonical_name: canonical_name.to_string(),
            module: Some("combinatorics".to_string()),
            item: Some("n-choose-k".to_string()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(())
}

/// Installs the concrete scalar and matrix n-choose-k runtime factories.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "n_choose_k")]
    {
        install_numeric_runtime_factories!(builder, NChooseK)?;
        #[cfg(feature = "matrix")]
        install_numeric_runtime_factories!(builder, NChooseKMatrix)?;
    }
    Ok(())
}

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(all(test, feature = "n_choose_k"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    #[test]
    fn n_choose_k_is_the_only_module_only_source_operation() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let operation = OperationId::from_name("combinatorics/n-choose-k");

        assert_eq!(catalog.specializer_count(), 1);
        let export = catalog
            .module_export("combinatorics", "n-choose-k")
            .unwrap();
        assert_eq!(export.operation, operation);
        assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
        assert_eq!(catalog.exports_for_operation(operation), [export.clone()]);
    }
}
