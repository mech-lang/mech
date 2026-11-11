//! The original total scalar CFG, outlined inside one counted buffer traversal.
use crate::native_artifact::codegen::common::{
    ssa_words::Words, terminators::emit_source_terminator,
};
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::{backend, primitive};
use crate::native_artifact::program::prepare::word_collections::Region;
use crate::native_artifact::{NativeCompileError, abi::STATUS_OK};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMatchAction as Scalar};
use std::collections::{BTreeMap, BTreeSet};
mod saturating;
mod vector;

pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    code: CodePlan<'_>,
    region: &Region,
    options: crate::native_artifact::codegen::options::OptimizationOptions,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let mut signature = module.make_signature();
    // Immutable scalar homes, immutable source, private sink, count.
    signature.params = vec![AbiParam::new(types::I64); 4];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(
            &format!("mech_word_collection_{}", region.collection),
            Linkage::Local,
            &signature,
        )
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let header = b.create_block();
        let done = b.create_block();
        let failed = b.create_block();
        b.append_block_param(header, types::I64);
        b.append_block_param(failed, types::I64);
        b.append_block_params_for_function_params(entry);
        b.set_cold_block(failed);
        b.switch_to_block(entry);
        let arguments = b.block_params(entry).to_vec();
        let (locals, source, sink, count) =
            (arguments[0], arguments[1], arguments[2], arguments[3]);
        let words = Words::new_with(
            &mut b,
            locals,
            code,
            region
                .words
                .iter()
                .map(|&v| v as usize)
                .collect::<BTreeSet<_>>(),
            |b, slot| {
                Ok(if region.captures.binary_search(&(slot as u32)).is_ok() {
                    None
                } else {
                    Some(b.ins().iconst(types::I64, 0))
                })
            },
        )?;
        let blocks = region
            .sites
            .iter()
            .map(|&site| (site, b.create_block()))
            .collect::<BTreeMap<_, _>>();
        let start = if let Some(threshold) = saturating::select(code, region) {
            vector::emit(
                &mut b,
                &words,
                source,
                sink,
                count,
                region.input_width,
                threshold,
            )
        } else {
            b.ins().iconst(types::I64, 0)
        };
        b.ins().jump(header, &[start.into()]);
        b.switch_to_block(header);
        let index = b.block_params(header)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
        b.ins().brif(more, blocks[&region.body], &[], done, &[]);
        for &site in &region.sites {
            b.switch_to_block(blocks[&site]);
            let block = &code.blocks()[site as usize];
            match block.action() {
                Action::CollectionElement { .. } => {
                    let displacement = b.ins().imul_imm(index, i64::from(region.input_width));
                    let address = b.ins().iadd(source, displacement);
                    let ty = memory_type(region.input_width);
                    let word = b.ins().load(ty, MemFlags::new(), address, 0);
                    let word = if ty == types::I64 {
                        word
                    } else {
                        b.ins().uextend(types::I64, word)
                    };
                    words.write(&mut b, region.element as usize, word);
                }
                Action::CollectionPush { .. } => {
                    let displacement = b.ins().imul_imm(index, i64::from(region.output_width));
                    let address = b.ins().iadd(sink, displacement);
                    let word = words.read(&mut b, region.yielded as usize);
                    let ty = memory_type(region.output_width);
                    let word = if ty == types::I64 {
                        word
                    } else {
                        b.ins().ireduce(ty, word)
                    };
                    b.ins().store(MemFlags::new(), word, address, 0);
                    let next = b.ins().iadd_imm(index, 1);
                    b.ins().jump(header, &[next.into()]);
                    continue;
                }
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let word = words.read(&mut b, source.get() as usize);
                    words.write(&mut b, output.get() as usize, word);
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => {
                    let inputs = operands
                        .iter()
                        .map(|v| words.read(&mut b, v.get() as usize))
                        .collect::<Vec<_>>();
                    // Keep the shared checked instruction sequence. Range
                    // reconstruction proves this valid typed loop is total.
                    let word = primitive(&mut b, *op, &inputs, failed);
                    words.write(&mut b, output.get() as usize, word);
                }
                _ => return Err(backend("word collection escaped its scalar proof")),
            }
            emit_source_terminator(
                &mut b,
                &words,
                code,
                site as usize,
                options,
                block.terminator(),
                failed,
                |target| blocks[&target],
            );
        }
        b.switch_to_block(done);
        let status = b.ins().iconst(types::I64, STATUS_OK as i64);
        b.ins().return_(&[status]);
        b.switch_to_block(failed);
        let status = b.block_params(failed)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("word collection code missing"))?;
    let stack = crate::native_artifact::compiler::compiled_stack_bytes(&compiled.buffer)?;
    Ok((function, stack, compiled.code_info().total_size as u64))
}
#[cfg(test)]
pub(in crate::native_artifact) fn vector_selected(code: CodePlan<'_>, region: &Region) -> bool {
    saturating::select(code, region).is_some()
}
fn memory_type(width: u8) -> cranelift_codegen::ir::Type {
    match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => unreachable!("derived unsigned width"),
    }
}
