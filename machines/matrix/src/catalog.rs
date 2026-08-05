#[cfg(feature = "complex")]
use mech_core::C64;
#[cfg(feature = "rational")]
use mech_core::R64;
use mech_core::{
    FunctionArgs, FunctionArgumentRole, FunctionCatalogBuilder, MResult, MechFunctionFactory,
    RuntimeFunctionContract, RuntimeOutputAliasPolicy, function_shape_contract_violation,
};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure, FunctionSpecializer};
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(feature = "source")]
fn install_operation<T>(
    builder: &mut FunctionCatalogBuilder,
    canonical_name: &str,
    compiler: T,
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
        exposure: FunctionExposure::Prelude,
    })
}

#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "dot")]
    install_operation(builder, "matrix/dot", crate::MatrixDot {})?;
    #[cfg(feature = "matmul")]
    install_operation(builder, "matrix/matmul", crate::MatrixMatMul {})?;
    #[cfg(feature = "solve")]
    install_operation(builder, "matrix/solve", crate::MatrixSolve {})?;
    #[cfg(feature = "transpose")]
    install_operation(builder, "matrix/transpose", crate::MatrixTranspose {})?;
    Ok(())
}

macro_rules! for_each_matrix_numeric_scalar {
    ($callback:ident, $context:tt) => {
        $callback!($context; u8; u8; "u8");
        $callback!($context; u16; u16; "u16");
        $callback!($context; u32; u32; "u32");
        $callback!($context; u64; u64; "u64");
        $callback!($context; u128; u128; "u128");
        $callback!($context; i8; i8; "i8");
        $callback!($context; i16; i16; "i16");
        $callback!($context; i32; i32; "i32");
        $callback!($context; i64; i64; "i64");
        $callback!($context; i128; i128; "i128");
        $callback!($context; f32; f32; "f32");
        $callback!($context; f64; f64; "f64");
    };
}

macro_rules! matrix_numeric_runtime_contract {
    (dot, DotScalar) => {
        RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias)
    };
    (dot, $factory:ident) => {
        RuntimeFunctionContract::custom(
            "dot_reduction",
            RuntimeOutputAliasPolicy::DisallowInputAlias,
            validate_dot_reduction,
        )
    };
    (matmul, MatMulScalar) => {
        RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias)
    };
    (matmul, $factory:ident) => {
        RuntimeFunctionContract::matrix_product(RuntimeOutputAliasPolicy::DisallowInputAlias)
    };
}

fn validate_dot_reduction(args: &FunctionArgs) -> MResult<()> {
    let contract = "dot_reduction";
    if args
        .output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)?
        .is_some()
    {
        return Err(function_shape_contract_violation(
            contract,
            "dot output must be scalar",
        ));
    }
    let lhs = args
        .input_value(0)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing lhs"))?
        .function_matrix_descriptor(FunctionArgumentRole::Input(0))?
        .ok_or_else(|| function_shape_contract_violation(contract, "lhs must be matrix-backed"))?;
    let rhs = args
        .input_value(1)
        .ok_or_else(|| function_shape_contract_violation(contract, "missing rhs"))?
        .function_matrix_descriptor(FunctionArgumentRole::Input(1))?
        .ok_or_else(|| function_shape_contract_violation(contract, "rhs must be matrix-backed"))?;
    if lhs.rows != rhs.rows || lhs.cols != rhs.cols {
        return Err(function_shape_contract_violation(
            contract,
            format!(
                "lhs is {}x{}, rhs is {}x{}",
                lhs.rows, lhs.cols, rhs.rows, rhs.cols,
            ),
        ));
    }
    Ok(())
}

macro_rules! declare_matrix_numeric_factory {
    (($cfg:meta; $operation:literal; $module:ident; $factory:ident; [$($feature:literal),*]); $token:ident; $scalar:ty; $scalar_feature:literal) => {
        mech_core::paste::paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all($cfg, feature = $scalar_feature),
                registration: [<register_ $module:snake _ $factory:snake _ $token>],
                installer: [<install_ $module:snake _ $factory:snake _ $token>],
                name: concat!(stringify!($factory), "<", $scalar_feature, ">"),
                factory_type: crate::$module::$factory<$scalar>,
                contract: matrix_numeric_runtime_contract!($module, $factory),
                package: "mech-matrix", crate_name: "mech_matrix",
                installer_path: concat!("mech_matrix::__mech_native::", stringify!([<install_ $module:snake _ $factory:snake _ $token>])),
                extra_cargo_features: [$operation],
            }
        }
    };
}

macro_rules! declare_matrix_numeric_family {
    (cfg: $cfg:meta, operation: $operation:literal, module: $module:ident, factory: $factory:ident, features: [$($feature:literal),* $(,)?]) => {
        for_each_matrix_numeric_scalar!(declare_matrix_numeric_factory, ($cfg; $operation; $module; $factory; [$($feature),*]));
    };
}

macro_rules! register_matrix_numeric_factory {
    ($builder:expr, $module:ident, $factory:ident, $token:ident) => {
        mech_core::paste::paste! { [<register_ $module:snake _ $factory:snake _ $token>]($builder)?; }
    };
}

macro_rules! install_declared_matrix_numeric_family {
    ($builder:expr, $module:ident, $factory:ident) => {{
        #[cfg(feature = "u8")]
        register_matrix_numeric_factory!($builder, $module, $factory, u8);
        #[cfg(feature = "u16")]
        register_matrix_numeric_factory!($builder, $module, $factory, u16);
        #[cfg(feature = "u32")]
        register_matrix_numeric_factory!($builder, $module, $factory, u32);
        #[cfg(feature = "u64")]
        register_matrix_numeric_factory!($builder, $module, $factory, u64);
        #[cfg(feature = "u128")]
        register_matrix_numeric_factory!($builder, $module, $factory, u128);
        #[cfg(feature = "i8")]
        register_matrix_numeric_factory!($builder, $module, $factory, i8);
        #[cfg(feature = "i16")]
        register_matrix_numeric_factory!($builder, $module, $factory, i16);
        #[cfg(feature = "i32")]
        register_matrix_numeric_factory!($builder, $module, $factory, i32);
        #[cfg(feature = "i64")]
        register_matrix_numeric_factory!($builder, $module, $factory, i64);
        #[cfg(feature = "i128")]
        register_matrix_numeric_factory!($builder, $module, $factory, i128);
        #[cfg(feature = "f32")]
        register_matrix_numeric_factory!($builder, $module, $factory, f32);
        #[cfg(feature = "f64")]
        register_matrix_numeric_factory!($builder, $module, $factory, f64);
    }};
}

declare_matrix_numeric_family! { cfg: feature = "dot", operation: "dot", module: dot, factory: DotScalar, features: [] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vector2"), operation: "dot", module: dot, factory: DotR2R2, features: ["row_vector2"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vector2"), operation: "dot", module: dot, factory: DotV2V2, features: ["vector2"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vector3"), operation: "dot", module: dot, factory: DotR3R3, features: ["row_vector3"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vector3"), operation: "dot", module: dot, factory: DotV3V3, features: ["vector3"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vector4"), operation: "dot", module: dot, factory: DotR4R4, features: ["row_vector4"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vector4"), operation: "dot", module: dot, factory: DotV4V4, features: ["vector4"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix1"), operation: "dot", module: dot, factory: DotM1M1, features: ["matrix1"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix2"), operation: "dot", module: dot, factory: DotM2M2, features: ["matrix2"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix3"), operation: "dot", module: dot, factory: DotM3M3, features: ["matrix3"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix4"), operation: "dot", module: dot, factory: DotM4M4, features: ["matrix4"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrixd"), operation: "dot", module: dot, factory: DotMDMD, features: ["matrixd"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vectord"), operation: "dot", module: dot, factory: DotVDVD, features: ["vectord"] }
declare_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vectord"), operation: "dot", module: dot, factory: DotRDRD, features: ["row_vectord"] }

declare_matrix_numeric_family! { cfg: feature = "matmul", operation: "matmul", module: matmul, factory: MatMulScalar, features: [] }
declare_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "matrixd"), operation: "matmul", module: matmul, factory: MatMulMDMD, features: ["matrixd"] }
declare_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "matrixd", feature = "vectord"), operation: "matmul", module: matmul, factory: MatMulMDVD, features: ["matrixd", "vectord"] }
declare_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "matrixd", feature = "row_vectord"), operation: "matmul", module: matmul, factory: MatMulMDRD, features: ["matrixd", "row_vectord"] }
declare_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "row_vectord", feature = "vectord"), operation: "matmul", module: matmul, factory: MatMulRDVD, features: ["matrix1", "row_vectord", "vectord"] }
declare_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "row_vectord", feature = "matrixd"), operation: "matmul", module: matmul, factory: MatMulRDMD, features: ["matrixd", "row_vectord"] }
declare_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "vectord", feature = "row_vectord", feature = "matrixd"), operation: "matmul", module: matmul, factory: MatMulVDRD, features: ["matrixd", "row_vectord", "vectord"] }

// Fixed-shape matrix multiplication is emitted by the same compiler operation
// as the dynamic families above.  Keep its complete shape matrix in one
// declaration traversal so registration, linkage metadata, and native exports
// cannot diverge.
macro_rules! for_each_matrix_matmul_fixed_family {
    ($callback:ident, ($($context:tt)*)) => {
        #[cfg(all(feature = "row_vector4", feature = "vector4", feature = "matrix1"))] $callback!($($context)*; MatMulR4V4; ["row_vector4", "vector4", "matrix1"]);
        #[cfg(all(feature = "row_vector4", feature = "matrix4"))] $callback!($($context)*; MatMulR4M4; ["row_vector4", "matrix4"]);
        #[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vectord"))] $callback!($($context)*; MatMulR4MD; ["row_vector4", "matrixd", "row_vectord"]);
        #[cfg(all(feature = "row_vector3", feature = "vector3", feature = "matrix1"))] $callback!($($context)*; MatMulR3V3; ["row_vector3", "vector3", "matrix1"]);
        #[cfg(all(feature = "row_vector3", feature = "matrix3"))] $callback!($($context)*; MatMulR3M3; ["row_vector3", "matrix3"]);
        #[cfg(all(feature = "row_vector3", feature = "matrix3x2", feature = "row_vector2"))] $callback!($($context)*; MatMulR3M3x2; ["row_vector3", "matrix3x2", "row_vector2"]);
        #[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))] $callback!($($context)*; MatMulR3MD; ["row_vector3", "matrixd", "row_vectord"]);
        #[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))] $callback!($($context)*; MatMulR2V2; ["row_vector2", "vector2", "matrix1"]);
        #[cfg(all(feature = "row_vector2", feature = "matrix2"))] $callback!($($context)*; MatMulR2M2; ["row_vector2", "matrix2"]);
        #[cfg(all(feature = "row_vector2", feature = "matrix2x3", feature = "row_vector3"))] $callback!($($context)*; MatMulR2M2x3; ["row_vector2", "matrix2x3", "row_vector3"]);
        #[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))] $callback!($($context)*; MatMulR2MD; ["row_vector2", "matrixd", "row_vectord"]);
        #[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))] $callback!($($context)*; MatMulV4R4; ["vector4", "row_vector4", "matrix4"]);
        #[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))] $callback!($($context)*; MatMulV3R3; ["vector3", "row_vector3", "matrix3"]);
        #[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))] $callback!($($context)*; MatMulV2R2; ["vector2", "row_vector2", "matrix2"]);
        #[cfg(all(feature = "matrix4", feature = "vector4"))] $callback!($($context)*; MatMulM4V4; ["matrix4", "vector4"]);
        #[cfg(feature = "matrix4")] $callback!($($context)*; MatMulM4M4; ["matrix4"]);
        #[cfg(all(feature = "matrix4", feature = "matrixd"))] $callback!($($context)*; MatMulM4MD; ["matrix4", "matrixd"]);
        #[cfg(all(feature = "matrix2", feature = "matrix2x3"))] $callback!($($context)*; MatMulM2M2x3; ["matrix2", "matrix2x3"]);
        #[cfg(feature = "matrix2")] $callback!($($context)*; MatMulM2M2; ["matrix2"]);
        #[cfg(all(feature = "matrix2", feature = "vector2"))] $callback!($($context)*; MatMulM2V2; ["matrix2", "vector2"]);
        #[cfg(all(feature = "matrix2", feature = "matrixd"))] $callback!($($context)*; MatMulM2MD; ["matrix2", "matrixd"]);
        #[cfg(feature = "matrix3")] $callback!($($context)*; MatMulM3M3; ["matrix3"]);
        #[cfg(all(feature = "matrix3", feature = "matrix3x2"))] $callback!($($context)*; MatMulM2M3x2; ["matrix3", "matrix3x2"]);
        #[cfg(all(feature = "matrix3", feature = "vector3"))] $callback!($($context)*; MatMulM3V3; ["matrix3", "vector3"]);
        #[cfg(all(feature = "matrix3", feature = "matrixd"))] $callback!($($context)*; MatMulM3MD; ["matrix3", "matrixd"]);
        #[cfg(feature = "matrix1")] $callback!($($context)*; MatMulM1M1; ["matrix1"]);
        #[cfg(all(feature = "matrix2x3", feature = "vector3", feature = "vector2"))] $callback!($($context)*; MatMulM2x3V2; ["matrix2x3", "vector3", "vector2"]);
        #[cfg(all(feature = "matrix2x3", feature = "matrix3"))] $callback!($($context)*; MatMulM2x3M3; ["matrix2x3", "matrix3"]);
        #[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = "matrix2"))] $callback!($($context)*; MatMulM2x3M3x2; ["matrix2x3", "matrix3x2", "matrix2"]);
        #[cfg(all(feature = "matrix2x3", feature = "matrixd"))] $callback!($($context)*; MatMulM2x3MD; ["matrix2x3", "matrixd"]);
        #[cfg(all(feature = "matrix3x2", feature = "vector2", feature = "vector3"))] $callback!($($context)*; MatMulM3x2V2; ["matrix3x2", "vector2", "vector3"]);
        #[cfg(all(feature = "matrix3x2", feature = "matrix2"))] $callback!($($context)*; MatMulM3x2M2; ["matrix3x2", "matrix2"]);
        #[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = "matrix3"))] $callback!($($context)*; MatMulM3x2M2x3; ["matrix3x2", "matrix2x3", "matrix3"]);
        #[cfg(all(feature = "matrix3x2", feature = "matrixd"))] $callback!($($context)*; MatMulM3x2MD; ["matrix3x2", "matrixd"]);
        #[cfg(all(feature = "matrixd", feature = "matrix3x2"))] $callback!($($context)*; MatMulMDM3x2; ["matrixd", "matrix3x2"]);
    };
}

macro_rules! declare_matrix_matmul_fixed_family {
    (; $factory:ident; [$($feature:literal),+]) => {
        declare_matrix_numeric_family! {
            cfg: all(feature = "matmul", $(feature = $feature),+),
            operation: "matmul", module: matmul, factory: $factory,
            features: [$($feature),+]
        }
    };
}

for_each_matrix_matmul_fixed_family!(declare_matrix_matmul_fixed_family, ());

macro_rules! export_matrix_numeric_factory {
    (($cfg:meta; $module:ident; $factory:ident); $token:ident; $_scalar:ty; $scalar_feature:literal) => {
        #[cfg(all($cfg, feature = $scalar_feature))]
        mech_core::paste::paste! { pub use super::[<install_ $module:snake _ $factory:snake _ $token>]; }
    };
}

macro_rules! export_matrix_numeric_family {
    (cfg: $cfg:meta, module: $module:ident, factory: $factory:ident) => {
        for_each_matrix_numeric_scalar!(export_matrix_numeric_factory, ($cfg; $module; $factory));
    };
}

macro_rules! export_matrix_matmul_fixed_family {
    (; $factory:ident; [$($feature:literal),+]) => {
        export_matrix_numeric_family! {
            cfg: all(feature = "matmul", $(feature = $feature),+),
            module: matmul, factory: $factory
        }
    };
}

macro_rules! for_each_matrix_transpose_scalar {
    ($callback:ident, $context:tt) => {
        $callback!($context; bool; bool; "bool"; "bool"; "bool");
        $callback!($context; u8; u8; "u8"; "u8"; "u8");
        $callback!($context; u16; u16; "u16"; "u16"; "u16");
        $callback!($context; u32; u32; "u32"; "u32"; "u32");
        $callback!($context; u64; u64; "u64"; "u64"; "u64");
        $callback!($context; u128; u128; "u128"; "u128"; "u128");
        $callback!($context; i8; i8; "i8"; "i8"; "i8");
        $callback!($context; i16; i16; "i16"; "i16"; "i16");
        $callback!($context; i32; i32; "i32"; "i32"; "i32");
        $callback!($context; i64; i64; "i64"; "i64"; "i64");
        $callback!($context; i128; i128; "i128"; "i128"; "i128");
        $callback!($context; f32; f32; "f32"; "f32"; "f32");
        $callback!($context; f64; f64; "f64"; "f64"; "f64");
        $callback!($context; string; String; "string"; "string"; "string");
        $callback!($context; c64; C64; "c64"; "complex"; "complex");
        $callback!($context; r64; R64; "r64"; "rational"; "rational");
    };
}

macro_rules! declare_matrix_transpose_factory {
    (($factory:ident; [$($shape_feature:literal),+]); $token:ident; $scalar:ty; $name:literal; $scalar_cfg:literal; $scalar_feature:literal) => {
        mech_core::paste::paste! { mech_core::declare_native_runtime_factory! {
            cfg: all(feature = "transpose", feature = $scalar_cfg, $(feature = $shape_feature),+),
            registration: [<register_transpose_ $factory:snake _ $token>],
            installer: [<install_transpose_ $factory:snake _ $token>],
            name: concat!(stringify!($factory), "<", $name, ">"),
            factory_type: crate::transpose::$factory<$scalar>,
            contract: RuntimeFunctionContract::transpose(RuntimeOutputAliasPolicy::DisallowInputAlias),
            package: "mech-matrix", crate_name: "mech_matrix",
            installer_path: concat!("mech_matrix::__mech_native::", stringify!([<install_transpose_ $factory:snake _ $token>])),
            extra_cargo_features: ["transpose"],
        }}
    };
}

macro_rules! declare_matrix_transpose_family {
    ($factory:ident; [$($shape_feature:literal),+]) => {
        for_each_matrix_transpose_scalar!(declare_matrix_transpose_factory, ($factory; [$($shape_feature),+]));
    };
}

macro_rules! register_matrix_transpose_factory {
    (($builder:ident; $factory:ident); $token:ident; $_scalar:ty; $_name:literal; $scalar_cfg:literal; $_scalar_feature:literal) => {
        #[cfg(feature = $scalar_cfg)]
        mech_core::paste::paste! { [<register_transpose_ $factory:snake _ $token>]($builder)?; }
    };
}

macro_rules! install_declared_matrix_transpose_family {
    ($builder:ident; $factory:ident) => { for_each_matrix_transpose_scalar!(register_matrix_transpose_factory, ($builder; $factory)); };
}

macro_rules! export_matrix_transpose_factory {
    (($factory:ident; [$($shape_feature:literal),+]); $token:ident; $_scalar:ty; $_name:literal; $scalar_cfg:literal; $_scalar_feature:literal) => {
        #[cfg(all(feature = "transpose", feature = $scalar_cfg, $(feature = $shape_feature),+))]
        mech_core::paste::paste! { pub use super::[<install_transpose_ $factory:snake _ $token>]; }
    };
}

macro_rules! export_matrix_transpose_family {
    ($factory:ident; [$($shape_feature:literal),+]) => { for_each_matrix_transpose_scalar!(export_matrix_transpose_factory, ($factory; [$($shape_feature),+])); };
}

declare_matrix_transpose_family!(TransposeMD; ["matrixd"]);
declare_matrix_transpose_family!(TransposeVD; ["vectord", "row_vectord"]);
declare_matrix_transpose_family!(TransposeRD; ["row_vectord", "vectord"]);

macro_rules! for_each_matrix_transpose_fixed_family {
    ($callback:ident, ($($context:tt)*)) => {
        #[cfg(feature = "matrix1")] $callback!($($context)*; TransposeM1; ["matrix1"]);
        #[cfg(feature = "matrix2")] $callback!($($context)*; TransposeM2; ["matrix2"]);
        #[cfg(feature = "matrix3")] $callback!($($context)*; TransposeM3; ["matrix3"]);
        #[cfg(feature = "matrix4")] $callback!($($context)*; TransposeM4; ["matrix4"]);
        #[cfg(all(feature = "matrix2x3", feature = "matrix3x2"))] $callback!($($context)*; TransposeM2x3; ["matrix2x3", "matrix3x2"]);
        #[cfg(all(feature = "matrix3x2", feature = "matrix2x3"))] $callback!($($context)*; TransposeM3x2; ["matrix3x2", "matrix2x3"]);
        #[cfg(all(feature = "vector2", feature = "row_vector2"))] $callback!($($context)*; TransposeV2; ["vector2", "row_vector2"]);
        #[cfg(all(feature = "vector3", feature = "row_vector3"))] $callback!($($context)*; TransposeV3; ["vector3", "row_vector3"]);
        #[cfg(all(feature = "vector4", feature = "row_vector4"))] $callback!($($context)*; TransposeV4; ["vector4", "row_vector4"]);
        #[cfg(all(feature = "row_vector2", feature = "vector2"))] $callback!($($context)*; TransposeR2; ["row_vector2", "vector2"]);
        #[cfg(all(feature = "row_vector3", feature = "vector3"))] $callback!($($context)*; TransposeR3; ["row_vector3", "vector3"]);
        #[cfg(all(feature = "row_vector4", feature = "vector4"))] $callback!($($context)*; TransposeR4; ["row_vector4", "vector4"]);
    };
}

macro_rules! declare_matrix_transpose_fixed_family {
    (; $factory:ident; [$($feature:literal),+]) => {
        declare_matrix_transpose_family!($factory; [$($feature),+]);
    };
}

for_each_matrix_transpose_fixed_family!(declare_matrix_transpose_fixed_family, ());

macro_rules! export_matrix_transpose_fixed_family {
    (; $factory:ident; [$($feature:literal),+]) => {
        export_matrix_transpose_family!($factory; [$($feature),+]);
    };
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "solve", feature = "matrixd", feature = "vectord", feature = "f64"),
    registration: register_matrix_solve_mdvd_f64,
    installer: install_matrix_solve_mdvd_f64,
    name: "MatrixSolveMDVD<f64>",
    factory_type: crate::solve::MatrixSolveMDVD<f64>,
    contract: RuntimeFunctionContract::linear_solve(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-matrix", crate_name: "mech_matrix",
    installer_path: "mech_matrix::__mech_native::install_matrix_solve_mdvd_f64",
    extra_cargo_features: ["solve"],
}

#[cfg(feature = "dot")]
fn install_dot_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_declared_matrix_numeric_family!(builder, dot, DotScalar);
    #[cfg(feature = "row_vector2")]
    install_declared_matrix_numeric_family!(builder, dot, DotR2R2);
    #[cfg(feature = "vector2")]
    install_declared_matrix_numeric_family!(builder, dot, DotV2V2);
    #[cfg(feature = "row_vector3")]
    install_declared_matrix_numeric_family!(builder, dot, DotR3R3);
    #[cfg(feature = "vector3")]
    install_declared_matrix_numeric_family!(builder, dot, DotV3V3);
    #[cfg(feature = "row_vector4")]
    install_declared_matrix_numeric_family!(builder, dot, DotR4R4);
    #[cfg(feature = "vector4")]
    install_declared_matrix_numeric_family!(builder, dot, DotV4V4);
    #[cfg(feature = "matrix1")]
    install_declared_matrix_numeric_family!(builder, dot, DotM1M1);
    #[cfg(feature = "matrix2")]
    install_declared_matrix_numeric_family!(builder, dot, DotM2M2);
    #[cfg(feature = "matrix3")]
    install_declared_matrix_numeric_family!(builder, dot, DotM3M3);
    #[cfg(feature = "matrix4")]
    install_declared_matrix_numeric_family!(builder, dot, DotM4M4);
    #[cfg(feature = "matrixd")]
    install_declared_matrix_numeric_family!(builder, dot, DotMDMD);
    #[cfg(feature = "vectord")]
    install_declared_matrix_numeric_family!(builder, dot, DotVDVD);
    #[cfg(feature = "row_vectord")]
    install_declared_matrix_numeric_family!(builder, dot, DotRDRD);
    Ok(())
}

#[cfg(feature = "matmul")]
fn install_matmul_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_declared_matrix_numeric_family!(builder, matmul, MatMulScalar);

    #[cfg(all(feature = "row_vector4", feature = "vector4", feature = "matrix1"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR4V4);
    #[cfg(all(feature = "row_vector4", feature = "matrix4"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR4M4);
    #[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vectord"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR4MD);

    #[cfg(all(feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR3V3);
    #[cfg(all(feature = "row_vector3", feature = "matrix3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR3M3);
    #[cfg(all(
        feature = "row_vector3",
        feature = "matrix3x2",
        feature = "row_vector2"
    ))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR3M3x2);
    #[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR3MD);

    #[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR2V2);
    #[cfg(all(feature = "row_vector2", feature = "matrix2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR2M2);
    #[cfg(all(
        feature = "row_vector2",
        feature = "matrix2x3",
        feature = "row_vector3"
    ))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR2M2x3);
    #[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulR2MD);

    #[cfg(all(feature = "row_vectord", feature = "vectord"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulRDVD);
    #[cfg(all(feature = "row_vectord", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulRDMD);

    #[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulV4R4);
    #[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulV3R3);
    #[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulV2R2);
    #[cfg(all(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulVDRD);

    #[cfg(all(feature = "matrix4", feature = "vector4"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM4V4);
    #[cfg(feature = "matrix4")]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM4M4);
    #[cfg(all(feature = "matrix4", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM4MD);

    #[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2M2x3);
    #[cfg(feature = "matrix2")]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2M2);
    #[cfg(all(feature = "matrix2", feature = "vector2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2V2);
    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2MD);

    #[cfg(feature = "matrix3")]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3M3);
    #[cfg(all(feature = "matrix3", feature = "matrix3x2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2M3x2);
    #[cfg(all(feature = "matrix3", feature = "vector3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3V3);
    #[cfg(all(feature = "matrix3", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3MD);

    #[cfg(feature = "matrix1")]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM1M1);

    #[cfg(all(feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2x3V2);
    #[cfg(all(feature = "matrix2x3", feature = "matrix3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2x3M3);
    #[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = "matrix2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2x3M3x2);
    #[cfg(all(feature = "matrix2x3", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM2x3MD);

    #[cfg(all(feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3x2V2);
    #[cfg(all(feature = "matrix3x2", feature = "matrix2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3x2M2);
    #[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = "matrix3"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3x2M2x3);
    #[cfg(all(feature = "matrix3x2", feature = "matrixd"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulM3x2MD);

    #[cfg(feature = "matrixd")]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulMDMD);
    #[cfg(all(feature = "matrixd", feature = "matrix3x2"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulMDM3x2);
    #[cfg(all(feature = "matrixd", feature = "vectord"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulMDVD);
    #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
    install_declared_matrix_numeric_family!(builder, matmul, MatMulMDRD);
    Ok(())
}

#[cfg(feature = "transpose")]
fn install_transpose_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix1")]
    install_declared_matrix_transpose_family!(builder; TransposeM1);
    #[cfg(feature = "matrix2")]
    install_declared_matrix_transpose_family!(builder; TransposeM2);
    #[cfg(feature = "matrix3")]
    install_declared_matrix_transpose_family!(builder; TransposeM3);
    #[cfg(feature = "matrix4")]
    install_declared_matrix_transpose_family!(builder; TransposeM4);
    #[cfg(all(feature = "matrix2x3", feature = "matrix3x2"))]
    install_declared_matrix_transpose_family!(builder; TransposeM2x3);
    #[cfg(all(feature = "matrix3x2", feature = "matrix2x3"))]
    install_declared_matrix_transpose_family!(builder; TransposeM3x2);
    #[cfg(feature = "matrixd")]
    install_declared_matrix_transpose_family!(builder; TransposeMD);
    #[cfg(all(feature = "vector2", feature = "row_vector2"))]
    install_declared_matrix_transpose_family!(builder; TransposeV2);
    #[cfg(all(feature = "vector3", feature = "row_vector3"))]
    install_declared_matrix_transpose_family!(builder; TransposeV3);
    #[cfg(all(feature = "vector4", feature = "row_vector4"))]
    install_declared_matrix_transpose_family!(builder; TransposeV4);
    #[cfg(all(feature = "vectord", feature = "row_vectord"))]
    install_declared_matrix_transpose_family!(builder; TransposeVD);
    #[cfg(all(feature = "row_vector2", feature = "vector2"))]
    install_declared_matrix_transpose_family!(builder; TransposeR2);
    #[cfg(all(feature = "row_vector3", feature = "vector3"))]
    install_declared_matrix_transpose_family!(builder; TransposeR3);
    #[cfg(all(feature = "row_vector4", feature = "vector4"))]
    install_declared_matrix_transpose_family!(builder; TransposeR4);
    #[cfg(all(feature = "row_vectord", feature = "vectord"))]
    install_declared_matrix_transpose_family!(builder; TransposeRD);
    Ok(())
}

/// Installs every enabled concrete bytecode factory owned by `mech-matrix`.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "dot")]
    install_dot_runtime(builder)?;
    #[cfg(feature = "matmul")]
    install_matmul_runtime(builder)?;
    #[cfg(all(
        feature = "solve",
        feature = "matrixd",
        feature = "vectord",
        feature = "f64"
    ))]
    register_matrix_solve_mdvd_f64(builder)?;
    #[cfg(feature = "transpose")]
    install_transpose_runtime(builder)?;
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    export_matrix_numeric_family! { cfg: feature = "dot", module: dot, factory: DotScalar }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vector2"), module: dot, factory: DotR2R2 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vector2"), module: dot, factory: DotV2V2 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vector3"), module: dot, factory: DotR3R3 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vector3"), module: dot, factory: DotV3V3 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vector4"), module: dot, factory: DotR4R4 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vector4"), module: dot, factory: DotV4V4 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix1"), module: dot, factory: DotM1M1 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix2"), module: dot, factory: DotM2M2 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix3"), module: dot, factory: DotM3M3 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrix4"), module: dot, factory: DotM4M4 }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "matrixd"), module: dot, factory: DotMDMD }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "vectord"), module: dot, factory: DotVDVD }
    export_matrix_numeric_family! { cfg: all(feature = "dot", feature = "row_vectord"), module: dot, factory: DotRDRD }
    export_matrix_numeric_family! { cfg: feature = "matmul", module: matmul, factory: MatMulScalar }
    export_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "matrixd"), module: matmul, factory: MatMulMDMD }
    export_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "matrixd", feature = "vectord"), module: matmul, factory: MatMulMDVD }
    export_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "matrixd", feature = "row_vectord"), module: matmul, factory: MatMulMDRD }
    export_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "row_vectord", feature = "vectord"), module: matmul, factory: MatMulRDVD }
    export_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "row_vectord", feature = "matrixd"), module: matmul, factory: MatMulRDMD }
    export_matrix_numeric_family! { cfg: all(feature = "matmul", feature = "vectord", feature = "row_vectord", feature = "matrixd"), module: matmul, factory: MatMulVDRD }
    for_each_matrix_matmul_fixed_family!(export_matrix_matmul_fixed_family, ());
    export_matrix_transpose_family!(TransposeMD; ["matrixd"]);
    export_matrix_transpose_family!(TransposeVD; ["vectord", "row_vectord"]);
    export_matrix_transpose_family!(TransposeRD; ["row_vectord", "vectord"]);
    for_each_matrix_transpose_fixed_family!(export_matrix_transpose_fixed_family, ());
    #[cfg(all(
        feature = "solve",
        feature = "matrixd",
        feature = "vectord",
        feature = "f64"
    ))]
    pub use super::install_matrix_solve_mdvd_f64;
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    fn expected_operations() -> Vec<&'static str> {
        let mut expected = Vec::new();
        #[cfg(feature = "dot")]
        expected.push("matrix/dot");
        #[cfg(feature = "matmul")]
        expected.push("matrix/matmul");
        #[cfg(feature = "solve")]
        expected.push("matrix/solve");
        #[cfg(feature = "transpose")]
        expected.push("matrix/transpose");
        expected
    }

    #[test]
    fn source_catalog_matches_the_frozen_matrix_surface() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let expected = expected_operations();

        #[cfg(all(
            feature = "dot",
            feature = "matmul",
            feature = "solve",
            feature = "transpose"
        ))]
        assert_eq!(expected.len(), 4);
        assert_eq!(catalog.specializer_count(), expected.len());
        assert_eq!(catalog.runtime_factory_count(), 0);
        for name in expected {
            let operation = OperationId::from_name(name);
            assert_eq!(catalog.specializer(operation).unwrap().canonical_name, name);
            assert_eq!(
                catalog.exports_for_operation(operation),
                &[FunctionExport {
                    operation,
                    canonical_name: name.to_string(),
                    module: None,
                    item: None,
                    exposure: FunctionExposure::Prelude,
                }],
            );
        }
    }
}

#[cfg(test)]
mod runtime_signature_tests {
    use super::*;
    use mech_core::{FunctionRuntimeType, RuntimeFunctionSignature};

    #[cfg(all(feature = "dot", feature = "f64", feature = "matrix1"))]
    #[test]
    fn dot_matrix1_signature_is_matrix1_by_matrix1_to_scalar() {
        use nalgebra::Matrix1;

        assert_eq!(
            <crate::dot::DotM1M1<f64> as MechFunctionFactory>::SIGNATURE,
            RuntimeFunctionSignature::binary(
                <f64 as FunctionRuntimeType>::REPRESENTATION,
                <Matrix1<f64> as FunctionRuntimeType>::REPRESENTATION,
                <Matrix1<f64> as FunctionRuntimeType>::REPRESENTATION,
            ),
        );
    }

    #[cfg(all(
        feature = "matmul",
        feature = "f64",
        feature = "row_vector3",
        feature = "matrix3x2",
        feature = "row_vector2"
    ))]
    #[test]
    fn fixed_matmul_signature_preserves_every_exact_storage_type() {
        use nalgebra::{Matrix3x2, RowVector2, RowVector3};

        assert_eq!(
            <crate::matmul::MatMulR3M3x2<f64> as MechFunctionFactory>::SIGNATURE,
            RuntimeFunctionSignature::binary(
                <RowVector2<f64> as FunctionRuntimeType>::REPRESENTATION,
                <RowVector3<f64> as FunctionRuntimeType>::REPRESENTATION,
                <Matrix3x2<f64> as FunctionRuntimeType>::REPRESENTATION,
            ),
        );
    }

    #[cfg(all(
        feature = "matmul",
        feature = "f64",
        feature = "row_vectord",
        feature = "vectord"
    ))]
    #[test]
    fn dynamic_row_by_vector_matmul_always_returns_matrix1() {
        use nalgebra::{DVector, Matrix1, RowDVector};

        let expected = RuntimeFunctionSignature::binary(
            <Matrix1<f64> as FunctionRuntimeType>::REPRESENTATION,
            <RowDVector<f64> as FunctionRuntimeType>::REPRESENTATION,
            <DVector<f64> as FunctionRuntimeType>::REPRESENTATION,
        );
        assert_eq!(
            <crate::matmul::MatMulRDVD<f64> as MechFunctionFactory>::SIGNATURE,
            expected,
        );

        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let entry = catalog
            .runtime_entry(mech_core::RuntimeFunctionId::from_name("MatMulRDVD<f64>"))
            .unwrap();
        assert_eq!(entry.signature(), expected);
    }
}
