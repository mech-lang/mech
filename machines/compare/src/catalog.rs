#[cfg(any(feature = "eq", feature = "gt", feature = "gte", feature = "lt", feature = "lte", feature = "max", feature = "min", feature = "neq"))]
use crate::*;
#[cfg(feature = "complex")]
use mech_core::C64;
#[cfg(feature = "rational")]
use mech_core::R64;
use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(any(feature = "seq", feature = "sneq"))]
use mech_core::ValueCell;
#[cfg(any(
    feature = "seq",
    feature = "sneq",
    all(feature = "eq", feature = "atom"),
    all(feature = "eq", feature = "table"),
    all(feature = "neq", feature = "atom"),
    all(feature = "neq", feature = "table")
))]
use mech_core::{RuntimeFunctionContract, RuntimeOutputAliasPolicy};
#[cfg(feature = "source")]
use mech_core::{CanonicalFunctionSpecializer, FunctionExport, FunctionExposure};
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(feature = "source")]
fn install_canonical_operation<T>(
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
    #[cfg(feature = "eq")]
    install_canonical_operation(
        builder,
        "compare/eq",
        crate::CompareEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "gt")]
    install_canonical_operation(
        builder,
        "compare/gt",
        crate::CompareGreaterThan {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "gte")]
    install_canonical_operation(
        builder,
        "compare/gte",
        crate::CompareGreaterThanEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "lt")]
    install_canonical_operation(
        builder,
        "compare/lt",
        crate::CompareLessThan {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "lte")]
    install_canonical_operation(
        builder,
        "compare/lte",
        crate::CompareLessThanEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "max")]
    install_canonical_operation(
        builder,
        "compare/max",
        crate::CompareMax {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "min")]
    install_canonical_operation(
        builder,
        "compare/min",
        crate::CompareMin {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "neq")]
    install_canonical_operation(
        builder,
        "compare/neq",
        crate::CompareNotEqual {},
        FunctionExposure::Prelude,
    )?;
    #[cfg(feature = "seq")]
    install_canonical_operation(
        builder,
        "compare/seq",
        crate::CompareStrictEqual {},
        FunctionExposure::Internal,
    )?;
    #[cfg(feature = "sneq")]
    install_canonical_operation(
        builder,
        "compare/sneq",
        crate::CompareStrictNotEqual {},
        FunctionExposure::Internal,
    )?;
    Ok(())
}

#[cfg(any(feature = "eq", feature = "gt", feature = "gte", feature = "lt", feature = "lte", feature = "max", feature = "min", feature = "neq"))]
macro_rules! install_compare_binop_runtime {
    ($builder:expr, $operation:ident) => {
        mech_core::install_native_binop_runtime_factories!(
            $builder,
            $operation;
            ("bool", bool, "bool", bool),
            ("string", String, "string", string),
            ("u8", u8, "u8", u8), ("i8", i8, "i8", i8),
            ("u16", u16, "u16", u16), ("i16", i16, "i16", i16),
            ("u32", u32, "u32", u32), ("i32", i32, "i32", i32),
            ("u64", u64, "u64", u64), ("i64", i64, "i64", i64),
            ("u128", u128, "u128", u128), ("i128", i128, "i128", i128),
            ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
            ("r64", R64, "r64", r64), ("c64", C64, "c64", c64),
        )?;
    };
}

macro_rules! declare_compare_binop_native_factories {
    ($operation:ident, $operation_feature:literal) => {
        mech_core::declare_native_binop_runtime_factories! {
            package: "mech-compare",
            crate_name: "mech_compare",
            operation: $operation,
            operation_feature: $operation_feature,
            additional_features: [],
            scalars: ("bool", bool, "bool", bool),
        }
        mech_core::declare_native_binop_runtime_factories! {
            package: "mech-compare",
            crate_name: "mech_compare",
            operation: $operation,
            operation_feature: $operation_feature,
            additional_features: ["bool"],
            scalars:
                ("string", String, "string", string),
                ("u8", u8, "u8", u8), ("i8", i8, "i8", i8),
                ("u16", u16, "u16", u16), ("i16", i16, "i16", i16),
                ("u32", u32, "u32", u32), ("i32", i32, "i32", i32),
                ("u64", u64, "u64", u64), ("i64", i64, "i64", i64),
                ("u128", u128, "u128", u128), ("i128", i128, "i128", i128),
                ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
                ("r64", R64, "r64", r64), ("c64", C64, "c64", c64),
        }
    };
}

declare_compare_binop_native_factories!(EQ, "eq");
declare_compare_binop_native_factories!(GT, "gt");
declare_compare_binop_native_factories!(GTE, "gte");
declare_compare_binop_native_factories!(LT, "lt");
declare_compare_binop_native_factories!(LTE, "lte");
declare_compare_binop_native_factories!(Max, "max");
declare_compare_binop_native_factories!(Min, "min");
declare_compare_binop_native_factories!(NEQ, "neq");

#[cfg(any(feature = "seq", feature = "sneq"))]
fn validate_strict_comparison_canonical(_: &ValueCell, _: &[ValueCell]) -> MResult<()> {
    Ok(())
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "seq",
    registration: register_strict_eq,
    installer: install_strict_eq,
    name: "compare/seq",
    factory_type: crate::StrictEqValue,
    contract: RuntimeFunctionContract::canonical_custom(
        "strict_comparison",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_strict_comparison_canonical,
    ),
    package: "mech-compare",
    crate_name: "mech_compare",
    installer_path: "mech_compare::__mech_native::install_strict_eq",
    extra_cargo_features: ["seq"],
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "sneq",
    registration: register_strict_not_eq,
    installer: install_strict_not_eq,
    name: "compare/sneq",
    factory_type: crate::StrictNotEqValue,
    contract: RuntimeFunctionContract::canonical_custom(
        "strict_comparison",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_strict_comparison_canonical,
    ),
    package: "mech-compare",
    crate_name: "mech_compare",
    installer_path: "mech_compare::__mech_native::install_strict_not_eq",
    extra_cargo_features: ["sneq"],
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "eq", feature = "atom"),
    registration: register_atom_eq,
    installer: install_atom_eq,
    name: "AtomEq",
    factory_type: AtomEq,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-compare",
    crate_name: "mech_compare",
    installer_path: "mech_compare::__mech_native::install_atom_eq",
    extra_cargo_features: ["eq"],
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "eq", feature = "table"),
    registration: register_table_eq,
    installer: install_table_eq,
    name: "TableEq",
    factory_type: TableEq,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-compare",
    crate_name: "mech_compare",
    installer_path: "mech_compare::__mech_native::install_table_eq",
    extra_cargo_features: ["eq"],
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "neq", feature = "atom"),
    registration: register_atom_neq,
    installer: install_atom_neq,
    name: "AtomNeq",
    factory_type: AtomNeq,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-compare",
    crate_name: "mech_compare",
    installer_path: "mech_compare::__mech_native::install_atom_neq",
    extra_cargo_features: ["neq"],
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "neq", feature = "table"),
    registration: register_table_neq,
    installer: install_table_neq,
    name: "TableNeq",
    factory_type: TableNeq,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-compare",
    crate_name: "mech_compare",
    installer_path: "mech_compare::__mech_native::install_table_neq",
    extra_cargo_features: ["neq"],
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_compare_binop_native_factories {
        ($operation:ident, $operation_feature:literal) => {
            mech_core::export_native_binop_runtime_factories! {
                operation_feature: $operation_feature,
                operation: $operation;
                ("bool", bool, "bool", bool),
                ("string", String, "string", string),
                ("u8", u8, "u8", u8), ("i8", i8, "i8", i8),
                ("u16", u16, "u16", u16), ("i16", i16, "i16", i16),
                ("u32", u32, "u32", u32), ("i32", i32, "i32", i32),
                ("u64", u64, "u64", u64), ("i64", i64, "i64", i64),
                ("u128", u128, "u128", u128), ("i128", i128, "i128", i128),
                ("f32", f32, "f32", f32), ("f64", f64, "f64", f64),
                ("r64", R64, "r64", r64), ("c64", C64, "c64", c64),
            }
        };
    }
    export_compare_binop_native_factories!(EQ, "eq");
    export_compare_binop_native_factories!(GT, "gt");
    export_compare_binop_native_factories!(GTE, "gte");
    export_compare_binop_native_factories!(LT, "lt");
    export_compare_binop_native_factories!(LTE, "lte");
    export_compare_binop_native_factories!(Max, "max");
    export_compare_binop_native_factories!(Min, "min");
    export_compare_binop_native_factories!(NEQ, "neq");
    #[cfg(feature = "seq")]
    pub use super::install_strict_eq;
    #[cfg(feature = "sneq")]
    pub use super::install_strict_not_eq;
    #[cfg(all(feature = "eq", feature = "atom"))]
    pub use super::install_atom_eq;
    #[cfg(all(feature = "neq", feature = "atom"))]
    pub use super::install_atom_neq;
    #[cfg(all(feature = "eq", feature = "table"))]
    pub use super::install_table_eq;
    #[cfg(all(feature = "neq", feature = "table"))]
    pub use super::install_table_neq;
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
    #[cfg(feature = "seq")]
    register_strict_eq(builder)?;
    #[cfg(feature = "sneq")]
    register_strict_not_eq(builder)?;

    #[cfg(all(feature = "eq", feature = "atom"))]
    register_atom_eq(builder)?;
    #[cfg(all(feature = "eq", feature = "table"))]
    register_table_eq(builder)?;
    #[cfg(all(feature = "neq", feature = "atom"))]
    register_atom_neq(builder)?;
    #[cfg(all(feature = "neq", feature = "table"))]
    register_table_neq(builder)?;

    Ok(())
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

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
}
