//! One current immutable source loan and one private native-written output.
//! A failed arithmetic preflight discards its private bytes; the original CFG
//! then produces the original error at its original source visit.
use super::*;
use crate::native_artifact::program::prepare::numeric_maps::{MAX_MATRICES, Region};
use mech_core::snapshot::SequenceView;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct MatrixView {
    pub input: u64,
    pub count: u64,
    pub rows: u64,
    pub columns: u64,
}
struct MatrixOwner {
    handle: NativeValueHandle,
    owner: Value,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Frame {
    pub input: u64,
    pub input_bytes: u64,
    pub output: u64,
    pub capacity_bytes: u64,
    pub count: u64,
    pub input_width: u64,
    pub output_width: u64,
    pub epoch: u64,
    pub issued: u64,
    pub next: u64,
    pub range_start: u64,
    pub matrices: [MatrixView; MAX_MATRICES],
}
pub(super) struct Loan {
    frame: Frame,
    expected: Frame,
    values: data::CollectedValues,
    source: Option<Value>,
    source_handle: Option<NativeValueHandle>,
    source_slot: u32,
    matrices: [Option<MatrixOwner>; MAX_MATRICES],
    output_kind: NativeScalarKind,
    collection: u32,
    iteration: u32,
    header: usize,
    commit_compute: u64,
    commit_cloned: u64,
    work: mech_engine::native_artifact::NativeLibraryWorkBudget,
    output_reservation: ManagedMemoryReservation,
    _metadata: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
fn fits(need: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|cap| need <= cap)
}
fn matrix_kind(body: &mech_core::SchemaBody) -> Option<NativeScalarKind> {
    let mech_core::SchemaBody::Matrix {
        element,
        dimensions,
    } = body
    else {
        return None;
    };
    (dimensions.len() == 2)
        .then(|| NativeScalarKind::from_schema_body(element))
        .flatten()
}
fn source_pointer(owner: &Value, kind: NativeScalarKind) -> Option<(u64, usize)> {
    let ValueData::Matrix(matrix) = owner.data() else {
        return None;
    };
    use NativeScalarKind as K;
    Some(match (kind, matrix.elements()) {
        (K::Bool, SequenceView::Bool(xs)) => (xs.as_ptr() as u64, xs.len()),
        (K::U8, SequenceView::U8(xs)) => (xs.as_ptr() as u64, xs.len()),
        (K::U16, SequenceView::U16(xs)) => (xs.as_ptr() as u64, xs.len()),
        (K::U32, SequenceView::U32(xs)) => (xs.as_ptr() as u64, xs.len()),
        (K::U64, SequenceView::U64(xs)) => (xs.as_ptr() as u64, xs.len()),
        (K::I64, SequenceView::I64(xs)) => (xs.as_ptr() as u64, xs.len()),
        (K::F64, SequenceView::F64(xs)) => (xs.as_ptr() as u64, xs.len()),
        _ => return None,
    })
}
impl State {
    /// Pure eligibility credit, before caller/arena queries spend any work.
    /// A fallible preflight must leave enough for the complete ordinary prefix
    /// on a miss. This is not charged, refunded, or granted to generated code.
    pub(in crate::native_artifact::program::runtime) fn numeric_replay_credit(
        &self,
        descriptor: &NativeProgramDescriptor,
        region: &Region,
        retirements: &[u64],
    ) -> Result<Option<u64>, Error> {
        use crate::native_artifact::program::prepare::retirement::RetirementKind;
        let metadata = descriptor.metadata();
        let slots = metadata
            .retirements
            .slots(retirements, region.iteration, RetirementKind::Collection)
            .map_err(invalid)?;
        // The body produces only unboxed scalars. A managed retirement recipe
        // needs a separate payload bound and keeps the ordinary route.
        if !slots.is_empty() {
            return Ok(None);
        }
        let Some(source) = self
            .sources
            .get(region.iteration as usize)
            .and_then(Option::as_ref)
        else {
            return Ok(None);
        };
        let credit = (|| {
            let mut visits = 0u64;
            for &site in &region.sites {
                visits = visits.checked_add(metadata.visits.get(site as usize)?.work)?;
                if let Some(group) = metadata.map_admission.group(site as usize) {
                    visits = visits.checked_add(group.work)?;
                }
            }
            // Element admission32, lexical retirement64, original source
            // clone128 and packed push128, plus their actual byte fees.
            let source_floor = 352u64.checked_add(
                (u64::from(region.input_width) + u64::from(region.output_width)).checked_mul(16)?,
            )?;
            let frozen = (metadata.frozen_maps.regions.len() as u64)
                .checked_mul(64)?
                .checked_add(64)?;
            // Geometric packed-Vec growth, relocation and fixed push debits:
            // sum capacities<4*n, plus the initial four-word bank.
            let growth = u64::from(region.output_width)
                .checked_mul(64)?
                .checked_add(256)?;
            let inventory = (metadata.plan.values().len() as u64)
                .checked_add(metadata.plan.blocks().len() as u64)?
                .checked_add(metadata.plan.iterations().len() as u64)?
                .checked_add(metadata.plan.collections().len() as u64)?;
            let read_replay = region.reads.iter().try_fold(0u64, |n, read| {
                let call = metadata.calls.get(read.site as usize)?.as_ref()?;
                n.checked_add(call.scalar_matrix_element_native_work()?)?
                    .checked_add(inventory.checked_mul(8192)?)?
                    .checked_add(metadata.observation_schema_bytes.checked_mul(128)?)?
                    .checked_add(65536)
            })?;
            let traversal = visits
                .checked_add(read_replay)?
                .checked_add(source_floor)?
                .checked_add(frozen)?
                .checked_add(growth)?
                .checked_add(4096)?;
            let replay = source
                .count
                .checked_add(1)?
                .checked_mul(traversal)?
                .checked_add(4096)?;
            // Complete descriptor inventories bound caller geometry and all
            // fixed owner/type/cache queries, including their finish checks.
            let probe = metadata
                .scalar_bytes
                .checked_add(metadata.observation_schema_bytes)?
                .checked_mul(128)?
                .checked_add(inventory.checked_mul(8192)?)?
                .checked_add(65536)?;
            replay.checked_add(probe)
        })();
        Ok(credit)
    }
    pub(in crate::native_artifact::program::runtime) fn has_numeric_map(&self) -> bool {
        self.native_map.is_some()
    }
    pub(in crate::native_artifact::program::runtime) fn fresh_numeric_header(
        &self,
        collection: u32,
        iteration: u32,
        progress: &[u64],
    ) -> bool {
        self.native_map.is_none()
            && self.native_fill.is_none()
            && !self.has_utf8_read()
            && self.current == Some(collection)
            && self
                .sources
                .get(iteration as usize)
                .and_then(Option::as_ref)
                .is_some_and(|source| {
                    matches!(
                        source.backing,
                        SourceBacking::Value(_) | SourceBacking::U64Range { .. }
                    ) && progress.get(iteration as usize * 2) == Some(&source.count)
                        && progress.get(iteration as usize * 2 + 1) == Some(&1)
                        && source.index_offset == 0
                        && source.element.is_none()
                })
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_numeric_map(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &Region,
        prior: mech_core::ResourceDemand,
        replay_credit: u64,
    ) -> Result<Option<u64>, Error> {
        if !self.fresh_numeric_header(region.collection, region.iteration, ctx.progress())
            || ctx.ledger().current != Some(region.iteration)
        {
            return Ok(None);
        }
        let metadata = ctx.descriptor().metadata();
        let plan = &metadata.plan;
        let source = self.sources[region.iteration as usize].as_ref().unwrap();
        let count = source.count;
        let memory = ctx.library().memory_budget();
        let (
            owner,
            source_handle,
            input,
            host_count,
            range_start,
            mut source_bytes,
            mut source_nodes,
        ) = match &source.backing {
            SourceBacking::Value(owner) if !region.range => {
                let ty = &metadata.types[region.source as usize];
                let kind = matrix_kind(plan.values()[region.source as usize].definition().body())
                    .ok_or_else(|| invalid("numeric map source kind disappeared"))?;
                let Some((input, host_count)) = source_pointer(owner, kind) else {
                    return Ok(None);
                };
                if host_count as u64 != count || !ctx.arena_entry_is_current(region.source)? {
                    return Ok(None);
                }
                let handle = ctx.arena_current_handle(region.source)?;
                let current = ctx.arena_get(handle)?;
                if !current.shares_frozen_storage(owner)
                    || !std::ptr::eq(current.shape(), owner.shape())
                    || owner.schema() != ty.schema
                    || owner.schema_key() != ty.key
                {
                    return Ok(None);
                }
                let Some(bytes) = owner.memory_budget_retained_bytes(&memory) else {
                    return Ok(None);
                };
                let Some(nodes) = owner.memory_budget_retained_nodes(&memory) else {
                    return Ok(None);
                };
                (
                    Some(owner.clone()),
                    Some(handle),
                    input,
                    host_count,
                    0,
                    bytes,
                    nodes,
                )
            }
            SourceBacking::U64Range { start } if region.range => {
                let Ok(host_count) = usize::try_from(count) else {
                    return Ok(None);
                };
                if count != 0 && start.checked_add(count - 1).is_none() {
                    return Ok(None);
                }
                (None, None, 0, host_count, *start, 0, 0)
            }
            _ => return Ok(None),
        };
        let mut matrix_owners: [Option<MatrixOwner>; MAX_MATRICES] = std::array::from_fn(|_| None);
        // A bounded stack inventory retains genuine current immutable owners.
        // It grants no pointer until all finite admission below has succeeded.
        for (at, matrix) in region.matrices.iter().enumerate() {
            if !ctx.arena_entry_is_current(matrix.value)? {
                return Ok(None);
            }
            let handle = ctx.arena_current_handle(matrix.value)?;
            let value = ctx.arena_get(handle)?;
            let Some(bytes) = value.memory_budget_retained_bytes(&memory) else {
                return Ok(None);
            };
            let Some(nodes) = value.memory_budget_retained_nodes(&memory) else {
                return Ok(None);
            };
            source_bytes = add(source_bytes, bytes)?;
            source_nodes = add(source_nodes, nodes)?;
            matrix_owners[at] = Some(MatrixOwner {
                handle,
                owner: value.clone(),
            });
        }
        let old_live = self.live_bytes;
        let retained = self.retained_nodes()?;
        let builder = self.builder(region.collection)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.values.packed_width() != Some(region.output_width as u64)
            || builder.join.is_some()
            || builder.query.is_some()
            || builder.histogram.is_some()
        {
            return Ok(None);
        }
        let output_definition = plan.values()[region.output as usize].definition();
        let output_kind = matrix_kind(output_definition.body())
            .ok_or_else(|| invalid("numeric map output kind disappeared"))?;
        let read_work = region.reads.iter().try_fold(0u64, |n, read| {
            let call = metadata.calls.get(read.site as usize)?.as_ref()?;
            n.checked_add(call.scalar_matrix_element_native_work()?)
        });
        let Some(read_work) = read_work else {
            return Ok(None);
        };
        let geometry = (|| {
            let input_bytes = if region.range {
                0
            } else {
                count.checked_mul(u64::from(region.input_width))?
            };
            let output_bytes = count.checked_mul(u64::from(region.output_width))?;
            if input_bytes > isize::MAX as u64 || output_bytes > isize::MAX as u64 {
                return None;
            }
            let metadata_bytes = size_of::<Loan>() as u64;
            let envelope = metadata
                .scalar_bytes
                .checked_add(output_bytes.checked_mul(4)?)?
                .checked_add(4096)?;
            // Retain both source element and packed push logical floors.
            let commit_compute = count.checked_mul(
                352u64
                    .checked_add(
                        (u64::from(region.input_width) + u64::from(region.output_width))
                            .checked_mul(16)?,
                    )?
                    .checked_add(read_work)?,
            )?;
            let commit_cloned = input_bytes.checked_add(output_bytes)?;
            let finish = envelope.checked_mul(16)?.checked_add(256)?;
            let allocation = output_bytes
                .checked_mul(16)?
                .checked_add(metadata_bytes.checked_mul(16)?)?
                .checked_add(512)?;
            // A bounded scalar step emits at most1024 physical work units;
            // reads, guards and the loop/store have their own4096-unit floor.
            let per_element = (region.steps.len() as u64)
                .checked_mul(1024)?
                .checked_add(4096)?;
            let preflight = count.checked_mul(per_element)?.checked_add(8192)?;
            let setup = source_bytes
                .checked_mul(64)?
                .checked_add(metadata.observation_schema_bytes.checked_mul(16)?)?
                .checked_add(allocation)?
                .checked_add(2048)?;
            let total = setup
                .checked_add(preflight)?
                .checked_add(commit_compute)?
                .checked_add(finish)?;
            let compute = builder
                .compute
                .checked_add(commit_compute)?
                .checked_add(finish)?;
            let cloned = builder.cloned_bytes.checked_add(commit_cloned)?;
            let temporary = prior
                .turn_peak_bytes
                .checked_add(old_live)?
                .checked_add(source_bytes)?
                .checked_add(metadata_bytes)?
                .checked_add(output_bytes)?
                .checked_add(envelope)?;
            let nodes = prior
                .retained_nodes
                .checked_add(source_nodes)?
                .checked_add(retained)?
                .checked_add(8)?;
            let output_bound = builder
                .values
                .output_bytes(
                    output_bytes,
                    count,
                    host_count,
                    output_definition.dimension_parameters().len(),
                )
                .ok()?;
            Some((
                input_bytes,
                output_bytes,
                metadata_bytes,
                envelope,
                commit_compute,
                commit_cloned,
                preflight,
                setup,
                total,
                compute,
                cloned,
                temporary,
                nodes,
                output_bound,
            ))
        })();
        let Some((
            input_bytes,
            output_bytes,
            metadata_bytes,
            envelope,
            commit_compute,
            commit_cloned,
            preflight,
            setup,
            total,
            compute,
            cloned,
            temporary,
            nodes,
            output_bound,
        )) = geometry
        else {
            return Ok(None);
        };
        let limits = ctx.limits();
        let read_output_bytes = region
            .matrices
            .iter()
            .map(|m| u64::from(m.width))
            .max()
            .unwrap_or(0);
        let scalar_read_temporary = prior
            .turn_peak_bytes
            .checked_add(old_live)
            .and_then(|v| v.checked_add(source_bytes))
            .and_then(|v| v.checked_add(32));
        let scalar_read_nodes = prior
            .retained_nodes
            .checked_add(retained)
            .and_then(|v| v.checked_add(source_nodes))
            .and_then(|v| v.checked_add(4));
        if !fits(read_output_bytes, limits.max_output_bytes)
            || scalar_read_temporary.is_none_or(|v| !fits(v, limits.max_temporary_bytes))
            || scalar_read_nodes.is_none_or(|v| !fits(v, limits.max_retained_nodes))
            || !fits(count, limits.max_output_elements)
            || !fits(compute, limits.max_compute_work)
            || !fits(cloned, limits.max_cloned_bytes)
            || !fits(temporary, limits.max_temporary_bytes)
            || !fits(nodes, limits.max_retained_nodes)
            || !fits(output_bound, limits.max_output_bytes)
        {
            return Ok(None);
        }
        let Some(admission) = total.checked_add(replay_credit) else {
            return Ok(None);
        };
        if ctx
            .library()
            .unscoped_remaining()
            .is_none_or(|(remaining, _)| remaining < admission)
        {
            return Ok(None);
        }
        let remaining = {
            let work = ctx.work();
            work.limit().saturating_sub(work.used())
        };
        if admission > remaining {
            return Ok(None);
        }
        let physical = add(add(output_bytes, metadata_bytes)?, envelope)?;
        match memory.reserve_capacity(physical) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        match memory.reserve_nodes(8) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        // Every metadata/schema walk and full native attempt is paid even if
        // a checked scalar later fails. No failed allocator is cold-retried.
        charge(&mut ctx.work(), setup)?;
        if let Some(owner) = &owner {
            owner
                .validate_against(&metadata.schemas)
                .map_err(Error::Snapshot)?;
        }
        let mut matrices = [MatrixView::default(); MAX_MATRICES];
        for (at, matrix) in region.matrices.iter().enumerate() {
            let call = metadata.calls[matrix.witness as usize]
                .as_ref()
                .ok_or_else(|| invalid("numeric map read factory disappeared"))?;
            let pin = matrix_owners[at].as_ref().unwrap();
            let Some([input, cells, rows, columns]) =
                call.scalar_matrix_element_native_view(&pin.owner, &memory)
            else {
                return Ok(None);
            };
            if rows.checked_mul(columns) != Some(cells)
                || cells
                    .checked_mul(u64::from(matrix.width))
                    .is_none_or(|n| n > isize::MAX as u64)
            {
                return Ok(None);
            }
            matrices[at] = MatrixView {
                input,
                count: cells,
                rows,
                columns,
            };
        }
        let output_reservation = memory.reserve_capacity(output_bytes)?;
        let metadata_reservation = memory.reserve_capacity(metadata_bytes)?;
        let nodes = memory.reserve_nodes(3)?;
        let mut values = data::CollectedValues::new(output_definition.body());
        values.reserve(host_count)?;
        if values.capacity() != host_count {
            return Err(invalid("numeric map output capacity differs"));
        }
        let output = values.scalar_capacity_pointer(output_kind)?;
        let epoch = ctx
            .ledger()
            .visits
            .checked_add(1)
            .ok_or_else(|| invalid("numeric map epoch exhausted"))?;
        ctx.library().charge(preflight, 0).map_err(|error| {
            crate::native_artifact::NativeExecutionError::LibraryWork {
                phase: Phase::Setup,
                error,
            }
        })?;
        let frame = Frame {
            input,
            input_bytes,
            output,
            capacity_bytes: output_bytes,
            count,
            input_width: u64::from(region.input_width),
            output_width: u64::from(region.output_width),
            epoch,
            issued: epoch,
            next: 0,
            range_start,
            matrices,
        };
        let loan = super::utf8_read::boxed(Loan {
            frame,
            expected: frame,
            values,
            source: owner,
            source_handle,
            source_slot: region.source,
            matrices: matrix_owners,
            output_kind,
            collection: region.collection,
            iteration: region.iteration,
            header: region.header as usize,
            commit_compute,
            commit_cloned,
            work: ctx.library().clone(),
            output_reservation,
            _metadata: metadata_reservation,
            _nodes: nodes,
        })
        .ok_or_else(|| invalid("numeric map loan allocation"))?;
        let pointer = (&loan.frame as *const Frame) as u64;
        self.native_map = Some(loan);
        Ok(Some(pointer))
    }
    pub(in crate::native_artifact::program::runtime) fn finish_numeric_map(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        pointer: u64,
        status: u64,
    ) -> Result<bool, Error> {
        let mut loan = self
            .native_map
            .take()
            .ok_or_else(|| invalid("numeric map finish has no loan"))?;
        let mut actual = loan.frame;
        actual.next = 0;
        if pointer != (&loan.frame as *const Frame) as u64
            || loan.header != site
            || self.current != Some(loan.collection)
            || ctx.ledger().current != Some(loan.iteration)
            || ctx.ledger().visits.checked_add(1) != Some(loan.expected.epoch)
            || actual != loan.expected
            || loan.frame.next > loan.frame.count
            || status > 1
            || !loan.work.shares_account(ctx.library())
            || loan.frame.output != loan.values.scalar_capacity_pointer(loan.output_kind)?
            || loan.output_reservation.capacity_bytes() != loan.expected.capacity_bytes
            || loan.values.len() != 0
            || loan.values.capacity() as u64 != loan.expected.count
        {
            return Err(invalid("numeric map changed its exact admitted loan"));
        }
        let source = self.sources[loan.iteration as usize]
            .as_ref()
            .ok_or_else(|| invalid("numeric map source disappeared"))?;
        let source_current = match (&source.backing, &loan.source, loan.source_handle) {
            (SourceBacking::Value(value), Some(owner), Some(handle)) => {
                let kind = matrix_kind(
                    ctx.descriptor().metadata().plan.values()[loan.source_slot as usize]
                        .definition()
                        .body(),
                )
                .ok_or_else(|| invalid("numeric map source kind changed"))?;
                value.shares_frozen_storage(owner)
                    && ctx.arena_get(handle)?.shares_frozen_storage(owner)
                    && source_pointer(owner, kind)
                        == Some((loan.expected.input, loan.expected.count as usize))
            }
            (SourceBacking::U64Range { start }, None, None) => *start == loan.expected.range_start,
            _ => false,
        };
        if !source_current
            || ctx.progress().get(loan.iteration as usize * 2) != Some(&loan.expected.count)
            || ctx.progress().get(loan.iteration as usize * 2 + 1) != Some(&1)
            || source.index_offset != 0
            || source.element.is_some()
        {
            return Err(invalid("numeric map source visit changed"));
        }
        for pin in loan.matrices.iter().flatten() {
            if !ctx.arena_get(pin.handle)?.shares_frozen_storage(&pin.owner) {
                return Err(invalid("numeric map matrix owner changed"));
            }
        }
        if status == 1 {
            return Ok(false);
        }
        if loan.frame.next != loan.frame.count {
            return Err(invalid("numeric map incomplete success"));
        }
        self.account(
            ctx,
            loan.collection,
            loan.commit_cloned,
            loan.commit_compute,
        )?;
        // SAFETY: only the genuine emitted kernel initializes this fresh exact
        // capacity. Full completion, kind/width, current owners and epoch were
        // checked above. The emitted Bool store additionally guards <=1.
        unsafe {
            loan.values
                .set_initialized_scalar_len(loan.expected.count as usize, loan.output_kind)?
        };
        let old_live = self.live_bytes;
        let builder = self.builder(loan.collection)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
        {
            return Err(invalid("numeric map builder changed during loan"));
        }
        let Loan {
            values,
            output_reservation,
            ..
        } = *loan;
        let bytes = output_reservation.capacity_bytes();
        builder.values = values;
        builder.reservation = output_reservation;
        builder.payload_bytes = bytes;
        self.live_bytes = add(old_live, bytes)?;
        Ok(true)
    }
}
