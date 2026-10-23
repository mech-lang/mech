//! One already-evaluated String yield, never a producer or graph evaluator.
//! The selected factory/typed CFG proof owns every elided intermediate.
use super::{Context, Dimension, Error, State, add, charge, check, invalid, mul};
use crate::native_artifact::{NativeProgramDescriptor, NativeValueHandle};
use core::mem::size_of;
use mech_core::{Value, ValueData, snapshot::canonical_data_retained_footprint};
use mech_engine::native_artifact::{
    NativeCollectionResult, NativeGuardedHistogramQuery, NativeHistogramQueryPhase as Phase,
    NativeHistogramQueryStepKind as Step, NativeMatchValue,
};

#[path = "program_window_binding.rs"]
mod compiled_window;

pub(super) struct Builder {
    // Retire the captured root before the enclosing admitted metadata lane.
    query: Value,
    handle: NativeValueHandle,
    histogram_site: u32,
    query_bytes: u64,
    scalar_bytes: u64,
    scalar_nodes: u64,
    accepted: u64,
    matched: u64,
    pub(super) comparison: u64,
    phase: Option<Phase>,
}
impl Builder {
    pub(super) fn caller_retained_nodes(&self) -> Result<u64, Error> {
        // The captured query is also covered by the snapshot census. Keep a
        // conservative bound for private phase/scalar state without a scan.
        add(self.scalar_nodes, 2)
    }
}

fn sink(
    descriptor: &NativeProgramDescriptor,
    id: u32,
) -> Result<&NativeGuardedHistogramQuery, Error> {
    match descriptor
        .plan
        .collections()
        .get(id as usize)
        .map(|data| data.result())
    {
        Some(NativeCollectionResult::GuardedHistogramQuery(value)) => Ok(value),
        _ => Err(invalid("histogram query lacks selected strategy")),
    }
}

impl State {
    fn query_builder(&self, id: u32) -> Result<&Builder, Error> {
        self.builders
            .get(id as usize)
            .and_then(Option::as_ref)
            .and_then(|builder| builder.query.as_ref())
            .ok_or_else(|| invalid("histogram query has no current invocation"))
    }

    /// `external` is debited immediately afterwards by the existing scalar
    /// constructor/arena transport, rather than charged twice globally. It is
    /// nevertheless admitted in this component's finite cumulative allowance.
    fn query_account(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        clones: u64,
        compute: u64,
        comparison: u64,
        external: u64,
    ) -> Result<(), Error> {
        let origin = ctx.control.descriptor.plan.collections()[id as usize].origin();
        let builder = self
            .builders
            .get(id as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("histogram query accounting without builder"))?;
        let cloned = add(builder.cloned_bytes, clones)?;
        let used = add(builder.compute, add(compute, external)?)?;
        let compared = add(
            builder.query.as_ref().map_or(0, |q| q.comparison),
            comparison,
        )?;
        check(
            origin,
            Dimension::ClonedBytes,
            cloned,
            ctx.limits.max_cloned_bytes,
        )?;
        check(
            origin,
            Dimension::ComputeWork,
            used,
            ctx.limits.max_compute_work,
        )?;
        check(
            origin,
            Dimension::ComparisonWork,
            compared,
            ctx.limits.max_comparison_work,
        )?;
        charge(&mut ctx.control.work, compute)?;
        ctx.library
            .charge(0, comparison)
            .map_err(|error| Error::Collection { origin, error })?;
        let builder = self.builders[id as usize].as_mut().unwrap();
        builder.cloned_bytes = cloned;
        builder.compute = used;
        if let Some(query) = builder.query.as_mut() {
            query.comparison = compared;
        }
        Ok(())
    }

    pub(super) fn begin_histogram_query(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let NativeCollectionResult::GuardedHistogramQuery(definition) =
            ctx.control.descriptor.plan.collections()[id as usize].result()
        else {
            return Ok(());
        };
        // Fixed header/sizeof arithmetic only. Complete immutable selected
        // association is proved by the descriptor constructor; each original
        // runtime stage still funds its fixed header and dynamic checks.
        self.query_account(
            ctx,
            id,
            0,
            add(
                mul(definition.steps().len() as u64, 128)?,
                super::histogram_layout::CERTIFICATION_WORK,
            )?,
            0,
            0,
        )?;
        let work = ctx.control.work.limit().min(
            ctx.limits
                .max_compute_work
                .ok_or_else(|| invalid("histogram query needs finite compute"))?,
        );
        let producer_parameters =
            ctx.control.descriptor.plan.values()[ctx.control.descriptor.plan.collections()
                [id as usize]
                .output()
                .get() as usize]
                .definition()
                .dimension_parameters()
                .len();
        let parameters = definition
            .steps()
            .iter()
            .filter_map(|step| step.template())
            .flat_map(|template| {
                template
                    .inputs()
                    .iter()
                    .chain(core::iter::once(template.output()))
            })
            .map(|schema| schema.dimension_parameters().len())
            .max()
            .unwrap_or(0)
            .max(producer_parameters);
        let histogram_site = definition
            .steps()
            .iter()
            .find(|step| step.kind() == Step::Histogram)
            .ok_or_else(|| invalid("histogram query lacks maintained layout witness"))?
            .site();
        let histogram = ctx.control.descriptor.calls[histogram_site as usize]
            .as_ref()
            .ok_or_else(|| invalid("histogram query lacks maintained layout witness"))?;
        let scratch = histogram
            .selected_histogram_original_scratch_bytes(work / 128)
            .map_err(|_| {
                invalid("histogram policy has no maintained scratch layout certificate")
            })?;
        super::histogram_layout::certify(
            work,
            parameters,
            scratch,
            ctx.control.descriptor.scalar_bytes,
        )?;
        let handle = ctx
            .arena
            .current_handle(definition.query().get(), &mut ctx.control.work)?;
        let query = ctx.arena.get(handle, &mut ctx.control.work)?.clone();
        if !matches!(query.data(), ValueData::String(_)) {
            return Err(invalid("histogram query captured non-String"));
        }
        let query_bytes = query
            .memory_budget_retained_bytes(&ctx.library.memory_budget())
            .ok_or_else(|| invalid("histogram query captured unadmitted owner"))?;
        let mut scalar_bytes = 0;
        let mut scalar_nodes = 0;
        for step in definition.steps() {
            if matches!(step.kind(), Step::Membership | Step::Lookup) {
                let output = step
                    .output()
                    .ok_or_else(|| invalid("histogram query scalar stage lacks output"))?;
                if let Some(bytes) = ctx
                    .arena
                    .current_retained_bytes(output.get(), &mut ctx.control.work)?
                {
                    scalar_bytes = add(scalar_bytes, bytes)?;
                    scalar_nodes = add(scalar_nodes, 2)?;
                }
            }
        }
        // Previously published scalar roots remain owned by the arena even
        // between invocations. State already tracks their full bytes/nodes;
        // this private capture must not count them twice or release them.
        if scalar_bytes > self.query_scalar_bytes || scalar_nodes > self.query_scalar_nodes {
            return Err(invalid("histogram query lost scalar owner accounting"));
        }
        let origin = ctx.control.descriptor.plan.collections()[id as usize].origin();
        check(
            origin,
            Dimension::TemporaryBytes,
            add(self.live_bytes, query_bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        check(
            origin,
            Dimension::RetainedNodes,
            add(self.retained_nodes()?, 4)?,
            ctx.limits.max_retained_nodes,
        )?;
        self.builders[id as usize].as_mut().unwrap().query = Some(Builder {
            query,
            handle,
            histogram_site,
            query_bytes,
            scalar_bytes,
            scalar_nodes,
            accepted: 0,
            matched: 0,
            comparison: 0,
            phase: None,
        });
        self.live_bytes = add(self.live_bytes, query_bytes)?;
        Ok(())
    }

    pub(super) fn push_histogram_query(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        value: NativeMatchValue,
    ) -> Result<(), Error> {
        let yielded = self.take_string_yield(ctx, id, value)?;
        let result = self.append_histogram_query(ctx, id, &yielded);
        self.release_string_yield(yielded)?;
        result
    }

    fn append_histogram_query(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        yielded: &super::string_span::Yield,
    ) -> Result<(), Error> {
        self.builder(id)?;
        let part = yielded.text()?;
        let query = self.query_builder(id)?;
        if query.phase.is_some() {
            return Err(invalid("histogram query append after park"));
        }
        let ValueData::String(key) = query.query.data() else {
            return Err(invalid("histogram query key kind changed"));
        };
        let pair = add(part.len().max(key.len()) as u64, 8)?;
        let accepted = add(query.accepted, 1)?;
        let scalar_nodes = self.retained_nodes()?;
        let origin = ctx.control.descriptor.plan.collections()[id as usize].origin();
        // Pay exact source-alias header inspection before scanning handles.
        self.query_account(ctx, id, 0, self.string_yield_alias_work(yielded)?, 0, 0)?;
        let untracked = self.untracked_string_yield(ctx, yielded)?;
        check(
            origin,
            Dimension::TemporaryBytes,
            add(self.live_bytes, untracked)?,
            ctx.limits.max_temporary_bytes,
        )?;
        check(
            origin,
            Dimension::RetainedNodes,
            add(scalar_nodes, 4)?,
            ctx.limits.max_retained_nodes,
        )?;
        // Entire worst-case raw key equality and checked-counter update are
        // prepaid before the first byte read. Never stop after the first hit.
        self.query_account(ctx, id, 0, add(pair, 128)?, pair, 0)?;
        let query = self.query_builder(id)?;
        let current = ctx.arena.get(query.handle, &mut ctx.control.work)?;
        if !query.query.shares_frozen_storage(current) {
            return Err(invalid("histogram query generation changed"));
        }
        let ValueData::String(key) = query.query.data() else {
            unreachable!("validated query String")
        };
        let histogram = ctx.control.descriptor.calls[query.histogram_site as usize]
            .as_ref()
            .ok_or_else(|| invalid("histogram query lost maintained comparison witness"))?;
        // The maintained family's exact raw UTF-8 order supplies equality;
        // no normalization, alphabet, source text or workload name is involved.
        let equal = histogram
            .selected_histogram_compare(part, key)
            .map_err(|error| Error::Library {
                origin: histogram.template().origin(),
                error,
            })?
            .is_eq();
        let matched = if equal {
            histogram
                .selected_histogram_increment(query.matched)
                .map_err(|error| Error::Library {
                    origin: histogram.template().origin(),
                    error,
                })?
        } else {
            query.matched
        };
        let query = self.builders[id as usize]
            .as_mut()
            .unwrap()
            .query
            .as_mut()
            .unwrap();
        query.accepted = accepted;
        query.matched = matched;
        Ok(())
    }

    pub(super) fn park_histogram_query(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        self.builder(id)?;
        let definition = &ctx.control.descriptor.plan.collections()[id as usize];
        let metadata = ctx.control.descriptor.scalar_bytes;
        check(
            definition.origin(),
            Dimension::TemporaryBytes,
            add(self.live_bytes, metadata)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.query_account(ctx, id, 0, add(mul(metadata, 8)?, 64)?, 0, 0)?;
        let _scratch = ctx.library.memory_budget().reserve_capacity(metadata)?;
        let query = self.query_builder(id)?;
        if query.phase.is_some() {
            return Err(invalid("duplicate histogram query producer finish"));
        }
        mech_core::shape_for_resolved_extents(
            ctx.control.descriptor.plan.values()[definition.output().get() as usize].definition(),
            &[1, query.accepted],
        )
        .map_err(|_| invalid("collected output shape"))?;
        self.builders[id as usize]
            .as_mut()
            .unwrap()
            .query
            .as_mut()
            .unwrap()
            .phase = Some(Phase::ProducerReady);
        self.current = definition.parent_collection();
        Ok(())
    }

    pub(super) fn step_histogram_query(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        ordinal: u32,
    ) -> Result<Option<u64>, Error> {
        let definition = sink(ctx.control.descriptor, id)?;
        let step = definition
            .steps()
            .get(ordinal as usize)
            .ok_or_else(|| invalid("histogram query step outside proof"))?;
        let collection = &ctx.control.descriptor.plan.collections()[id as usize];
        if self.current != collection.parent_collection() {
            return Err(invalid(
                "histogram query consumer outside lexical invocation",
            ));
        }
        let query = self.query_builder(id)?;
        let expected = if matches!(step.kind(), Step::Cleanup) && query.matched > 0 {
            Phase::LookupRead
        } else {
            step.required_phase()
        };
        if query.phase != Some(expected)
            || (matches!(step.kind(), Step::Lookup) && query.matched == 0)
        {
            return Err(invalid("histogram query stage order or lazy arm differs"));
        }
        // Saved generation and same-account arena are checked on every stage,
        // including the false arm's cleanup. No source/output payload is read.
        ctx.arena.get(query.handle, &mut ctx.control.work)?;
        // The immutable descriptor already proved this SAME step's complete
        // selected template against its actual artifact and maintained factory.
        // No binder, planner, schema clone or intermediate Value is built here.
        // Preserve the existing fixed stage-header allowance for association,
        // counter/phase reads and transition; dynamic checks remain per visit.
        self.query_account(ctx, id, 0, 128, 0, 0)?;
        ctx.control
            .descriptor
            .query_stage(id, ordinal)
            .map_err(invalid)?;
        let word = match step.kind() {
            Step::Membership => Some(u64::from(self.query_builder(id)?.matched != 0)),
            Step::Lookup => Some(self.query_builder(id)?.matched),
            _ => None,
        };
        if let Some(word) = word {
            let output = step
                .output()
                .ok_or_else(|| invalid("histogram query omitted scalar output"))?;
            let ty = &ctx.control.descriptor.plan.values()[output.get() as usize];
            // No new scalar authority: reuse the existing admitted constructor
            // and Value arena, including schema/shape validation and complete
            // native-account ownership plus unchanged 4*retained+32 retirement.
            let raw = if matches!(step.kind(), Step::Membership) {
                ValueData::Bool(word != 0)
            } else {
                ValueData::U64(word)
            };
            let footprint = canonical_data_retained_footprint(ty.definition().body(), &raw)
                .map_err(|_| invalid("histogram query scalar footprint"))?;
            let output_bytes = add(footprint.retained_bytes, size_of::<Value>() as u64)?;
            let previous = ctx
                .arena
                .current_retained_bytes(output.get(), &mut ctx.control.work)?;
            // The new scalar coexists with every old scalar owner until put;
            // replacing an occupied slot is not an early node credit.
            let nodes = add(self.retained_nodes()?, 4)?;
            check(
                collection.origin(),
                Dimension::OutputElements,
                1,
                ctx.limits.max_output_elements,
            )?;
            check(
                collection.origin(),
                Dimension::OutputBytes,
                output_bytes,
                ctx.limits.max_output_bytes,
            )?;
            check(
                collection.origin(),
                Dimension::RetainedNodes,
                nodes,
                ctx.limits.max_retained_nodes,
            )?;
            let envelope = ctx.control.descriptor.scalar_bytes;
            check(
                collection.origin(),
                Dimension::TemporaryBytes,
                add(self.live_bytes, envelope)?,
                ctx.limits.max_temporary_bytes,
            )?;
            let setup = add(mul(envelope, 8)?, 128)?;
            self.query_account(ctx, id, footprint.encoded_bytes, 128, 0, setup)?;
            // The test-only same-account pressure surrounds only the actual
            // scalar constructor reservation. It cannot alter the
            // policy, constructor, selected body, or old-owner lifetime.
            #[cfg(all(test, feature = "canonical-native-program"))]
            let scalar_pressure = super::super::histogram_query_observation_tests::before_scalar(
                self, ctx, id, ordinal, previous,
            );
            let value = ctx.scalar(output, word)?;
            #[cfg(all(test, feature = "canonical-native-program"))]
            drop(scalar_pressure);
            let retained = value
                .memory_budget_retained_bytes(&ctx.library.memory_budget())
                .ok_or_else(|| invalid("histogram query scalar is unadmitted"))?;
            self.query_account(ctx, id, 0, 0, 0, add(mul(retained, 4)?, 64)?)?;
            let old = previous.unwrap_or(0);
            let live_bytes = add(
                self.live_bytes
                    .checked_sub(old)
                    .ok_or_else(|| invalid("histogram query previous scalar accounting"))?,
                retained,
            )?;
            let scalar_bytes = add(
                self.query_scalar_bytes
                    .checked_sub(old)
                    .ok_or_else(|| invalid("histogram query aggregate scalar accounting"))?,
                retained,
            )?;
            let scalar_nodes = if previous.is_none() {
                add(self.query_scalar_nodes, 2)?
            } else {
                self.query_scalar_nodes
            };
            let query = self.query_builder(id)?;
            let local_bytes = add(
                query
                    .scalar_bytes
                    .checked_sub(old)
                    .ok_or_else(|| invalid("histogram query previous scalar owner accounting"))?,
                retained,
            )?;
            let local_nodes = if previous.is_none() {
                add(query.scalar_nodes, 2)?
            } else {
                query.scalar_nodes
            };
            ctx.arena.put(output.get(), &value, &mut ctx.control.work)?;
            // Replacement credit begins only AFTER put dropped the old root.
            self.live_bytes = live_bytes;
            self.query_scalar_bytes = scalar_bytes;
            self.query_scalar_nodes = scalar_nodes;
            let query = self.builders[id as usize]
                .as_mut()
                .unwrap()
                .query
                .as_mut()
                .unwrap();
            query.scalar_bytes = local_bytes;
            query.scalar_nodes = local_nodes;
        }
        if matches!(step.kind(), Step::Cleanup) {
            let query = self.query_builder(id)?;
            let released = query.query_bytes;
            let builder = self.builders[id as usize].take().unwrap();
            // Only the private capture pin ends. Its original source owner
            // keeps its own admission. Genuine Bool/U64 arena owners remain
            // in State.live_bytes/query_scalar_* through all later stages and
            // repetitions; they are NOT physical-release credit at Cleanup.
            drop(builder);
            self.live_bytes = self
                .live_bytes
                .checked_sub(released)
                .ok_or_else(|| invalid("histogram query completion accounting"))?;
        } else {
            self.builders[id as usize]
                .as_mut()
                .unwrap()
                .query
                .as_mut()
                .unwrap()
                .phase = step.resulting_phase();
        }
        Ok(word)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct QueryDemand {
    pub collection: u32,
    pub step: Option<u32>,
    pub accepted: u64,
    pub matched: u64,
    pub phase: Option<Phase>,
    pub compute: u64,
    pub cloned: u64,
    pub comparison_required: u64,
    pub temporary_required: u64,
    pub reservation_bytes: u64,
    pub query_bytes: u64,
    pub scalar_bytes: u64,
    pub live_bytes: u64,
}
#[cfg(all(test, feature = "canonical-native-program"))]
impl State {
    pub(in crate::native_artifact) const fn test_query_scalar_owners(&self) -> (u64, u64) {
        (self.query_scalar_bytes, self.query_scalar_nodes)
    }
    pub(in crate::native_artifact) fn test_query_state(&self, id: u32) -> Option<QueryDemand> {
        let builder = self.builders.get(id as usize)?.as_ref()?;
        let query = builder.query.as_ref()?;
        Some(QueryDemand {
            collection: id,
            step: None,
            accepted: query.accepted,
            matched: query.matched,
            phase: query.phase,
            compute: builder.compute,
            cloned: builder.cloned_bytes,
            comparison_required: query.comparison,
            temporary_required: self.live_bytes,
            reservation_bytes: 0,
            query_bytes: query.query_bytes,
            scalar_bytes: query.scalar_bytes,
            live_bytes: self.live_bytes,
        })
    }
    /// Test-only, header-only observations, without work/allocation or scans of
    /// String payloads. It cannot select a body, alter caps or admit a helper.
    pub(in crate::native_artifact) fn test_query_demand(
        &self,
        ctx: &Context<'_, '_>,
        site: usize,
    ) -> Option<QueryDemand> {
        use mech_engine::native_artifact::NativeIterationAction as Action;
        let plan = &ctx.control.descriptor.plan;
        let (id, value, ordinal) = match plan.blocks().get(site)?.action() {
            Action::CollectionPush { collection, value } => (*collection, Some(*value), None),
            Action::HistogramQueryStep { collection, step } => (*collection, None, Some(*step)),
            _ => return None,
        };
        let mut demand = self.test_query_state(id)?;
        demand.step = ordinal;
        let query = self.query_builder(id).ok()?;
        let ValueData::String(key) = query.query.data() else {
            return None;
        };
        if let Some(value) = value {
            let value = ctx.arena.test_current_value(value.get())?;
            let ValueData::String(text) = value.data() else {
                return None;
            };
            let untracked = if self.sources.iter().flatten().any(|source| {
                source
                    .element
                    .and_then(|(handle, _)| ctx.arena.test_handle_value(handle))
                    .is_some_and(|tracked| value.shares_frozen_storage(tracked))
            }) {
                0
            } else {
                value.memory_budget_retained_bytes(&ctx.library.memory_budget())?
            };
            demand.comparison_required = query
                .comparison
                .checked_add(text.len().max(key.len()) as u64)?
                .checked_add(8)?;
            demand.temporary_required = self.live_bytes.checked_add(untracked)?;
        } else {
            let step = sink(ctx.control.descriptor, id)
                .ok()?
                .steps()
                .get(ordinal? as usize)?;
            if matches!(step.kind(), Step::Membership | Step::Lookup) {
                demand.reservation_bytes = ctx.control.descriptor.scalar_bytes;
                demand.temporary_required =
                    self.live_bytes.checked_add(demand.reservation_bytes)?;
            } else {
                // A fixed immutable selected-stage lookup has no allocation.
                demand.reservation_bytes = 0;
                demand.temporary_required = self.live_bytes;
            }
        }
        Some(demand)
    }
}
