//! Native finite fuel for a proved callback-free Tail, inside the same round.
use super::super::{
    NativeCompileError,
    compiler::offset,
    program_abi,
    program_hooks::Hooks,
    program_tail_layout::{Layout, Region},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

fn load(b: &mut FunctionBuilder, locals: Value, word: usize) -> Result<Value, NativeCompileError> {
    Ok(b.ins()
        .load(types::I64, MemFlags::new(), locals, offset(word)?))
}
fn store(
    b: &mut FunctionBuilder,
    locals: Value,
    word: usize,
    value: Value,
) -> Result<(), NativeCompileError> {
    b.ins().store(MemFlags::new(), value, locals, offset(word)?);
    Ok(())
}
fn leave(
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    site: Value,
    mode: i64,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let ready = load(b, locals, region.words)?;
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    let finish = b.create_block();
    let done = b.create_block();
    b.ins().brif(ready, finish, &[], done, &[]);
    b.switch_to_block(finish);
    let compute = load(b, locals, region.words + 1)?;
    let comparison = load(b, locals, region.words + 2)?;
    let zero = b.ins().iconst(types::I64, 0);
    store(b, locals, region.words, zero)?;
    let mode = b.ins().iconst(types::I64, mode);
    hooks.call_at(
        b,
        site,
        program_abi::CLOSED_TAIL_END,
        [compute, comparison, mode],
        error,
    );
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    Ok(())
}
pub(in crate::native_artifact) fn failure(
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    if let Some(region) = layout.sites.get(site).copied().flatten() {
        let site = b.ins().iconst(types::I64, site as i64);
        leave(b, locals, &layout.regions[region], site, 2, hooks, error)?;
    }
    Ok(())
}
pub(in crate::native_artifact) fn abort(
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: Value,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    for region in &layout.regions {
        leave(b, locals, region, site, 3, hooks, error)?;
    }
    Ok(())
}
pub(in crate::native_artifact) fn prepare(
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
    prepare_region(
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
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    if let Some(region) = layout.sites.get(site).copied().flatten() {
        let site = b.ins().iconst(types::I64, site as i64);
        leave(b, locals, &layout.regions[region], site, 4, hooks, error)?;
        let zero = b.ins().iconst(types::I64, 0);
        // A declined loan may leave capture words in the unused fuel lane.
        for word in layout.regions[region].words..layout.regions[region].words + 3 {
            store(b, locals, word, zero)?;
        }
    }
    Ok(())
}
fn prepare_region(
    b: &mut FunctionBuilder,
    locals: Value,
    region: &Region,
    site: usize,
    operation: u64,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, operation, [zero; 3], error);
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    store(b, locals, region.words, ready)?;
    for i in 0..2 {
        let word = load(b, hooks.arguments, i)?;
        store(b, locals, region.words + 1 + i, word)?;
    }
    Ok(())
}
/// Emit native admission or leave in the ordinary callback arm; the caller
/// joins that callback at the returned block. No helper occurs on a funded body.
pub(in crate::native_artifact) fn admission(
    b: &mut FunctionBuilder,
    locals: Value,
    layout: &Layout,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<Option<Block>, NativeCompileError> {
    if let Some(region) = layout.exits.get(site).copied().flatten() {
        let site = b.ins().iconst(types::I64, site as i64);
        leave(b, locals, &layout.regions[region], site, 0, hooks, error)?;
    }
    let Some(region) = layout.sites.get(site).copied().flatten() else {
        return Ok(None);
    };
    let region = &layout.regions[region];
    let ready = load(b, locals, region.words)?;
    let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(ready, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    if site == region.tail.header() as usize {
        let compute = load(b, locals, region.words + 1)?;
        let comparison = load(b, locals, region.words + 2)?;
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
        leave(b, locals, region, site, 1, hooks, error)?;
        b.ins().jump(ordinary, &[]);
        b.switch_to_block(funded);
        let compute = b.ins().isub(compute, compute_cost);
        let comparison = b.ins().isub(comparison, comparison_cost);
        store(b, locals, region.words + 1, compute)?;
        store(b, locals, region.words + 2, comparison)?;
    }
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    Ok(Some(joined))
}
