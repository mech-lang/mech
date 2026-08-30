#![cfg(all(feature = "functions", feature = "f64"))]

use mech_core::*;
use nalgebra::{
    DMatrix, DVector, Matrix1, Matrix2, Matrix2x3, Matrix3, Matrix3x2, Matrix4, RowDVector,
    RowVector2, RowVector3, RowVector4, Vector2, Vector3, Vector4,
};
use std::marker::PhantomData;

macro_rules! define_probe_family {
    ($name:ident, $lhs:ty, $rhs:ty, $out:ty, $operation:ident) => {
        struct $name<T>(PhantomData<(T, $lhs, $rhs, $out)>);
        const _: usize = std::mem::size_of::<$name<u8>>();
    };
}

// This expansion intentionally has no `use paste::paste`. Exported stdlib
// macros must resolve their proc-macro dependency at the definition crate.
mech_core::impl_fxns!(HygieneProbe, T, T, define_probe_family);

struct NativeProbe;

impl MechFunctionImpl for NativeProbe {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn to_string(&self) -> String {
        "NativeProbe".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for NativeProbe {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct BinopProbeSS<T>(PhantomData<T>);

impl MechFunctionFactory for BinopProbeSS<f64> {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::F64,
        FunctionValueRepresentation::F64,
        FunctionValueRepresentation::F64,
    );

    fn new_invocation(_invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        Ok(Box::new(NativeProbe))
    }
}

struct UnopProbeF64S;

impl MechFunctionFactory for UnopProbeF64S {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::F64,
        FunctionValueRepresentation::F64,
    );

    fn new_invocation(_invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        Ok(Box::new(NativeProbe))
    }
}

#[test]
fn exported_stdlib_macros_are_hygienic_without_consumer_paste_imports() -> MResult<()> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_core::__mech_install_binop_runtime_factory!(builder, BinopProbe, SS, f64, "f64");
    mech_core::__mech_install_unop_runtime_factory!(builder, UnopProbe, F64, S);
    let catalog = builder.build()?;
    assert_eq!(catalog.runtime_factory_count(), 2);
    Ok(())
}
