#[cfg(feature = "complex")]
use mech_core::C64;
#[cfg(feature = "rational")]
use mech_core::R64;
use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, FunctionSpecializer, MResult,
    MechFunctionFactory,
};
use std::sync::Arc;

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

macro_rules! install_typed_factory {
    ($builder:expr, $module:ident, $factory:ident, $scalar:ty, $scalar_name:literal) => {
        $builder.insert_runtime_factory(
            concat!(stringify!($factory), "<", $scalar_name, ">"),
            <crate::$module::$factory<$scalar> as MechFunctionFactory>::new,
        )?;
    };
}

macro_rules! install_numeric_factories {
    ($builder:expr, $module:ident, $factory:ident) => {{
        #[cfg(feature = "u8")]
        install_typed_factory!($builder, $module, $factory, u8, "u8");
        #[cfg(feature = "u16")]
        install_typed_factory!($builder, $module, $factory, u16, "u16");
        #[cfg(feature = "u32")]
        install_typed_factory!($builder, $module, $factory, u32, "u32");
        #[cfg(feature = "u64")]
        install_typed_factory!($builder, $module, $factory, u64, "u64");
        #[cfg(feature = "u128")]
        install_typed_factory!($builder, $module, $factory, u128, "u128");
        #[cfg(feature = "i8")]
        install_typed_factory!($builder, $module, $factory, i8, "i8");
        #[cfg(feature = "i16")]
        install_typed_factory!($builder, $module, $factory, i16, "i16");
        #[cfg(feature = "i32")]
        install_typed_factory!($builder, $module, $factory, i32, "i32");
        #[cfg(feature = "i64")]
        install_typed_factory!($builder, $module, $factory, i64, "i64");
        #[cfg(feature = "i128")]
        install_typed_factory!($builder, $module, $factory, i128, "i128");
        #[cfg(feature = "f32")]
        install_typed_factory!($builder, $module, $factory, f32, "f32");
        #[cfg(feature = "f64")]
        install_typed_factory!($builder, $module, $factory, f64, "f64");
    }};
}

macro_rules! install_transpose_factories {
    ($builder:expr, $factory:ident) => {{
        install_numeric_factories!($builder, transpose, $factory);
        #[cfg(feature = "bool")]
        install_typed_factory!($builder, transpose, $factory, bool, "bool");
        #[cfg(feature = "string")]
        install_typed_factory!($builder, transpose, $factory, String, "string");
        #[cfg(feature = "complex")]
        install_typed_factory!($builder, transpose, $factory, C64, "c64");
        #[cfg(feature = "rational")]
        install_typed_factory!($builder, transpose, $factory, R64, "r64");
    }};
}

#[cfg(feature = "dot")]
fn install_dot_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_numeric_factories!(builder, dot, DotScalar);
    #[cfg(feature = "row_vector2")]
    install_numeric_factories!(builder, dot, DotR2R2);
    #[cfg(feature = "vector2")]
    install_numeric_factories!(builder, dot, DotV2V2);
    #[cfg(feature = "row_vector3")]
    install_numeric_factories!(builder, dot, DotR3R3);
    #[cfg(feature = "vector3")]
    install_numeric_factories!(builder, dot, DotV3V3);
    #[cfg(feature = "row_vector4")]
    install_numeric_factories!(builder, dot, DotR4R4);
    #[cfg(feature = "vector4")]
    install_numeric_factories!(builder, dot, DotV4V4);
    #[cfg(feature = "matrix1")]
    install_numeric_factories!(builder, dot, DotM1M1);
    #[cfg(feature = "matrix2")]
    install_numeric_factories!(builder, dot, DotM2M2);
    #[cfg(feature = "matrix3")]
    install_numeric_factories!(builder, dot, DotM3M3);
    #[cfg(feature = "matrix4")]
    install_numeric_factories!(builder, dot, DotM4M4);
    #[cfg(feature = "matrixd")]
    install_numeric_factories!(builder, dot, DotMDMD);
    #[cfg(feature = "vectord")]
    install_numeric_factories!(builder, dot, DotVDVD);
    #[cfg(feature = "row_vectord")]
    install_numeric_factories!(builder, dot, DotRDRD);
    Ok(())
}

#[cfg(feature = "matmul")]
fn install_matmul_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_numeric_factories!(builder, matmul, MatMulScalar);

    #[cfg(all(feature = "row_vector4", feature = "vector4", feature = "matrix1"))]
    install_numeric_factories!(builder, matmul, MatMulR4V4);
    #[cfg(all(feature = "row_vector4", feature = "matrix4"))]
    install_numeric_factories!(builder, matmul, MatMulR4M4);
    #[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vectord"))]
    install_numeric_factories!(builder, matmul, MatMulR4MD);

    #[cfg(all(feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
    install_numeric_factories!(builder, matmul, MatMulR3V3);
    #[cfg(all(feature = "row_vector3", feature = "matrix3"))]
    install_numeric_factories!(builder, matmul, MatMulR3M3);
    #[cfg(all(feature = "row_vector3", feature = "matrix3x2"))]
    install_numeric_factories!(builder, matmul, MatMulR3M3x2);
    #[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
    install_numeric_factories!(builder, matmul, MatMulR3MD);

    #[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
    install_numeric_factories!(builder, matmul, MatMulR2V2);
    #[cfg(all(feature = "row_vector2", feature = "matrix2"))]
    install_numeric_factories!(builder, matmul, MatMulR2M2);
    #[cfg(all(
        feature = "row_vector2",
        feature = "matrix2x3",
        feature = "row_vector3"
    ))]
    install_numeric_factories!(builder, matmul, MatMulR2M2x3);
    #[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
    install_numeric_factories!(builder, matmul, MatMulR2MD);

    #[cfg(all(
        feature = "row_vectord",
        feature = "vectord",
        any(feature = "matrix1", feature = "matrixd")
    ))]
    install_numeric_factories!(builder, matmul, MatMulRDVD);
    #[cfg(all(feature = "row_vectord", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulRDMD);

    #[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
    install_numeric_factories!(builder, matmul, MatMulV4R4);
    #[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
    install_numeric_factories!(builder, matmul, MatMulV3R3);
    #[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
    install_numeric_factories!(builder, matmul, MatMulV2R2);
    #[cfg(all(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulVDRD);

    #[cfg(all(feature = "matrix4", feature = "vector4"))]
    install_numeric_factories!(builder, matmul, MatMulM4V4);
    #[cfg(feature = "matrix4")]
    install_numeric_factories!(builder, matmul, MatMulM4M4);
    #[cfg(all(feature = "matrix4", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulM4MD);

    #[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
    install_numeric_factories!(builder, matmul, MatMulM2M2x3);
    #[cfg(feature = "matrix2")]
    install_numeric_factories!(builder, matmul, MatMulM2M2);
    #[cfg(all(feature = "matrix2", feature = "vector2"))]
    install_numeric_factories!(builder, matmul, MatMulM2V2);
    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulM2MD);

    #[cfg(feature = "matrix3")]
    install_numeric_factories!(builder, matmul, MatMulM3M3);
    #[cfg(all(feature = "matrix3", feature = "matrix3x2"))]
    install_numeric_factories!(builder, matmul, MatMulM2M3x2);
    #[cfg(all(feature = "matrix3", feature = "vector3"))]
    install_numeric_factories!(builder, matmul, MatMulM3V3);
    #[cfg(all(feature = "matrix3", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulM3MD);

    #[cfg(feature = "matrix1")]
    install_numeric_factories!(builder, matmul, MatMulM1M1);

    #[cfg(all(feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
    install_numeric_factories!(builder, matmul, MatMulM2x3V2);
    #[cfg(all(feature = "matrix2x3", feature = "matrix3"))]
    install_numeric_factories!(builder, matmul, MatMulM2x3M3);
    #[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = "matrix2"))]
    install_numeric_factories!(builder, matmul, MatMulM2x3M3x2);
    #[cfg(all(feature = "matrix2x3", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulM2x3MD);

    #[cfg(all(feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
    install_numeric_factories!(builder, matmul, MatMulM3x2V2);
    #[cfg(all(feature = "matrix3x2", feature = "matrix2"))]
    install_numeric_factories!(builder, matmul, MatMulM3x2M2);
    #[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = "matrix3"))]
    install_numeric_factories!(builder, matmul, MatMulM3x2M2x3);
    #[cfg(all(feature = "matrix3x2", feature = "matrixd"))]
    install_numeric_factories!(builder, matmul, MatMulM3x2MD);

    #[cfg(feature = "matrixd")]
    install_numeric_factories!(builder, matmul, MatMulMDMD);
    #[cfg(all(feature = "matrixd", feature = "matrix3x2"))]
    install_numeric_factories!(builder, matmul, MatMulMDM3x2);
    #[cfg(all(feature = "matrixd", feature = "vectord"))]
    install_numeric_factories!(builder, matmul, MatMulMDVD);
    #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
    install_numeric_factories!(builder, matmul, MatMulMDRD);
    Ok(())
}

#[cfg(feature = "transpose")]
fn install_transpose_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "matrix1")]
    install_transpose_factories!(builder, TransposeM1);
    #[cfg(feature = "matrix2")]
    install_transpose_factories!(builder, TransposeM2);
    #[cfg(feature = "matrix3")]
    install_transpose_factories!(builder, TransposeM3);
    #[cfg(feature = "matrix4")]
    install_transpose_factories!(builder, TransposeM4);
    #[cfg(all(feature = "matrix2x3", feature = "matrix3x2"))]
    install_transpose_factories!(builder, TransposeM2x3);
    #[cfg(all(feature = "matrix3x2", feature = "matrix2x3"))]
    install_transpose_factories!(builder, TransposeM3x2);
    #[cfg(feature = "matrixd")]
    install_transpose_factories!(builder, TransposeMD);
    #[cfg(all(feature = "vector2", feature = "row_vector2"))]
    install_transpose_factories!(builder, TransposeV2);
    #[cfg(all(feature = "vector3", feature = "row_vector3"))]
    install_transpose_factories!(builder, TransposeV3);
    #[cfg(all(feature = "vector4", feature = "row_vector4"))]
    install_transpose_factories!(builder, TransposeV4);
    #[cfg(all(feature = "vectord", feature = "row_vectord"))]
    install_transpose_factories!(builder, TransposeVD);
    #[cfg(all(feature = "row_vector2", feature = "vector2"))]
    install_transpose_factories!(builder, TransposeR2);
    #[cfg(all(feature = "row_vector3", feature = "vector3"))]
    install_transpose_factories!(builder, TransposeR3);
    #[cfg(all(feature = "row_vector4", feature = "vector4"))]
    install_transpose_factories!(builder, TransposeR4);
    #[cfg(all(feature = "row_vectord", feature = "vectord"))]
    install_transpose_factories!(builder, TransposeRD);
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
    builder.insert_runtime_factory(
        "MatrixSolveMDVD<f64>",
        <crate::solve::MatrixSolveMDVD<f64> as MechFunctionFactory>::new,
    )?;
    #[cfg(feature = "transpose")]
    install_transpose_runtime(builder)?;
    Ok(())
}

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(test)]
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
