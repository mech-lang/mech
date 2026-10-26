//! Retire only locals whose previous lexical invocation cannot be read again.
//! The shared CFG/current-visit proof forbids their escape; yielded drafts are
//! independent copies. Ancestor captures and this generator's old element stay
//! live until replacement, including during the next allocation's admission.
use super::super::super::program_descriptor::RetirementKind;
use super::{Error, Invocation, State, add, invalid, mul};
use mech_engine::native_artifact::NativeControlLoop;

impl State {
    /// A body-produced String is still retained by the arena and the owned
    /// local while its draft is cloned into the builder. Source-element
    /// aliases already have that allocation charged in live_bytes; compare
    /// exact frozen storage through generation-checked handles, not equality.
    pub(super) fn untracked_yield_bytes(
        &self,
        ctx: &mut Invocation<'_, '_, '_>,
        value: Option<&mech_core::Value>,
    ) -> Result<u64, Error> {
        let Some(value) = value else { return Ok(0) };
        for source in self.sources.iter().flatten() {
            if let Some((handle, _)) = source.element {
                let tracked = ctx.state.arena.get(handle, &mut ctx.state.control.work)?;
                if value.shares_frozen_storage(tracked) {
                    return Ok(0);
                }
            }
        }
        value
            .memory_budget_retained_bytes(&ctx.state.library.memory_budget())
            .ok_or_else(|| invalid("unadmitted body-produced collection value"))
    }

    pub(super) fn retire_locals(
        &mut self,
        ctx: &mut Invocation<'_, '_, '_>,
        iteration: u32,
    ) -> Result<(), Error> {
        ctx.retire_frozen(iteration)?;
        let plan = &ctx.descriptor.metadata().plan;
        let NativeControlLoop::Collection(definition) = &plan.iterations()[iteration as usize]
        else {
            return Err(invalid("collection retirement on a fold"));
        };
        let slots = ctx
            .descriptor
            .metadata()
            .retirements
            .slots(ctx.retirements, iteration, RetirementKind::Collection)
            .map_err(invalid)?;
        // The set is immutable and exact. Pay only the slots actually read;
        // arena retirement keeps its existing per-slot/generation debit.
        self.account(
            ctx,
            definition.collection(),
            0,
            add(mul(slots.len() as u64, 16)?, 64)?,
        )?;
        for &slot in slots {
            ctx.state
                .arena
                .retire_current(slot as u32, &mut ctx.state.control.work)?;
        }
        Ok(())
    }

    pub(in super::super) fn release_source(
        &mut self,
        ctx: &mut Invocation<'_, '_, '_>,
        iteration: u32,
    ) -> Result<(), Error> {
        if self.active_string_spans != 0 || self.active_string_maps != 0 {
            return Err(invalid("collection exit leaked a private String span"));
        }
        if self
            .sources
            .get(iteration as usize)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(invalid("collection exited without its admitted source"));
        }
        self.retire_locals(ctx, iteration)?;
        let source = self.sources[iteration as usize].take().unwrap();
        if let Some((handle, retained)) = source.element {
            // take validates the exact saved generation. Discard only after
            // all aliases in this lexical invocation have really been retired.
            drop(ctx.state.arena.take(handle, &mut ctx.state.control.work)?);
            self.live_bytes = self
                .live_bytes
                .checked_sub(retained)
                .ok_or_else(|| invalid("collection released element accounting"))?;
        }
        Ok(())
    }
}
