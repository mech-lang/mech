//! Shared cold first-change bridge for native replacement and append stores.
use super::*;
use crate::native_artifact::program::prepare::tail::Region;

pub(super) fn materialize(
    b: &mut FunctionBuilder<'_>,
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
    words.spill_all(b, locals)?;
    let zero = b.ins().iconst(types::I64, 0);
    // No active generated loan survives a failed cold call.
    b.ins()
        .store(MemFlags::new(), zero, locals, offset(region.words)?);
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
    b.ins()
        .store(MemFlags::new(), one, locals, offset(region.words)?);
    Ok(())
}
