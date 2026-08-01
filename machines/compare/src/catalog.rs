use crate::*;
#[cfg(feature = "complex")]
use mech_core::C64;
#[cfg(feature = "rational")]
use mech_core::R64;
use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult, MechFunctionFactory,
    NativeFunctionCompiler, install_binop_runtime_factories, legacy_source_specializer,
};
use paste::paste;

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
    #[cfg(feature = "eq")]
    install_operation(
        builder,
        "compare/eq",
        crate::CompareEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "gt")]
    install_operation(
        builder,
        "compare/gt",
        crate::CompareGreaterThan {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "gte")]
    install_operation(
        builder,
        "compare/gte",
        crate::CompareGreaterThanEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "lt")]
    install_operation(
        builder,
        "compare/lt",
        crate::CompareLessThan {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "lte")]
    install_operation(
        builder,
        "compare/lte",
        crate::CompareLessThanEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "max")]
    install_operation(
        builder,
        "compare/max",
        crate::CompareMax {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "min")]
    install_operation(
        builder,
        "compare/min",
        crate::CompareMin {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "neq")]
    install_operation(
        builder,
        "compare/neq",
        crate::CompareNotEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "seq")]
    install_operation(
        builder,
        "compare/seq",
        crate::CompareStrictEqual {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "sneq")]
    install_operation(
        builder,
        "compare/sneq",
        crate::CompareStrictNotEqual {},
        FunctionExposure::Internal,
    )?;
    Ok(())
}

macro_rules! install_compare_binop_runtime {
    ($builder:expr, $operation:ident) => {
        install_binop_runtime_factories!(
            $builder,
            $operation;
            ("bool", bool, "bool"),
            ("string", String, "string"),
            ("u8", u8, "u8"),
            ("i8", i8, "i8"),
            ("u16", u16, "u16"),
            ("i16", i16, "i16"),
            ("u32", u32, "u32"),
            ("i32", i32, "i32"),
            ("u64", u64, "u64"),
            ("i64", i64, "i64"),
            ("u128", u128, "u128"),
            ("i128", i128, "i128"),
            ("f32", f32, "f32"),
            ("f64", f64, "f64"),
            ("r64", R64, "r64"),
            ("c64", C64, "c64"),
        )?;
    };
}

#[cfg(feature = "eq")]
fn install_eq_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, EQ);
    Ok(())
}

#[cfg(feature = "gt")]
fn install_gt_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, GT);
    Ok(())
}

#[cfg(feature = "gte")]
fn install_gte_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, GTE);
    Ok(())
}

#[cfg(feature = "lt")]
fn install_lt_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, LT);
    Ok(())
}

#[cfg(feature = "lte")]
fn install_lte_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, LTE);
    Ok(())
}

#[cfg(feature = "max")]
fn install_max_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, Max);
    Ok(())
}

#[cfg(feature = "min")]
fn install_min_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, Min);
    Ok(())
}

#[cfg(feature = "neq")]
fn install_neq_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_compare_binop_runtime!(builder, NEQ);
    Ok(())
}

/// Installs every enabled concrete bytecode factory owned by `mech-compare`.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "eq")]
    install_eq_runtime(builder)?;
    #[cfg(feature = "gt")]
    install_gt_runtime(builder)?;
    #[cfg(feature = "gte")]
    install_gte_runtime(builder)?;
    #[cfg(feature = "lt")]
    install_lt_runtime(builder)?;
    #[cfg(feature = "lte")]
    install_lte_runtime(builder)?;
    #[cfg(feature = "max")]
    install_max_runtime(builder)?;
    #[cfg(feature = "min")]
    install_min_runtime(builder)?;
    #[cfg(feature = "neq")]
    install_neq_runtime(builder)?;

    #[cfg(all(feature = "eq", feature = "atom"))]
    builder.insert_runtime_factory("AtomEq", <AtomEq as MechFunctionFactory>::new)?;
    #[cfg(all(feature = "eq", feature = "table"))]
    builder.insert_runtime_factory("TableEq", <TableEq as MechFunctionFactory>::new)?;
    #[cfg(all(feature = "neq", feature = "atom"))]
    builder.insert_runtime_factory("AtomNeq", <AtomNeq as MechFunctionFactory>::new)?;
    #[cfg(all(feature = "neq", feature = "table"))]
    builder.insert_runtime_factory("TableNeq", <TableNeq as MechFunctionFactory>::new)?;

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
        #[cfg(feature = "eq")]
        expected.push(("compare/eq", FunctionExposure::Prelude));
        #[cfg(feature = "gt")]
        expected.push(("compare/gt", FunctionExposure::Prelude));
        #[cfg(feature = "gte")]
        expected.push(("compare/gte", FunctionExposure::Prelude));
        #[cfg(feature = "lt")]
        expected.push(("compare/lt", FunctionExposure::Prelude));
        #[cfg(feature = "lte")]
        expected.push(("compare/lte", FunctionExposure::Prelude));
        #[cfg(feature = "max")]
        expected.push(("compare/max", FunctionExposure::Internal));
        #[cfg(feature = "min")]
        expected.push(("compare/min", FunctionExposure::Internal));
        #[cfg(feature = "neq")]
        expected.push(("compare/neq", FunctionExposure::Prelude));
        #[cfg(feature = "seq")]
        expected.push(("compare/seq", FunctionExposure::Internal));
        #[cfg(feature = "sneq")]
        expected.push(("compare/sneq", FunctionExposure::Internal));
        expected
    }

    #[test]
    fn source_catalog_matches_the_frozen_compare_surface() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let expected = expected_operations();

        #[cfg(all(
            feature = "eq",
            feature = "gt",
            feature = "gte",
            feature = "lt",
            feature = "lte",
            feature = "max",
            feature = "min",
            feature = "neq",
            feature = "seq",
            feature = "sneq",
        ))]
        assert_eq!(expected.len(), 10);
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
    fn owns_runtime_name(name: &str) -> bool {
        let stem = name.split('<').next().unwrap_or(name);
        matches!(stem, "AtomEq" | "TableEq" | "AtomNeq" | "TableNeq")
            || ["EQ", "GT", "GTE", "LT", "LTE", "Max", "Min", "NEQ"]
                .iter()
                .any(|prefix| stem.starts_with(prefix))
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
            if owns_runtime_name(descriptor.name) {
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
            feature = "eq",
            feature = "gt",
            feature = "gte",
            feature = "lt",
            feature = "lte",
            feature = "max",
            feature = "min",
            feature = "neq",
            feature = "atom",
            feature = "table",
            feature = "matrixd",
            feature = "vectord",
            feature = "row_vectord",
            not(feature = "matrix1"),
            not(feature = "matrix2"),
            not(feature = "matrix3"),
            not(feature = "matrix4"),
            not(feature = "matrix2x3"),
            not(feature = "matrix3x2"),
            not(feature = "vector2"),
            not(feature = "vector3"),
            not(feature = "vector4"),
            not(feature = "row_vector2"),
            not(feature = "row_vector3"),
            not(feature = "row_vector4"),
        ))]
        assert_eq!(explicit.len(), 1_796);
    }
}
