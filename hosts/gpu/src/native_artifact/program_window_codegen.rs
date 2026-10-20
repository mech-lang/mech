//! Whole-loop machine code selected by the canonical window/histogram proof.
//! No source frontend, evaluator, allocation or admission callback in this body.
//! The canonical caller retains genuine source/query or scratch owners, checks representation and
//! admits finite region memory/work within the containing round before entry.
use super::{NativeCompileError, backend};
use cranelift_codegen::ir::{AbiParam, Endianness, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeWindowRegion;

#[path = "program_window_histogram_codegen.rs"]
mod histogram;

#[path = "program_window_query_codegen.rs"]
mod query;

// (source pointer, source bytes, query pointer, query bytes, width) -> count.
// Caller must prove that source is ASCII without CRLF grapheme pairs. Then
// byte indices are EXACT ordinary String/grapheme indices, including NUL.
pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    region: &NativeWindowRegion,
) -> Result<(FuncId, String, u64, u64), NativeCompileError> {
    let vectorized =
        module.isa().name() == "aarch64" && module.isa().endianness() == Endianness::Little;
    define_impl(module, region, vectorized)
}

fn define_impl<M: Module>(
    module: &mut M,
    region: &NativeWindowRegion,
    vectorized: bool,
) -> Result<(FuncId, String, u64, u64), NativeCompileError> {
    define_with(module, region, |b| {
        if region.query().is_none() {
            histogram::emit(b);
        } else {
            query::emit(b, vectorized);
        }
    })
}

fn define_with<M: Module>(
    module: &mut M,
    region: &NativeWindowRegion,
    emit: impl FnOnce(&mut FunctionBuilder<'_>),
) -> Result<(FuncId, String, u64, u64), NativeCompileError> {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns.push(AbiParam::new(types::I64));
    let name = format!("mech_window_region_{}", region.collection());
    let function = module
        .declare_function(&name, Linkage::Local, &signature)
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        emit(&mut b);
        b.seal_all_blocks();
        b.finalize();
    }
    let ir = context.func.display().to_string();
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("compiled window code missing"))?;
    let stack = super::super::compiler::compiled_stack_bytes(&compiled.buffer)?;
    Ok((function, ir, stack, compiled.code_info().total_size as u64))
}

#[cfg(all(test, feature = "canonical-native-program-host"))]
#[path = "program_window_codegen_tests.rs"]
mod tests;
