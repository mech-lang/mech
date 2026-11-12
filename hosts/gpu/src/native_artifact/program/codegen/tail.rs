//! Native finite fuel for closed scalar/map Tail bodies, inside the same round.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::options::{OptimizationOptions, OptimizationPass};
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::codegen::hooks::Hooks;
use crate::native_artifact::program::prepare::tail::{Layout, Region};
#[cfg(feature = "canonical-native-program-runtime")]
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::protocol::{OwnerOperation, abi as program_abi};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, InstBuilder, InstructionData, MemFlags, Opcode, Value, ValueDef, types,
};
use cranelift_frontend::FunctionBuilder;

pub(in crate::native_artifact) fn flags(options: OptimizationOptions, region: &Region) -> MemFlags {
    if region.map.is_none() {
        crate::native_artifact::codegen::common::memory_regions::scalar_tail(options)
    } else {
        MemFlags::new()
    }
}

fn load(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    word: usize,
) -> Result<Value, NativeCompileError> {
    Ok(b.ins()
        .load(types::I64, flags(options, region), locals, offset(word)?))
}
fn store(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    word: usize,
    value: Value,
) -> Result<(), NativeCompileError> {
    b.ins()
        .store(flags(options, region), value, locals, offset(word)?);
    Ok(())
}
pub(in crate::native_artifact) fn leave(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    site: Value,
    mode: i64,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    leave_at(options, b, locals, region, site, mode, None, hooks, error)
}

/// Exact relative argument-bank inputs consumed by numeric synchronization.
/// Preserve store order and duplicates: this also describes existing success
/// and ordinary-continuation transport, which is not narrowed here.
pub(in crate::native_artifact) fn numeric_exit_words(
    region: &Region,
    mode: i64,
    continuation: Option<usize>,
) -> Result<Vec<usize>, NativeCompileError> {
    let proof = region
        .numeric
        .as_ref()
        .ok_or_else(|| backend("numeric exit inventory lacks its proof"))?;
    let terminal = proof
        .result
        .frame()
        .filter(|_| mode == 0)
        .and_then(|value| proof.frame(value));
    let mut words = proof
        .frame(region.tail.carrier().get())
        .into_iter()
        .flat_map(|frame| frame.scalar_slots())
        .chain(terminal.into_iter().flat_map(|frame| frame.scalar_slots()))
        .filter(|_| region.numeric_tree.is_none() || (mode != 0 && mode != 3))
        .collect::<Vec<_>>();
    if matches!(mode, 6 | 7) {
        for value in proof
            .exits
            .iter()
            .filter(|exit| continuation.is_none_or(|site| exit.to == site))
            .flat_map(|exit| &exit.frames)
        {
            words.extend(
                proof
                    .fields(*value)
                    .ok_or_else(|| backend("cold frame lacks scalar lanes"))?
                    .iter()
                    .flatten()
                    .copied(),
            );
        }
    }
    if let Some(phase) = &proof.phase {
        words.extend(phase.loops.iter().map(|debit| debit.counter));
    }
    for (array, _) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
        words.push(proof.view(array) + 1);
    }
    Ok(words)
}

pub(in crate::native_artifact) fn leave_at(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    site: Value,
    mode: i64,
    continuation: Option<usize>,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let ready = load(options, b, locals, region, region.words)?;
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    let finish = b.create_block();
    let done = b.create_block();
    b.ins().brif(ready, finish, &[], done, &[]);
    b.switch_to_block(finish);
    let compute = load(options, b, locals, region, region.words + 1)?;
    let comparison = load(options, b, locals, region, region.words + 2)?;
    let zero = b.ins().iconst(types::I64, 0);
    store(options, b, locals, region, region.words, zero)?;
    if let Some(proof) = &region.numeric {
        let shadow = region
            .shadow
            .ok_or_else(|| backend("numeric exit lacks scratch home"))?;
        hooks.require_argument_words(b, proof.words + 2, error);
        for slot in numeric_exit_words(region, mode, continuation)? {
            let value = load(options, b, locals, region, shadow + slot)?;
            b.ins()
                .store(MemFlags::new(), value, hooks.arguments, offset(2 + slot)?);
        }
        if let Some(tree) = &region.numeric_tree {
            if mode == 1 && (!proof.cold_scalars.is_empty() || !proof.cold_frames.is_empty()) {
                // Header bailouts have a literal same-plan source site. Read
                // only that header's live frame words; other virtual frames
                // may not yet have a current value in this activation.
                let value = b.func.dfg.resolve_aliases(site);
                let ValueDef::Result(inst, 0) = b.func.dfg.value_def(value) else {
                    return Err(backend("numeric cold scalar exit has a dynamic header"));
                };
                let InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                    ..
                } = b.func.dfg.insts[inst]
                else {
                    return Err(backend("numeric cold scalar exit lacks a literal header"));
                };
                let header = usize::try_from(imm.bits())
                    .map_err(|_| backend("numeric cold scalar header is out of range"))?;
                for (_, bindings) in proof
                    .cold_scalars
                    .iter()
                    .filter(|(site, _)| *site == header)
                {
                    for binding in bindings {
                        let value = load(options, b, locals, region, shadow + binding.word)?;
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            hooks.arguments,
                            offset(2 + binding.word)?,
                        );
                    }
                }
            }
            // Every cold boundary carries the sealed generated ancestry and
            // exact debit projection. Inactive carrier words grant no authority;
            // runtime reifies only the checked active path.
            if mode != 3 {
                for node in &tree.nodes {
                    for binding in &node.scalar_bindings {
                        let value = load(options, b, locals, region, shadow + binding.word)?;
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            hooks.arguments,
                            offset(2 + binding.word)?,
                        );
                    }
                }
            }
            let settled = super::numeric_tree::settled_debits(
                b,
                tree,
                options.enabled(OptimizationPass::ProjectedNumericDebits),
                |b, slot| load(options, b, locals, region, shadow + slot),
            )?;
            for slot in tree.counters..tree.bank() {
                let value = match settled.get(&slot) {
                    Some(&value) => value,
                    None => load(options, b, locals, region, shadow + slot)?,
                };
                b.ins()
                    .store(MemFlags::new(), value, hooks.arguments, offset(2 + slot)?);
            }
        }
    }
    let normal_exit = mode == 0;
    let mode = b.ins().iconst(types::I64, mode);
    hooks.call_at(
        b,
        site,
        GlobalHelper::ClosedTailEnd,
        [compute, comparison, mode],
        error,
    );
    if normal_exit {
        if let Some(proof) = &region.numeric {
            if !proof.scalar_inputs.is_empty() || !proof.scalar_outputs.is_empty() {
                let bank = b
                    .ins()
                    .load(types::I64, MemFlags::new(), hooks.arguments, 16);
                let shadow = region
                    .shadow
                    .ok_or_else(|| backend("numeric export lacks scratch"))?;
                let ready = b.ins().iconst(types::I64, 1);
                for binding in proof.scalar_inputs.iter().chain(&proof.scalar_outputs) {
                    let value = load(options, b, locals, region, shadow + binding.word)?;
                    let at = binding
                        .physical
                        .checked_mul(2)
                        .ok_or_else(|| backend("numeric scalar export offset overflows"))?;
                    b.ins().store(MemFlags::new(), value, bank, offset(at)?);
                    b.ins().store(MemFlags::new(), ready, bank, offset(at + 1)?);
                }
            }
        }
    }
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    Ok(())
}
pub(in crate::native_artifact) fn failure(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    if let Some(region) = layout.sites.get(site).copied().flatten() {
        let site = b.ins().iconst(types::I64, site as i64);
        leave(
            options,
            b,
            locals,
            &layout.regions[region],
            site,
            2,
            hooks,
            error,
        )?;
    }
    Ok(())
}
pub(in crate::native_artifact) fn abort(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: Value,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    for region in &layout.regions {
        leave(options, b, locals, region, site, 3, hooks, error)?;
    }
    Ok(())
}
pub(in crate::native_artifact) fn prepare(
    controls: &crate::native_artifact::codegen::options::CompilationControls,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    fixed_keys: Option<&crate::native_artifact::program::prepare::fixed_key_loops::Layout>,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let options = controls.options();
    let Some(region) = layout.initializers.get(site).copied().flatten() else {
        return Ok(());
    };
    let region = &layout.regions[region];
    if region
        .map
        .is_some_and(|owner| fixed_keys.is_some_and(|keys| keys.owner(owner).is_some()))
        && !controls
            .enabled(crate::native_artifact::codegen::options::OptimizationPass::FixedKeyMapLoops)
    {
        return Ok(());
    }
    if region.numeric_subtree()
        && !controls
            .enabled(crate::native_artifact::codegen::options::OptimizationPass::NumericTrees)
    {
        return Ok(());
    }
    if region.captures.is_some() && !controls.scalar_capture_selected(region.tail.header() as usize)
    {
        return Ok(());
    }
    if region.numeric.is_some()
        && !options.enabled(crate::native_artifact::codegen::options::OptimizationPass::ScalarLoops)
    {
        return Ok(());
    }
    prepare_region(
        options,
        b,
        locals,
        region,
        site,
        program_abi::CLOSED_TAIL_BEGIN,
        hooks,
        error,
    )
}
pub(in crate::native_artifact) fn resume(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some(region) = layout.sites.get(site).copied().flatten() else {
        return Ok(());
    };
    prepare_region(
        options,
        b,
        locals,
        &layout.regions[region],
        site,
        program_abi::PREFIX_TAIL_RESUME,
        hooks,
        error,
    )
}
pub(in crate::native_artifact) fn pause(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    if let Some(region) = layout.sites.get(site).copied().flatten() {
        let site = b.ins().iconst(types::I64, site as i64);
        leave(
            options,
            b,
            locals,
            &layout.regions[region],
            site,
            4,
            hooks,
            error,
        )?;
        let zero = b.ins().iconst(types::I64, 0);
        // A declined loan may leave capture words in the unused fuel lane.
        for word in layout.regions[region].words..layout.regions[region].words + 3 {
            store(options, b, locals, &layout.regions[region], word, zero)?;
        }
    }
    Ok(())
}
fn prepare_region(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    site: usize,
    operation: u64,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    if let Some(packed) = region.packed.as_ref() {
        let mut offsets = [zero; 2];
        for (word, value) in offsets
            .iter_mut()
            .zip([packed.start_offset(), packed.end_offset()])
        {
            if let Some(value) = value {
                *word = b
                    .ins()
                    .load(types::I64, MemFlags::new(), locals, offset(value as usize)?);
            }
        }
        // The map initializer's output words have already been copied into
        // locals. Reuse words 0/1 as input, consumed before the entry callback
        // replaces words 0..4 with its private return values.
        hooks.write_entry_words(b, &offsets, error)?;
    }
    hooks.call(b, site, GlobalHelper::decode(operation), [zero; 3], error);
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    store(options, b, locals, region, region.words, ready)?;
    let inputs = if let Some(proof) = region
        .numeric
        .as_ref()
        .filter(|proof| proof.phase.is_some())
    {
        (0..2)
            .chain(
                proof
                    .entry_words(region.tail.carrier().get())
                    .map_err(backend)?
                    .into_iter()
                    .map(|word| word + 2),
            )
            .chain(2..2 + region.captures.as_ref().map_or(0, |p| p.words()))
            .collect::<Vec<_>>()
    } else {
        (0..if region.map.is_some() {
            4
        } else {
            2 + region.captures.as_ref().map_or(0, |p| p.words())
        })
            .collect()
    };
    for i in inputs {
        let word = b
            .ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, offset(i)?);
        let destination = if (region.numeric.is_some() || region.captures.is_some()) && i >= 2 {
            region
                .shadow
                .ok_or_else(|| backend("numeric entry lacks scratch home"))?
                + i
                - 2
        } else {
            region.words + 1 + i
        };
        store(options, b, locals, region, destination, word)?;
    }
    if let Some(word) = region.cold_resume() {
        store(options, b, locals, region, word, zero)?;
    }
    Ok(())
}

/// A cold ordinary suffix may re-enter only at the original next header. The
/// flag is per invocation and cleared even when the fresh same-account view
/// preview declines; an ordinary loop never repeatedly attempts a refused loan.
pub(in crate::native_artifact) fn resume_cold(
    options: OptimizationOptions,
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    region: &Region,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some(word) = region.cold_resume() else {
        return Ok(());
    };
    let pending = load(options, b, locals, region, word)?;
    let resume = b.create_block();
    let ready = b.create_block();
    b.set_cold_block(resume);
    b.ins().brif(pending, resume, &[], ready, &[]);
    b.switch_to_block(resume);
    prepare_region(
        options,
        b,
        locals,
        region,
        site,
        program_abi::CLOSED_TAIL_BEGIN,
        hooks,
        error,
    )?;
    b.ins().jump(ready, &[]);
    b.switch_to_block(ready);
    Ok(())
}
/// Emit native admission or leave in the ordinary callback arm; the caller
/// joins that callback at the returned block. Closed map bodies call only their
/// private storage binding; scalar bodies have no callbacks.
pub(in crate::native_artifact) fn admission(
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<Option<Block>, NativeCompileError> {
    if let Some(region) = layout.exits.get(site).copied().flatten() {
        let site = b.ins().iconst(types::I64, site as i64);
        leave(
            options,
            b,
            locals,
            &layout.regions[region],
            site,
            0,
            hooks,
            error,
        )?;
    }
    let Some(region) = layout.sites.get(site).copied().flatten() else {
        return Ok(None);
    };
    let region = &layout.regions[region];
    if region.numeric_tree.is_some() {
        // The complete tree has its own generated CFG. Its successful return,
        // finite fallback and error/first-write boundaries settle the same
        // loan before any ordinary source block can execute. An ordinary join
        // here would only suppress the independently proved smaller-piece
        // entry fusion, including when the complete tree is ablated/declined.
        // Runtime still rejects every ordinary callback with a live tree loan.
        return Ok(None);
    }
    let ready = load(options, b, locals, region, region.words)?;
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(ready, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    if site == region.tail.header() as usize && region.map.is_some() {
        let remaining = load(options, b, locals, region, region.words + 1)?;
        let empty = b.ins().icmp_imm(IntCC::Equal, remaining, 0);
        let replenish = b.create_block();
        let funded = b.create_block();
        b.ins().brif(empty, replenish, &[], funded, &[]);
        b.switch_to_block(replenish);
        let function = load(options, b, locals, region, region.words + 3)?;
        let context = load(options, b, locals, region, region.words + 4)?;
        let zero = b.ins().iconst(types::I64, 0);
        hooks.bound_call(
            b,
            function,
            context,
            site,
            OwnerOperation::Refill,
            [zero; 3],
            error,
        );
        let count = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        store(options, b, locals, region, region.words + 1, count)?;
        b.ins().jump(funded, &[]);
        b.switch_to_block(funded);
        let remaining = load(options, b, locals, region, region.words + 1)?;
        let remaining = b.ins().iadd_imm(remaining, -1);
        store(options, b, locals, region, region.words + 1, remaining)?;
        if region.packed.is_some() {
            let context = load(options, b, locals, region, region.words + 4)?;
            let frame = b.ins().load(types::I64, MemFlags::new(), context, 0);
            let selected = b.ins().icmp_imm(IntCC::NotEqual, frame, 0);
            let sync = b.create_block();
            let ready = b.create_block();
            b.ins().brif(selected, sync, &[], ready, &[]);
            b.switch_to_block(sync);
            b.ins().store(
                MemFlags::new(),
                remaining,
                frame,
                core::mem::offset_of!(
                    crate::native_artifact::program::runtime::maps::packed::Frame,
                    fuel
                ) as i32,
            );
            b.ins().jump(ready, &[]);
            b.switch_to_block(ready);
        }
    } else if site == region.tail.header() as usize {
        let compute = load(options, b, locals, region, region.words + 1)?;
        let comparison = load(options, b, locals, region, region.words + 2)?;
        let compute_cost = b.ins().iconst(types::I64, region.compute as i64);
        let comparison_cost = b.ins().iconst(types::I64, region.comparison as i64);
        let a = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, compute, compute_cost);
        let c = b.ins().icmp(
            IntCC::UnsignedGreaterThanOrEqual,
            comparison,
            comparison_cost,
        );
        let enough = b.ins().band(a, c);
        let funded = b.create_block();
        let fallback = b.create_block();
        b.ins().brif(enough, funded, &[], fallback, &[]);
        b.switch_to_block(fallback);
        let site = b.ins().iconst(types::I64, site as i64);
        leave(options, b, locals, region, site, 1, hooks, error)?;
        b.ins().jump(ordinary, &[]);
        b.switch_to_block(funded);
        let compute = b.ins().isub(compute, compute_cost);
        let comparison = b.ins().isub(comparison, comparison_cost);
        store(options, b, locals, region, region.words + 1, compute)?;
        store(options, b, locals, region, region.words + 2, comparison)?;
    }
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    Ok(Some(joined))
}
