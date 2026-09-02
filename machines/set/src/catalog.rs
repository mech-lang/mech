#[cfg(feature = "source")]
use mech_core::{CanonicalFunctionSpecializer, FunctionExport, FunctionExposure};
use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(any(
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
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
use mech_core::{
    RuntimeFunctionContract, RuntimeOutputAliasPolicy, SchemaBody, ValueCell,
    function_shape_contract_violation,
};
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

#[cfg(any(
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
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
fn set_contract_error(contract: &'static str, reason: impl Into<String>) -> mech_core::MechError {
    function_shape_contract_violation(contract, reason)
}

#[cfg(any(
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
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
fn set_element(cell: &ValueCell, contract: &'static str, label: &str) -> MResult<SchemaBody> {
    let SchemaBody::Set { element, .. } = cell.closed_schema_body()? else {
        return Err(set_contract_error(
            contract,
            format!("{label} must be set-backed"),
        ));
    };
    Ok(*element)
}

#[cfg(any(
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
    all(feature = "size", feature = "u64"),
    feature = "subset",
    feature = "superset",
    feature = "symmetric_difference",
    feature = "union",
))]
fn expect_arity<'a>(
    inputs: &'a [ValueCell],
    expected: usize,
    contract: &'static str,
) -> MResult<&'a [ValueCell]> {
    if inputs.len() != expected {
        return Err(set_contract_error(
            contract,
            format!("expected {expected} inputs, found {}", inputs.len()),
        ));
    }
    Ok(inputs)
}

#[cfg(any(
    feature = "disjoint",
    feature = "element_of",
    feature = "equals",
    feature = "not_element_of",
    feature = "not_equals",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "subset",
    feature = "superset",
))]
fn require_bool_output(output: &ValueCell, contract: &'static str) -> MResult<()> {
    if output.closed_schema_body()? != SchemaBody::Bool {
        return Err(set_contract_error(contract, "output must be bool"));
    }
    Ok(())
}

#[cfg(any(
    feature = "difference",
    feature = "intersection",
    feature = "symmetric_difference",
    feature = "union",
))]
fn validate_set_algebra(
    output: &ValueCell,
    inputs: &[ValueCell],
    contract: &'static str,
) -> MResult<()> {
    let inputs = expect_arity(inputs, 2, contract)?;
    let lhs = set_element(&inputs[0], contract, "lhs")?;
    let rhs = set_element(&inputs[1], contract, "rhs")?;
    let out = set_element(output, contract, "output")?;
    if lhs != rhs || lhs != out {
        return Err(set_contract_error(
            contract,
            format!("lhs, rhs, and output element schemas must match: {lhs:?}, {rhs:?}, {out:?}"),
        ));
    }
    Ok(())
}

#[cfg(any(
    feature = "disjoint",
    feature = "equals",
    feature = "not_equals",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "subset",
    feature = "superset",
))]
fn validate_set_relation(
    output: &ValueCell,
    inputs: &[ValueCell],
    contract: &'static str,
) -> MResult<()> {
    require_bool_output(output, contract)?;
    let inputs = expect_arity(inputs, 2, contract)?;
    let lhs = set_element(&inputs[0], contract, "lhs")?;
    let rhs = set_element(&inputs[1], contract, "rhs")?;
    if lhs != rhs {
        return Err(set_contract_error(
            contract,
            format!("lhs element schema {lhs:?} differs from rhs {rhs:?}"),
        ));
    }
    Ok(())
}

#[cfg(any(feature = "element_of", feature = "not_element_of"))]
fn validate_set_membership(
    output: &ValueCell,
    inputs: &[ValueCell],
    contract: &'static str,
) -> MResult<()> {
    require_bool_output(output, contract)?;
    let inputs = expect_arity(inputs, 2, contract)?;
    let element = inputs[0].closed_schema_body()?;
    let set_element = set_element(&inputs[1], contract, "set input")?;
    if element != set_element {
        return Err(set_contract_error(
            contract,
            format!("candidate schema {element:?} differs from set element {set_element:?}"),
        ));
    }
    Ok(())
}

#[cfg(any(feature = "insert", feature = "remove"))]
fn validate_set_mutation(
    output: &ValueCell,
    inputs: &[ValueCell],
    contract: &'static str,
) -> MResult<()> {
    let inputs = expect_arity(inputs, 2, contract)?;
    let input_element = set_element(&inputs[0], contract, "set input")?;
    let candidate = inputs[1].closed_schema_body()?;
    let output_element = set_element(output, contract, "output")?;
    if input_element != candidate || input_element != output_element {
        return Err(set_contract_error(
            contract,
            format!(
                "set input, candidate, and output element schemas must match: {input_element:?}, {candidate:?}, {output_element:?}"
            ),
        ));
    }
    Ok(())
}

#[cfg(feature = "cartesian_product")]
fn validate_set_cartesian_product(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let contract = "set_cartesian_product";
    let inputs = expect_arity(inputs, 2, contract)?;
    let lhs = set_element(&inputs[0], contract, "lhs")?;
    let rhs = set_element(&inputs[1], contract, "rhs")?;
    let output_element = set_element(output, contract, "output")?;
    let expected = SchemaBody::Tuple(vec![lhs, rhs].into_boxed_slice());
    if output_element != expected {
        return Err(set_contract_error(
            contract,
            format!("output element schema {output_element:?} must be {expected:?}"),
        ));
    }
    Ok(())
}

#[cfg(feature = "powerset")]
fn validate_set_powerset(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let contract = "set_powerset";
    let inputs = expect_arity(inputs, 1, contract)?;
    let input_element = set_element(&inputs[0], contract, "input")?;
    let output_element = set_element(output, contract, "output")?;
    let SchemaBody::Set {
        element: nested_element,
        ..
    } = output_element
    else {
        return Err(set_contract_error(
            contract,
            "output elements must themselves be sets",
        ));
    };
    if *nested_element != input_element {
        return Err(set_contract_error(
            contract,
            format!(
                "nested output element {nested_element:?} differs from input {input_element:?}"
            ),
        ));
    }
    Ok(())
}

#[cfg(all(feature = "size", feature = "u64"))]
fn validate_set_size(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let contract = "set_size";
    let inputs = expect_arity(inputs, 1, contract)?;
    set_element(&inputs[0], contract, "input")?;
    if output.closed_schema_body()? != SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64) {
        return Err(set_contract_error(contract, "output must be u64"));
    }
    Ok(())
}

#[cfg(any(
    feature = "difference",
    feature = "intersection",
    feature = "symmetric_difference",
    feature = "union",
))]
macro_rules! set_algebra_validator {
    ($name:ident, $contract:literal) => {
        fn $name(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_set_algebra(output, inputs, $contract)
        }
    };
}

#[cfg(any(
    feature = "disjoint",
    feature = "equals",
    feature = "not_equals",
    feature = "proper_subset",
    feature = "proper_superset",
    feature = "subset",
    feature = "superset",
))]
macro_rules! set_relation_validator {
    ($name:ident, $contract:literal) => {
        fn $name(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_set_relation(output, inputs, $contract)
        }
    };
}

#[cfg(any(feature = "element_of", feature = "not_element_of"))]
macro_rules! set_membership_validator {
    ($name:ident, $contract:literal) => {
        fn $name(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_set_membership(output, inputs, $contract)
        }
    };
}

#[cfg(any(feature = "insert", feature = "remove"))]
macro_rules! set_mutation_validator {
    ($name:ident, $contract:literal) => {
        fn $name(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
            validate_set_mutation(output, inputs, $contract)
        }
    };
}

#[cfg(feature = "difference")]
set_algebra_validator!(validate_set_difference, "set_difference");
#[cfg(feature = "intersection")]
set_algebra_validator!(validate_set_intersection, "set_intersection");
#[cfg(feature = "symmetric_difference")]
set_algebra_validator!(
    validate_set_symmetric_difference,
    "set_symmetric_difference"
);
#[cfg(feature = "union")]
set_algebra_validator!(validate_set_union, "set_union");
#[cfg(feature = "disjoint")]
set_relation_validator!(validate_set_disjoint, "set_disjoint");
#[cfg(feature = "equals")]
set_relation_validator!(validate_set_equals, "set_equals");
#[cfg(feature = "not_equals")]
set_relation_validator!(validate_set_not_equals, "set_not_equals");
#[cfg(feature = "proper_subset")]
set_relation_validator!(validate_set_proper_subset, "set_proper_subset");
#[cfg(feature = "proper_superset")]
set_relation_validator!(validate_set_proper_superset, "set_proper_superset");
#[cfg(feature = "subset")]
set_relation_validator!(validate_set_subset, "set_subset");
#[cfg(feature = "superset")]
set_relation_validator!(validate_set_superset, "set_superset");
#[cfg(feature = "element_of")]
set_membership_validator!(validate_set_element_of, "set_element_of");
#[cfg(feature = "not_element_of")]
set_membership_validator!(validate_set_not_element_of, "set_not_element_of");
#[cfg(feature = "insert")]
set_mutation_validator!(validate_set_insert, "set_insert");
#[cfg(feature = "remove")]
set_mutation_validator!(validate_set_remove, "set_remove");

macro_rules! for_each_set_runtime_factory {
    ($callback:ident) => {
        $callback!(feature = "cartesian_product"; register_set_cartesian_product_fxn; install_set_cartesian_product_fxn; "SetCartesianProductFxn"; crate::operations::cartesian_product::SetCartesianProductFxn; ["cartesian_product"]; RuntimeFunctionContract::canonical_custom("set_cartesian_product", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_cartesian_product));
        $callback!(feature = "difference"; register_set_difference_fxn; install_set_difference_fxn; "SetDifferenceFxn"; crate::operations::difference::SetDifferenceFxn; ["difference"]; RuntimeFunctionContract::canonical_custom("set_difference", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_difference));
        $callback!(feature = "disjoint"; register_set_disjoint_fxn; install_set_disjoint_fxn; "SetDisjointFxn"; crate::relations::disjoint::SetDisjointFxn; ["disjoint"]; RuntimeFunctionContract::canonical_custom("set_disjoint", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_disjoint));
        $callback!(feature = "element_of"; register_set_element_of_fxn; install_set_element_of_fxn; "SetElementOfFxn"; crate::membership::element_of::SetElementOfFxn; ["element_of"]; RuntimeFunctionContract::canonical_custom("set_element_of", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_element_of));
        $callback!(feature = "equals"; register_set_equals_fxn; install_set_equals_fxn; "SetEqualsFxn"; crate::relations::equals::SetEqualsFxn; ["equals"]; RuntimeFunctionContract::canonical_custom("set_equals", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_equals));
        $callback!(feature = "insert"; register_set_insert_fxn; install_set_insert_fxn; "SetInsertFxn"; crate::modify::insert::SetInsertFxn; ["insert"]; RuntimeFunctionContract::canonical_custom("set_insert", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_insert));
        $callback!(feature = "intersection"; register_set_intersection_fxn; install_set_intersection_fxn; "SetIntersectionFxn"; crate::operations::intersection::SetIntersectionFxn; ["intersection"]; RuntimeFunctionContract::canonical_custom("set_intersection", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_intersection));
        $callback!(feature = "not_element_of"; register_set_not_element_of_fxn; install_set_not_element_of_fxn; "SetNotElementOfFxn"; crate::membership::not_element_of::SetNotElementOfFxn; ["not_element_of"]; RuntimeFunctionContract::canonical_custom("set_not_element_of", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_not_element_of));
        $callback!(feature = "not_equals"; register_set_not_equals_fxn; install_set_not_equals_fxn; "SetNotEqualsFxn"; crate::relations::not_equals::SetNotEqualsFxn; ["not_equals"]; RuntimeFunctionContract::canonical_custom("set_not_equals", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_not_equals));
        $callback!(feature = "powerset"; register_set_powerset_fxn; install_set_powerset_fxn; "SetPowersetFxn"; crate::operations::powerset::SetPowersetFxn; ["powerset"]; RuntimeFunctionContract::canonical_custom("set_powerset", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_powerset));
        $callback!(feature = "proper_subset"; register_set_proper_subset_fxn; install_set_proper_subset_fxn; "SetProperSubsetFxn"; crate::relations::proper_subset::SetProperSubsetFxn; ["proper_subset"]; RuntimeFunctionContract::canonical_custom("set_proper_subset", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_proper_subset));
        $callback!(feature = "proper_superset"; register_set_proper_superset_fxn; install_set_proper_superset_fxn; "SetProperSupersetFxn"; crate::relations::proper_superset::SetProperSupersetFxn; ["proper_superset"]; RuntimeFunctionContract::canonical_custom("set_proper_superset", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_proper_superset));
        $callback!(feature = "remove"; register_set_remove_fxn; install_set_remove_fxn; "SetRemoveFxn"; crate::modify::remove::SetRemoveFxn; ["remove"]; RuntimeFunctionContract::canonical_custom("set_remove", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_remove));
        $callback!(all(feature = "size", feature = "u64"); register_set_size_fxn; install_set_size_fxn; "SetSizeFxn"; crate::setdata::size::SetSizeFxn; ["size"]; RuntimeFunctionContract::canonical_custom("set_size", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_size));
        $callback!(feature = "subset"; register_set_subset_fxn; install_set_subset_fxn; "SetSubsetFxn"; crate::relations::subset::SetSubsetFxn; ["subset"]; RuntimeFunctionContract::canonical_custom("set_subset", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_subset));
        $callback!(feature = "superset"; register_set_superset_fxn; install_set_superset_fxn; "SetSupersetFxn"; crate::relations::superset::SetSupersetFxn; ["superset"]; RuntimeFunctionContract::canonical_custom("set_superset", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_superset));
        $callback!(feature = "symmetric_difference"; register_set_symmetric_difference_fxn; install_set_symmetric_difference_fxn; "SetSymDifferenceFxn"; crate::operations::symmetric_difference::SetSymDifferenceFxn; ["symmetric_difference"]; RuntimeFunctionContract::canonical_custom("set_symmetric_difference", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_symmetric_difference));
        $callback!(feature = "union"; register_set_union_fxn; install_set_union_fxn; "SetUnionFxn"; crate::operations::union::SetUnionFxn; ["union"]; RuntimeFunctionContract::canonical_custom("set_union", RuntimeOutputAliasPolicy::DisallowInputAlias, validate_set_union));
    };
}

macro_rules! declare_set_runtime_factory {
    ($cfg:meta; $registration:ident; $installer:ident; $name:literal; $factory:path; [$($feature:literal),* $(,)?]; $contract:expr) => {
        mech_core::declare_native_runtime_factory! {
            cfg: $cfg,
            registration: $registration,
            installer: $installer,
            name: $name,
            factory_type: $factory,
            contract: $contract,
            package: "mech-set",
            crate_name: "mech_set",
            installer_path: concat!("mech_set::__mech_native::", stringify!($installer)),
            extra_cargo_features: [$($feature),*],
        }
    };
}

for_each_set_runtime_factory!(declare_set_runtime_factory);

/// Installs every concrete runtime factory linked by the enabled set features.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(not(any(
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
        all(feature = "size", feature = "u64"),
        feature = "subset",
        feature = "superset",
        feature = "symmetric_difference",
        feature = "union",
    )))]
    let _ = builder;

    macro_rules! register_set_runtime_factory {
        ($cfg:meta; $registration:ident; $_installer:ident; $_name:literal; $_factory:path; [$($_feature:literal),* $(,)?]; $_contract:expr) => {
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
        ($cfg:meta; $_registration:ident; $installer:ident; $_name:literal; $_factory:path; [$($_feature:literal),* $(,)?]; $_contract:expr) => {
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
    use mech_core::{
        CardinalitySpec, FunctionInvocation, IntegerWidth, OperationId, RuntimeFunctionId,
        ValueCell,
    };

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

    fn empty_set(element: SchemaBody) -> ValueCell {
        ValueCell::empty_dynamic_set(element).unwrap()
    }

    fn runtime_entry<'a>(
        catalog: &'a mech_core::FunctionCatalog,
        name: &str,
    ) -> &'a mech_core::RuntimeFunctionEntry {
        catalog
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .unwrap_or_else(|| panic!("missing installed runtime factory {name}"))
    }

    #[test]
    fn installed_set_contracts_reject_schema_mismatches_and_wrong_outputs() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let u8_body = SchemaBody::UnsignedInteger(IntegerWidth::W8);
        let u16_body = SchemaBody::UnsignedInteger(IntegerWidth::W16);

        for name in [
            "SetUnionFxn",
            "SetIntersectionFxn",
            "SetDifferenceFxn",
            "SetSymDifferenceFxn",
        ] {
            let algebra = runtime_entry(&catalog, name);
            let error = algebra
                .validate_invocation(&FunctionInvocation::binary(
                    empty_set(u8_body.clone()),
                    empty_set(u8_body.clone()),
                    empty_set(u16_body.clone()),
                ))
                .unwrap_err();
            assert!(error.kind_message().contains("element schemas must match"));
            let error = algebra
                .validate_invocation(&FunctionInvocation::binary(
                    empty_set(u16_body.clone()),
                    empty_set(u8_body.clone()),
                    empty_set(u8_body.clone()),
                ))
                .unwrap_err();
            assert!(error.kind_message().contains("element schemas must match"));
        }

        for name in ["SetElementOfFxn", "SetNotElementOfFxn"] {
            let membership = runtime_entry(&catalog, name);
            let error = membership
                .validate_invocation(&FunctionInvocation::binary(
                    ValueCell::from_exact(false).unwrap(),
                    ValueCell::from_exact(1_u16).unwrap(),
                    empty_set(u8_body.clone()),
                ))
                .unwrap_err();
            assert!(error.kind_message().contains("candidate schema"));
        }

        for name in ["SetInsertFxn", "SetRemoveFxn"] {
            let mutation = runtime_entry(&catalog, name);
            let error = mutation
                .validate_invocation(&FunctionInvocation::binary(
                    empty_set(u8_body.clone()),
                    empty_set(u8_body.clone()),
                    ValueCell::from_exact(1_u16).unwrap(),
                ))
                .unwrap_err();
            assert!(error.kind_message().contains("candidate"));
        }

        let cartesian = runtime_entry(&catalog, "SetCartesianProductFxn");
        let error = cartesian
            .validate_invocation(&FunctionInvocation::binary(
                empty_set(u8_body.clone()),
                empty_set(u8_body.clone()),
                empty_set(u16_body.clone()),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("output element schema"));

        let powerset = runtime_entry(&catalog, "SetPowersetFxn");
        let error = powerset
            .validate_invocation(&FunctionInvocation::unary(
                empty_set(u8_body.clone()),
                empty_set(u8_body.clone()),
            ))
            .unwrap_err();
        assert!(
            error
                .kind_message()
                .contains("output elements must themselves be sets")
        );
        let wrong_nested = SchemaBody::Set {
            element: Box::new(u16_body),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        };
        let error = powerset
            .validate_invocation(&FunctionInvocation::unary(
                empty_set(wrong_nested),
                empty_set(u8_body.clone()),
            ))
            .unwrap_err();
        assert!(error.kind_message().contains("nested output element"));

        for name in [
            "SetDisjointFxn",
            "SetEqualsFxn",
            "SetNotEqualsFxn",
            "SetProperSubsetFxn",
            "SetProperSupersetFxn",
            "SetSubsetFxn",
            "SetSupersetFxn",
        ] {
            let relation = runtime_entry(&catalog, name);
            let error = relation
                .validate_invocation(&FunctionInvocation::binary(
                    ValueCell::from_exact(0_u64).unwrap(),
                    empty_set(u8_body.clone()),
                    empty_set(u8_body.clone()),
                ))
                .unwrap_err();
            assert!(
                error
                    .kind_message()
                    .contains("rejected its argument contract")
            );
        }

        let size = runtime_entry(&catalog, "SetSizeFxn");
        let error = size
            .validate_invocation(&FunctionInvocation::unary(
                ValueCell::from_exact(false).unwrap(),
                empty_set(u8_body),
            ))
            .unwrap_err();
        assert!(
            error
                .kind_message()
                .contains("rejected its argument contract")
        );
    }
}
