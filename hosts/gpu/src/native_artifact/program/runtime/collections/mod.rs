//! Admitted element transport and collection construction, not body execution.
//! Generated blocks own every generator, filter, arithmetic op and backedge.
use crate::native_artifact::program::runtime::invocation::CollectionExecution;
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{
    NativeExecutionPhase as Phase, NativeProgramDescriptor, NativeValueHandle, NativeWorkMeter,
};
use core::mem::size_of;
use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
use mech_core::{
    ManagedMemoryBudget, ManagedMemoryReservation, ManagedNodeReservation,
    MemoryBudgetDimension as Dimension, MemoryBudgetLimits, ResidentKernelError,
    ResidentResourcePhase, Value, ValueData,
};
use mech_engine::ComprehensionKind;
use mech_engine::native_artifact::{
    NativeCollectionSource, NativeControlLoop, NativeIterationAction as Action,
    NativeMatchOrigin as Origin, NativeScalarKind,
};

#[path = "allocation/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod allocation;
#[path = "ascii_facts/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod ascii_facts;
pub(in crate::native_artifact::program::runtime) fn admitted_capacity<T>(
    values: Vec<T>,
    count: usize,
    reason: &'static str,
) -> Result<Vec<T>, Error> {
    allocation::exact_capacity(values, count, reason)
}
#[path = "data.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod data;
pub(in crate::native_artifact::program::runtime) fn source_count(
    value: &Value,
) -> Result<u64, Error> {
    data::count(value)
}
/// Optional constant-time scheduling hint for an already admitted Matrix/Set.
/// Atomic elements count as one unit; text uses its retained UTF-8 byte length.
/// This borrows no new owner and grants no source visit or execution authority.
pub(in crate::native_artifact::program::runtime) fn element_work_hint(
    value: &Value,
    ordinal: u64,
) -> u64 {
    if !matches!(value.data(), ValueData::Matrix(_) | ValueData::Set(_)) {
        return 1;
    }
    match data::element(value, ordinal, 0) {
        Ok((data::Element::Text(text), _)) => (text.len() as u64).max(1),
        _ => 1,
    }
}
#[path = "histogram/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod histogram;
#[path = "histogram_layout/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod histogram_layout;
#[path = "histogram_query/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod histogram_query;
#[path = "lifetime.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod lifetime;
#[path = "string_map_entry/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod string_map_entry;
#[path = "string_span/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod string_span;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::string_span::before_helper as before_span_helper;
#[path = "text_join/mod.rs"]
pub(in crate::native_artifact::program::runtime::collections) mod text_join;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::histogram::HistogramDemand;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::histogram::HistogramStage;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::histogram::HistogramVisit;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::histogram_query::QueryDemand;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::text_join::TextJoinDemand;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) use crate::native_artifact::program::runtime::collections::text_join::TextJoinStage;

struct Source {
    backing: SourceBacking,
    // Present only for a certified borrowed String-element generator.
    handle: Option<NativeValueHandle>,
    count: u64,
    index_offset: u64,
    byte_offset: usize,
    element: Option<(NativeValueHandle, u64)>,
}
#[derive(Clone)]
enum SourceBacking {
    Value(Value),
    U64Range { start: u64 },
    // Only the closed compiled String-map region may consume this source.
    // Its complete same-length result remains in the charged map buffer.
    ConsumedString,
}
#[derive(Clone, Copy)]
struct PreparedRange {
    start: u64,
    count: u64,
}
struct Builder {
    // Owned drafts and their backing are released before their reservation.
    values: Vec<ValueDataDraft>,
    join: Option<text_join::Builder>,
    query: Option<histogram_query::Builder>,
    histogram: Option<histogram::Builder>,
    payload_bytes: u64,
    cloned_bytes: u64,
    compute: u64,
    projection_comparison: u64,
    reservation: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
}
pub(in crate::native_artifact::program::runtime) struct State {
    sources: Vec<Option<Source>>,
    prepared_ranges: Vec<Option<PreparedRange>>,
    builders: Vec<Option<Builder>>,
    string_spans: Vec<Option<string_span::Lane>>,
    string_maps: Vec<Option<string_map_entry::Lane>>,
    current: Option<u32>,
    live_bytes: u64,
    // Genuine query scalar roots stay in the arena after token cleanup.
    // Keep their complete bound in every later collection workspace peak.
    query_scalar_bytes: u64,
    query_scalar_nodes: u64,
    // Admitted selected-projection cache owners and currently pinned String
    // roots coexist with every later sink and canonical scalar publication.
    string_span_nodes: u64,
    active_string_spans: u64,
    string_map_nodes: u64,
    active_string_maps: u64,
    // Only immutable representation facts; the weak shell retires before its
    // complete fixed metadata reservation below. No canonical root is kept.
    ascii_facts: ascii_facts::Cache,
    _metadata: ManagedMemoryReservation,
    _metadata_nodes: ManagedNodeReservation,
}
fn add(left: u64, right: u64) -> Result<u64, Error> {
    left.checked_add(right)
        .ok_or_else(|| invalid("native collection size/work overflow"))
}
fn mul(left: u64, right: u64) -> Result<u64, Error> {
    left.checked_mul(right)
        .ok_or_else(|| invalid("native collection size/work overflow"))
}
pub(in crate::native_artifact::program::runtime) fn check(
    origin: Origin,
    dimension: Dimension,
    required: u64,
    limit: Option<u64>,
) -> Result<(), Error> {
    let limit = limit.ok_or_else(|| invalid("collection requires a finite dimension"))?;
    if required > limit {
        return Err(Error::Collection {
            origin,
            error: ResidentKernelError::ResourceLimit {
                phase: ResidentResourcePhase::Admission,
                dimension,
                required,
                limit,
            },
        });
    }
    Ok(())
}
pub(in crate::native_artifact::program::runtime) fn charge(
    work: &mut NativeWorkMeter,
    amount: u64,
) -> Result<(), Error> {
    work.charge(Phase::Setup, amount).map_err(Into::into)
}
impl State {
    /// Only otherwise-unclaimed private owners in the parked parent. The
    /// common lane, drafts, TextJoin and canonical roots have their own claims.
    pub(in crate::native_artifact::program::runtime) fn parked_private_nodes(
        &self,
        work: &mut NativeWorkMeter,
    ) -> Result<Option<u64>, Error> {
        if self.active_string_spans != 0 || self.active_string_maps != 0 {
            return Ok(None);
        }
        charge(
            work,
            crate::native_artifact::program::runtime::amount(self.builders.len(), 256, 512)?,
        )?;
        let mut nodes = 0;
        for builder in self.builders.iter().flatten() {
            if builder
                .join
                .as_ref()
                .is_some_and(|join| join.has_compiled_map())
            {
                return Ok(None);
            }
            if let Some(histogram) = &builder.histogram {
                nodes = add(nodes, histogram.caller_retained_nodes()?)?;
            }
            if builder.query.is_some() {
                // Query scalars already have canonical snapshot claims.
                nodes = add(nodes, 2)?;
            }
        }
        Ok(Some(nodes))
    }

    pub(in crate::native_artifact::program::runtime) fn caller_retained_nodes(
        &self,
        work: &mut NativeWorkMeter,
    ) -> Result<u64, Error> {
        charge(
            work,
            crate::native_artifact::program::runtime::amount(self.builders.len(), 256, 512)?,
        )?;
        // This reservation contains fixed per-site carriers and weak ASCII
        // metadata, not collection payloads. Its capacity bounds those nodes.
        let mut nodes = add(self._metadata.capacity_bytes(), self.retained_nodes()?)?;
        for builder in self.builders.iter().flatten() {
            // CollectionPush accepts only scalar/Index/String drafts. Each
            // draft contributes at most one node, plus the flat carrier/root.
            // Bodies and input payloads are never traversed here.
            nodes = add(nodes, add(builder.values.len() as u64, 2)?)?;
            if let Some(join) = &builder.join {
                nodes = add(nodes, join.caller_retained_nodes())?;
            }
            if let Some(histogram) = &builder.histogram {
                nodes = add(nodes, histogram.caller_retained_nodes()?)?;
            }
            if let Some(query) = &builder.query {
                nodes = add(nodes, query.caller_retained_nodes()?)?;
            }
        }
        Ok(nodes)
    }

    pub(in crate::native_artifact::program::runtime) fn retained_demand(
        &self,
    ) -> Result<mech_core::ResourceDemand, Error> {
        Ok(mech_core::ResourceDemand {
            turn_peak_bytes: self.live_bytes,
            retained_nodes: self.retained_nodes()?,
            ..mech_core::ResourceDemand::default()
        })
    }

    pub(in crate::native_artifact::program::runtime) fn metadata_bytes(
        descriptor: &NativeProgramDescriptor,
    ) -> Result<u64, Error> {
        let plan = &descriptor.metadata().plan;
        if plan.collections().is_empty() {
            return Ok(0);
        }
        add(
            (size_of::<[u64; 6]>()
                + size_of::<Vec<Option<string_span::Lane>>>()
                + size_of::<Vec<Option<string_map_entry::Lane>>>()) as u64
                + if descriptor.metadata().window_regions.is_empty() {
                    0
                } else {
                    ascii_facts::Cache::metadata_bytes()
                },
            add(
                mul(
                    plan.iterations().len() as u64,
                    (size_of::<Option<Source>>() + size_of::<Option<PreparedRange>>()) as u64,
                )?,
                mul(
                    plan.collections().len() as u64,
                    (size_of::<Option<Builder>>()
                        + size_of::<Option<string_span::Lane>>()
                        + size_of::<Option<string_map_entry::Lane>>()) as u64,
                )?,
            )?,
        )
    }
    pub(in crate::native_artifact::program::runtime) fn new(
        descriptor: &NativeProgramDescriptor,
        limits: MemoryBudgetLimits,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Option<Self>, Error> {
        let plan = &descriptor.metadata().plan;
        if plan.collections().is_empty() {
            return Ok(None);
        }
        let bytes = Self::metadata_bytes(descriptor)?;
        check(
            plan.collections()[0].origin(),
            Dimension::TemporaryBytes,
            bytes,
            limits.max_temporary_bytes,
        )?;
        charge(work, add(mul(bytes, 8)?, 256)?)?;
        let metadata = memory.reserve_capacity(bytes)?;
        let metadata_nodes = memory.reserve_nodes(bytes)?;
        let sources = allocation::metadata(
            plan.iterations().len(),
            "collection source metadata allocation",
        )?;
        let prepared_ranges =
            allocation::metadata(plan.iterations().len(), "range source metadata allocation")?;
        let builders = allocation::metadata(
            plan.collections().len(),
            "collection builder metadata allocation",
        )?;
        let string_spans =
            allocation::metadata(plan.collections().len(), "String span metadata allocation")?;
        let string_maps = allocation::metadata(
            plan.collections().len(),
            "String map projection metadata allocation",
        )?;
        Ok(Some(Self {
            sources,
            prepared_ranges,
            builders,
            string_spans,
            string_maps,
            current: None,
            live_bytes: bytes,
            query_scalar_bytes: 0,
            query_scalar_nodes: 0,
            string_span_nodes: 0,
            active_string_spans: 0,
            string_map_nodes: 0,
            active_string_maps: 0,
            ascii_facts: ascii_facts::Cache::default(),
            _metadata: metadata,
            _metadata_nodes: metadata_nodes,
        }))
    }
    pub(in crate::native_artifact::program::runtime) fn is_complete(&self) -> bool {
        self.current.is_none()
            && self.sources.iter().all(Option::is_none)
            && self.prepared_ranges.iter().all(Option::is_none)
            && self.builders.iter().all(Option::is_none)
            && self.active_string_spans == 0
            && self.active_string_maps == 0
    }
    #[cfg(test)]
    pub(in crate::native_artifact::program::runtime) fn test_live_bytes(&self) -> u64 {
        self.live_bytes
    }
    #[cfg(test)]
    pub(in crate::native_artifact::program::runtime) fn test_builder_kinds(&self) -> u8 {
        self.builders.iter().flatten().fold(0, |kinds, builder| {
            kinds
                | if builder.join.is_some() {
                    2
                } else if builder.histogram.is_some() {
                    4
                } else if builder.query.is_some() {
                    8
                } else {
                    1
                }
        })
    }
    fn builder(&mut self, id: u32) -> Result<&mut Builder, Error> {
        if self.current != Some(id) {
            return Err(invalid(
                "collection helper outside exact builder invocation",
            ));
        }
        self.builders
            .get_mut(id as usize)
            .and_then(Option::as_mut)
            .ok_or_else(|| invalid("collection builder is not active"))
    }
    fn account(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        id: u32,
        clones: u64,
        compute: u64,
    ) -> Result<(), Error> {
        let origin = ctx.descriptor().metadata().plan.collections()[id as usize].origin();
        let builder = self.builder(id)?;
        let cloned = add(builder.cloned_bytes, clones)?;
        let used = add(builder.compute, compute)?;
        check(
            origin,
            Dimension::ClonedBytes,
            cloned,
            ctx.limits().max_cloned_bytes,
        )?;
        check(
            origin,
            Dimension::ComputeWork,
            used,
            ctx.limits().max_compute_work,
        )?;
        charge(&mut ctx.work(), compute)?;
        builder.cloned_bytes = cloned;
        builder.compute = used;
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn helper(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        words: [u64; 2],
    ) -> Result<Option<u64>, Error> {
        #[cfg(all(test, feature = "canonical-native-program"))]
        if string_span::before_stage(self, ctx, site, words)? {
            return Ok(None);
        }
        #[cfg(all(test, feature = "canonical-native-program"))]
        if string_map_entry::before_stage(self, ctx, site)? {
            return Ok(None);
        }
        let word = words[0];
        let descriptor = ctx.descriptor();
        let plan = &descriptor.metadata().plan;
        if !matches!(
            plan.blocks()[site].action(),
            Action::PrepareU64Range { .. } | Action::PrepareStringSpanSelector { .. }
        ) && words[1] != 0
        {
            return Err(invalid(
                "collection helper has an unexpected second payload",
            ));
        }
        match plan.blocks()[site].action() {
            Action::PrepareStringSpanSelector { collection } => {
                self.prepare_string_span(ctx, site, *collection, words)?;
                Ok(None)
            }
            Action::ProjectStringSpan { collection } => {
                if words != [0; 2] {
                    return Err(invalid("String span projection has a native payload"));
                }
                self.project_string_span(ctx, site, *collection)?;
                Ok(None)
            }
            Action::ProjectStringMapEntry { collection } => {
                if words != [0; 2] {
                    return Err(invalid("String map projection has a native payload"));
                }
                self.project_string_map_entry(ctx, site, *collection)?;
                Ok(None)
            }
            Action::PrepareU64Range { iteration } => {
                let NativeControlLoop::Collection(definition) =
                    &plan.iterations()[*iteration as usize]
                else {
                    return Err(invalid("range preparation outside a collection"));
                };
                let NativeCollectionSource::U64Range(cursor) = definition.source() else {
                    return Err(invalid("range preparation lacks selected source proof"));
                };
                let collection = &plan.collections()[definition.collection() as usize];
                let owner = if cursor.before_collection_begin() {
                    collection.parent_collection()
                } else {
                    Some(definition.collection())
                };
                if self.current != owner
                    || self.sources[*iteration as usize].is_some()
                    || self.prepared_ranges[*iteration as usize].is_some()
                {
                    return Err(invalid(
                        "range preparation outside its fresh lexical invocation",
                    ));
                }
                let selected = descriptor.metadata().calls[site]
                    .as_ref()
                    .ok_or_else(|| invalid("range preparation lacks actual selected factory"))?;
                let bytes = selected.binding_metadata_bytes();
                let work = selected.binding_work();
                check(
                    cursor.template().origin(),
                    Dimension::TemporaryBytes,
                    add(self.live_bytes, bytes)?,
                    ctx.limits().max_temporary_bytes,
                )?;
                check(
                    cursor.template().origin(),
                    Dimension::ComputeWork,
                    work,
                    ctx.limits().max_compute_work,
                )?;
                charge(&mut ctx.work(), work)?;
                let _scratch = ctx.library().memory_budget().reserve_capacity(bytes)?;
                // Full template equality may traverse schema/contract trees;
                // it belongs AFTER the unchanged complete work/scratch debit.
                if selected.template() != cursor.template() {
                    return Err(invalid("range preparation selection differs"));
                }
                let count = selected
                    .selected_u64_range_cardinality(words[0], words[1])
                    .map_err(|error| Error::Library {
                        origin: cursor.template().origin(),
                        error,
                    })?;
                self.prepared_ranges[*iteration as usize] = Some(PreparedRange {
                    start: words[0],
                    count,
                });
                Ok(None)
            }
            Action::CollectionBegin { collection } => {
                let definition = &plan.collections()[*collection as usize];
                if self.current != definition.parent_collection()
                    || self.builders[*collection as usize].is_some()
                {
                    return Err(invalid("collection begin outside exact enclosing builder"));
                }
                charge(&mut ctx.work(), 64)?;
                let nodes = ctx.library().memory_budget().reserve_nodes(2)?;
                self.builders[*collection as usize] = Some(Builder {
                    values: Vec::new(),
                    join: None,
                    query: None,
                    histogram: None,
                    payload_bytes: 0,
                    cloned_bytes: 0,
                    compute: 0,
                    projection_comparison: 0,
                    reservation: ctx.library().memory_budget().reserve_capacity(0)?,
                    nodes,
                });
                self.current = Some(*collection);
                self.begin_text_join(ctx, *collection)?;
                self.begin_histogram_query(ctx, *collection)?;
                self.begin_histogram(ctx, *collection)?;
                Ok(None)
            }
            Action::Initialize { iteration } => {
                let NativeControlLoop::Collection(definition) =
                    &plan.iterations()[*iteration as usize]
                else {
                    return Err(invalid("collection initializer on a fold"));
                };
                self.builder(definition.collection())?;
                if self.sources[*iteration as usize].is_some() {
                    return Err(invalid("collection source already active"));
                }
                let (backing, handle, count, cost) = match definition.source() {
                    NativeCollectionSource::Value(source) => {
                        if let Some(count) = ctx.initialize_joined_map(*iteration, source.get())? {
                            self.account(
                                ctx,
                                definition.collection(),
                                0,
                                add(mul(count, 64)?, 64)?,
                            )?;
                            (SourceBacking::ConsumedString, None, count, 0)
                        } else {
                            let handle = if plan.collections()[definition.collection() as usize]
                                .string_element_map_projection()
                                .is_some()
                            {
                                Some(ctx.arena_current_handle(source.get())?)
                            } else {
                                None
                            };
                            let value = ctx.arena_copy_current(source.get())?;
                            let cost = match value.data() {
                                ValueData::String(text) => add(mul(text.len() as u64, 64)?, 64)?,
                                _ => 64,
                            };
                            self.account(ctx, definition.collection(), 0, cost)?;
                            // A window-bearing descriptor has already admitted
                            // the one weak fact slot. Keep the full fresh scan
                            // charge above even when this exact root is a hit.
                            let count = match value.data() {
                                ValueData::String(text)
                                    if !descriptor.metadata().window_regions.is_empty()
                                        && self.ascii_facts.inspect(&value)? =>
                                {
                                    u64::try_from(text.len())
                                        .map_err(|_| invalid("generator count exceeds u64"))?
                                }
                                _ => data::count(&value)?,
                            };
                            (SourceBacking::Value(value), handle, count, 0)
                        }
                    }
                    NativeCollectionSource::U64Range(_) => {
                        let range = self.prepared_ranges[*iteration as usize]
                            .take()
                            .ok_or_else(|| {
                                invalid("range generator has no current prepared source")
                            })?;
                        (
                            SourceBacking::U64Range { start: range.start },
                            None,
                            range.count,
                            64,
                        )
                    }
                };
                if cost != 0 {
                    self.account(ctx, definition.collection(), 0, cost)?;
                }
                let (count, index_offset) = if let Some(worker) = &ctx.worker()
                    && descriptor.metadata().parallel.regions[worker.region].iteration == *iteration
                {
                    if count != worker.count
                        || worker.index >= count
                        || !matches!(&backing, SourceBacking::Value(value)
                            if matches!(value.data(), ValueData::Matrix(_) | ValueData::Set(_)))
                    {
                        return Err(invalid(
                            "worker source differs from admitted original sequence",
                        ));
                    }
                    (1, worker.index)
                } else {
                    (count, 0)
                };
                ctx.control().bind_collection_count(*iteration, count)?;
                self.sources[*iteration as usize] = Some(Source {
                    backing,
                    handle,
                    count,
                    index_offset,
                    byte_offset: 0,
                    element: None,
                });
                Ok(Some(count))
            }
            Action::CollectionElement { iteration } => {
                let NativeControlLoop::Collection(definition) =
                    &plan.iterations()[*iteration as usize]
                else {
                    return Err(invalid("collection element on a fold"));
                };
                // Even discarded wildcard visits consume this builder's
                // cumulative compute allowance, not just global native work.
                self.account(ctx, definition.collection(), 0, 32)?;
                self.retire_locals(ctx, *iteration)?;
                let source = self.sources[*iteration as usize]
                    .as_ref()
                    .ok_or_else(|| invalid("missing collection source"))?;
                let index = source
                    .count
                    .checked_sub(ctx.progress()[*iteration as usize * 2])
                    .filter(|index| *index < source.count)
                    .ok_or_else(|| invalid("collection element outside current count"))?
                    .checked_add(source.index_offset)
                    .ok_or_else(|| invalid("worker source index overflow"))?;
                let Some(output) = definition.element() else {
                    return Ok(None);
                };
                if plan.collections()[definition.collection() as usize]
                    .string_element_map_projection()
                    .is_some()
                {
                    self.prepare_string_map_key(ctx, site, *iteration, index)?;
                    return Ok(None);
                }
                let backing = source.backing.clone();
                let (element, next) = match &backing {
                    SourceBacking::Value(value) => data::element(value, index, source.byte_offset)?,
                    SourceBacking::ConsumedString => {
                        return Err(invalid("consumed String source revisited"));
                    }
                    SourceBacking::U64Range { start } => (
                        data::Element::Scalar(
                            NativeScalarKind::U64,
                            start
                                .checked_add(index)
                                .ok_or_else(|| invalid("range cursor endpoint overflow"))?,
                        ),
                        0,
                    ),
                };
                let bytes = element.bytes();
                self.account(
                    ctx,
                    definition.collection(),
                    bytes,
                    add(mul(bytes, 16)?, 128)?,
                )?;
                let result = match (plan.values()[output.get() as usize].scalar_kind(), element) {
                    (Some(kind), data::Element::Scalar(actual, word)) if kind == actual => {
                        Some(word)
                    }
                    (None, element @ (data::Element::Text(_) | data::Element::Index(_))) => {
                        let ty = &plan.values()[output.get() as usize];
                        let valid = matches!(
                            (&element, ty.definition().body()),
                            (data::Element::Text(_), mech_core::SchemaBody::String)
                                | (data::Element::Index(_), mech_core::SchemaBody::Index)
                        );
                        if !valid {
                            return Err(invalid("managed generator element schema differs"));
                        }
                        let envelope = add(descriptor.metadata().scalar_bytes, mul(bytes, 4)?)?;
                        check(
                            definition.origin(),
                            Dimension::TemporaryBytes,
                            add(self.live_bytes, envelope)?,
                            ctx.limits().max_temporary_bytes,
                        )?;
                        charge(&mut ctx.work(), add(mul(envelope, 8)?, 128)?)?;
                        let mut reservation =
                            ctx.library().memory_budget().reserve_capacity(envelope)?;
                        let data = match element {
                            data::Element::Text(text) => ValueDataDraft::String(text.to_owned()),
                            data::Element::Index(index) => ValueDataDraft::Index(index),
                            _ => unreachable!("exact managed element variants selected above"),
                        };
                        let value = ValueDraft {
                            schema: ty.schema(),
                            shape_values: Box::new([]),
                            data,
                        }
                        .finalize(&SnapshotValidationContext::with_shared_schemas(
                            &descriptor.metadata().schemas,
                        ))
                        .map_err(Error::Snapshot)?
                        .into_memory_budget(&mut reservation)?;
                        let retained = value
                            .memory_budget_retained_bytes(&ctx.library().memory_budget())
                            .ok_or_else(|| invalid("unadmitted collection element"))?;
                        let old = self.sources[*iteration as usize]
                            .as_ref()
                            .unwrap()
                            .element
                            .map_or(0, |(_, bytes)| bytes);
                        let live = add(
                            self.live_bytes
                                .checked_sub(old)
                                .ok_or_else(|| invalid("collection element accounting"))?,
                            retained,
                        )?;
                        // The old element remains charged through construction
                        // of the new envelope. All its iteration-local aliases
                        // were retired above; put now releases its final slot.
                        let handle = ctx.arena_put(output.get(), &value)?;
                        self.sources[*iteration as usize].as_mut().unwrap().element =
                            Some((handle, retained));
                        self.live_bytes = live;
                        None
                    }
                    _ => {
                        return Err(invalid(
                            "collection element differs from exact canonical kind",
                        ));
                    }
                };
                self.sources[*iteration as usize]
                    .as_mut()
                    .unwrap()
                    .byte_offset = next;
                Ok(result)
            }
            Action::CollectionPush { collection, value } => {
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(_)
                ) {
                    if word != 0 {
                        return Err(invalid("histogram query managed yield has native payload"));
                    }
                    self.push_histogram_query(ctx, *collection, *value)?;
                    return Ok(None);
                }
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    mech_engine::native_artifact::NativeCollectionResult::Histogram(_)
                ) {
                    if word != 0 {
                        return Err(invalid("histogram managed yield has native payload"));
                    }
                    self.push_histogram(ctx, *collection, *value)?;
                    return Ok(None);
                }
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    mech_engine::native_artifact::NativeCollectionResult::TextJoin(_)
                ) {
                    self.push_text_join(ctx, *collection, *value)?;
                    return Ok(None);
                }
                let ty = &plan.values()[value.get() as usize];
                let owned = if ty.scalar_kind().is_none() {
                    Some(ctx.arena_copy_current(value.get())?)
                } else {
                    None
                };
                let payload = match (ty.scalar_kind(), owned.as_ref().map(Value::data)) {
                    (Some(NativeScalarKind::Bool | NativeScalarKind::U8), _) => 1,
                    (Some(NativeScalarKind::U16), _) => 2,
                    (Some(NativeScalarKind::U32), _) => 4,
                    (Some(_), _) => 8,
                    (None, Some(ValueData::Index(_))) => 8,
                    (None, Some(ValueData::String(text))) => text.len() as u64,
                    _ => return Err(invalid("unsupported collected value kind")),
                };
                let alias_work = if owned.is_some() {
                    add(mul(self.sources.len() as u64, 64)?, 32)?
                } else {
                    0
                };
                self.account(
                    ctx,
                    *collection,
                    payload,
                    add(add(mul(payload, 16)?, 128)?, alias_work)?,
                )?;
                let untracked_yield = self.untracked_yield_bytes(ctx, owned.as_ref())?;
                let origin = plan.collections()[*collection as usize].origin();
                let old_live = self.live_bytes;
                let query_scalar_nodes = self.retained_nodes()?;
                let builder = self.builder(*collection)?;
                let count = builder
                    .values
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| invalid("collection count overflow"))?;
                let capacity = if count > builder.values.capacity() {
                    builder
                        .values
                        .capacity()
                        .checked_mul(2)
                        .map(|n| n.max(4))
                        .ok_or_else(|| invalid("collection capacity overflow"))?
                } else {
                    builder.values.capacity()
                };
                let payload_bytes = add(builder.payload_bytes, payload)?;
                let backing = add(
                    mul(capacity as u64, size_of::<ValueDataDraft>() as u64)?,
                    payload_bytes,
                )?;
                let extra = backing
                    .checked_sub(builder.reservation.capacity_bytes())
                    .ok_or_else(|| invalid("collection capacity moved backwards"))?;
                check(
                    origin,
                    Dimension::OutputElements,
                    count as u64,
                    ctx.limits().max_output_elements,
                )?;
                check(
                    origin,
                    Dimension::OutputBytes,
                    add(add(payload_bytes, mul(count as u64, 32)?)?, 256)?,
                    ctx.limits().max_output_bytes,
                )?;
                check(
                    origin,
                    Dimension::RetainedNodes,
                    add(
                        data::output_nodes(
                            plan.values()
                                [plan.collections()[*collection as usize].output().get() as usize]
                                .definition()
                                .body(),
                            count as u64,
                        )?,
                        query_scalar_nodes,
                    )?,
                    ctx.limits().max_retained_nodes,
                )?;
                check(
                    origin,
                    Dimension::TemporaryBytes,
                    add(add(old_live, extra)?, untracked_yield)?,
                    ctx.limits().max_temporary_bytes,
                )?;
                // Covers relocation, eventual destruction and scalar/String packing.
                let moved = if capacity > builder.values.capacity() {
                    mul(
                        builder.values.capacity() as u64,
                        size_of::<ValueDataDraft>() as u64,
                    )?
                } else {
                    0
                };
                charge(
                    &mut ctx.work(),
                    add(add(mul(extra, 16)?, mul(moved, 4)?)?, 128)?,
                )?;
                // The pending draft and replacement carrier coexist with the
                // original builder until every fallible allocation succeeds.
                let memory = ctx.library().memory_budget();
                let pending_node = memory.reserve_nodes(1)?;
                let _replacement_node =
                    memory.reserve_nodes(u64::from(capacity > builder.values.capacity()))?;
                builder.reservation.resize_capacity(backing)?;
                if capacity > builder.values.capacity() {
                    builder
                        .values
                        .try_reserve_exact(capacity - builder.values.len())
                        .map_err(|_| invalid("collection draft allocation"))?;
                    builder.values = allocation::exact_capacity(
                        core::mem::take(&mut builder.values),
                        capacity,
                        "collection draft exceeds admitted capacity",
                    )?;
                }
                let data = match ty.scalar_kind() {
                    Some(NativeScalarKind::Bool) if word <= 1 => ValueDataDraft::Bool(word != 0),
                    Some(NativeScalarKind::U8) => ValueDataDraft::U8(u8::try_from(word).map_err(|_| invalid("narrow collected word exceeds its type"))?),
                    Some(NativeScalarKind::U16) => ValueDataDraft::U16(u16::try_from(word).map_err(|_| invalid("narrow collected word exceeds its type"))?),
                    Some(NativeScalarKind::U32) => ValueDataDraft::U32(u32::try_from(word).map_err(|_| invalid("narrow collected word exceeds its type"))?),
                    Some(NativeScalarKind::U64) => ValueDataDraft::U64(word),
                    Some(NativeScalarKind::I64) => ValueDataDraft::I64(word as i64),
                    Some(NativeScalarKind::F64) => {
                        ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_bits(word))
                    }
                    None => owned
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .map_err(Error::Snapshot)?,
                    _ => return Err(invalid("invalid collected scalar word")),
                };
                builder.values.push(data);
                builder.nodes.merge(pending_node)?;
                builder.payload_bytes = payload_bytes;
                self.live_bytes = add(old_live, extra)?;
                Ok(None)
            }
            Action::HistogramQueryStep { collection, step } => {
                if words != [0; 2] {
                    return Err(invalid("histogram query step has unexpected payload"));
                }
                self.step_histogram_query(ctx, *collection, *step)
            }
            Action::HistogramFinish { collection } => {
                if words != [0; 2] {
                    return Err(invalid("histogram finish has unexpected payload"));
                }
                self.finish_histogram(ctx, *collection)?;
                Ok(None)
            }
            Action::TextJoinFinish { collection } => {
                self.finish_text_join(ctx, *collection)?;
                Ok(None)
            }
            Action::CollectionFinish { collection } => {
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(_)
                ) {
                    self.park_histogram_query(ctx, *collection)?;
                    return Ok(None);
                }
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    mech_engine::native_artifact::NativeCollectionResult::Histogram(_)
                ) {
                    self.park_histogram(ctx, *collection)?;
                    return Ok(None);
                }
                if matches!(
                    plan.collections()[*collection as usize].result(),
                    mech_engine::native_artifact::NativeCollectionResult::TextJoin(_)
                ) {
                    self.park_text_join(ctx, *collection)?;
                    return Ok(None);
                }
                self.finish(ctx, *collection)?;
                Ok(None)
            }
            _ => Err(invalid("collection helper outside a collection action")),
        }
    }
    fn finish(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let descriptor = ctx.descriptor();
        let definition = &descriptor.metadata().plan.collections()[id as usize];
        let output = &descriptor.metadata().plan.values()[definition.output().get() as usize];
        let builder = self.builder(id)?;
        let count = builder.values.len() as u64;
        let payload = builder.payload_bytes;
        let envelope = add(
            descriptor.metadata().scalar_bytes,
            add(mul(builder.reservation.capacity_bytes(), 4)?, 4096)?,
        )?;
        self.account(ctx, id, payload, add(mul(envelope, 16)?, 256)?)?;
        check(
            definition.origin(),
            Dimension::TemporaryBytes,
            add(self.live_bytes, envelope)?,
            ctx.limits().max_temporary_bytes,
        )?;
        check(
            definition.origin(),
            Dimension::OutputBytes,
            add(add(payload, mul(count, 32)?)?, 256)?,
            ctx.limits().max_output_bytes,
        )?;
        check(
            definition.origin(),
            Dimension::OutputElements,
            count,
            ctx.limits().max_output_elements,
        )?;
        check(
            definition.origin(),
            Dimension::RetainedNodes,
            add(
                data::output_nodes(output.definition().body(), count)?,
                self.retained_nodes()?,
            )?,
            ctx.limits().max_retained_nodes,
        )?;
        let _publication_nodes = ctx
            .library()
            .memory_budget()
            .reserve_nodes(data::output_nodes(output.definition().body(), count)?)?;
        let mut reservation = ctx.library().memory_budget().reserve_capacity(envelope)?;
        // Normal scalar comprehensions are always 1xN, including 1x0.
        let extents = if definition.kind() == ComprehensionKind::MatrixPreserveShape {
            let NativeControlLoop::Collection(generator) =
                &descriptor.metadata().plan.iterations()[definition.generators()[0] as usize]
            else {
                return Err(invalid("preserved collection lacks a matrix generator"));
            };
            let source_value = generator
                .source()
                .materialized_value()
                .ok_or_else(|| invalid("shape preservation requires a materialized source"))?;
            let source = ctx.arena_copy_current(source_value.get())?;
            let schema =
                descriptor.metadata().plan.values()[source_value.get() as usize].definition();
            let mech_core::SchemaBody::Matrix { dimensions, .. } = schema.body() else {
                return Err(invalid("preserved source is not a matrix"));
            };
            if dimensions.len() != 2 {
                return Err(invalid("preserved source rank differs"));
            }
            let rows = source
                .shape()
                .resolve_dimension(&dimensions[0])
                .map_err(|_| invalid("source rows"))?;
            let columns = source
                .shape()
                .resolve_dimension(&dimensions[1])
                .map_err(|_| invalid("source columns"))?;
            if mul(rows, columns)? != count {
                return Err(invalid("preserved collection cardinality differs"));
            }
            [rows, columns]
        } else {
            [1, count]
        };
        let shape = mech_core::shape_for_resolved_extents(output.definition(), &extents)
            .map_err(|_| invalid("collected output shape"))?;
        let builder = self.builders[id as usize].as_mut().unwrap();
        let data = core::mem::take(&mut builder.values).into_boxed_slice();
        let value = ValueDraft {
            schema: output.schema(),
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data: ValueDataDraft::Matrix(data),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(
            &descriptor.metadata().schemas,
        ))
        .map_err(Error::Snapshot)?
        .into_memory_budget(&mut reservation)?;
        ctx.arena_put(definition.output().get(), &value)?;
        let builder = self.builders[id as usize].take().unwrap();
        self.live_bytes = self
            .live_bytes
            .checked_sub(builder.reservation.capacity_bytes())
            .ok_or_else(|| invalid("collection workspace accounting"))?;
        self.current = definition.parent_collection();
        Ok(())
    }
}
