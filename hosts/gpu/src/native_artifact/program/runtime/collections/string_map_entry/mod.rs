//! A compiler-proved borrowed String element and selected Map entry. Neither
//! absent semantic Value exists in the arena; generated code retains all visits.
use crate::native_artifact::NativeValueHandle;
use crate::native_artifact::program::runtime::collections::string_span::{Visit, Yield};
use crate::native_artifact::program::runtime::collections::{
    SourceBacking, State, add, check, data, mul,
};
use crate::native_artifact::program::runtime::invocation::CollectionExecution;
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use mech_core::{MemoryBudgetDimension as Dimension, ResourceDemand, Value, ValueData, WorkDemand};
use mech_engine::native_artifact::{NativeControlLoop, NativeMatchValue};
#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime::collections) use crate::native_artifact::program::runtime::collections::string_map_entry::tests::before_stage;

pub(in crate::native_artifact::program::runtime::collections) enum Lane {
    Key {
        visit: Visit,
        handle: NativeValueHandle,
        owner: Value,
        bytes: core::ops::Range<usize>,
        retained: u64,
    },
    Entry {
        visit: Visit,
        key_handle: NativeValueHandle,
        key_owner: Value,
        map_handle: NativeValueHandle,
        map_owner: Value,
        ordinal: usize,
        retained: u64,
        nodes: u64,
    },
}

impl State {
    pub(in crate::native_artifact::program::runtime::collections) fn prepare_string_map_key(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        iteration: u32,
        ordinal: u64,
    ) -> Result<(), Error> {
        let descriptor = ctx.descriptor();
        let NativeControlLoop::Collection(generator) =
            &descriptor.metadata().plan.iterations()[iteration as usize]
        else {
            return Err(invalid("String map key outside a collection generator"));
        };
        let id = generator.collection();
        self.builder(id)?;
        let proof = descriptor.metadata().plan.collections()[id as usize]
            .string_element_map_projection()
            .ok_or_else(|| invalid("String map key lacks selected proof"))?;
        if proof.iteration() != iteration
            || proof.element_site() as usize != site
            || self.string_maps[id as usize].is_some()
        {
            return Err(invalid(
                "String map key outside its fresh original element visit",
            ));
        }
        let source = self.sources[iteration as usize]
            .as_ref()
            .ok_or_else(|| invalid("String map key has no current source"))?;
        let (SourceBacking::Value(owner), Some(handle)) = (&source.backing, source.handle) else {
            return Err(invalid("String map key lacks its genuine source handle"));
        };
        // This handle was captured with this exact source at Initialize.
        // Reject replacement before borrowing the maintained cursor's bytes.
        ctx.arena_get(handle)?;
        let (data::Element::Text(text), next) = data::element(owner, ordinal, source.byte_offset)?
        else {
            return Err(invalid("String map key source element differs"));
        };
        let start = source.byte_offset;
        if next
            != start
                .checked_add(text.len())
                .ok_or_else(|| invalid("String map key cursor overflow"))?
            || !matches!(owner.data(), ValueData::String(_))
        {
            return Err(invalid("String map key cursor is not a String span"));
        }
        let bytes = text.len() as u64;
        let retained = owner
            .memory_budget_retained_bytes(&ctx.library().memory_budget())
            .ok_or_else(|| invalid("String map key has no complete same-account owner"))?;
        // Keep the maintained cursor/header work. No key bytes are copied, so
        // the eliminated canonical element's clone traffic is genuinely zero.
        self.account(ctx, id, 0, add(mul(bytes, 16)?, 128)?)?;
        let live = add(self.live_bytes, retained)?;
        let nodes = add(self.string_map_nodes, 2)?;
        check(
            generator.origin(),
            Dimension::TemporaryBytes,
            live,
            ctx.limits().max_temporary_bytes,
        )?;
        check(
            generator.origin(),
            Dimension::RetainedNodes,
            add(add(self.query_scalar_nodes, self.string_span_nodes)?, nodes)?,
            ctx.limits().max_retained_nodes,
        )?;
        let visit = Visit::current(ctx, site)?;
        let active = add(self.active_string_maps, 1)?;
        if active > self.string_maps.len() as u64 {
            return Err(invalid(
                "String map active lane counter exceeds admitted storage",
            ));
        }
        let source = self.sources[iteration as usize].as_mut().unwrap();
        let SourceBacking::Value(owner) = &source.backing else {
            unreachable!("source variant checked");
        };
        self.string_maps[id as usize] = Some(Lane::Key {
            visit,
            handle,
            owner: owner.clone(),
            bytes: start..next,
            retained,
        });
        source.byte_offset = next;
        self.live_bytes = live;
        self.string_map_nodes = nodes;
        self.active_string_maps = active;
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime::collections) fn project_string_map_entry(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        id: u32,
    ) -> Result<(), Error> {
        self.builder(id)?;
        let descriptor = ctx.descriptor();
        let proof = descriptor.metadata().plan.collections()[id as usize]
            .string_element_map_projection()
            .ok_or_else(|| invalid("String map projection lacks selected proof"))?;
        if proof.selection_site() as usize != site {
            return Err(invalid("String map projection at another original site"));
        }
        let Some(Lane::Key {
            visit,
            handle,
            owner,
            bytes,
            retained,
        }) = self.string_maps[id as usize].as_ref()
        else {
            return Err(invalid("String map entry before key or after replay"));
        };
        if *visit != Visit::current(ctx, site)? {
            return Err(invalid("String map key belongs to another candidate"));
        }
        if ctx.arena_current_handle(proof.source().get())? != *handle {
            return Err(invalid(
                "String map key belongs to another source generation",
            ));
        }
        ctx.arena_get(*handle)?;
        let (visit, key_handle, key_owner, key_bytes, key_retained) =
            (*visit, *handle, owner.clone(), bytes.clone(), *retained);
        let selected = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("String map entry lacks actual selected factory"))?;
        // Map acquisition remains LAZY at this original MapKeyAccess visit.
        // Empty generators never inspect a map or reserve its owner footprint.
        let map_handle = ctx.arena_current_handle(proof.map().get())?;
        let map_owner = ctx.arena_get(map_handle)?.clone();
        let map_retained = map_owner
            .memory_budget_retained_bytes(&ctx.library().memory_budget())
            .ok_or_else(|| invalid("String map source has no complete same-account owner"))?;
        let ValueData::Map(map) = map_owner.data() else {
            return Err(invalid("String map source is not Map"));
        };
        // Exact canonical retained-data nodes: one Value/root Map plus TWO
        // scalar String child nodes per entry. Schema/import owner BYTES are
        // separately included in the complete retained owner bound above.
        let map_nodes = add(mul(map.entries().len() as u64, 2)?, 2)?;
        let (prior_clones, prior_compute, prior_comparison) = {
            let builder = self.builder(id)?;
            (
                builder.cloned_bytes,
                builder.compute,
                builder.projection_comparison,
            )
        };
        let coexistence = ResourceDemand {
            turn_peak_bytes: add(self.live_bytes, map_retained)?,
            retained_nodes: add(self.retained_nodes()?, map_nodes)?,
            cloned_bytes: prior_clones,
            work: WorkDemand {
                compute: prior_compute,
                comparison: prior_comparison,
                ..WorkDemand::default()
            },
            ..ResourceDemand::default()
        };
        let before = ctx.library().used();
        let outcome = selected.selected_string_map_entry(
            &map_owner,
            &key_owner,
            key_bytes,
            ctx.limits(),
            &ctx.library(),
            coexistence,
        );
        let after = ctx.library().used();
        #[cfg(all(test, feature = "canonical-native-program"))]
        tests::after_project(
            site,
            outcome.is_ok(),
            coexistence.turn_peak_bytes,
            coexistence.retained_nodes,
        );
        let compute = after
            .0
            .checked_sub(before.0)
            .ok_or_else(|| invalid("String map work moved backwards"))?;
        let comparison = after
            .1
            .checked_sub(before.1)
            .ok_or_else(|| invalid("String map comparison moved backwards"))?;
        // The maintained context checked BEFORE each same-ledger debit. Keep
        // both cumulative component counters even on an actual lookup denial.
        let builder = self.builder(id)?;
        builder.compute = add(prior_compute, compute)?;
        builder.projection_comparison = add(prior_comparison, comparison)?;
        let ordinal = outcome.map_err(|error| Error::Library {
            origin: proof.selection_template().origin(),
            error,
        })?;
        let retained = add(key_retained, map_retained)?;
        let nodes = add(2, map_nodes)?;
        self.string_maps[id as usize] = Some(Lane::Entry {
            visit,
            key_handle,
            key_owner,
            map_handle,
            map_owner,
            ordinal,
            retained,
            nodes,
        });
        self.live_bytes = coexistence.turn_peak_bytes;
        self.string_map_nodes = add(self.string_map_nodes, map_nodes)?;
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime::collections) fn take_string_map_yield(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        id: u32,
        value: NativeMatchValue,
    ) -> Result<Yield, Error> {
        self.builder(id)?;
        let collection = &ctx.descriptor().metadata().plan.collections()[id as usize];
        let proof = collection
            .string_element_map_projection()
            .ok_or_else(|| invalid("String map sink lacks selected proof"))?;
        let site = ctx
            .ledger()
            .previous
            .ok_or_else(|| invalid("String map yield before visit"))?;
        if site != collection.push() || value != proof.output() {
            return Err(invalid("String map entry consumed by another sink"));
        }
        let Some(Lane::Entry {
            visit,
            key_handle,
            map_handle,
            ..
        }) = self.string_maps[id as usize].as_ref()
        else {
            return Err(invalid("String map entry absent or replayed"));
        };
        if *visit != Visit::current(ctx, site as usize)? {
            return Err(invalid("String map entry belongs to another candidate"));
        }
        if ctx.arena_current_handle(proof.source().get())? != *key_handle
            || ctx.arena_current_handle(proof.map().get())? != *map_handle
        {
            return Err(invalid(
                "String map entry belongs to another source generation",
            ));
        }
        ctx.arena_get(*key_handle)?;
        ctx.arena_get(*map_handle)?;
        let active = self
            .active_string_maps
            .checked_sub(1)
            .ok_or_else(|| invalid("String map active lane counter underflow"))?;
        let Lane::Entry {
            key_owner,
            map_owner,
            ordinal,
            retained,
            nodes,
            ..
        } = self.string_maps[id as usize].take().unwrap()
        else {
            unreachable!("entry phase checked before take");
        };
        self.active_string_maps = active;
        Ok(Yield {
            owner: map_owner,
            bytes: None,
            entry: Some(ordinal),
            _key_owner: Some(key_owner),
            accounted: retained,
            accounted_nodes: nodes,
        })
    }
}
