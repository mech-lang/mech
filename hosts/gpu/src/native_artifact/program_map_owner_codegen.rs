//! Conditional native words around the ordinary generated CFG. Rust helpers
//! provide storage only; source blocks still choose successors and compute values.
use super::super::{
    NativeCompileError,
    compiler::{backend, offset},
    iteration_code_plan::CodePlan,
    program_abi::{
        FROZEN_MAP_COPY, FROZEN_MAP_READ, MAP_OWNER_BEGIN, MAP_OWNER_CALL, MAP_OWNER_END,
        MAP_OWNER_SPAN,
    },
    program_hooks::Hooks,
    program_map_owner_layout as layout,
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeIterationAction, NativeMapOwnerWords};

fn load(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    word: usize,
) -> Result<Value, NativeCompileError> {
    Ok(b.ins()
        .load(types::I64, MemFlags::new(), base, offset(word)?))
}
fn store(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    word: usize,
    value: Value,
) -> Result<(), NativeCompileError> {
    b.ins().store(MemFlags::new(), value, base, offset(word)?);
    Ok(())
}

/// Only a live owner selects native paths. Retained map helpers admit their
/// boundary in the helper callback itself, avoiding a separate round trip.
/// Refused entry keeps the original canonical callbacks and work prices.
pub(in crate::native_artifact) fn admission(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    owner: Option<&NativeMapOwnerWords>,
    layout: &super::super::program_map_admission::Layout,
    site: usize,
) -> Result<Option<Block>, NativeCompileError> {
    let Some(owner) = owner else {
        return Ok(None);
    };
    let index = layout
        .group(site)
        .filter(|group| group.start() as usize != site)
        .map(|group| group.owner)
        .or_else(|| owner.call_region(site))
        .or_else(|| owner.span_site(site).map(|id| owner.spans()[id].region()));
    let Some(index) = index else {
        return Ok(None);
    };
    let flag = owner
        .frame_words()
        .checked_add(index)
        .ok_or_else(|| backend("map admission flag overflow"))?;
    let active = load(b, locals, flag)?;
    let active = b.ins().icmp_imm(IntCC::NotEqual, active, 0);
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(active, joined, &[], ordinary, &[]);
    b.switch_to_block(ordinary);
    Ok(Some(joined))
}

/// Return the common continuation when a native branch skips the ordinary
/// action, and whether entry was completely emitted (including fallback MOVE).
pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    owner: Option<&NativeMapOwnerWords>,
    closed: &super::super::program_tail_layout::Layout,
    frozen: &super::super::program_frozen_map_layout::Layout,
    site: usize,
    hooks: Option<&Hooks>,
    code: CodePlan<'_>,
    error: Block,
) -> Result<(Option<Block>, bool), NativeCompileError> {
    let Some(owner) = owner else {
        return Ok((None, false));
    };
    let frozen_copy = frozen.copies.get(site).copied().flatten();
    let frozen_read = frozen.reads.get(site).copied().flatten();
    let frozen_id = frozen_copy.or(frozen_read);
    let entry = owner.initializer(site);
    let exit = owner.exit(site);
    let call = layout::call_region(owner, site);
    let action = layout::action_region(owner, site);
    let span = owner.span_site(site).map(|index| &owner.spans()[index]);
    let Some(index) = entry
        .or(exit)
        .or(call)
        .or(action)
        .or(span.map(|span| span.region()))
        .or(frozen_id.map(|id| frozen.regions[id].owner))
    else {
        return Ok((None, false));
    };
    let hooks = hooks.ok_or_else(|| backend("map owner transport lacks checked hooks"))?;
    let region = &owner.regions()[index];
    let flag = owner
        .frame_words()
        .checked_add(index)
        .ok_or_else(|| backend("map owner flag overflow"))?;
    let zero = b.ins().iconst(types::I64, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    if entry.is_some() {
        hooks.call(b, site, MAP_OWNER_BEGIN, [zero; 3], error);
        let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        store(b, locals, flag, ready)?;
        let selected = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
        b.ins().brif(selected, native, &[], ordinary, &[]);
        b.switch_to_block(native);
        let (base, count) = owner
            .aggregates()
            .slot(region.tail().carrier())
            .ok_or_else(|| backend("map owner carrier has no fixed slice"))?;
        for word in 0..count {
            let value = load(b, hooks.arguments, word)?;
            store(b, locals, base + word, value)?;
        }
        b.ins().jump(joined, &[]);
        b.switch_to_block(ordinary);
        // The checked begin helper already performed the canonical MOVE on
        // refusal, using the same visit and cumulative work ledger.
        b.ins().jump(joined, &[]);
        b.switch_to_block(joined);
        store(
            b,
            locals,
            code.value_count() + region.iteration() as usize,
            zero,
        )?;
        return Ok((None, true));
    }
    let active = load(b, locals, flag)?;
    let active = b.ins().icmp_imm(IntCC::NotEqual, active, 0);
    b.ins().brif(active, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    if let Some(id) = frozen_id {
        let proof = &frozen.regions[id];
        let (source, output, operation) = match code.blocks()[site].action() {
            NativeIterationAction::Scalar(
                mech_engine::native_artifact::NativeMatchAction::Copy { source, output },
            ) if frozen_copy.is_some() => (*source, *output, FROZEN_MAP_COPY),
            NativeIterationAction::LibraryCall {
                operands, output, ..
            } if frozen_read.is_some() => (operands[0], *output, FROZEN_MAP_READ),
            _ => return Err(backend("frozen map site differs from complete-use proof")),
        };
        if proof.owner != index {
            return Err(backend("frozen map owner mismatch"));
        }
        let token = load(b, locals, source.get() as usize)?;
        hooks.call(b, site, operation, [token, zero, zero], error);
        let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        store(b, locals, output.get() as usize, word)?;
        b.ins().jump(joined, &[]);
        b.switch_to_block(ordinary);
        return Ok((Some(joined), false));
    }
    if (span.is_some() || call.is_some())
        && let Some(id) = closed.sites.get(site).copied().flatten()
        && closed.regions[id].map == Some(index)
    {
        let proof = &closed.regions[id];
        let ready = load(b, locals, proof.words)?;
        let ready = b.ins().icmp_imm(IntCC::NotEqual, ready, 0);
        let thin = b.create_block();
        let guarded = b.create_block();
        b.ins().brif(ready, thin, &[], guarded, &[]);
        b.switch_to_block(thin);
        let function = load(b, locals, proof.words + 3)?;
        let context = load(b, locals, proof.words + 4)?;
        if let Some(packed_proof) = &proof.packed {
            // Constructor-owned prefix: zero declines packed storage while
            // retaining the existing closed String-map implementation.
            let frame = b.ins().load(types::I64, MemFlags::new(), context, 0);
            let selected = b.ins().icmp_imm(IntCC::NotEqual, frame, 0);
            let packed = b.create_block();
            let general = b.create_block();
            b.ins().brif(selected, packed, &[], general, &[]);
            b.switch_to_block(packed);
            if let Some(span) = span {
                if span.range_site() as usize == site {
                    let first = load(b, locals, span.operands()[0].get() as usize)?;
                    let last = load(b, locals, span.operands()[1].get() as usize)?;
                    super::packed_maps::select(
                        b,
                        frame,
                        packed_proof.packing_domain(),
                        first,
                        last,
                        hooks,
                        function,
                        context,
                        site,
                        error,
                    );
                }
            } else {
                use mech_engine::native_artifact::NativeCollectionPrimitive as P;
                let NativeIterationAction::LibraryCall {
                    template,
                    operands,
                    output,
                } = code.blocks()[site].action()
                else {
                    return Err(backend("packed map call lacks library action"));
                };
                let op = match template.primitive() {
                    P::MapContainsKey => 3,
                    P::MapKeyAccess => 4,
                    P::MapInsert => 5,
                    _ => return Err(backend("packed map call kind")),
                };
                let value = if op == 5 {
                    load(b, locals, operands[2].get() as usize)?
                } else {
                    zero
                };
                let token = load(b, locals, operands[0].get() as usize)?;
                super::packed_maps::operation(
                    b,
                    frame,
                    packed_proof.packing_domain(),
                    op,
                    value,
                    locals,
                    output.get() as usize,
                    token,
                    hooks,
                    function,
                    context,
                    site,
                    error,
                )?;
            }
            b.ins().jump(joined, &[]);
            b.switch_to_block(general);
        }
        let (op, args, output) = if let Some(span) = span {
            if span.range_site() as usize == site {
                // The selected-pair proof permits only empty scalar blocks
                // and jumps between this range and its sole String selection.
                // Keep those generated visits, but bind the key in one call.
                (
                    Some(6),
                    [
                        load(b, locals, span.operands()[0].get() as usize)?,
                        load(b, locals, span.operands()[1].get() as usize)?,
                        zero,
                    ],
                    None,
                )
            } else {
                (None, [zero; 3], None)
            }
        } else {
            use mech_engine::native_artifact::NativeCollectionPrimitive as P;
            let NativeIterationAction::LibraryCall {
                template,
                operands,
                output,
            } = code.blocks()[site].action()
            else {
                return Err(backend("closed map call lacks library action"));
            };
            let op = match template.primitive() {
                P::MapContainsKey => 3,
                P::MapKeyAccess => 4,
                P::MapInsert => 5,
                _ => return Err(backend("closed map call kind")),
            };
            let value = if op == 5 {
                load(b, locals, operands[2].get() as usize)?
            } else {
                zero
            };
            (Some(op), [zero, zero, value], Some(*output))
        };
        if let Some(op) = op {
            hooks.bound_call(b, function, context, site, op, args, error);
        }
        if let Some(output) = output {
            let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
            store(b, locals, output.get() as usize, word)?;
        }
        b.ins().jump(joined, &[]);
        b.switch_to_block(guarded);
    }
    if let Some(span) = span {
        let args = if span.range_site() as usize == site {
            [
                load(b, locals, span.operands()[0].get() as usize)?,
                load(b, locals, span.operands()[1].get() as usize)?,
                zero,
            ]
        } else {
            [zero; 3]
        };
        hooks.call(b, site, MAP_OWNER_SPAN, args, error);
    } else if exit.is_some() {
        let token = load(b, locals, region.tail().returned().get() as usize)?;
        hooks.call(b, site, MAP_OWNER_END, [token, zero, zero], error);
        let token = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        store(b, locals, flag, token)?;
        // Canonical publication returns zero; private publication returns the
        // invocation/generation token. No canonical Map is forged in the arena.
        let NativeIterationAction::Scalar(mech_engine::native_artifact::NativeMatchAction::Copy {
            output,
            ..
        }) = code.blocks()[site].action()
        else {
            return Err(backend("map exit lacks copy"));
        };
        store(b, locals, output.get() as usize, token)?;
    } else if call.is_some() {
        let NativeIterationAction::LibraryCall {
            operands, output, ..
        } = code.blocks()[site].action()
        else {
            return Err(backend("map owner call is not a library action"));
        };
        hooks.library_operation(
            b,
            site,
            MAP_OWNER_CALL,
            operands,
            locals,
            |value| value == operands[0] || code.kind(value).is_some(),
            error,
        )?;
        let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
        store(b, locals, output.get() as usize, word)?;
    } else if !super::scalar_aggregates::emit(b, locals, owner.aggregates(), site, error)? {
        return Err(backend("map owner action lacks word transport"));
    }
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    Ok((Some(joined), false))
}
