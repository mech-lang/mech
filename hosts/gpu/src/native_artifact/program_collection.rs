//! Admitted element transport and collection construction, not body execution.
//! Generated blocks own every generator, filter, arithmetic op and backedge.
use super::{Context, NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{
    NativeExecutionPhase as Phase, NativeProgramDescriptor, NativeValueHandle, NativeWorkMeter,
};
use core::mem::size_of;
use mech_core::{
    ManagedMemoryBudget, ManagedMemoryReservation, MemoryBudgetDimension as Dimension,
    MemoryBudgetLimits, ResidentKernelError, ResidentResourcePhase, Value, ValueData,
    snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft},
};
use mech_engine::{
    ComprehensionKind,
    native_artifact::{
        NativeCollectionSource, NativeControlLoop, NativeIterationAction as Action,
        NativeMatchOrigin as Origin, NativeProgramArtifactPlan, NativeScalarKind,
    },
};

#[path = "program_collection_data.rs"]
mod data;
#[path = "program_histogram.rs"]
mod histogram;
#[path = "program_histogram_layout.rs"]
mod histogram_layout;
#[path = "program_histogram_query.rs"]
mod histogram_query;
#[path = "program_collection_lifetime.rs"]
mod lifetime;
#[path = "program_string_map_entry.rs"]
mod string_map_entry;
#[path = "program_string_span.rs"]
mod string_span;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) use string_span::before_helper as before_span_helper;
#[path = "program_text_join.rs"]
mod text_join;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) use histogram::{HistogramDemand, HistogramStage, HistogramVisit};
#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) use histogram_query::QueryDemand;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) use text_join::{TextJoinDemand, TextJoinStage};

struct Source {
    backing: SourceBacking,
    // Present only for a certified borrowed String-element generator.
    handle: Option<NativeValueHandle>,
    count: u64,
    byte_offset: usize,
    element: Option<(NativeValueHandle, u64)>,
}
#[derive(Clone)]
enum SourceBacking {
    Value(Value),
    U64Range { start: u64 },
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
}
pub(super) struct State {
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
    _metadata: ManagedMemoryReservation,
}
fn add(left: u64, right: u64) -> Result<u64, Error> {
    left.checked_add(right)
        .ok_or_else(|| invalid("native collection size/work overflow"))
}
fn mul(left: u64, right: u64) -> Result<u64, Error> {
    left.checked_mul(right)
        .ok_or_else(|| invalid("native collection size/work overflow"))
}
fn check(
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
fn charge(work: &mut NativeWorkMeter, amount: u64) -> Result<(), Error> {
    work.charge(Phase::Setup, amount).map_err(Into::into)
}
impl State {
    pub(super) fn metadata_bytes(plan: &NativeProgramArtifactPlan) -> Result<u64, Error> {
        if plan.collections().is_empty() {
            return Ok(0);
        }
        add(
            (size_of::<[u64; 6]>()
                + size_of::<Vec<Option<string_span::Lane>>>()
                + size_of::<Vec<Option<string_map_entry::Lane>>>()) as u64,
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
    pub(super) fn new(
        descriptor: &NativeProgramDescriptor,
        limits: MemoryBudgetLimits,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
    ) -> Result<Option<Self>, Error> {
        let plan = &descriptor.plan;
        if plan.collections().is_empty() {
            return Ok(None);
        }
        let bytes = Self::metadata_bytes(plan)?;
        check(
            plan.collections()[0].origin(),
            Dimension::TemporaryBytes,
            bytes,
            limits.max_temporary_bytes,
        )?;
        charge(work, add(mul(bytes, 8)?, 256)?)?;
        let metadata = memory.reserve_capacity(bytes)?;
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(plan.iterations().len())
            .map_err(|_| invalid("collection source metadata allocation"))?;
        sources.resize_with(plan.iterations().len(), || None);
        let mut prepared_ranges = Vec::new();
        prepared_ranges
            .try_reserve_exact(plan.iterations().len())
            .map_err(|_| invalid("range source metadata allocation"))?;
        prepared_ranges.resize_with(plan.iterations().len(), || None);
        let mut builders = Vec::new();
        builders
            .try_reserve_exact(plan.collections().len())
            .map_err(|_| invalid("collection builder metadata allocation"))?;
        builders.resize_with(plan.collections().len(), || None);
        let mut string_spans = Vec::new();
        string_spans
            .try_reserve_exact(plan.collections().len())
            .map_err(|_| invalid("String span metadata allocation"))?;
        string_spans.resize_with(plan.collections().len(), || None);
        let mut string_maps = Vec::new();
        string_maps
            .try_reserve_exact(plan.collections().len())
            .map_err(|_| invalid("String map projection metadata allocation"))?;
        string_maps.resize_with(plan.collections().len(), || None);
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
            _metadata: metadata,
        }))
    }
    pub(super) fn is_complete(&self) -> bool {
        self.current.is_none()
            && self.sources.iter().all(Option::is_none)
            && self.prepared_ranges.iter().all(Option::is_none)
            && self.builders.iter().all(Option::is_none)
            && self.active_string_spans == 0
            && self.active_string_maps == 0
    }
    #[cfg(test)]
    pub(super) fn test_live_bytes(&self) -> u64 {
        self.live_bytes
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
        ctx: &mut Context<'_, '_>,
        id: u32,
        clones: u64,
        compute: u64,
    ) -> Result<(), Error> {
        let origin = ctx.control.descriptor.plan.collections()[id as usize].origin();
        let builder = self.builder(id)?;
        let cloned = add(builder.cloned_bytes, clones)?;
        let used = add(builder.compute, compute)?;
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
        charge(&mut ctx.control.work, compute)?;
        builder.cloned_bytes = cloned;
        builder.compute = used;
        Ok(())
    }
    pub(super) fn helper(
        &mut self,
        ctx: &mut Context<'_, '_>,
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
        let descriptor = ctx.control.descriptor;
        let plan = &descriptor.plan;
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
                let selected = descriptor.calls[site]
                    .as_ref()
                    .ok_or_else(|| invalid("range preparation lacks actual selected factory"))?;
                let bytes = selected.binding_metadata_bytes();
                let work = selected.binding_work();
                check(
                    cursor.template().origin(),
                    Dimension::TemporaryBytes,
                    add(self.live_bytes, bytes)?,
                    ctx.limits.max_temporary_bytes,
                )?;
                check(
                    cursor.template().origin(),
                    Dimension::ComputeWork,
                    work,
                    ctx.limits.max_compute_work,
                )?;
                charge(&mut ctx.control.work, work)?;
                let _scratch = ctx.library.memory_budget().reserve_capacity(bytes)?;
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
                charge(&mut ctx.control.work, 64)?;
                self.builders[*collection as usize] = Some(Builder {
                    values: Vec::new(),
                    join: None,
                    query: None,
                    histogram: None,
                    payload_bytes: 0,
                    cloned_bytes: 0,
                    compute: 0,
                    projection_comparison: 0,
                    reservation: ctx.library.memory_budget().reserve_capacity(0)?,
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
                        let handle = if plan.collections()[definition.collection() as usize]
                            .string_element_map_projection()
                            .is_some()
                        {
                            Some(
                                ctx.arena
                                    .current_handle(source.get(), &mut ctx.control.work)?,
                            )
                        } else {
                            None
                        };
                        let value = ctx
                            .arena
                            .copy_current(source.get(), &mut ctx.control.work)?;
                        let cost = match value.data() {
                            ValueData::String(text) => add(mul(text.len() as u64, 64)?, 64)?,
                            _ => 64,
                        };
                        self.account(ctx, definition.collection(), 0, cost)?;
                        let count = data::count(&value)?;
                        (SourceBacking::Value(value), handle, count, 0)
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
                ctx.control.bind_collection_count(*iteration, count)?;
                self.sources[*iteration as usize] = Some(Source {
                    backing,
                    handle,
                    count,
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
                    .checked_sub(ctx.control.progress[*iteration as usize * 2])
                    .filter(|index| *index < source.count)
                    .ok_or_else(|| invalid("collection element outside current count"))?;
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
                        let envelope = add(descriptor.scalar_bytes, mul(bytes, 4)?)?;
                        check(
                            definition.origin(),
                            Dimension::TemporaryBytes,
                            add(self.live_bytes, envelope)?,
                            ctx.limits.max_temporary_bytes,
                        )?;
                        charge(&mut ctx.control.work, add(mul(envelope, 8)?, 128)?)?;
                        let mut reservation =
                            ctx.library.memory_budget().reserve_capacity(envelope)?;
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
                            &descriptor.schemas,
                        ))
                        .map_err(Error::Snapshot)?
                        .into_memory_budget(&mut reservation)?;
                        let retained = value
                            .memory_budget_retained_bytes(&ctx.library.memory_budget())
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
                        let handle = ctx.arena.put(output.get(), &value, &mut ctx.control.work)?;
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
                    Some(ctx.arena.copy_current(value.get(), &mut ctx.control.work)?)
                } else {
                    None
                };
                let payload = match (ty.scalar_kind(), owned.as_ref().map(Value::data)) {
                    (Some(NativeScalarKind::Bool), _) => 1,
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
                    ctx.limits.max_output_elements,
                )?;
                check(
                    origin,
                    Dimension::OutputBytes,
                    add(add(payload_bytes, mul(count as u64, 32)?)?, 256)?,
                    ctx.limits.max_output_bytes,
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
                    ctx.limits.max_retained_nodes,
                )?;
                check(
                    origin,
                    Dimension::TemporaryBytes,
                    add(add(old_live, extra)?, untracked_yield)?,
                    ctx.limits.max_temporary_bytes,
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
                    &mut ctx.control.work,
                    add(add(mul(extra, 16)?, mul(moved, 4)?)?, 128)?,
                )?;
                builder.reservation.resize_capacity(backing)?;
                if capacity > builder.values.capacity() {
                    builder
                        .values
                        .try_reserve_exact(capacity - builder.values.len())
                        .map_err(|_| invalid("collection draft allocation"))?;
                }
                let data = match ty.scalar_kind() {
                    Some(NativeScalarKind::Bool) if word <= 1 => ValueDataDraft::Bool(word != 0),
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
    fn finish(&mut self, ctx: &mut Context<'_, '_>, id: u32) -> Result<(), Error> {
        let descriptor = ctx.control.descriptor;
        let definition = &descriptor.plan.collections()[id as usize];
        let output = &descriptor.plan.values()[definition.output().get() as usize];
        let builder = self.builder(id)?;
        let count = builder.values.len() as u64;
        let payload = builder.payload_bytes;
        let envelope = add(
            descriptor.scalar_bytes,
            add(mul(builder.reservation.capacity_bytes(), 4)?, 4096)?,
        )?;
        self.account(ctx, id, payload, add(mul(envelope, 16)?, 256)?)?;
        check(
            definition.origin(),
            Dimension::TemporaryBytes,
            add(self.live_bytes, envelope)?,
            ctx.limits.max_temporary_bytes,
        )?;
        check(
            definition.origin(),
            Dimension::OutputBytes,
            add(add(payload, mul(count, 32)?)?, 256)?,
            ctx.limits.max_output_bytes,
        )?;
        check(
            definition.origin(),
            Dimension::OutputElements,
            count,
            ctx.limits.max_output_elements,
        )?;
        check(
            definition.origin(),
            Dimension::RetainedNodes,
            add(
                data::output_nodes(output.definition().body(), count)?,
                self.retained_nodes()?,
            )?,
            ctx.limits.max_retained_nodes,
        )?;
        let mut reservation = ctx.library.memory_budget().reserve_capacity(envelope)?;
        // Normal scalar comprehensions are always 1xN, including 1x0.
        let extents = if definition.kind() == ComprehensionKind::MatrixPreserveShape {
            let NativeControlLoop::Collection(generator) =
                &descriptor.plan.iterations()[definition.generators()[0] as usize]
            else {
                return Err(invalid("preserved collection lacks a matrix generator"));
            };
            let source_value = generator
                .source()
                .materialized_value()
                .ok_or_else(|| invalid("shape preservation requires a materialized source"))?;
            let source = ctx
                .arena
                .copy_current(source_value.get(), &mut ctx.control.work)?;
            let schema = descriptor.plan.values()[source_value.get() as usize].definition();
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
            &descriptor.schemas,
        ))
        .map_err(Error::Snapshot)?
        .into_memory_budget(&mut reservation)?;
        ctx.arena
            .put(definition.output().get(), &value, &mut ctx.control.work)?;
        let builder = self.builders[id as usize].take().unwrap();
        self.live_bytes = self
            .live_bytes
            .checked_sub(builder.reservation.capacity_bytes())
            .ok_or_else(|| invalid("collection workspace accounting"))?;
        self.current = definition.parent_collection();
        Ok(())
    }
}
