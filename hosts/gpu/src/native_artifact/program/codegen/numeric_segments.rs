//! SSA native parent pieces around the original managed/child boundaries.
use super::{hooks::Hooks, numeric_words};
use crate::native_artifact::codegen::{
    common::{ssa_words::Words, terminators},
    control::{CodegenReport, plan::CodePlan},
    options::{CompilationControls, OptimizationPass},
};
use crate::native_artifact::compiler::{backend, offset, primitive, set_origin};
use crate::native_artifact::program::prepare::{
    composition::numeric::Layout, readonly_numeric::Operation,
};
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::{NativeCompileError, abi::STATUS_ABI};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMatchAction as Scalar};
use std::collections::{BTreeMap, BTreeSet};

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub iteration: u32,
    pub entry: usize,
    pub source_sites: Vec<u32>,
    pub frontiers: Vec<(u32, u32)>,
    pub register_words: usize,
    pub typed_loads: Vec<(usize, u8)>,
    pub successful_interior_helpers: usize,
    pub total_ir_instructions: usize,
}

fn end(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    locals: Value,
    shadow: usize,
    proof: &Layout,
    hooks: &Hooks,
    site: usize,
    mode: u64,
    next: u64,
    frame: Value,
    error: Block,
) -> Result<(), NativeCompileError> {
    words.spill_all(b, locals)?;
    set_origin(b, frame, site as u64);
    hooks.require_argument_words(b, proof.words, error);
    for word in 0..proof.words {
        let value = words.read(b, shadow + word);
        b.ins()
            .store(MemFlags::new(), value, hooks.arguments, offset(word)?);
    }
    let args = [mode, next, 0].map(|v| b.ins().iconst(types::I64, v as i64));
    hooks.call(b, site, GlobalHelper::NumericSegmentEnd, args, error);
    Ok(())
}

pub(in crate::native_artifact) fn emit_entry(
    controls: &CompilationControls,
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    code: CodePlan<'_>,
    site: usize,
    hooks: &Hooks,
    ordinary_blocks: &[Option<Block>],
    error: Block,
    reports: &mut CodegenReport,
) -> Result<usize, NativeCompileError> {
    if !controls.enabled(OptimizationPass::ScalarLoops) {
        return Ok(0);
    }
    let CodePlan::Program(program) = code else {
        return Ok(0);
    };
    let metadata = program.metadata();
    let Some(id) = metadata.compositions.entry(site) else {
        return Ok(0);
    };
    let (parent, segment) = metadata
        .compositions
        .region(id)
        .ok_or_else(|| backend("parent emission lost its selected segment"))?;
    let proof = segment
        .numeric
        .as_ref()
        .ok_or_else(|| backend("parent emission lost numeric proof"))?;
    let frames = segment
        .frame_plan
        .as_ref()
        .ok_or_else(|| backend("parent emission lost frame proof"))?;
    let base = metadata.local_words().map_err(backend)? - metadata.compositions.words;
    let shadow = base + 1;
    let before = b.func.dfg.num_insts();
    hooks.require_argument_words(b, proof.words, error);
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, GlobalHelper::NumericSegmentBegin, [zero; 3], error);
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let selected = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(ready, selected, &[], ordinary, &[]);
    // The fixed entry/decline guard belongs to the ordinary source visit.
    // Only the selected body is excluded from that visit's instruction audit.
    let entry_ir = b.func.dfg.num_insts() - before;
    b.switch_to_block(selected);
    for word in 0..proof.words {
        let value = b
            .ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, offset(word)?);
        b.ins()
            .store(MemFlags::new(), value, locals, offset(shadow + word)?);
    }
    let slots = numeric_words::source_slots(
        code,
        segment.sites.iter().map(|&s| s as usize),
        &proof.actions,
        shadow..shadow + proof.words,
    )?;
    let register_words = slots.len();
    let words = Words::new(b, locals, code, slots)?;
    let blocks = segment
        .sites
        .iter()
        .map(|&s| (s, b.create_block()))
        .collect::<BTreeMap<_, _>>();
    let exits = segment
        .exits
        .iter()
        .map(|&edge| (edge, b.create_block()))
        .collect::<BTreeMap<_, _>>();
    let mut boundaries = exits.values().copied().collect::<BTreeSet<_>>();
    b.ins().jump(blocks[&(site as u32)], &[]);
    let mut typed_loads = Vec::new();
    for &at in &segment.sites {
        let source = at as usize;
        b.switch_to_block(blocks[&at]);
        let failure = b.create_block();
        b.append_block_param(failure, types::I64);
        b.set_cold_block(failure);
        boundaries.insert(failure);
        match proof
            .actions
            .iter()
            .find(|(s, _)| *s == source)
            .map(|(_, a)| a)
        {
            Some(Operation::Frame) => {
                numeric_words::frame_words(b, &words, shadow, code, source, &frames.frames)?
            }
            Some(Operation::Length { array, output }) => {
                let value = words.read(b, shadow + frames.scalar_words + array * 4 + 1);
                words.write(b, *output as usize, value);
            }
            Some(Operation::Read {
                array,
                index,
                output,
            }) => {
                let view = shadow + frames.scalar_words + array * 4;
                let index = words.read(b, *index as usize);
                let ordinal = b.ins().iadd_imm(index, -1);
                let length = words.read(b, view + 1);
                let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, length);
                let good = b.create_block();
                let bad = b.create_block();
                b.set_cold_block(bad);
                boundaries.insert(bad);
                b.ins().brif(valid, good, &[], bad, &[]);
                b.switch_to_block(bad);
                end(
                    b, &words, locals, shadow, proof, hooks, source, 2, 0, frame, error,
                )?;
                let Action::LibraryCall { operands, .. } = code.blocks()[source].action() else {
                    return Err(backend("parent bounds exit lost its original call"));
                };
                hooks.library(
                    b,
                    source,
                    operands,
                    locals,
                    |v| code.kind(v).is_some(),
                    error,
                )?;
                let status = b.ins().iconst(types::I64, STATUS_ABI as i64);
                b.ins().jump(error, &[status.into()]);
                b.switch_to_block(good);
                let rows = words.read(b, view + 2);
                let columns = words.read(b, view + 3);
                let width = proof.views[*array].width;
                let Action::LibraryCall { template, .. } =
                    code.blocks()[proof.views[*array].witness].action()
                else {
                    return Err(backend("parent address lost its entry witness"));
                };
                let (address, ty) = numeric_words::linear_address(
                    b,
                    ordinal,
                    rows,
                    columns,
                    width,
                    template.inputs()[0].body(),
                    |b| words.read(b, view),
                )?;
                let value = b.ins().load(ty, MemFlags::new(), address, 0);
                let value = if width < 8 {
                    b.ins().uextend(types::I64, value)
                } else {
                    value
                };
                words.write(b, *output as usize, value);
                typed_loads.push((source, width));
            }
            None => match code.blocks()[source].action() {
                Action::Scalar(Scalar::None) => {}
                Action::Scalar(Scalar::Copy { source, output }) => {
                    let value = words.read(b, source.get() as usize);
                    words.write(b, output.get() as usize, value);
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                    ..
                }) => {
                    let args = operands
                        .iter()
                        .map(|v| words.read(b, v.get() as usize))
                        .collect::<Vec<_>>();
                    let value = primitive(b, *op, &args, failure);
                    words.write(b, output.get() as usize, value);
                }
                _ => return Err(backend("parent piece contains an unproved action")),
            },
            _ => return Err(backend("parent piece has a writable numeric operation")),
        }
        if let Some(flag) = frames.retirements.iter().position(|(s, _)| *s == at) {
            let one = b.ins().iconst(types::I64, 1);
            words.write(
                b,
                shadow + frames.scalar_words + proof.views.len() * 4 + flag,
                one,
            );
        }
        terminators::emit_source_terminator(
            b,
            &words,
            code,
            code.blocks()[source].terminator(),
            failure,
            |next| {
                blocks
                    .get(&next)
                    .copied()
                    .unwrap_or_else(|| exits[&(at, next)])
            },
        );
        b.switch_to_block(failure);
        let status = b.block_params(failure)[0];
        end(
            b, &words, locals, shadow, proof, hooks, source, 3, 0, frame, error,
        )?;
        b.ins().jump(error, &[status.into()]);
    }
    for (&(from, to), &block) in &exits {
        b.switch_to_block(block);
        end(
            b,
            &words,
            locals,
            shadow,
            proof,
            hooks,
            from as usize,
            0,
            u64::from(to),
            frame,
            error,
        )?;
        b.ins().jump(
            ordinary_blocks[to as usize]
                .ok_or_else(|| backend("parent continuation is not emitted"))?,
            &[],
        );
    }
    let cfg = cranelift_codegen::flowgraph::ControlFlowGraph::with_function(b.func);
    let mut seen = BTreeSet::new();
    let mut pending = vec![blocks[&(site as u32)]];
    let mut calls = 0;
    while let Some(block) = pending.pop() {
        if boundaries.contains(&block) || !seen.insert(block) {
            continue;
        }
        calls += b
            .func
            .layout
            .block_insts(block)
            .filter(|i| b.func.dfg.insts[*i].opcode().is_call())
            .count();
        pending.extend(cfg.succ_iter(block));
    }
    if calls != 0 {
        return Err(backend("parent native interior contains a helper"));
    }
    let total_ir_instructions = b.func.dfg.num_insts() - before;
    if total_ir_instructions as u64 > proof.compute {
        return Err(backend(
            "parent native instructions exceed their independently derived price",
        ));
    }
    reports.numeric_segments.push(Report {
        iteration: parent.iteration,
        entry: site,
        source_sites: segment.sites.clone(),
        frontiers: segment.exits.clone(),
        register_words,
        typed_loads,
        successful_interior_helpers: calls,
        total_ir_instructions,
    });
    b.switch_to_block(ordinary);
    Ok(total_ir_instructions - entry_ir)
}
