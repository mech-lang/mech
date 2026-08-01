#[cfg(feature = "variable_define")]
use crate::stdlib::define::VarDefine;
use crate::*;
use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult, NativeFunctionCompiler,
    legacy_source_specializer,
};

fn install_named<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    module: Option<&str>,
    item: Option<&str>,
    exposure: FunctionExposure,
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
        module: module.map(str::to_string),
        item: item.map(str::to_string),
        exposure,
    })
}

fn install_intrinsic<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
) -> MResult<()>
where
    T: NativeFunctionCompiler + 'static,
{
    builder
        .insert_intrinsic_specializer(canonical_name, legacy_source_specializer(compiler))
        .map(|_| ())
}

pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
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
    for (canonical_name, compiler) in [
        ("table/join", legacy_source_specializer(TableInnerJoin {})),
        (
            "table/left-outer-join",
            legacy_source_specializer(TableLeftOuterJoin {}),
        ),
        (
            "table/right-outer-join",
            legacy_source_specializer(TableRightOuterJoin {}),
        ),
        (
            "table/full-outer-join",
            legacy_source_specializer(TableFullOuterJoin {}),
        ),
        (
            "table/left-semi-join",
            legacy_source_specializer(TableLeftSemiJoin {}),
        ),
        (
            "table/left-anti-join",
            legacy_source_specializer(TableLeftAntiJoin {}),
        ),
    ] {
        let operation = builder.insert_specializer(canonical_name, compiler)?;
        builder.insert_export(FunctionExport {
            operation,
            canonical_name: canonical_name.to_string(),
            module: None,
            item: None,
            exposure: FunctionExposure::Internal,
        })?;
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
    #[cfg(feature = "math_mul_assign")]
    install_intrinsic(builder, "math/mul-assign", mech_math::MulAssignValue {})?;
    #[cfg(feature = "convert")]
    install_intrinsic(builder, "convert/kind", ConvertKind {})?;
    #[cfg(feature = "variable_define")]
    install_intrinsic(builder, "var/define", VarDefine {})?;

    Ok(())
}

/// Installs the concrete bytecode factories owned by the engine fragment.
/// Machine-owned factories are installed by their respective machine crates.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "access")]
    super::access::install_runtime(builder)?;
    #[cfg(feature = "assign")]
    super::assign::install_runtime(builder)?;
    #[cfg(feature = "convert")]
    super::convert::scalar::install_runtime(builder)?;
    #[cfg(feature = "variable_define")]
    super::define::install_runtime(builder)?;

    #[cfg(feature = "set")]
    builder.insert_runtime_factory("set/define", <ValueSet as MechFunctionFactory>::new)?;
    #[cfg(feature = "set_comprehensions")]
    builder.insert_runtime_factory(
        "set/comprehension",
        <ValueSetComprehension as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "matrix_comprehensions")]
    builder.insert_runtime_factory(
        "matrix/comprehension",
        <ValueMatrixComprehension as MechFunctionFactory>::new,
    )?;

    #[cfg(feature = "matrix_horzcat")]
    super::horzcat::install_runtime(builder)?;
    #[cfg(feature = "matrix_vertcat")]
    super::vertcat::install_runtime(builder)?;

    Ok(())
}

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}
