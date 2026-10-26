//! Bind real canonical roots and fund the complete loop BEFORE its native body.
use super::*;
use mech_engine::native_artifact::NativeWindowRegion;

impl State {
    pub(in crate::native_artifact::program_runtime) fn prepare_compiled_window(
        &mut self,
        ctx: &mut Invocation<'_, '_, '_>,
        region: &NativeWindowRegion,
        width: u64,
        length: u64,
    ) -> Result<Option<([u64; 5], u64)>, Error> {
        let id = region.collection();
        super::charge(
            &mut ctx.state.control.work,
            super::super::super::amount(ctx.state.call_inputs.values().len(), 16, 128)?,
        )?;
        self.builder(id)?;
        let query = self.query_builder(id)?;
        if query.phase.is_some()
            || query.accepted != 0
            || query.matched != 0
            || ctx.state.call_inputs.values().len() < 2
            || ctx.state.call_inputs.values().iter().any(Option::is_some)
        {
            return Err(invalid("compiled window has stale owners or query state"));
        }
        let current_query = ctx
            .state
            .arena
            .get(query.handle, &mut ctx.state.control.work)?;
        if !query.query.shares_frozen_storage(current_query) {
            return Err(invalid("compiled window query generation changed"));
        }
        let query_owner = query.query.clone();
        let source = ctx
            .state
            .arena
            .copy_current(region.source().get(), &mut ctx.state.control.work)?;
        let (ValueData::String(text), ValueData::String(key)) = (source.data(), query_owner.data())
        else {
            return Err(invalid("compiled window requires genuine String roots"));
        };
        // Fund the representation scan, including the decline path, before
        // reading any bytes. ASCII alone is insufficient: CRLF is one grapheme.
        let scan = add(mul(text.len() as u64, 64)?, 256)?;
        self.query_account(ctx, id, 0, scan, 0, 0)?;
        if !self.ascii_facts.inspect(&source)? {
            return Ok(None);
        }
        if text.len() as u64 != length {
            return Err(invalid(
                "compiled window length differs from its same source",
            ));
        }
        let accepted = if width > 0 && width <= length {
            length - width + 1
        } else {
            0
        };
        // Entry/outer-loop invariants prove every read stays inside these real
        // owners. Count <= accepted <= source length cannot overflow u64.
        // Work policy22 bounds the actual native byte loop, not the removed
        // selector allocations or old per-operation callback protocol.
        let compared = if key.len() as u64 == width {
            mul(accepted, width)?
        } else {
            0
        };
        let compute = add(
            mul(accepted, add(mul(width.min(key.len() as u64), 16)?, 64)?)?,
            512,
        )?;
        let source_bytes = source
            .memory_budget_retained_bytes(&ctx.state.library.memory_budget())
            .ok_or_else(|| invalid("compiled window source is unadmitted"))?;
        let query_bytes = query_owner
            .memory_budget_retained_bytes(&ctx.state.library.memory_budget())
            .ok_or_else(|| invalid("compiled window query is unadmitted"))?;
        let origin = ctx.descriptor.metadata().plan.collections()[id as usize].origin();
        check(
            origin,
            Dimension::TemporaryBytes,
            add(add(self.live_bytes, source_bytes)?, query_bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        check(
            origin,
            Dimension::RetainedNodes,
            add(self.retained_nodes()?, 4)?,
            ctx.limits.max_retained_nodes,
        )?;
        self.query_account(ctx, id, 0, compute, compared, 0)?;
        let words = [
            text.as_ptr() as u64,
            length,
            key.as_ptr() as u64,
            key.len() as u64,
            width,
        ];
        // Existing admitted typed storage retains BOTH owners until the
        // matching finish callback, including query/source aliasing and error.
        ctx.state.call_inputs.values_mut()[0] = Some(source);
        ctx.state.call_inputs.values_mut()[1] = Some(query_owner);
        Ok(Some((words, accepted)))
    }

    pub(in crate::native_artifact::program_runtime) fn finish_compiled_window(
        &mut self,
        id: u32,
        accepted: u64,
        matched: u64,
    ) -> Result<(), Error> {
        self.builder(id)?;
        let query = self.builders[id as usize]
            .as_mut()
            .and_then(|builder| builder.query.as_mut())
            .ok_or_else(|| invalid("compiled window omitted its canonical query sink"))?;
        if query.phase.is_some() || query.accepted != 0 || query.matched != 0 || matched > accepted
        {
            return Err(invalid("compiled window sink replay or invalid result"));
        }
        query.accepted = accepted;
        query.matched = matched;
        Ok(())
    }
}
