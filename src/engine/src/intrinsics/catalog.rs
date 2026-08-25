#[cfg(feature = "matrix_comprehensions")]
use crate::intrinsics::constructors::ValueMatrixComprehension;
#[cfg(feature = "set")]
use crate::intrinsics::constructors::ValueSet;
#[cfg(feature = "set_comprehensions")]
use crate::intrinsics::constructors::ValueSetComprehension;
#[cfg(all(feature = "semantic-compiler", feature = "variable_define"))]
use crate::intrinsics::define::VarDefine;
#[cfg(any(
    feature = "semantic-compiler",
    feature = "set",
    feature = "invariant_define",
    feature = "set_comprehensions",
    feature = "matrix_comprehensions"
))]
use crate::*;
#[cfg(any(
    feature = "invariant_define",
    feature = "matrix_comprehensions",
    feature = "set_comprehensions"
))]
use mech_core::{FunctionArgs, FunctionArgumentRole, function_shape_contract_violation};
use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(feature = "semantic-compiler")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
#[cfg(feature = "semantic-compiler")]
use std::sync::Arc;

#[cfg(feature = "semantic-compiler")]
fn install_named<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    module: Option<&str>,
    item: Option<&str>,
    exposure: FunctionExposure,
    compiler: T,
) -> MResult<()>
where
    T: FunctionSpecializer + 'static,
{
    let operation = builder.insert_specializer(canonical_name, Arc::new(compiler))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: canonical_name.to_string(),
        module: module.map(str::to_string),
        item: item.map(str::to_string),
        exposure,
    })
}

#[cfg(feature = "semantic-compiler")]
fn install_intrinsic<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
) -> MResult<()>
where
    T: FunctionSpecializer + 'static,
{
    builder
        .insert_intrinsic_specializer(canonical_name, Arc::new(compiler))
        .map(|_| ())
}

#[cfg(feature = "matrix_comprehensions")]
fn validate_matrix_comprehension(args: &FunctionArgs) -> MResult<()> {
    let contract = "matrix_comprehension";
    args.output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "output must be matrix-backed")
        })?;
    Ok(())
}

#[cfg(feature = "set_comprehensions")]
fn validate_set_comprehension(_args: &FunctionArgs) -> MResult<()> {
    Ok(())
}

#[cfg(feature = "invariant_define")]
fn validate_integrity_constraint_marker(args: &FunctionArgs) -> MResult<()> {
    if args.input_count() == 6 {
        Ok(())
    } else {
        Err(function_shape_contract_violation(
            "integrity_constraint_marker",
            format!("expected 6 metadata inputs, found {}", args.input_count()),
        ))
    }
}

#[cfg(feature = "semantic-compiler")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix_horzcat")]
    crate::intrinsics::horzcat::install_source_runtime(builder)?;
    #[cfg(feature = "matrix_comprehensions")]
    install_named(
        builder,
        "matrix/comprehension",
        None,
        None,
        FunctionExposure::Prelude,
        MatrixComprehensionDefine {},
    )?;
    #[cfg(feature = "matrix_horzcat")]
    install_named(
        builder,
        "matrix/horzcat",
        None,
        None,
        FunctionExposure::Prelude,
        MatrixHorzCat {},
    )?;
    #[cfg(feature = "matrix_vertcat")]
    install_named(
        builder,
        "matrix/vertcat",
        None,
        None,
        FunctionExposure::Prelude,
        MatrixVertCat {},
    )?;
    #[cfg(feature = "set_comprehensions")]
    install_named(
        builder,
        "set/comprehension",
        None,
        None,
        FunctionExposure::Prelude,
        SetComprehensionDefine {},
    )?;
    #[cfg(feature = "set")]
    install_named(
        builder,
        "set/define",
        None,
        None,
        FunctionExposure::Prelude,
        SetDefine {},
    )?;

    #[cfg(feature = "table")]
    {
        let table_specializers: [(&str, Arc<dyn FunctionSpecializer>); 6] = [
            ("table/join", Arc::new(TableInnerJoin {})),
            ("table/left-outer-join", Arc::new(TableLeftOuterJoin {})),
            ("table/right-outer-join", Arc::new(TableRightOuterJoin {})),
            ("table/full-outer-join", Arc::new(TableFullOuterJoin {})),
            ("table/left-semi-join", Arc::new(TableLeftSemiJoin {})),
            ("table/left-anti-join", Arc::new(TableLeftAntiJoin {})),
        ];
        for (canonical_name, compiler) in table_specializers {
            let operation = builder.insert_specializer(canonical_name, compiler)?;
            builder.insert_export(FunctionExport {
                operation,
                canonical_name: canonical_name.to_string(),
                module: None,
                item: None,
                exposure: FunctionExposure::Internal,
            })?;
        }
    }

    #[cfg(feature = "access")]
    {
        install_intrinsic(builder, "access/scalar", AccessScalar {})?;
        install_intrinsic(builder, "access/range", AccessRange {})?;
        install_intrinsic(builder, "access/column", AccessColumn {})?;
        install_intrinsic(builder, "access/swizzle", AccessSwizzle {})?;
    }
    #[cfg(feature = "assign")]
    {
        install_intrinsic(builder, "assign", AssignValue {})?;
        install_intrinsic(builder, "assign/column", AssignColumn {})?;
        install_intrinsic(builder, "assign/add", AddAssignValue {})?;
    }
    #[cfg(feature = "convert")]
    install_intrinsic(builder, "convert/kind", ConvertKind {})?;
    #[cfg(feature = "variable_define")]
    install_intrinsic(builder, "var/define", VarDefine {})?;

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
    contract: RuntimeFunctionContract::custom(
        "integrity_constraint_marker",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_integrity_constraint_marker,
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
    contract: RuntimeFunctionContract::custom(
        "set_comprehension",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_set_comprehension,
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
    contract: RuntimeFunctionContract::custom(
        "matrix_comprehension",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_matrix_comprehension,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_matrix_comprehension",
    extra_cargo_features: ["matrix_comprehensions"],
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
