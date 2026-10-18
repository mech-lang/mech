//! Retire only locals whose previous lexical invocation cannot be read again.
//! The shared CFG/current-visit proof forbids their escape; yielded drafts are
//! independent copies. Ancestor captures and this generator's old element stay
//! live until replacement, including during the next allocation's admission.
use super::{Context, Error, State, add, invalid, mul};
use mech_engine::native_artifact::{NativeControlLoop, NativeIterationValueScope};

impl State {
    pub(super) fn retire_locals(
        &mut self,
        ctx: &mut Context<'_>,
        iteration: u32,
    ) -> Result<(), Error> {
        let plan = &ctx.control.descriptor.plan;
        let NativeControlLoop::Collection(definition) = &plan.iterations()[iteration as usize]
        else {
            return Err(invalid("collection retirement on a fold"));
        };
        // Bound the complete static scope walk before doing it. No unmetered
        // payload traversal, allocation or body interpretation occurs here.
        self.account(
            ctx,
            definition.collection(),
            0,
            add(
                mul(
                    plan.values().len() as u64,
                    mul(add(plan.iterations().len() as u64, 1)?, 16)?,
                )?,
                64,
            )?,
        )?;
        for (slot, scope) in plan.value_scopes().iter().enumerate() {
            if plan.values()[slot].scalar_kind().is_some()
                || definition
                    .element()
                    .is_some_and(|element| element.get() as usize == slot)
            {
                continue;
            }
            let mut owner = match scope {
                NativeIterationValueScope::Immutable { iteration } => *iteration,
                NativeIterationValueScope::Carrier { iteration } => Some(*iteration),
            };
            while let Some(id) = owner {
                if id == iteration {
                    ctx.arena
                        .retire_current(slot as u32, &mut ctx.control.work)?;
                    break;
                }
                // Certified loop ids are pre-order and parents strictly earlier.
                owner = plan.iterations()[id as usize].as_ref().parent();
            }
        }
        Ok(())
    }

    pub(in super::super) fn release_source(
        &mut self,
        ctx: &mut Context<'_>,
        iteration: u32,
    ) -> Result<(), Error> {
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
            drop(ctx.arena.take(handle, &mut ctx.control.work)?);
            self.live_bytes = self
                .live_bytes
                .checked_sub(retained)
                .ok_or_else(|| invalid("collection released element accounting"))?;
        }
        Ok(())
    }
}
