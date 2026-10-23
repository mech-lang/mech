//! One compiler-proved current-visit String span, never a canonical Value.
//! Original Range and String selector visits still run independently. The
//! selected sink borrows bytes only while consuming this private lane.
use super::{Action, Context, Dimension, Error, State, add, check, invalid};
use crate::native_artifact::NativeValueHandle;
use mech_core::{ResourceDemand, Value, ValueData, WorkDemand};
use mech_engine::native_artifact::{NativeMatchValue, NativeSelectedStringSpanSelector};

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_string_span_boundary_tests.rs"]
mod boundary_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_string_span_selector_tests.rs"]
mod selector_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_string_span_tests.rs"]
mod tests;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in super::super) use tests::before_helper;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) use tests::before_stage;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Visit {
    pub(super) iteration: u32,
    pub(super) remaining: u64,
}
impl Visit {
    pub(super) fn current(ctx: &Context<'_, '_>, site: usize) -> Result<Self, Error> {
        let iteration = ctx.control.descriptor.plan.blocks()[site]
            .iteration()
            .ok_or_else(|| invalid("String span outside a generated candidate"))?;
        let remaining = *ctx
            .control
            .progress
            .get(iteration as usize * 2)
            .ok_or_else(|| invalid("String span iteration outside progress"))?;
        Ok(Self {
            iteration,
            remaining,
        })
    }
}

pub(super) enum Lane {
    Selector {
        visit: Visit,
        selector: NativeSelectedStringSpanSelector,
    },
    Span {
        visit: Visit,
        handle: NativeValueHandle,
        owner: Value,
        bytes: core::ops::Range<usize>,
        retained: u64,
    },
}

/// An actual owner is always retained. `accounted` is zero for an ordinary
/// canonical yield, or its FULL owner bound already present in State.live.
/// No Value or handle is manufactured for the projected String itself.
pub(super) struct Yield {
    pub(super) owner: Value,
    pub(super) bytes: Option<core::ops::Range<usize>>,
    pub(super) entry: Option<usize>,
    // Keep the complete genuine key source alive through the final append.
    pub(super) _key_owner: Option<Value>,
    pub(super) accounted: u64,
    pub(super) accounted_nodes: u64,
}
impl Yield {
    pub(super) fn text(&self) -> Result<&str, Error> {
        if let Some(ordinal) = self.entry {
            let ValueData::Map(map) = self.owner.data() else {
                return Err(invalid("selected map yield owner is not Map"));
            };
            let Some(ValueData::String(text)) =
                map.entries().get(ordinal).map(|entry| entry.value())
            else {
                return Err(invalid("selected map entry changed"));
            };
            return Ok(text);
        }
        let ValueData::String(text) = self.owner.data() else {
            return Err(invalid("selected sink yield owner is not String"));
        };
        match &self.bytes {
            Some(bytes) => text
                .get(bytes.clone())
                .ok_or_else(|| invalid("String span bounds changed")),
            None => Ok(text),
        }
    }
}

impl State {
    pub(super) fn retained_nodes(&self) -> Result<u64, Error> {
        add(
            add(self.query_scalar_nodes, self.string_span_nodes)?,
            self.string_map_nodes,
        )
    }

    pub(in super::super) fn validate_span_boundary(&self, action: &Action) -> Result<(), Error> {
        if self.active_string_maps != 0
            && !matches!(
                action,
                Action::ProjectStringMapEntry { .. }
                    | Action::CollectionPush { .. }
                    | Action::Scalar(mech_engine::native_artifact::NativeMatchAction::None)
            )
        {
            return Err(invalid(
                "String map projection escaped its current candidate",
            ));
        }
        if matches!(
            action,
            Action::Initialize { .. }
                | Action::Latch { .. }
                | Action::CollectionElement { .. }
                | Action::CollectionBegin { .. }
                | Action::CollectionFinish { .. }
                | Action::TextJoinFinish { .. }
                | Action::HistogramFinish { .. }
                | Action::HistogramQueryStep { .. }
        ) && self.active_string_spans != 0
        {
            return Err(invalid("String span escaped its current candidate"));
        }
        Ok(())
    }

    pub(super) fn prepare_string_span(
        &mut self,
        ctx: &mut Context<'_, '_>,
        site: usize,
        id: u32,
        words: [u64; 2],
    ) -> Result<(), Error> {
        self.builder(id)?;
        if self.string_spans[id as usize].is_some() {
            return Err(invalid("String span selector replayed before consumption"));
        }
        let descriptor = ctx.control.descriptor;
        let proof = descriptor.plan.collections()[id as usize]
            .string_span_projection()
            .ok_or_else(|| invalid("String span selector lacks selected proof"))?;
        if proof.range_site() as usize != site {
            return Err(invalid("String span selector at another original site"));
        }
        let selected = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("String span selector lacks retained factory"))?;
        // Descriptor preparation already bound this immutable exact template.
        // A projected range does not build a full library binding or selector
        // Value. Pay only the maintained cardinality/shape/layout witness's
        // actual small scratch and validation work at this original visit.
        let bytes = selected.selected_u64_string_span_selector_metadata_bytes();
        check(
            proof.range_template().origin(),
            Dimension::TemporaryBytes,
            add(self.live_bytes, bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        let price = selected.selected_u64_string_span_selector_work();
        #[cfg(all(test, feature = "canonical-native-program"))]
        selector_tests::before_account(self, ctx, site, id, words, price);
        let account = self.account(ctx, id, 0, price);
        #[cfg(all(test, feature = "canonical-native-program"))]
        selector_tests::after_account(self, ctx, account.is_ok());
        account?;
        let _binding = ctx.library.memory_budget().reserve_capacity(bytes)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        selector_tests::before_constructor();
        let constructed = selected.selected_u64_string_span_selector(words[0], words[1]);
        #[cfg(all(test, feature = "canonical-native-program"))]
        selector_tests::after_constructor(ctx, constructed.is_ok());
        let selector = constructed.map_err(|error| Error::Library {
            origin: proof.range_template().origin(),
            error,
        })?;
        let visit = Visit::current(ctx, site)?;
        let active = add(self.active_string_spans, 1)?;
        if active > self.string_spans.len() as u64 {
            return Err(invalid("String span active lane counter exceeds storage"));
        }
        self.string_spans[id as usize] = Some(Lane::Selector { visit, selector });
        self.active_string_spans = active;
        #[cfg(all(test, feature = "canonical-native-program"))]
        selector_tests::after_install(self, id);
        Ok(())
    }

    pub(super) fn project_string_span(
        &mut self,
        ctx: &mut Context<'_, '_>,
        site: usize,
        id: u32,
    ) -> Result<(), Error> {
        self.builder(id)?;
        let descriptor = ctx.control.descriptor;
        let proof = descriptor.plan.collections()[id as usize]
            .string_span_projection()
            .ok_or_else(|| invalid("String span projection lacks selected proof"))?;
        if proof.selection_site() as usize != site {
            return Err(invalid("String span projection at another original site"));
        }
        let Some(Lane::Selector { visit, selector }) = self.string_spans[id as usize].as_ref()
        else {
            return Err(invalid("String span projection before its selector"));
        };
        let (visit, selector) = (*visit, *selector);
        if visit != Visit::current(ctx, site)? {
            return Err(invalid("String span selector belongs to another candidate"));
        }
        let selected = descriptor.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("String span projection lacks retained factory"))?;
        let handle = ctx
            .arena
            .current_handle(proof.source().get(), &mut ctx.control.work)?;
        // get() validates this SAME arena/account/generation before taking an
        // exact strong clone. No storage-content comparison deduplicates an
        // import/schema wrapper, and no leaf-sized claim replaces this owner.
        let owner = ctx.arena.get(handle, &mut ctx.control.work)?.clone();
        let retained = owner
            .memory_budget_retained_bytes(&ctx.library.memory_budget())
            .ok_or_else(|| invalid("String span source has no complete same-account owner"))?;
        let (prior_clones, prior_compute, prior_comparison) = {
            let builder = self.builder(id)?;
            let comparison = builder.histogram.as_ref().map_or_else(
                || builder.query.as_ref().map_or(0, |query| query.comparison),
                |histogram| histogram.comparisons,
            );
            (builder.cloned_bytes, builder.compute, comparison)
        };
        let super::super::session::CallCacheEntry::Repeated(cache, _) =
            &ctx.call_caches.entries[site]
        else {
            return Err(invalid(
                "String span projection lacks its repeated-site cache",
            ));
        };
        let old_bytes = cache.retained_projection_bytes();
        let old_nodes = cache.retained_projection_nodes();
        let coexistence = ResourceDemand {
            turn_peak_bytes: self
                .live_bytes
                .checked_sub(old_bytes)
                .ok_or_else(|| invalid("String span cache byte accounting"))?,
            retained_nodes: self
                .retained_nodes()?
                .checked_sub(old_nodes)
                .ok_or_else(|| invalid("String span cache node accounting"))?,
            cloned_bytes: prior_clones,
            work: WorkDemand {
                compute: prior_compute,
                comparison: prior_comparison,
                ..WorkDemand::default()
            },
            ..ResourceDemand::default()
        };
        // The maintained wrapper admits the full source owner + real selected
        // metadata/index residency, using the same finite account and Slot.
        // It does not create a selector Value or synthetic TurnMemoryPlan.
        let before_work = ctx.library.used();
        let outcome = ctx.call_caches.with_repeated(site, &ctx.library, |cache|
            selected.selected_u64_string_span(cache,
            &owner,
            &selector,
            ctx.limits,
            &ctx.library,
            coexistence,
        ));
        let super::super::session::CallCacheEntry::Repeated(cache, _) =
            &ctx.call_caches.entries[site] else { unreachable!("retained cache kind"); };
        let new_bytes = cache.retained_projection_bytes();
        let new_nodes = cache.retained_projection_nodes();
        #[cfg(all(test, feature = "canonical-native-program"))]
        boundary_tests::observe(site as u32, visit, outcome.is_ok(), new_bytes, new_nodes);
        let after_work = ctx.library.used();
        let compute = after_work
            .0
            .checked_sub(before_work.0)
            .ok_or_else(|| invalid("String span work account moved backwards"))?;
        let comparison = after_work
            .1
            .checked_sub(before_work.1)
            .ok_or_else(|| invalid("String span comparison account moved backwards"))?;
        let builder = self.builder(id)?;
        // The maintained projection context admitted these exact cumulative
        // amounts BEFORE each real debit. Record spent work on errors too;
        // never charge twice globally or renew the builder's finite allowance.
        builder.compute = add(prior_compute, compute)?;
        if let Some(histogram) = builder.histogram.as_mut() {
            histogram.comparisons = add(prior_comparison, comparison)?;
        } else if let Some(query) = builder.query.as_mut() {
            query.comparison = add(prior_comparison, comparison)?;
        } else if comparison != 0 {
            return Err(invalid(
                "String span index unexpectedly charged key comparisons",
            ));
        }
        self.live_bytes = add(coexistence.turn_peak_bytes, new_bytes)?;
        self.string_span_nodes = add(
            self.string_span_nodes
                .checked_sub(old_nodes)
                .ok_or_else(|| invalid("String span previous cache nodes"))?,
            new_nodes,
        )?;
        let bytes = outcome.map_err(|error| Error::Library {
            origin: proof.selection_template().origin(),
            error,
        })?;
        if !matches!(owner.data(), ValueData::String(text) if text.get(bytes.clone()).is_some()) {
            return Err(invalid(
                "String span planner returned invalid source bounds",
            ));
        }
        self.live_bytes = add(self.live_bytes, retained)?;
        self.string_span_nodes = add(self.string_span_nodes, 2)?;
        self.string_spans[id as usize] = Some(Lane::Span {
            visit,
            handle,
            owner,
            bytes,
            retained,
        });
        Ok(())
    }

    pub(super) fn take_string_yield(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        value: NativeMatchValue,
    ) -> Result<Yield, Error> {
        self.builder(id)?;
        let descriptor = ctx.control.descriptor;
        let collection = &descriptor.plan.collections()[id as usize];
        if collection.string_element_map_projection().is_some() {
            return self.take_string_map_yield(ctx, id, value);
        }
        if let Some(proof) = collection.string_span_projection() {
            let site = ctx
                .control
                .previous
                .ok_or_else(|| invalid("String span yield before visit"))?;
            if site != collection.push() || value != proof.output() {
                return Err(invalid("String span consumed at another sink"));
            }
            let Some(Lane::Span { visit, handle, .. }) = self.string_spans[id as usize].as_ref()
            else {
                return Err(invalid("String span yield absent or replayed"));
            };
            if *visit != Visit::current(ctx, site as usize)? {
                return Err(invalid("String span yield belongs to another candidate"));
            }
            ctx.arena.get(*handle, &mut ctx.control.work)?;
            let active = self
                .active_string_spans
                .checked_sub(1)
                .ok_or_else(|| invalid("String span active lane counter underflow"))?;
            let Lane::Span {
                owner,
                bytes,
                retained,
                ..
            } = self.string_spans[id as usize].take().unwrap()
            else {
                unreachable!("phase checked before taking this private lane")
            };
            self.active_string_spans = active;
            Ok(Yield {
                owner,
                bytes: Some(bytes),
                entry: None,
                _key_owner: None,
                accounted: retained,
                accounted_nodes: 2,
            })
        } else {
            Ok(Yield {
                owner: ctx.arena.copy_current(value.get(), &mut ctx.control.work)?,
                bytes: None,
                entry: None,
                _key_owner: None,
                accounted: 0,
                accounted_nodes: 0,
            })
        }
    }

    pub(super) fn untracked_string_yield(
        &self,
        ctx: &mut Context<'_, '_>,
        value: &Yield,
    ) -> Result<u64, Error> {
        if value.bytes.is_some() || value.entry.is_some() {
            Ok(0) // complete root is already in live_bytes, never span length
        } else {
            self.untracked_yield_bytes(ctx, Some(&value.owner))
        }
    }

    pub(super) fn string_yield_alias_work(&self, value: &Yield) -> Result<u64, Error> {
        if value.bytes.is_some() || value.entry.is_some() {
            // The pin's complete owner is already tracked. No source alias
            // scan occurs, so do not bill the old scan as fictitious work.
            Ok(0)
        } else {
            add(super::mul(self.sources.len() as u64, 64)?, 32)
        }
    }

    pub(super) fn release_string_yield(&mut self, value: Yield) -> Result<(), Error> {
        let bytes = value.accounted;
        let nodes = value.accounted_nodes;
        let mapped = value.entry.is_some();
        let projected = value.bytes.is_some() || mapped;
        // The actual strong pin is dropped BEFORE its local coexistence credit.
        // The source arena still owns its fully prepaid canonical retirement.
        drop(value);
        if projected {
            self.live_bytes = self
                .live_bytes
                .checked_sub(bytes)
                .ok_or_else(|| invalid("String span released owner accounting"))?;
            let counter = if mapped {
                &mut self.string_map_nodes
            } else {
                &mut self.string_span_nodes
            };
            *counter = counter
                .checked_sub(nodes)
                .ok_or_else(|| invalid("String projection released node accounting"))?;
        }
        Ok(())
    }
}
