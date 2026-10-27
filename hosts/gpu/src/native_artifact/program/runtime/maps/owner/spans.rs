//! Borrow a selected String key at its original range and selection visits.
//! The maintained String index handles Unicode and CRLF; no alternate index,
//! synthetic selector Value, map algorithm or canonical key is constructed.
use crate::native_artifact::program::runtime::invocation::MapExecution;
use crate::native_artifact::program::runtime::maps::owner::{State, add};
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{NativeExecutionPhase as Phase, NativeValueHandle};
use mech_core::{
    ManagedNodeReservation, MemoryBudgetDimension as Dimension, ResourceDemand, Value, ValueData,
};
use mech_engine::native_artifact::NativeSelectedStringSpanSelector;

pub(in crate::native_artifact::program::runtime::maps::owner) enum Lane {
    Selector {
        selector: NativeSelectedStringSpanSelector,
    },
    Span {
        handle: NativeValueHandle,
        owner: Value,
        bytes: core::ops::Range<usize>,
        _nodes: ManagedNodeReservation,
    },
}

impl State<'_> {
    pub(in crate::native_artifact::program::runtime::maps::owner) fn project_key(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
        args: [u64; 3],
    ) -> Result<(), Error> {
        self.guard(ctx, site)?;
        let layout = self
            .descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .ok_or_else(|| invalid("map key projection lacks layout"))?;
        let id = layout
            .span_site(site)
            .ok_or_else(|| invalid("unselected map key projection"))?;
        let proof = &layout.spans()[id];
        let region = &layout.regions()[proof.region()];
        if ctx.control().current != Some(region.iteration())
            || self.slots[proof.region()].binding.is_none()
        {
            return Err(invalid("map key projection outside owning loop"));
        }
        let selected = self.descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("map key projection lacks selected factory"))?;
        let origin = self.descriptor.metadata().plan.blocks()[site].origin();
        let geometry = self
            .geometry(ctx, site)?
            .ok_or_else(|| invalid("map key projection lacks caller geometry"))?;
        if proof.range_site() as usize == site {
            if args[2] != 0 || self.spans[id].is_some() {
                return Err(invalid("map key selector replay or malformed words"));
            }
            let bytes = selected.selected_u64_string_span_selector_metadata_bytes();
            let work = selected.selected_u64_string_span_selector_work();
            crate::native_artifact::program::runtime::collections::check(
                origin,
                Dimension::TemporaryBytes,
                add(geometry.live_bytes, bytes)?,
                ctx.limits().max_temporary_bytes,
            )?;
            crate::native_artifact::program::runtime::collections::check(
                origin,
                Dimension::ComputeWork,
                work,
                ctx.limits().max_compute_work,
            )?;
            ctx.work().charge(Phase::Setup, work)?;
            let _reservation = ctx.library().memory_budget().reserve_capacity(bytes)?;
            let selector = selected
                .selected_u64_string_span_selector(args[0], args[1])
                .map_err(|error| Error::Library { origin, error })?;
            self.spans[id] = Some(Lane::Selector { selector });
        } else {
            if args != [0; 3] || proof.selection_site() as usize != site {
                return Err(invalid("malformed map key selection"));
            }
            let Some(Lane::Selector { selector }) = self.spans[id].as_ref() else {
                return Err(invalid("map key selection before its range"));
            };
            let selector = *selector;
            let nodes = ctx
                .library()
                .memory_budget()
                .with_limits(
                    ctx.limits().max_temporary_bytes.unwrap_or(0),
                    ctx.limits().max_retained_nodes.unwrap_or(0),
                )
                .reserve_nodes(2)?;
            let handle = ctx.arena_current_handle(proof.source().get())?;
            let owner = ctx.arena_get(handle)?.clone();
            let crate::native_artifact::program::runtime::session::CallCacheEntry::Repeated(
                cache,
                _,
            ) = &ctx.caches().entries[site]
            else {
                return Err(invalid("map key selection lacks repeated cache"));
            };
            let coexistence = ResourceDemand {
                turn_peak_bytes: geometry
                    .live_bytes
                    .checked_sub(cache.retained_projection_bytes())
                    .ok_or_else(|| invalid("map key cache bytes exceed caller"))?,
                retained_nodes: geometry
                    .live_nodes
                    .checked_sub(cache.retained_projection_nodes())
                    .ok_or_else(|| invalid("map key cache nodes exceed caller"))?,
                ..ResourceDemand::default()
            };
            let limits = ctx.limits();
            let (caches, library) = ctx.caches_and_library();
            let bytes = caches
                .with_repeated(site, library, |cache| {
                    selected.selected_u64_string_span(
                        cache,
                        &owner,
                        &selector,
                        limits,
                        library,
                        coexistence,
                    )
                })
                .map_err(|error| Error::Library { origin, error })?;
            if !matches!(owner.data(), ValueData::String(text) if text.get(bytes.clone()).is_some())
            {
                return Err(invalid("map key planner returned invalid bounds"));
            }
            self.spans[id] = Some(Lane::Span {
                handle,
                owner,
                bytes,
                _nodes: nodes,
            });
        }
        self.last_owned_visit = Some(ctx.control().visits);
        Ok(())
    }
}
