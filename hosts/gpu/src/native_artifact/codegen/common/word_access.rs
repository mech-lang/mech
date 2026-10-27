//! Compile-time storage adapters; no guest dispatch, vtable or allocation.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::offset;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;

#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct LocalWord(pub usize);

pub(in crate::native_artifact) trait WordAccess {
    fn read(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        word: LocalWord,
    ) -> Result<Value, NativeCompileError>;
    fn write(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        word: LocalWord,
        value: Value,
    ) -> Result<(), NativeCompileError>;
}
pub(in crate::native_artifact) struct MemoryWords(pub Value);
impl WordAccess for MemoryWords {
    fn read(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        word: LocalWord,
    ) -> Result<Value, NativeCompileError> {
        Ok(b.ins()
            .load(types::I64, MemFlags::new(), self.0, offset(word.0)?))
    }
    fn write(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        word: LocalWord,
        value: Value,
    ) -> Result<(), NativeCompileError> {
        b.ins()
            .store(MemFlags::new(), value, self.0, offset(word.0)?);
        Ok(())
    }
}
pub(in crate::native_artifact) struct SsaWords<'a>(
    pub &'a crate::native_artifact::codegen::common::ssa_words::Words,
);
impl WordAccess for SsaWords<'_> {
    fn read(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        word: LocalWord,
    ) -> Result<Value, NativeCompileError> {
        Ok(self.0.read(b, word.0))
    }
    fn write(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        word: LocalWord,
        value: Value,
    ) -> Result<(), NativeCompileError> {
        self.0.write(b, word.0, value);
        Ok(())
    }
}
