use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult, MechFunctionFactory,
    NativeFunctionCompiler, legacy_source_specializer,
};

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

fn install_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
    exposure: FunctionExposure,
) -> MResult<()>
where
    T: NativeFunctionCompiler + 'static,
{
    let operation =
        builder.insert_specializer(canonical_name, legacy_source_specializer(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: None,
        item: None,
        exposure,
    })
}

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
        $builder.insert_runtime_factory(
            concat!(
                stringify!($factory),
                "<",
                $scalar_name,
                stringify!($shape),
                ">"
            ),
            <crate::$module::$factory<$scalar, $shape<$scalar>> as MechFunctionFactory>::new,
        )?;
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

/// Installs every enabled concrete bytecode factory owned by `mech-range`.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
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

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{FunctionDescriptor, OperationId};
    use std::collections::BTreeMap;

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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn runtime_catalog_matches_legacy_inventory_names_ids_and_pointers() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let explicit: BTreeMap<_, _> = catalog
            .runtime_entries()
            .map(|entry| (entry.name.clone(), entry.factory as usize))
            .collect();
        let mut legacy = BTreeMap::new();
        for descriptor in inventory::iter::<FunctionDescriptor> {
            if [
                "RangeExclusiveScalar",
                "RangeIncrementExclusiveScalar",
                "RangeInclusiveScalar",
                "RangeIncrementInclusiveScalar",
            ]
            .iter()
            .any(|prefix| descriptor.name.starts_with(prefix))
            {
                if let Some(existing) = legacy.insert(descriptor.name, descriptor.ptr as usize) {
                    assert_eq!(existing, descriptor.ptr as usize);
                }
            }
        }

        assert_eq!(explicit.len(), legacy.len());
        for (name, pointer) in legacy {
            let entry = catalog
                .runtime_entry(mech_core::RuntimeFunctionId::from_name(name))
                .unwrap_or_else(|| panic!("missing explicit runtime factory {name}"));
            assert_eq!(entry.name, name);
            assert_eq!(
                entry.factory as usize, pointer,
                "factory mismatch for {name}"
            );
        }

        #[cfg(all(
            feature = "exclusive",
            feature = "inclusive",
            feature = "matrixd",
            feature = "row_vectord",
            not(feature = "matrix1"),
            not(feature = "row_vector2"),
            not(feature = "row_vector3"),
            not(feature = "row_vector4"),
        ))]
        assert_eq!(explicit.len(), 96);
    }
}
