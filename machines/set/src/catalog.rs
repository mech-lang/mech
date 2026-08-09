use mech_core::{
    function_shape_contract_violation, FunctionArgs, FunctionCatalogBuilder, MResult,
    MechFunctionFactory, RuntimeFunctionContract, RuntimeOutputAliasPolicy, ValueKind,
};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(all(feature = "source", feature = "cartesian_product"))]
use crate::SetCartesianProduct;
#[cfg(all(feature = "source", feature = "difference"))]
use crate::SetDifference;
#[cfg(all(feature = "source", feature = "disjoint"))]
use crate::SetDisjoint;
#[cfg(all(feature = "source", feature = "element_of"))]
use crate::SetElementOf;
#[cfg(all(feature = "source", feature = "equals"))]
use crate::SetEquals;
#[cfg(all(feature = "source", feature = "insert"))]
use crate::SetInsert;
#[cfg(all(feature = "source", feature = "intersection"))]
use crate::SetIntersection;
#[cfg(all(feature = "source", feature = "not_element_of"))]
use crate::SetNotElementOf;
#[cfg(all(feature = "source", feature = "not_equals"))]
use crate::SetNotEquals;
#[cfg(all(feature = "source", feature = "powerset"))]
use crate::SetPowerset;
#[cfg(all(feature = "source", feature = "proper_subset"))]
use crate::SetProperSubset;
#[cfg(all(feature = "source", feature = "proper_superset"))]
use crate::SetProperSuperset;
#[cfg(all(feature = "source", feature = "remove"))]
use crate::SetRemove;
#[cfg(all(feature = "source", feature = "size", feature = "u64"))]
use crate::SetSize;
#[cfg(all(feature = "source", feature = "subset"))]
use crate::SetSubset;
#[cfg(all(feature = "source", feature = "superset"))]
use crate::SetSuperset;
#[cfg(all(feature = "source", feature = "symmetric_difference"))]
use crate::SetSymmetricDifference;
#[cfg(all(feature = "source", feature = "union"))]
use crate::SetUnion;

#[cfg(feature = "source")]
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
#[cfg(feature = "source")]
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

macro_rules! for_each_set_runtime_factory {
    ($callback:ident) => {
        $callback!(feature = "cartesian_product"; register_set_cartesian_product_fxn; install_set_cartesian_product_fxn; "SetCartesianProductFxn"; crate::operations::cartesian_product::SetCartesianProductFxn; ["cartesian_product"]);
        $callback!(feature = "difference"; register_set_difference_fxn; install_set_difference_fxn; "SetDifferenceFxn"; crate::operations::difference::SetDifferenceFxn; ["difference"]);
        $callback!(feature = "disjoint"; register_set_disjoint_fxn; install_set_disjoint_fxn; "SetDisjointFxn"; crate::relations::disjoint::SetDisjointFxn; ["disjoint"]);
        $callback!(feature = "element_of"; register_set_element_of_fxn; install_set_element_of_fxn; "SetElementOfFxn"; crate::membership::element_of::SetElementOfFxn; ["element_of"]);
        $callback!(feature = "equals"; register_set_equals_fxn; install_set_equals_fxn; "SetEqualsFxn"; crate::relations::equals::SetEqualsFxn; ["equals"]);
        $callback!(feature = "insert"; register_set_insert_fxn; install_set_insert_fxn; "SetInsertFxn"; crate::modify::insert::SetInsertFxn; ["insert"]);
        $callback!(feature = "intersection"; register_set_intersection_fxn; install_set_intersection_fxn; "SetIntersectionFxn"; crate::operations::intersection::SetIntersectionFxn; ["intersection"]);
        $callback!(feature = "not_element_of"; register_set_not_element_of_fxn; install_set_not_element_of_fxn; "SetNotElementOfFxn"; crate::membership::not_element_of::SetNotElementOfFxn; ["not_element_of"]);
        $callback!(feature = "not_equals"; register_set_not_equals_fxn; install_set_not_equals_fxn; "SetNotEqualsFxn"; crate::relations::not_equals::SetNotEqualsFxn; ["not_equals"]);
        $callback!(feature = "powerset"; register_set_powerset_fxn; install_set_powerset_fxn; "SetPowersetFxn"; crate::operations::powerset::SetPowersetFxn; ["powerset"]);
        $callback!(feature = "proper_subset"; register_set_proper_subset_fxn; install_set_proper_subset_fxn; "SetProperSubsetFxn"; crate::relations::proper_subset::SetProperSubsetFxn; ["proper_subset"]);
        $callback!(feature = "proper_superset"; register_set_proper_superset_fxn; install_set_proper_superset_fxn; "SetProperSupersetFxn"; crate::relations::proper_superset::SetProperSupersetFxn; ["proper_superset"]);
        $callback!(feature = "remove"; register_set_remove_fxn; install_set_remove_fxn; "SetRemoveFxn"; crate::modify::remove::SetRemoveFxn; ["remove"]);
        $callback!(all(feature = "size", feature = "u64"); register_set_size_fxn; install_set_size_fxn; "SetSizeFxn"; crate::setdata::size::SetSizeFxn; ["size"]);
        $callback!(feature = "subset"; register_set_subset_fxn; install_set_subset_fxn; "SetSubsetFxn"; crate::relations::subset::SetSubsetFxn; ["subset"]);
        $callback!(feature = "superset"; register_set_superset_fxn; install_set_superset_fxn; "SetSupersetFxn"; crate::relations::superset::SetSupersetFxn; ["superset"]);
        $callback!(feature = "symmetric_difference"; register_set_symmetric_difference_fxn; install_set_symmetric_difference_fxn; "SetSymDifferenceFxn"; crate::operations::symmetric_difference::SetSymDifferenceFxn; ["symmetric_difference"]);
        $callback!(feature = "union"; register_set_union_fxn; install_set_union_fxn; "SetUnionFxn"; crate::operations::union::SetUnionFxn; ["union"]);
    };
}

macro_rules! declare_set_runtime_factory {
    ($cfg:meta; $registration:ident; $installer:ident; $name:literal; $factory:path; [$($feature:literal),* $(,)?]) => {
        mech_core::declare_native_runtime_factory! {
            cfg: $cfg,
            registration: $registration,
            installer: $installer,
            name: $name,
            factory_type: $factory,
            contract: set_runtime_contract!($name),
            package: "mech-set",
            crate_name: "mech_set",
            installer_path: concat!("mech_set::__mech_native::", stringify!($installer)),
            extra_cargo_features: [$($feature),*],
        }
    };
}

fn set_element_kind(value: &mech_core::LegacyValue) -> Option<ValueKind> {
    match value.deref_kind() {
        ValueKind::Set(element, _) => Some(*element),
        _ => None,
    }
}

fn required_set_element_kind(
    args: &FunctionArgs,
    index: usize,
    contract: &'static str,
) -> MResult<ValueKind> {
    args.input_value(index)
        .and_then(set_element_kind)
        .ok_or_else(|| {
            function_shape_contract_violation(contract, format!("input {index} must be a set"))
        })
}

fn required_output_set_element_kind(
    args: &FunctionArgs,
    contract: &'static str,
) -> MResult<ValueKind> {
    set_element_kind(args.output_value()).ok_or_else(|| {
        function_shape_contract_violation(contract, "output must be a set")
    })
}

fn require_same_kind(
    contract: &'static str,
    role: &'static str,
    expected: &ValueKind,
    found: &ValueKind,
) -> MResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(function_shape_contract_violation(
            contract,
            format!("{role} element schema is {found}, expected {expected}"),
        ))
    }
}

fn validate_binary_set_inputs(args: &FunctionArgs, contract: &'static str) -> MResult<ValueKind> {
    let lhs = required_set_element_kind(args, 0, contract)?;
    let rhs = required_set_element_kind(args, 1, contract)?;
    require_same_kind(contract, "input 1", &lhs, &rhs)?;
    Ok(lhs)
}

fn validate_set_binary_output_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "set_binary_same_schema";
    let input = validate_binary_set_inputs(args, contract)?;
    let output = required_output_set_element_kind(args, contract)?;
    require_same_kind(contract, "output", &input, &output)
}

fn validate_set_binary_relation_contract(args: &FunctionArgs) -> MResult<()> {
    validate_binary_set_inputs(args, "set_binary_relation_same_schema").map(|_| ())
}

fn validate_set_element_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "set_element_same_schema";
    let element = args
        .input_value(0)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 0"))?
        .deref_kind();
    let set = required_set_element_kind(args, 1, contract)?;
    require_same_kind(contract, "input 0", &set, &element)
}

fn validate_set_mutation_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "set_mutation_same_schema";
    let set = required_set_element_kind(args, 0, contract)?;
    let element = args
        .input_value(1)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing input 1"))?
        .deref_kind();
    require_same_kind(contract, "input 1", &set, &element)?;
    let output = required_output_set_element_kind(args, contract)?;
    require_same_kind(contract, "output", &set, &output)
}

fn validate_set_powerset_contract(args: &FunctionArgs) -> MResult<()> {
    let contract = "set_powerset_schema";
    let input = required_set_element_kind(args, 0, contract)?;
    let expected = ValueKind::Set(Box::new(input), None);
    let output = required_output_set_element_kind(args, contract)?;
    require_same_kind(contract, "output", &expected, &output)
}

macro_rules! set_runtime_contract {
    ("SetCartesianProductFxn") => {
        RuntimeFunctionContract::custom(
            "set_cartesian_product",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            crate::operations::cartesian_product::validate_set_cartesian_product_contract,
        )
    };
    ("SetDifferenceFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_output_contract,
        )
    };
    ("SetIntersectionFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_output_contract,
        )
    };
    ("SetSymDifferenceFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_output_contract,
        )
    };
    ("SetUnionFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_output_contract,
        )
    };
    ("SetDisjointFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetEqualsFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetNotEqualsFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetProperSubsetFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetProperSupersetFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetSubsetFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetSupersetFxn") => {
        RuntimeFunctionContract::custom(
            "set_binary_relation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_binary_relation_contract,
        )
    };
    ("SetElementOfFxn") => {
        RuntimeFunctionContract::custom(
            "set_element_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_element_contract,
        )
    };
    ("SetNotElementOfFxn") => {
        RuntimeFunctionContract::custom(
            "set_element_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_element_contract,
        )
    };
    ("SetInsertFxn") => {
        RuntimeFunctionContract::custom(
            "set_mutation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_mutation_contract,
        )
    };
    ("SetRemoveFxn") => {
        RuntimeFunctionContract::custom(
            "set_mutation_same_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_mutation_contract,
        )
    };
    ("SetPowersetFxn") => {
        RuntimeFunctionContract::custom(
            "set_powerset_schema",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_set_powerset_contract,
        )
    };
    ($_name:literal) => {
        RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias)
    };
}

for_each_set_runtime_factory!(declare_set_runtime_factory);

/// Installs every concrete runtime factory linked by the enabled set features.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register_set_runtime_factory {
        ($cfg:meta; $registration:ident; $_installer:ident; $_name:literal; $_factory:path; [$($_feature:literal),* $(,)?]) => {
            #[cfg($cfg)]
            $registration(builder)?;
        };
    }

    for_each_set_runtime_factory!(register_set_runtime_factory);
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    macro_rules! export_set_runtime_factory {
        ($cfg:meta; $_registration:ident; $installer:ident; $_name:literal; $_factory:path; [$($_feature:literal),* $(,)?]) => {
            #[cfg($cfg)]
            pub use super::$installer;
        };
    }

    for_each_set_runtime_factory!(export_set_runtime_factory);
}

#[cfg(all(
    test,
    feature = "source",
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

#[cfg(all(
    test,
    feature = "u8",
    feature = "bool",
    feature = "cartesian_product"
))]
mod runtime_schema_contract_tests {
    use super::*;
    use mech_core::{set::MechSet, Ref, LegacyValue};

    fn set(kind: ValueKind) -> LegacyValue {
        LegacyValue::Set(Ref::new(MechSet::new(kind, 0)))
    }

    #[test]
    fn typed_set_contracts_reject_erased_element_schema_mismatches() {
        let same_set_args = FunctionArgs::Binary(
            set(ValueKind::U8),
            set(ValueKind::U8),
            set(ValueKind::U8),
        );
        validate_set_binary_output_contract(&same_set_args).unwrap();
        validate_set_binary_relation_contract(&same_set_args).unwrap();

        let mismatched_sets = FunctionArgs::Binary(
            set(ValueKind::U8),
            set(ValueKind::U8),
            set(ValueKind::Bool),
        );
        assert_eq!(
            validate_set_binary_output_contract(&mismatched_sets)
                .unwrap_err()
                .kind_name(),
            "FunctionShapeContractViolation",
        );
        assert_eq!(
            validate_set_binary_relation_contract(&mismatched_sets)
                .unwrap_err()
                .kind_name(),
            "FunctionShapeContractViolation",
        );

        let matching_element = FunctionArgs::Binary(
            LegacyValue::Bool(Ref::new(false)),
            LegacyValue::U8(Ref::new(1)),
            set(ValueKind::U8),
        );
        validate_set_element_contract(&matching_element).unwrap();
        let mismatched_element = FunctionArgs::Binary(
            LegacyValue::Bool(Ref::new(false)),
            LegacyValue::Bool(Ref::new(true)),
            set(ValueKind::U8),
        );
        assert!(validate_set_element_contract(&mismatched_element).is_err());

        let matching_mutation = FunctionArgs::Binary(
            set(ValueKind::U8),
            set(ValueKind::U8),
            LegacyValue::U8(Ref::new(1)),
        );
        validate_set_mutation_contract(&matching_mutation).unwrap();
        let mismatched_mutation = FunctionArgs::Binary(
            set(ValueKind::U8),
            set(ValueKind::U8),
            LegacyValue::Bool(Ref::new(true)),
        );
        assert!(validate_set_mutation_contract(&mismatched_mutation).is_err());
    }

    #[test]
    fn derived_set_output_schemas_are_exact() {
        let powerset = FunctionArgs::Unary(
            set(ValueKind::Set(Box::new(ValueKind::U8), None)),
            set(ValueKind::U8),
        );
        validate_set_powerset_contract(&powerset).unwrap();
        let wrong_powerset = FunctionArgs::Unary(set(ValueKind::U8), set(ValueKind::U8));
        assert!(validate_set_powerset_contract(&wrong_powerset).is_err());

        let cartesian = FunctionArgs::Binary(
            set(ValueKind::Tuple(vec![ValueKind::U8, ValueKind::Bool])),
            set(ValueKind::U8),
            set(ValueKind::Bool),
        );
        crate::operations::cartesian_product::validate_set_cartesian_product_contract(&cartesian)
            .unwrap();
        let wrong_cartesian = FunctionArgs::Binary(
            set(ValueKind::Tuple(vec![ValueKind::Bool, ValueKind::U8])),
            set(ValueKind::U8),
            set(ValueKind::Bool),
        );
        assert!(
            crate::operations::cartesian_product::validate_set_cartesian_product_contract(
                &wrong_cartesian,
            )
            .is_err()
        );
    }
}
