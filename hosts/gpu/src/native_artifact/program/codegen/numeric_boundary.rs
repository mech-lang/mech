//! Shared cold first-change bridge for native replacement and append stores.
use super::*;
use crate::native_artifact::program::prepare::tail::Region;

/// Settle the loan and run the exact original rejecting selected operation.
/// Only its scalar operands and the existing settlement inputs need spilling.
pub(super) fn source_failure(
    b: &mut FunctionBuilder<'_>,
    controls: &CompilationControls,
    words: &Words,
    frame: Value,
    locals: Value,
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<usize, NativeCompileError> {
    set_origin(b, frame, site as u64);
    let Action::LibraryCall { operands, .. } = code.blocks()[site].action() else {
        return Err(backend("numeric rejection lost selected call"));
    };
    if !closed.regions[id]
        .numeric
        .as_ref()
        .is_some_and(|proof| proof.sites.binary_search(&site).is_ok())
    {
        return Err(backend("numeric rejection differs from its active region"));
    }
    let region = &closed.regions[id];
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric rejection lacks scratch home"))?;
    let live = (region.words..region.words + 3)
        .chain(
            tail::numeric_exit_words(region, 2, None)?
                .into_iter()
                .map(|word| shadow + word),
        )
        .chain(
            operands
                .iter()
                .filter(|value| code.kind(**value).is_some())
                .map(|value| value.get() as usize),
        )
        .collect::<BTreeSet<_>>();
    words.spill_live(b, locals, &live)?;
    let origin = b.ins().iconst(types::I64, site as i64);
    tail::leave(
        controls.options(),
        b,
        locals,
        region,
        origin,
        2,
        hooks,
        error,
    )?;
    hooks.library(b, site, operands, locals, |v| code.kind(v).is_some(), error)?;
    let status = b.ins().iconst(types::I64, STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    Ok(live.len())
}

pub(super) fn materialize(
    b: &mut FunctionBuilder<'_>,
    options: crate::native_artifact::codegen::options::OptimizationOptions,
    words: &Words,
    region: &Region,
    proof: &Proof,
    array: usize,
    site: usize,
    frame: Value,
    locals: Value,
    index: Value,
    incoming: Value,
    hooks: &Hooks,
    error: Block,
) -> Result<(), NativeCompileError> {
    let shadow = region
        .shadow
        .ok_or_else(|| backend("numeric materialization lacks scratch home"))?;
    set_origin(b, frame, site as u64);
    let zero = b.ins().iconst(types::I64, 0);
    // No active generated loan survives a failed cold call.
    b.ins().store(
        tail::flags(options, region),
        zero,
        locals,
        offset(region.words)?,
    );
    hooks.require_argument_words(b, proof.words + 4, error);
    for field in proof
        .fields(region.tail.carrier().get())
        .unwrap()
        .iter()
        .flatten()
    {
        let value = words.read(b, shadow + field);
        b.ins()
            .store(MemFlags::new(), value, hooks.arguments, offset(2 + field)?);
    }
    if let Some(phase) = &proof.phase {
        for debit in &phase.loops {
            let value = words.read(b, shadow + debit.counter);
            b.ins().store(
                MemFlags::new(),
                value,
                hooks.arguments,
                offset(2 + debit.counter)?,
            );
        }
    }
    b.ins().store(
        MemFlags::new(),
        index,
        hooks.arguments,
        offset(2 + proof.words)?,
    );
    b.ins().store(
        MemFlags::new(),
        incoming,
        hooks.arguments,
        offset(3 + proof.words)?,
    );
    let compute = words.read(b, region.words + 1);
    let comparison = words.read(b, region.words + 2);
    hooks.call(
        b,
        site,
        crate::native_artifact::program::protocol::GlobalHelper::NumericWritePrepare,
        [compute, comparison, zero],
        error,
    );
    let mut reload = vec![
        region.words + 1,
        region.words + 2,
        shadow + proof.view(array),
        shadow + proof.private_flag(array),
    ];
    if let Some(capacity) = proof.capacity(array) {
        reload.extend([
            shadow + proof.view(array) + 1,
            shadow + proof.view(array) + 2,
            shadow + proof.view(array) + 3,
            shadow + capacity,
        ]);
    }
    if let Some(phase) = &proof.phase {
        reload.extend(phase.loops.iter().map(|debit| shadow + debit.counter));
    }
    for slot in reload {
        let argument = if slot == region.words + 1 {
            0
        } else if slot == region.words + 2 {
            1
        } else {
            2 + slot - shadow
        };
        let value = b.ins().load(
            types::I64,
            MemFlags::new(),
            hooks.arguments,
            offset(argument)?,
        );
        words.write(b, slot, value);
    }
    let one = b.ins().iconst(types::I64, 1);
    words.write(b, region.words, one);
    b.ins().store(
        tail::flags(options, region),
        one,
        locals,
        offset(region.words)?,
    );
    Ok(())
}
