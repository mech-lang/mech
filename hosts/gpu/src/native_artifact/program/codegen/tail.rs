//! Native finite fuel for closed scalar/map Tail bodies, inside the same round.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::options::OptimizationOptions;
use crate::native_artifact::compiler::offset;
use crate::native_artifact::program::codegen::hooks::Hooks;
use crate::native_artifact::program::prepare::tail::{Layout, Region};
#[cfg(feature = "canonical-native-program-runtime")]
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::protocol::{OwnerOperation, abi as program_abi};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
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
        hooks.require_argument_words(b, proof.words + 2, error);
        let terminal = proof
            .result
            .frame()
            .filter(|_| mode == 0)
            .and_then(|value| proof.fields(value))
            .unwrap_or(&[]);
        for slot in proof
            .fields(region.tail.carrier().get())
            .unwrap()
            .iter()
            .chain(terminal.iter())
            .flatten()
        {
            let value = load(options, b, locals, region, region.words + 3 + slot)?;
            b.ins()
                .store(MemFlags::new(), value, hooks.arguments, offset(2 + slot)?);
        }
        for (array, _) in proof.arrays.iter().enumerate().filter(|(_, a)| a.appends) {
            let slot = proof.view(array) + 1;
            let value = load(options, b, locals, region, region.words + 3 + slot)?;
            b.ins()
                .store(MemFlags::new(), value, hooks.arguments, offset(2 + slot)?);
        }
    }
    let mode = b.ins().iconst(types::I64, mode);
    hooks.call_at(
        b,
        site,
        GlobalHelper::ClosedTailEnd,
        [compute, comparison, mode],
        error,
    );
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
    options: OptimizationOptions,
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some(region) = layout.initializers.get(site).copied().flatten() else {
        return Ok(());
    };
    let region = &layout.regions[region];
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
    for i in 0..if region.map.is_some() {
        4
    } else {
        2 + region.numeric.as_ref().map_or(0, |p| p.words)
    } {
        let word = b
            .ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, offset(i)?);
        store(options, b, locals, region, region.words + 1 + i, word)?;
    }
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
