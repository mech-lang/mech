use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionSpecializer, MResult,
    MechFunctionFactory,
};
use std::sync::Arc;

#[cfg(feature = "cartesian_product")]
use crate::SetCartesianProduct;
#[cfg(feature = "difference")]
use crate::SetDifference;
#[cfg(feature = "disjoint")]
use crate::SetDisjoint;
#[cfg(feature = "element_of")]
use crate::SetElementOf;
#[cfg(feature = "equals")]
use crate::SetEquals;
#[cfg(feature = "insert")]
use crate::SetInsert;
#[cfg(feature = "intersection")]
use crate::SetIntersection;
#[cfg(feature = "not_element_of")]
use crate::SetNotElementOf;
#[cfg(feature = "not_equals")]
use crate::SetNotEquals;
#[cfg(feature = "powerset")]
use crate::SetPowerset;
#[cfg(feature = "proper_subset")]
use crate::SetProperSubset;
#[cfg(feature = "proper_superset")]
use crate::SetProperSuperset;
#[cfg(feature = "remove")]
use crate::SetRemove;
#[cfg(all(feature = "size", feature = "u64"))]
use crate::SetSize;
#[cfg(feature = "subset")]
use crate::SetSubset;
#[cfg(feature = "superset")]
use crate::SetSuperset;
#[cfg(feature = "symmetric_difference")]
use crate::SetSymmetricDifference;
#[cfg(feature = "union")]
use crate::SetUnion;

fn install_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &'static str,
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

/// Installs the frozen named source-specializer surface for the set machine.
///
/// `set/complement` was not part of that surface, and the undeclared legacy
/// `src/union.rs` implementation is deliberately not reachable from here.
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "cartesian_product")]
    install_operation(
        builder,
        "set/cartesian-product",
        SetCartesianProduct {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "difference")]
    install_operation(
        builder,
        "set/difference",
        SetDifference {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "disjoint")]
    install_operation(
        builder,
        "set/disjoint",
        SetDisjoint {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "element_of")]
    install_operation(
        builder,
        "set/element-of",
        SetElementOf {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "equals")]
    install_operation(
        builder,
        "set/equals",
        SetEquals {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "insert")]
    install_operation(
        builder,
        "set/insert",
        SetInsert {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "intersection")]
    install_operation(
        builder,
        "set/intersection",
        SetIntersection {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "not_element_of")]
    install_operation(
        builder,
        "set/not-element-of",
        SetNotElementOf {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "not_equals")]
    install_operation(
        builder,
        "set/not_equals",
        SetNotEquals {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "powerset")]
    install_operation(
        builder,
        "set/powerset",
        SetPowerset {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "proper_superset")]
    install_operation(
        builder,
        "set/proper-superset",
        SetProperSuperset {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "proper_subset")]
    install_operation(
        builder,
        "set/proper_subset",
        SetProperSubset {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "remove")]
    install_operation(
        builder,
        "set/remove",
        SetRemove {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(all(feature = "size", feature = "u64"))]
    install_operation(builder, "set/size", SetSize {}, FunctionExposure::Prelude)?;
    #[cfg(feature = "subset")]
    install_operation(
        builder,
        "set/subset",
        SetSubset {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "superset")]
    install_operation(
        builder,
        "set/superset",
        SetSuperset {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "symmetric_difference")]
    install_operation(
        builder,
        "set/symmetric-difference",
        SetSymmetricDifference {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "union")]
    install_operation(builder, "set/union", SetUnion {}, FunctionExposure::Prelude)?;

    Ok(())
}

/// Installs every concrete runtime factory linked by the enabled set features.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "cartesian_product")]
    builder.insert_runtime_factory(
        "SetCartesianProductFxn",
        <crate::operations::cartesian_product::SetCartesianProductFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "difference")]
    builder.insert_runtime_factory(
        "SetDifferenceFxn",
        <crate::operations::difference::SetDifferenceFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "disjoint")]
    builder.insert_runtime_factory(
        "SetDisjointFxn",
        <crate::relations::disjoint::SetDisjointFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "element_of")]
    builder.insert_runtime_factory(
        "SetElementOfFxn",
        <crate::membership::element_of::SetElementOfFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "equals")]
    builder.insert_runtime_factory(
        "SetEqualsFxn",
        <crate::relations::equals::SetEqualsFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "insert")]
    builder.insert_runtime_factory(
        "SetInsertFxn",
        <crate::modify::insert::SetInsertFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "intersection")]
    builder.insert_runtime_factory(
        "SetIntersectionFxn",
        <crate::operations::intersection::SetIntersectionFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "not_element_of")]
    builder.insert_runtime_factory(
        "SetNotElementOfFxn",
        <crate::membership::not_element_of::SetNotElementOfFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "not_equals")]
    builder.insert_runtime_factory(
        "SetNotEqualsFxn",
        <crate::relations::not_equals::SetNotEqualsFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "powerset")]
    builder.insert_runtime_factory(
        "SetPowersetFxn",
        <crate::operations::powerset::SetPowersetFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "proper_subset")]
    builder.insert_runtime_factory(
        "SetProperSubsetFxn",
        <crate::relations::proper_subset::SetProperSubsetFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "proper_superset")]
    builder.insert_runtime_factory(
        "SetProperSupersetFxn",
        <crate::relations::proper_superset::SetProperSupersetFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "remove")]
    builder.insert_runtime_factory(
        "SetRemoveFxn",
        <crate::modify::remove::SetRemoveFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(all(feature = "size", feature = "u64"))]
    builder.insert_runtime_factory(
        "SetSizeFxn",
        <crate::setdata::size::SetSizeFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "subset")]
    builder.insert_runtime_factory(
        "SetSubsetFxn",
        <crate::relations::subset::SetSubsetFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "superset")]
    builder.insert_runtime_factory(
        "SetSupersetFxn",
        <crate::relations::superset::SetSupersetFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "symmetric_difference")]
    builder.insert_runtime_factory(
        "SetSymDifferenceFxn",
        <crate::operations::symmetric_difference::SetSymDifferenceFxn as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "union")]
    builder.insert_runtime_factory(
        "SetUnionFxn",
        <crate::operations::union::SetUnionFxn as MechFunctionFactory>::new,
    )?;

    Ok(())
}

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(all(
    test,
    feature = "cartesian_product",
    feature = "difference",
    feature = "disjoint",
    feature = "element_of",
    feature = "equals",
    feature = "insert",
    feature = "intersection",
    feature = "not_element_of",
    feature = "not_equals",
    feature = "powerset",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "remove",
    feature = "size",
    feature = "u64",
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    const PRELUDE: &[&str] = &[
        "set/difference",
        "set/disjoint",
        "set/equals",
        "set/insert",
        "set/intersection",
        "set/not_equals",
        "set/powerset",
        "set/proper_subset",
        "set/remove",
        "set/size",
        "set/subset",
        "set/superset",
        "set/union",
    ];

    const INTERNAL: &[&str] = &[
        "set/cartesian-product",
        "set/element-of",
        "set/not-element-of",
        "set/proper-superset",
        "set/symmetric-difference",
    ];

    #[test]
    fn source_catalog_matches_the_frozen_set_surface() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        assert_eq!(catalog.specializer_count(), 18);
        assert!(
            catalog
                .specializer(OperationId::from_name("set/complement"))
                .is_none()
        );

        for canonical_name in PRELUDE {
            let operation = OperationId::from_name(canonical_name);
            assert_eq!(catalog.exports_for_operation(operation).len(), 1);
            assert_eq!(
                catalog.exports_for_operation(operation)[0].exposure,
                FunctionExposure::Prelude,
                "{canonical_name}",
            );
        }

        for canonical_name in INTERNAL {
            let operation = OperationId::from_name(canonical_name);
            assert_eq!(catalog.exports_for_operation(operation).len(), 1);
            assert_eq!(
                catalog.exports_for_operation(operation)[0].exposure,
                FunctionExposure::Internal,
                "{canonical_name}",
            );
        }
    }
}
