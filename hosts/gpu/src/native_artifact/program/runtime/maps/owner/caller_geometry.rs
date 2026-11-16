//! Hoist the outer caller census only when a map loop cannot change it.
//! Snapshot holdings and call-cache metadata remain fresh observations; table
//! and projected-key holdings are added by the owning lane on every operation.
use crate::native_artifact::NativeExecutionPhase as Phase;
use crate::native_artifact::program::runtime::invocation::MapExecution;
use crate::native_artifact::program::runtime::maps::owner::{State, add};
use crate::native_artifact::program::runtime::maps::string::PublicationGeometry;
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMatchAction as Scalar};

pub(in crate::native_artifact::program::runtime::maps::owner) struct Caller {
    pub iteration: u32,
    pub(super) fixed_nodes: u64,
}

impl State<'_> {
    pub(in crate::native_artifact::program::runtime::maps::owner) fn capture_caller(
        &self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        owner: usize,
    ) -> Result<Option<Caller>, Error> {
        let site = self.region(owner)?.tail().initialize() as usize;
        self.capture_caller_at(ctx, owner, site)
    }

    pub(in crate::native_artifact::program::runtime::maps::owner) fn capture_caller_at(
        &self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        owner: usize,
        current_site: usize,
    ) -> Result<Option<Caller>, Error> {
        let layout = self.descriptor.metadata().map_owner_words.as_ref().unwrap();
        let iteration = layout.regions()[owner].iteration();
        let plan = &self.descriptor.metadata().plan;
        if current_site != layout.regions()[owner].tail().initialize() as usize
            && (ctx.control().current != Some(iteration)
                || !self
                    .descriptor
                    .metadata()
                    .matrix_map_keys
                    .producer(current_site)
                    .is_some_and(|(_, proof)| proof.owner == owner))
        {
            return Err(invalid(
                "deferred caller census outside its selected producer",
            ));
        }
        ctx.work().charge(
            Phase::Setup,
            add(
                crate::native_artifact::program::runtime::amount(plan.blocks().len(), 128, 512)?,
                crate::native_artifact::program::runtime::amount(plan.iterations().len(), 64, 128)?,
            )?,
        )?;
        if plan
            .iterations()
            .iter()
            .any(|child| child.as_ref().parent() == Some(iteration))
        {
            return Ok(None);
        }
        // No nested loop, ordinary library call, managed structural result or
        // other caller mutation may occur while this census is retained.
        for (site, block) in plan.blocks().iter().enumerate() {
            if block.iteration() != Some(iteration) {
                continue;
            }
            let mut native = layout.action_region(site) == Some(owner)
                || layout.call_region(site) == Some(owner)
                || layout
                    .span_site(site)
                    .is_some_and(|id| layout.spans()[id].region() == owner)
                || matches!(
                    block.action(),
                    Action::Scalar(Scalar::None | Scalar::Primitive { .. })
                )
                || matches!(block.action(), Action::Scalar(Scalar::Copy { output, .. })
                    if plan.values()[output.get() as usize].scalar_kind().is_some());
            if !native && !self.matrix_keys.is_empty() {
                ctx.work().charge(
                    Phase::Setup,
                    crate::native_artifact::program::runtime::amount(
                        self.matrix_keys.len(),
                        64,
                        0,
                    )?,
                )?;
                native = self
                    .descriptor
                    .metadata()
                    .matrix_map_keys
                    .producer(site)
                    .is_some_and(|(id, proof)| {
                        proof.owner == owner && self.matrix_keys[id].ready()
                    });
            }
            if !native || plan.activation_boundary() == Some(site as u32) {
                return Ok(None);
            }
        }
        if self.caller.is_some() {
            return Err(invalid("overlapping closed map caller census"));
        }
        let Some(fixed_nodes) = ctx.retained_caller_fixed_geometry(current_site)? else {
            return Ok(None);
        };
        Ok(Some(Caller {
            iteration,
            fixed_nodes,
        }))
    }

    pub(in crate::native_artifact::program::runtime::maps::owner) fn caller_geometry(
        &self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
    ) -> Result<Option<PublicationGeometry>, Error> {
        let cached = self.caller.as_ref().filter(|caller| {
            ctx.control().current == Some(caller.iteration)
                && (self.descriptor.metadata().plan.blocks()[site].iteration()
                    == Some(caller.iteration)
                    || self
                        .descriptor
                        .metadata()
                        .map_owner_words
                        .as_ref()
                        .unwrap()
                        .initializer(site)
                        .is_some_and(|id| {
                            self.descriptor
                                .metadata()
                                .map_owner_words
                                .as_ref()
                                .unwrap()
                                .regions()[id]
                                .iteration()
                                == caller.iteration
                        }))
        });
        #[cfg(all(test, feature = "canonical-native-program"))]
        crate::native_artifact::program::runtime::maps::owner::tests::observe_geometry(
            cached.is_some(),
        );
        let Some(caller) = cached else {
            return ctx.retained_caller_geometry(site);
        };
        ctx.work().charge(Phase::Setup, 256)?;
        let geometry = PublicationGeometry {
            live_bytes: ctx.library().memory_budget().used_bytes(),
            live_nodes: add(caller.fixed_nodes, ctx.retained_caller_dynamic_nodes(site)?)?,
        };
        // Independently check the retained local component. Shared snapshots
        // and bytes may change between observations as peer workers run; their
        // atomic reservations, not equality across two censuses, enforce the
        // combined limit. Cache metadata remains a fresh observation above.
        #[cfg(all(test, feature = "canonical-native-program"))]
        {
            let expected = ctx
                .retained_caller_fixed_inventory()?
                .ok_or_else(|| invalid("closed map census lost caller geometry"))?;
            if expected != caller.fixed_nodes {
                return Err(invalid(
                    "closed map caller census differs from complete inventory",
                ));
            }
        }
        Ok(Some(geometry))
    }
}
