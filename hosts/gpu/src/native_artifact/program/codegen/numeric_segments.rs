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
    pub source_home_words: usize,
    pub private_scratch_words: usize,
    pub typed_loads: Vec<(usize, u8)>,
    pub exit_argument_words: Vec<usize>,
    pub exit_argument_stores: usize,
    pub exit_boundaries: usize,
    pub successful_interior_helpers: usize,
    pub scalar_input_loads: usize,
    pub scalar_output_stores: usize,
    pub runtime_scalar_path_walks: usize,
    pub entry_callbacks: usize,
    pub total_ir_instructions: usize,
}

fn end(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    source_homes: &BTreeSet<usize>,
    locals: Value,
    shadow: usize,
    proof: &Layout,
    frames: &crate::native_artifact::program::prepare::composition::frames::Layout,
    exit_words: &[usize],
    hooks: &Hooks,
    site: usize,
    mode: u64,
    next: u64,
    frame: Value,
    error: Block,
) -> Result<usize, NativeCompileError> {
    // The runtime exit consumes only the argument exports below. Private SSA
    // scratch has no ordinary continuation reader; source homes still include
    // every original operand needed by a cold checked-read/arithmetic exit.
    words.spill_live(b, locals, source_homes)?;
    set_origin(b, frame, site as u64);
    hooks.require_argument_words(b, proof.words, error);
    let mut stores = 0;
    for &word in exit_words {
        let value = words.read(b, shadow + word);
        b.ins()
            .store(MemFlags::new(), value, hooks.arguments, offset(word)?);
        stores += 1;
    }
    let args = [mode, next, 0].map(|v| b.ins().iconst(types::I64, v as i64));
    hooks.call(b, site, GlobalHelper::NumericSegmentEnd, args, error);
    if mode == 0 {
        // Reborrow only after the helper's final Rust lane mutation has ended.
        // The entry pointer is never reused across that mutable boundary.
        let scalar_bank = if frames.scalar_bindings.is_empty() {
            b.ins().iconst(types::I64, 0)
        } else {
            b.ins()
                .load(types::I64, MemFlags::new(), hooks.arguments, 0)
        };
        let exit = frames
            .exits
            .iter()
            .find(|e| e.from as usize == site && u64::from(e.to) == next)
            .ok_or_else(|| backend("compiled scalar export lost its exact frontier"))?;
        // The checked end has completed original retirement and managed-owner
        // synchronization. No ordinary callback can observe the frame until
        // these already funded native stores publish its scalar leaves.
        let ready = b.ins().iconst(types::I64, 1);
        for binding in &frames.scalar_bindings {
            if exit.frames.iter().any(|value| value.get() == binding.value) {
                let value = words.read(b, shadow + binding.word);
                let payload = binding
                    .physical
                    .checked_mul(2)
                    .ok_or_else(|| backend("scalar export offset overflows"))?;
                b.ins()
                    .store(MemFlags::new(), value, scalar_bank, offset(payload)?);
                b.ins()
                    .store(MemFlags::new(), ready, scalar_bank, offset(payload + 1)?);
            }
        }
    }
    Ok(stores)
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
    fused_admission: bool,
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
    let exit_words = proof.exit_argument_words(frames).map_err(backend)?;
    let base = metadata.local_words().map_err(backend)? - metadata.compositions.words;
    let shadow = base + 1;
    let before = b.func.dfg.num_insts();
    // The source visit has just published whether its ordinary scalar prefix
    // was funded. NumericSegmentBegin uses the same result word for selection;
    // preserve the prefix decision if its larger piece declines.
    let scalar_group = metadata
        .map_admission
        .group(site)
        .is_some_and(|group| group.owner.is_none() && group.start() as usize == site);
    let prior_scalar_ready = (scalar_group && !fused_admission)
        .then(|| b.ins().load(types::I64, MemFlags::new(), hooks.result, 0));
    hooks.require_argument_words(b, proof.words, error);
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(
        b,
        site,
        if fused_admission {
            GlobalHelper::AdmitNumericSegmentBegin
        } else {
            GlobalHelper::NumericSegmentBegin
        },
        [zero; 3],
        error,
    );
    let result = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let scalar_ready = if fused_admission && scalar_group {
        Some(b.ins().band_imm(result, 1))
    } else {
        prior_scalar_ready
    };
    let ready = if fused_admission {
        b.ins().ushr_imm(result, 1)
    } else {
        result
    };
    let selected = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(ready, selected, &[], ordinary, &[]);
    // The fixed entry/decline guard belongs to the ordinary source visit.
    // Only the selected body is excluded from that visit's instruction audit.
    let entry_ir = b.func.dfg.num_insts() - before;
    b.switch_to_block(selected);
    let scalar_bank = if frames.scalar_bindings.is_empty() {
        b.ins().iconst(types::I64, 0)
    } else {
        b.ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, 0)
    };
    let source_homes = numeric_words::source_slots(
        code,
        segment.sites.iter().map(|&s| s as usize),
        &proof.actions,
        core::iter::empty(),
    )?;
    let scratch = shadow..shadow + proof.words;
    if source_homes.iter().any(|slot| scratch.contains(slot)) {
        return Err(backend("parent private scratch overlaps source homes"));
    }
    let slots: BTreeSet<_> = source_homes
        .iter()
        .copied()
        .chain(scratch.clone())
        .collect();
    let register_words = slots.len();
    let words = Words::new_with(b, locals, code, slots, |b, slot| {
        if scratch.contains(&slot) {
            if let Some(binding) = frames.scalar_bindings.iter().find(|binding| {
                shadow + binding.word == slot && frames.inputs.contains(&binding.value)
            }) {
                let payload = binding
                    .physical
                    .checked_mul(2)
                    .ok_or_else(|| backend("scalar import offset overflows"))?;
                return Ok(Some(b.ins().load(
                    types::I64,
                    MemFlags::new(),
                    scalar_bank,
                    offset(payload)?,
                )));
            }
            let relative = slot - shadow;
            let views_end = frames.scalar_words + proof.views.len() * 4;
            if relative >= frames.scalar_words && relative < views_end {
                Ok(Some(b.ins().load(
                    types::I64,
                    MemFlags::new(),
                    hooks.arguments,
                    offset(relative)?,
                )))
            } else {
                // Noninput virtual scalar lanes and reached-retirement flags
                // have no host producer. Initialize their SSA definitions here;
                // only witnessed views cross the argument bank at entry.
                Ok(Some(b.ins().iconst(types::I64, 0)))
            }
        } else {
            Ok(None)
        }
    })?;
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
    let mut exit_argument_stores = 0;
    let mut exit_boundaries = 0;
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
                exit_argument_stores += end(
                    b,
                    &words,
                    &source_homes,
                    locals,
                    shadow,
                    proof,
                    frames,
                    &exit_words,
                    hooks,
                    source,
                    2,
                    0,
                    frame,
                    error,
                )?;
                exit_boundaries += 1;
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
        exit_argument_stores += end(
            b,
            &words,
            &source_homes,
            locals,
            shadow,
            proof,
            frames,
            &exit_words,
            hooks,
            source,
            3,
            0,
            frame,
            error,
        )?;
        exit_boundaries += 1;
        b.ins().jump(error, &[status.into()]);
    }
    for (&(from, to), &block) in &exits {
        b.switch_to_block(block);
        exit_argument_stores += end(
            b,
            &words,
            &source_homes,
            locals,
            shadow,
            proof,
            frames,
            &exit_words,
            hooks,
            from as usize,
            0,
            u64::from(to),
            frame,
            error,
        )?;
        exit_boundaries += 1;
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
    let before_decline = b.func.dfg.num_insts();
    b.switch_to_block(ordinary);
    if let Some(ready) = scalar_ready {
        b.ins().store(MemFlags::new(), ready, hooks.result, 0);
    }
    let decline_ir = b.func.dfg.num_insts() - before_decline;
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
        source_home_words: source_homes.len(),
        private_scratch_words: proof.words,
        typed_loads,
        exit_argument_words: exit_words,
        exit_argument_stores,
        exit_boundaries,
        successful_interior_helpers: calls,
        scalar_input_loads: frames
            .scalar_bindings
            .iter()
            .filter(|b| frames.inputs.contains(&b.value))
            .count(),
        scalar_output_stores: frames
            .exits
            .iter()
            .map(|exit| {
                frames
                    .scalar_bindings
                    .iter()
                    .filter(|binding| exit.frames.iter().any(|value| value.get() == binding.value))
                    .count()
                    * 2
            })
            .sum(),
        runtime_scalar_path_walks: 0,
        entry_callbacks: if fused_admission { 1 } else { 2 },
        total_ir_instructions,
    });
    Ok(total_ir_instructions - entry_ir - decline_ir)
}
