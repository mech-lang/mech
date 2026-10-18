//! Admitted element transport and collection construction, not body execution.
//! Generated blocks own every generator, filter, arithmetic op and backedge.
use super::{Context, NativeProgramExecutionError as Error, invalid};
use crate::native_artifact::{
    NativeExecutionPhase as Phase, NativeProgramDescriptor, NativeWorkMeter,
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
        NativeControlLoop, NativeIterationAction as Action, NativeMatchOrigin as Origin,
        NativeProgramArtifactPlan, NativeScalarKind,
    },
};

#[path = "program_collection_data.rs"]
mod data;

struct Source {
    value: Value,
    count: u64,
    byte_offset: usize,
}
struct Builder {
    // Owned drafts and their backing are released before their reservation.
    values: Vec<ValueDataDraft>,
    payload_bytes: u64,
    cloned_bytes: u64,
    compute: u64,
    reservation: ManagedMemoryReservation,
}
pub(super) struct State {
    sources: Vec<Option<Source>>,
    builders: Vec<Option<Builder>>,
    current: Option<u32>,
    live_bytes: u64,
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
            mul(
                plan.iterations().len() as u64,
                size_of::<Option<Source>>() as u64,
            )?,
            mul(
                plan.collections().len() as u64,
                size_of::<Option<Builder>>() as u64,
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
        let mut builders = Vec::new();
        builders
            .try_reserve_exact(plan.collections().len())
            .map_err(|_| invalid("collection builder metadata allocation"))?;
        builders.resize_with(plan.collections().len(), || None);
        Ok(Some(Self {
            sources,
            builders,
            current: None,
            live_bytes: bytes,
            _metadata: metadata,
        }))
    }
    pub(super) fn is_complete(&self) -> bool {
        self.current.is_none()
            && self.sources.iter().all(Option::is_none)
            && self.builders.iter().all(Option::is_none)
    }
    pub(super) fn release_source(&mut self, iteration: u32) -> Result<(), Error> {
        self.sources
            .get_mut(iteration as usize)
            .and_then(Option::take)
            .ok_or_else(|| invalid("collection exited without its admitted source"))?;
        Ok(())
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
        ctx: &mut Context<'_>,
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
        ctx: &mut Context<'_>,
        site: usize,
        word: u64,
    ) -> Result<Option<u64>, Error> {
        let descriptor = ctx.control.descriptor;
        let plan = &descriptor.plan;
        match plan.blocks()[site].action() {
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
                    payload_bytes: 0,
                    cloned_bytes: 0,
                    compute: 0,
                    reservation: ctx.library.memory_budget().reserve_capacity(0)?,
                });
                self.current = Some(*collection);
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
                let value = ctx
                    .arena
                    .copy_current(definition.source().get(), &mut ctx.control.work)?;
                let cost = match value.data() {
                    ValueData::String(text) => add(mul(text.len() as u64, 64)?, 64)?,
                    _ => 64,
                };
                self.account(ctx, definition.collection(), 0, cost)?;
                let count = data::count(&value)?;
                ctx.control.bind_collection_count(*iteration, count)?;
                self.sources[*iteration as usize] = Some(Source {
                    value,
                    count,
                    byte_offset: 0,
                });
                Ok(Some(count))
            }
            Action::CollectionElement { iteration } => {
                let NativeControlLoop::Collection(definition) =
                    &plan.iterations()[*iteration as usize]
                else {
                    return Err(invalid("collection element on a fold"));
                };
                self.builder(definition.collection())?;
                charge(&mut ctx.control.work, 32)?;
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
                let value = source.value.clone();
                let (element, next) = data::element(&value, index, source.byte_offset)?;
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
                    (None, data::Element::Text(text)) => {
                        let envelope = add(descriptor.scalar_bytes, mul(text.len() as u64, 4)?)?;
                        check(
                            definition.origin(),
                            Dimension::TemporaryBytes,
                            add(self.live_bytes, envelope)?,
                            ctx.limits.max_temporary_bytes,
                        )?;
                        charge(&mut ctx.control.work, add(mul(envelope, 8)?, 128)?)?;
                        let mut reservation =
                            ctx.library.memory_budget().reserve_capacity(envelope)?;
                        let value = ValueDraft {
                            schema: plan.values()[output.get() as usize].schema(),
                            shape_values: Box::new([]),
                            data: ValueDataDraft::String(text.to_owned()),
                        }
                        .finalize(&SnapshotValidationContext::with_shared_schemas(
                            &descriptor.schemas,
                        ))
                        .map_err(Error::Snapshot)?
                        .into_memory_budget(&mut reservation)?;
                        ctx.arena.put(output.get(), &value, &mut ctx.control.work)?;
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
                let ty = &plan.values()[value.get() as usize];
                let owned = if ty.scalar_kind().is_none() {
                    Some(ctx.arena.copy_current(value.get(), &mut ctx.control.work)?)
                } else {
                    None
                };
                let payload = match (ty.scalar_kind(), owned.as_ref().map(Value::data)) {
                    (Some(NativeScalarKind::Bool), _) => 1,
                    (Some(_), _) => 8,
                    (None, Some(ValueData::String(text))) => text.len() as u64,
                    _ => return Err(invalid("unsupported collected value kind")),
                };
                self.account(ctx, *collection, payload, add(mul(payload, 16)?, 128)?)?;
                let origin = plan.collections()[*collection as usize].origin();
                let old_live = self.live_bytes;
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
                    add(count as u64, 1)?,
                    ctx.limits.max_retained_nodes,
                )?;
                check(
                    origin,
                    Dimension::TemporaryBytes,
                    add(old_live, extra)?,
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
            Action::CollectionFinish { collection } => {
                self.finish(ctx, *collection)?;
                Ok(None)
            }
            _ => Err(invalid("collection helper outside a collection action")),
        }
    }
    fn finish(&mut self, ctx: &mut Context<'_>, id: u32) -> Result<(), Error> {
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
            add(count, 1)?,
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
            let source = ctx
                .arena
                .copy_current(generator.source().get(), &mut ctx.control.work)?;
            let schema = descriptor.plan.values()[generator.source().get() as usize].definition();
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
