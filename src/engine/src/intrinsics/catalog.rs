#[cfg(all(feature = "semantic-compiler", feature = "set"))]
use crate::intrinsics::constructors::SetDefine;
#[cfg(feature = "matrix_horzcat")]
use crate::intrinsics::constructors::ValueHorizontalConcatenation;
#[cfg(feature = "matrix_comprehensions")]
use crate::intrinsics::constructors::ValueMatrixComprehension;
#[cfg(feature = "set")]
use crate::intrinsics::constructors::ValueSet;
#[cfg(feature = "set_comprehensions")]
use crate::intrinsics::constructors::ValueSetComprehension;
#[cfg(feature = "matrix_vertcat")]
use crate::intrinsics::constructors::ValueVerticalConcatenation;
#[cfg(all(feature = "semantic-compiler", feature = "variable_define"))]
use crate::intrinsics::define::VarDefine;
#[cfg(all(feature = "semantic-compiler", feature = "convert"))]
use crate::literals::ConvertKind;
#[cfg(any(
    feature = "semantic-compiler",
    feature = "set",
    feature = "invariant_define",
    feature = "set_comprehensions",
    feature = "matrix_comprehensions",
    feature = "matrix_horzcat",
    feature = "matrix_vertcat"
))]
use crate::*;
#[cfg(any(
    feature = "invariant_define",
    feature = "matrix_comprehensions",
    feature = "set_comprehensions"
))]
use mech_core::function_shape_contract_violation;
use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(all(
    feature = "semantic-compiler",
    any(
        feature = "matrix_comprehensions",
        feature = "matrix_horzcat",
        feature = "matrix_vertcat",
        feature = "set_comprehensions",
        feature = "set"
    )
))]
use mech_core::{FunctionExport, FunctionExposure};
#[cfg(feature = "semantic-compiler")]
use std::sync::Arc;

#[cfg(all(
    feature = "semantic-compiler",
    any(
        feature = "matrix_comprehensions",
        feature = "matrix_horzcat",
        feature = "matrix_vertcat",
        feature = "set_comprehensions",
        feature = "set"
    )
))]
fn install_canonical_named<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    module: Option<&str>,
    item: Option<&str>,
    exposure: FunctionExposure,
    compiler: T,
) -> MResult<()>
where
    T: CanonicalFunctionSpecializer + 'static,
{
    let operation = builder.insert_canonical_specializer(canonical_name, Arc::new(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: module.map(str::to_string),
        item: item.map(str::to_string),
        exposure,
    })
}

#[cfg(feature = "semantic-compiler")]
fn install_canonical_intrinsic<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
) -> MResult<()>
where
    T: CanonicalFunctionSpecializer + 'static,
{
    builder
        .insert_canonical_intrinsic_specializer(canonical_name, Arc::new(compiler))
        .map(|_| ())
}

#[cfg(feature = "matrix_comprehensions")]
fn validate_matrix_comprehension_canonical(output: &ValueCell, _: &[ValueCell]) -> MResult<()> {
    match output.closed_schema_body()? {
        SchemaBody::Matrix { .. } => Ok(()),
        _ => Err(function_shape_contract_violation(
            "matrix_comprehension",
            "output must be matrix-backed",
        )),
    }
}

#[cfg(feature = "set_comprehensions")]
fn validate_set_comprehension_canonical(output: &ValueCell, _: &[ValueCell]) -> MResult<()> {
    match output.closed_schema_body()? {
        SchemaBody::Set { .. } => Ok(()),
        _ => Err(function_shape_contract_violation(
            "set_comprehension",
            "output must be set-backed",
        )),
    }
}

#[cfg(feature = "invariant_define")]
fn validate_integrity_constraint_marker_canonical(
    _: &ValueCell,
    inputs: &[ValueCell],
) -> MResult<()> {
    if inputs.len() == 6 {
        Ok(())
    } else {
        Err(function_shape_contract_violation(
            "integrity_constraint_marker",
            format!("expected 6 metadata inputs, found {}", inputs.len()),
        ))
    }
}

#[cfg(feature = "semantic-compiler")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix_horzcat")]
    crate::intrinsics::horzcat::install_source_runtime(builder)?;
    #[cfg(feature = "matrix_comprehensions")]
    install_canonical_named(
        builder,
        "matrix/comprehension",
        None,
        None,
        FunctionExposure::Prelude,
        MatrixComprehensionDefine {},
    )?;
    #[cfg(feature = "matrix_horzcat")]
    install_canonical_named(
        builder,
        "matrix/horzcat",
        None,
        None,
        FunctionExposure::Prelude,
        MatrixHorzCat {},
    )?;
    #[cfg(feature = "matrix_vertcat")]
    install_canonical_named(
        builder,
        "matrix/vertcat",
        None,
        None,
        FunctionExposure::Prelude,
        MatrixVertCat {},
    )?;
    #[cfg(feature = "set_comprehensions")]
    install_canonical_named(
        builder,
        "set/comprehension",
        None,
        None,
        FunctionExposure::Prelude,
        SetComprehensionDefine {},
    )?;
    #[cfg(feature = "set")]
    install_canonical_named(
        builder,
        "set/define",
        None,
        None,
        FunctionExposure::Prelude,
        SetDefine,
    )?;
    #[cfg(feature = "table")]
    {
        install_canonical_named(
            builder,
            "table/join",
            None,
            None,
            FunctionExposure::Internal,
            TableInnerJoin,
        )?;
        install_canonical_named(
            builder,
            "table/left-outer-join",
            None,
            None,
            FunctionExposure::Internal,
            TableLeftOuterJoin,
        )?;
        install_canonical_named(
            builder,
            "table/right-outer-join",
            None,
            None,
            FunctionExposure::Internal,
            TableRightOuterJoin,
        )?;
        install_canonical_named(
            builder,
            "table/full-outer-join",
            None,
            None,
            FunctionExposure::Internal,
            TableFullOuterJoin,
        )?;
        install_canonical_named(
            builder,
            "table/left-semi-join",
            None,
            None,
            FunctionExposure::Internal,
            TableLeftSemiJoin,
        )?;
        install_canonical_named(
            builder,
            "table/left-anti-join",
            None,
            None,
            FunctionExposure::Internal,
            TableLeftAntiJoin,
        )?;
    }

    #[cfg(feature = "access")]
    {
        install_canonical_intrinsic(builder, "access/scalar", AccessScalar {})?;
        install_canonical_intrinsic(builder, "access/range", AccessRange {})?;
        install_canonical_intrinsic(builder, "access/column", AccessColumn {})?;
        install_canonical_intrinsic(builder, "access/swizzle", AccessSwizzle {})?;
    }
    #[cfg(feature = "assign")]
    {
        install_canonical_intrinsic(builder, "assign", AssignValue {})?;
        install_canonical_intrinsic(builder, "assign/column", AssignColumn {})?;
        install_canonical_intrinsic(builder, "assign/add", AddAssignValue {})?;
    }
    #[cfg(feature = "convert")]
    install_canonical_intrinsic(builder, "convert/kind", ConvertKind)?;
    #[cfg(feature = "variable_define")]
    install_canonical_intrinsic(builder, "var/define", VarDefine)?;
    Ok(())
}

// Installs the concrete bytecode factories owned by the engine fragment.
// Machine-owned factories are installed by their respective machine crates.
mech_core::declare_native_runtime_factory! {
    cfg: feature = "set",
    registration: register_set_define,
    installer: install_set_define,
    name: "set/define",
    factory_type: ValueSet,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_set_define",
    extra_cargo_features: [],
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "invariant_define",
    registration: register_integrity_constraint_marker,
    installer: install_integrity_constraint_marker,
    name: "integrity/constraint",
    factory_type: crate::intrinsics::define::BytecodeIntegrityConstraintMarker,
    contract: RuntimeFunctionContract::canonical_custom(
        "integrity_constraint_marker",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_integrity_constraint_marker_canonical,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_integrity_constraint_marker",
    extra_cargo_features: ["invariant_define"],
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "set_comprehensions",
    registration: register_set_comprehension,
    installer: install_set_comprehension,
    name: "set/comprehension",
    factory_type: ValueSetComprehension,
    contract: RuntimeFunctionContract::canonical_custom(
        "set_comprehension",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_set_comprehension_canonical,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_set_comprehension",
    extra_cargo_features: ["set_comprehensions"],
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "matrix_comprehensions",
    registration: register_matrix_comprehension,
    installer: install_matrix_comprehension,
    name: "matrix/comprehension",
    factory_type: ValueMatrixComprehension,
    contract: RuntimeFunctionContract::canonical_custom(
        "matrix_comprehension",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_matrix_comprehension_canonical,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_matrix_comprehension",
    extra_cargo_features: ["matrix_comprehensions"],
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "matrix_horzcat",
    registration: register_value_horizontal_concatenation,
    installer: install_value_horizontal_concatenation,
    name: "matrix/horzcat",
    factory_type: ValueHorizontalConcatenation,
    contract: RuntimeFunctionContract::horizontal_concatenation(
        RuntimeOutputAliasPolicy::DisallowInputAlias,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_value_horizontal_concatenation",
    extra_cargo_features: ["matrix_horzcat"],
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "matrix_vertcat",
    registration: register_value_vertical_concatenation,
    installer: install_value_vertical_concatenation,
    name: "matrix/vertcat",
    factory_type: ValueVerticalConcatenation,
    contract: RuntimeFunctionContract::vertical_concatenation(
        RuntimeOutputAliasPolicy::DisallowInputAlias,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_value_vertical_concatenation",
    extra_cargo_features: ["matrix_vertcat"],
}

pub fn install_runtime(
    #[cfg(any(
        feature = "access",
        feature = "assign",
        feature = "convert",
        feature = "variable_define",
        feature = "set",
        feature = "set_comprehensions",
        feature = "matrix_comprehensions",
        feature = "invariant_define",
        feature = "matrix_horzcat",
        feature = "matrix_vertcat",
        feature = "table"
    ))]
    builder: &mut FunctionCatalogBuilder,
    #[cfg(not(any(
        feature = "access",
        feature = "assign",
        feature = "convert",
        feature = "variable_define",
        feature = "set",
        feature = "set_comprehensions",
        feature = "matrix_comprehensions",
        feature = "invariant_define",
        feature = "matrix_horzcat",
        feature = "matrix_vertcat",
        feature = "table"
    )))]
    _: &mut FunctionCatalogBuilder,
) -> MResult<()> {
    #[cfg(feature = "access")]
    super::access::install_runtime(builder)?;
    #[cfg(feature = "assign")]
    super::assign::install_runtime(builder)?;
    #[cfg(feature = "convert")]
    super::convert::scalar::install_runtime(builder)?;
    #[cfg(feature = "variable_define")]
    super::define::install_runtime(builder)?;

    #[cfg(feature = "set")]
    register_set_define(builder)?;
    #[cfg(feature = "set_comprehensions")]
    register_set_comprehension(builder)?;
    #[cfg(feature = "matrix_comprehensions")]
    register_matrix_comprehension(builder)?;
    #[cfg(feature = "matrix_horzcat")]
    register_value_horizontal_concatenation(builder)?;
    #[cfg(feature = "matrix_vertcat")]
    register_value_vertical_concatenation(builder)?;
    #[cfg(feature = "invariant_define")]
    register_integrity_constraint_marker(builder)?;

    #[cfg(feature = "matrix_horzcat")]
    super::horzcat::install_runtime(builder)?;
    #[cfg(feature = "matrix_vertcat")]
    super::vertcat::install_runtime(builder)?;
    #[cfg(feature = "table")]
    super::table_ops::install_runtime(builder)?;

    Ok(())
}

/// Installs engine-owned factories that are available only when constructing
/// native application plans.
#[cfg(feature = "native-plan")]
pub fn install_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "access")]
    crate::intrinsics::access::install_native_plan(builder)?;

    #[cfg(feature = "assign")]
    crate::intrinsics::assign::catalog::install_native_plan(builder)?;

    #[cfg(all(feature = "variable_define_matrix1", not(feature = "matrix1")))]
    crate::intrinsics::define::install_native_plan_runtime(builder)?;

    #[cfg(feature = "matrix_horzcat")]
    crate::intrinsics::horzcat::install_native_plan_runtime(builder)?;

    Ok(())
}
