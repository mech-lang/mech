//! Canonical constant compilation for exact runtime backings.

#[cfg(feature = "semantic-compiler")]
use crate::{
    CanonicalCellBacking, FunctionRuntimeType, MResult, ValueCell,
    program::bytecode::constants::encode_canonical_exact_backing,
};

#[cfg(feature = "semantic-compiler")]
use super::BytecodeCompilerContext;

#[cfg(feature = "semantic-compiler")]
pub trait CompileConst: FunctionRuntimeType {
    fn compile_const(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<u32>;
}

#[cfg(feature = "semantic-compiler")]
impl<T> CompileConst for T
where
    T: CanonicalCellBacking + FunctionRuntimeType,
{
    fn compile_const(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<u32> {
        let value = ValueCell::from_exact(self.clone())?.snapshot()?;
        context.intern_constant(encode_canonical_exact_backing(&value, T::REPRESENTATION)?)
    }
}
