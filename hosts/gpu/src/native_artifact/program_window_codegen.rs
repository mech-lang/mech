//! Whole-loop machine code selected by the canonical window/histogram proof.
//! No source frontend, evaluator, allocation or admission callback in this body.
//! The canonical caller retains genuine source/query or scratch owners, checks representation and
//! admits finite region memory/work within the containing round before entry.
use super::{NativeCompileError, backend};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::NativeWindowRegion;

#[path = "program_window_histogram_codegen.rs"]
mod histogram;

// (source pointer, source bytes, query pointer, query bytes, width) -> count.
// Caller must prove that source is ASCII without CRLF grapheme pairs. Then
// byte indices are EXACT ordinary String/grapheme indices, including NUL.
pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    region: &NativeWindowRegion,
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
        if region.query().is_none() {
            histogram::emit(&mut b);
        } else {
            let entry = b.create_block();
            let empty = b.create_block();
            let outer = b.create_block();
            let compare = b.create_block();
            let equal = b.create_block();
            let matched = b.create_block();
            let advance = b.create_block();
            let done = b.create_block();
            b.append_block_params_for_function_params(entry);
            for _ in 0..2 {
                b.append_block_param(outer, types::I64);
            }
            b.append_block_param(compare, types::I64);
            b.append_block_param(advance, types::I64);
            b.append_block_param(done, types::I64);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let (source, length, query, query_length, width) =
                (args[0], args[1], args[2], args[3], args[4]);
            let zero = b.ins().iconst(types::I64, 0);
            let positive = b.ins().icmp_imm(IntCC::NotEqual, width, 0);
            let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, width, length);
            let same_width = b.ins().icmp(IntCC::Equal, width, query_length);
            let valid = b.ins().band(positive, fits);
            let valid = b.ins().band(valid, same_width);
            b.ins()
                .brif(valid, outer, &[zero.into(), zero.into()], empty, &[]);
            b.switch_to_block(empty);
            b.ins().return_(&[zero]);
            b.switch_to_block(outer);
            let offset = b.block_params(outer)[0];
            let count = b.block_params(outer)[1];
            b.ins().jump(compare, &[zero.into()]);
            b.switch_to_block(compare);
            let index = b.block_params(compare)[0];
            let position = b.ins().iadd(offset, index);
            let source_address = b.ins().iadd(source, position);
            let query_address = b.ins().iadd(query, index);
            // Entry + loop bounds prove position < length and index < width ==
            // query_length. Owned String loans supplied by the caller keep every
            // byte live; this compiled body neither allocates nor publishes.
            let left = b.ins().load(types::I8, MemFlags::new(), source_address, 0);
            let right = b.ins().load(types::I8, MemFlags::new(), query_address, 0);
            let same = b.ins().icmp(IntCC::Equal, left, right);
            b.ins().brif(same, equal, &[], advance, &[count.into()]);
            b.switch_to_block(equal);
            let next = b.ins().iadd_imm(index, 1);
            let more = b.ins().icmp(IntCC::UnsignedLessThan, next, width);
            b.ins().brif(more, compare, &[next.into()], matched, &[]);
            b.switch_to_block(matched);
            let incremented = b.ins().iadd_imm(count, 1);
            b.ins().jump(advance, &[incremented.into()]);
            b.switch_to_block(advance);
            let next_count = b.block_params(advance)[0];
            let last = b.ins().isub(length, width);
            let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, last);
            let next_offset = b.ins().iadd_imm(offset, 1);
            b.ins().brif(
                more,
                outer,
                &[next_offset.into(), next_count.into()],
                done,
                &[next_count.into()],
            );
            b.switch_to_block(done);
            let answer = b.block_params(done)[0];
            b.ins().return_(&[answer]);
        }
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
