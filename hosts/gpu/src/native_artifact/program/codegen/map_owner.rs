//! Conditional native words around the ordinary generated CFG. Rust helpers
//! provide storage only; source blocks still choose successors and compute values.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::codegen::hooks::Hooks;
use crate::native_artifact::program::prepare::map_owner as layout;
#[cfg(feature = "canonical-native-program-runtime")]
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::protocol::OwnerOperation;
use crate::native_artifact::program::protocol::abi::{
    FROZEN_MAP_COPY, FROZEN_MAP_READ, MAP_OWNER_CALL,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
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

/// A paid scalar entry or a live map owner selects native paths. Retained map
/// helpers admit their boundary inside their helper callback. Refused scalar
/// funding or owner entry keeps the original ordered callbacks and prices.
pub(in crate::native_artifact) fn admission(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    owner: Option<&NativeMapOwnerWords>,
    layout: &crate::native_artifact::program::prepare::map_admission::Layout,
    site: usize,
    hooks: Option<&Hooks>,
) -> Result<Option<Block>, NativeCompileError> {
    if layout
        .group(site)
        .is_some_and(|group| group.owner.is_none() && group.start() as usize != site)
    {
        let hooks = hooks.ok_or_else(|| backend("scalar admission lacks checked hooks"))?;
        let active = load(b, hooks.result, 0)?;
        return Ok(Some(admission_branch(b, active)));
    }
    let Some(owner) = owner else {
        return Ok(None);
    };
    let index = layout
        .group(site)
        .filter(|group| group.start() as usize != site)
        .and_then(|group| group.owner)
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
    Ok(Some(admission_branch(b, active)))
}

fn admission_branch(b: &mut FunctionBuilder<'_>, active: Value) -> Block {
    let active = b.ins().icmp_imm(IntCC::NotEqual, active, 0);
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(active, joined, &[], ordinary, &[]);
    b.switch_to_block(ordinary);
    joined
}

/// Return the common continuation when a native branch skips the ordinary
/// action, and whether entry was completely emitted (including fallback MOVE).
pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    owner: Option<&NativeMapOwnerWords>,
    closed: &crate::native_artifact::program::prepare::tail::Layout,
    frozen: &crate::native_artifact::program::prepare::frozen_map::Layout,
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
        hooks.call(b, site, GlobalHelper::MapOwnerBegin, [zero; 3], error);
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
        hooks.call(
            b,
            site,
            GlobalHelper::decode(operation),
            [token, zero, zero],
            error,
        );
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
                    crate::native_artifact::program::codegen::packed_map::select(
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
                crate::native_artifact::program::codegen::packed_map::operation(
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
            hooks.bound_call(
                b,
                function,
                context,
                site,
                OwnerOperation::decode(op),
                args,
                error,
            );
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
        hooks.call(b, site, GlobalHelper::MapOwnerSpan, args, error);
    } else if exit.is_some() {
        let token = load(b, locals, region.tail().returned().get() as usize)?;
        hooks.call(
            b,
            site,
            GlobalHelper::MapOwnerEnd,
            [token, zero, zero],
            error,
        );
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
    } else if !crate::native_artifact::codegen::common::aggregates::emit_memory(
        b,
        locals,
        owner.aggregates(),
        site,
        error,
    )? {
        return Err(backend("map owner action lacks word transport"));
    }
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    Ok((Some(joined), false))
}
