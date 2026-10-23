//! Hoist the outer caller census only when a map loop cannot change it.
//! Snapshot holdings and call-cache metadata remain fresh observations; table
//! and projected-key holdings are added by the owning lane on every operation.
use super::{Context, Error, State, add, invalid};
use crate::native_artifact::{NativeExecutionPhase as Phase, owned_string_map::PublicationGeometry};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMatchAction as Scalar};

pub(super) struct Caller {
    pub iteration: u32,
    fixed_nodes: u64,
}

impl State<'_> {
    pub(super) fn capture_caller(&self, ctx: &mut Context<'_, '_>, owner: usize)
        -> Result<Option<Caller>, Error>
    {
        let layout = self.descriptor.map_owner_words.as_ref().unwrap();
        let iteration = layout.regions()[owner].iteration();
        let plan = &self.descriptor.plan;
        ctx.control.work.charge(Phase::Setup, add(super::amount(plan.blocks().len(), 128, 512)?,
            super::amount(plan.iterations().len(), 64, 128)?)?)?;
        if plan.iterations().iter().any(|child| child.as_ref().parent() == Some(iteration)) {
            return Ok(None);
        }
        // No nested loop, ordinary library call, managed structural result or
        // other caller mutation may occur while this census is retained.
        for (site, block) in plan.blocks().iter().enumerate() {
            if block.iteration() != Some(iteration) { continue; }
            let native = layout.action_region(site) == Some(owner)
                || layout.call_region(site) == Some(owner)
                || layout.span_site(site).is_some_and(|id| layout.spans()[id].region() == owner)
                || matches!(block.action(), Action::Scalar(Scalar::None | Scalar::Primitive { .. }))
                || matches!(block.action(), Action::Scalar(Scalar::Copy { output, .. })
                    if plan.values()[output.get() as usize].scalar_kind().is_some());
            if !native || plan.activation_boundary() == Some(site as u32) { return Ok(None); }
        }
        if self.caller.is_some() { return Err(invalid("overlapping closed map caller census")); }
        let site = layout.regions()[owner].tail().initialize() as usize;
        let Some(geometry) = ctx.retained_caller_geometry(site)? else { return Ok(None); };
        let dynamic = ctx.retained_caller_dynamic_nodes(site)?;
        let fixed_nodes = geometry.live_nodes.checked_sub(dynamic)
            .ok_or_else(|| invalid("map caller census is smaller than its dynamic holdings"))?;
        Ok(Some(Caller { iteration, fixed_nodes }))
    }

    pub(super) fn caller_geometry(&self, ctx: &mut Context<'_, '_>, site: usize)
        -> Result<Option<PublicationGeometry>, Error>
    {
        let cached = self.caller.as_ref().filter(|caller|
            ctx.control.current == Some(caller.iteration)
                && self.descriptor.plan.blocks()[site].iteration() == Some(caller.iteration));
        #[cfg(all(test, feature = "canonical-native-program"))]
        super::tests::observe_geometry(cached.is_some());
        let Some(caller) = cached else { return ctx.retained_caller_geometry(site); };
        ctx.control.work.charge(Phase::Setup, 256)?;
        let geometry = PublicationGeometry {
            live_bytes: ctx.library.memory_budget().used_bytes(),
            live_nodes: add(caller.fixed_nodes, ctx.retained_caller_dynamic_nodes(site)?)?,
        };
        // Independent complete census during real generated-code tests. This
        // detects drift after map growth, cache population and view retirement.
        #[cfg(all(test, feature = "canonical-native-program"))]
        {
            let expected = ctx.retained_caller_geometry(site)?
                .ok_or_else(|| invalid("closed map census lost caller geometry"))?;
            if (expected.live_bytes, expected.live_nodes) != (geometry.live_bytes, geometry.live_nodes) {
                return Err(invalid("closed map caller census differs from complete inventory"));
            }
        }
        Ok(Some(geometry))
    }
}
