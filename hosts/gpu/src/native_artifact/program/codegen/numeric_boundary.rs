//! Shared cold first-change bridge for native replacement and append stores.
use super::*;
use crate::native_artifact::program::prepare::tail::Region;

pub(super) fn materialize(
    controls: &CompilationControls,
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
    let zero = b.ins().iconst(types::I64, 0);
    // No active generated loan survives a failed cold call.
    b.ins()
        .store(MemFlags::new(), zero, locals, offset(region.words)?);
    hooks.require_argument_words(b, proof.words + 4, error);
    for field in proof
        .frame(region.tail.carrier().get())
        .into_iter()
        .flat_map(|frame| frame.scalar_slots())
    {
        let value = words.read(b, shadow + field);
        b.ins()
            .store(MemFlags::new(), value, hooks.arguments, offset(2 + field)?);
    }
    if let Some(tree) = &region.numeric_tree {
        let active = controls
            .enabled(OptimizationPass::PrivateLiveness)
            .then(|| super::super::private_liveness::carrier_words(tree, site))
            .transpose()?;
        for node in &tree.nodes {
            for binding in &node.scalar_bindings {
                if active
                    .as_ref()
                    .is_some_and(|words| !words.contains(&binding.word))
                {
                    continue;
                }
                let value = words.read(b, shadow + binding.word);
                b.ins().store(
                    MemFlags::new(),
                    value,
                    hooks.arguments,
                    offset(2 + binding.word)?,
                );
            }
        }
        let settled = super::super::numeric_tree::settled_debits(
            b,
            tree,
            controls.enabled(OptimizationPass::ProjectedNumericDebits),
            |b, slot| Ok(words.read(b, shadow + slot)),
        )?;
        for slot in tree.counters..tree.bank() {
            let value = settled
                .get(&slot)
                .copied()
                .unwrap_or_else(|| words.read(b, shadow + slot));
            b.ins()
                .store(MemFlags::new(), value, hooks.arguments, offset(2 + slot)?);
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
    if let Some(tree) = &region.numeric_tree {
        for slot in tree.counters..tree.bank() {
            let value = if tree.live_counter_word(
                slot,
                controls.enabled(OptimizationPass::ProjectedNumericDebits),
            ) {
                b.ins().load(
                    types::I64,
                    MemFlags::new(),
                    hooks.arguments,
                    offset(2 + slot)?,
                )
            } else {
                b.ins().iconst(types::I64, 0)
            };
            words.write(b, shadow + slot, value);
        }
        let bank = b
            .ins()
            .load(types::I64, MemFlags::new(), hooks.arguments, 16);
        words.write(b, shadow + tree.bank(), bank);
    }
    let one = b.ins().iconst(types::I64, 1);
    words.write(b, region.words, one);
    b.ins()
        .store(MemFlags::new(), one, locals, offset(region.words)?);
    Ok(())
}
