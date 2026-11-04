//! Atomic native preflight for a complete read-only String-key collection.
//! All private output is discarded on a miss; ordinary source visits then
//! retain their original errors. Only complete success commits body fees and
//! transfers the already filled U64 allocation to the maintained builder.
use super::*;
use crate::native_artifact::program::prepare::utf8_reads::Region;
use crate::native_artifact::program::runtime::maps::utf8_read::{
    Cell, Frame, HASH_OFFSET, HASH_PRIME, Slot,
};
use mech_core::snapshot::{MapValue, SequenceView};
use mech_core::{ResourceDemand, SchemaBody};

pub(super) struct Loan {
    header: usize,
    collection: usize,
    iteration: u32,
    source: Value,
    map_root: Value,
    source_handle: NativeValueHandle,
    map_handle: NativeValueHandle,
    slots: Box<[Slot]>,
    cells: Box<[Cell]>,
    output: Vec<u64>,
    pub(super) frame: Box<Frame>,
    commit_compute: u64,
    commit_cloned: u64,
    output_reservation: ManagedMemoryReservation,
    _metadata: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
fn fits(need: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|cap| need <= cap)
}
fn hash(key: &[u8]) -> u64 {
    key.iter().fold(HASH_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(HASH_PRIME)
    })
}
fn fixed<T: Copy>(len: usize, value: T) -> Option<Box<[T]>> {
    if len == 0 {
        return Some(Box::default());
    }
    let layout = std::alloc::Layout::array::<std::mem::MaybeUninit<T>>(len).ok()?;
    // SAFETY: the caller admitted this precise layout and all entries are
    // initialized before conversion. No intermediate grow/shrink is used.
    let ptr = unsafe { std::alloc::alloc(layout) }.cast::<std::mem::MaybeUninit<T>>();
    let ptr = std::ptr::NonNull::new(ptr)?;
    let mut out = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr.as_ptr(), len)) };
    for item in &mut out {
        item.write(value);
    }
    Some(unsafe { out.assume_init() })
}
pub(super) fn boxed<T>(value: T) -> Option<Box<T>> {
    let layout = std::alloc::Layout::new::<T>();
    if layout.size() == 0 {
        return Some(Box::new(value));
    }
    // SAFETY: caller prepaid exactly this layout. Null declines before any
    // public state changes; initialization writes one complete owned value.
    let pointer = unsafe { std::alloc::alloc(layout) }.cast::<T>();
    let pointer = std::ptr::NonNull::new(pointer)?;
    unsafe {
        pointer.as_ptr().write(value);
        Some(Box::from_raw(pointer.as_ptr()))
    }
}
fn map<'a>(owner: &'a Value, ordinal: Option<usize>) -> Option<&'a MapValue> {
    match (owner.data(), ordinal) {
        (ValueData::Map(map), None) => Some(map),
        (ValueData::Record(record), Some(ordinal)) => match record.fields().get(ordinal)? {
            ValueData::Map(map) => Some(map),
            _ => None,
        },
        _ => None,
    }
}
impl State {
    /// Fixed-size scheduling preview only. It grants neither a source visit
    /// nor a native loan; complete authority remains in prepare_utf8_read.
    pub(in crate::native_artifact::program::runtime) fn fresh_utf8_header(
        &self,
        collection: u32,
        iteration: u32,
        progress: &[u64],
    ) -> bool {
        self.native_read.is_none()
            && self.current == Some(collection)
            && self
                .sources
                .get(iteration as usize)
                .and_then(Option::as_ref)
                .is_some_and(|source| {
                    progress.get(iteration as usize * 2) == Some(&source.count)
                        && progress.get(iteration as usize * 2 + 1) == Some(&1)
                        && source.index_offset == 0
                        && source.element.is_none()
                })
    }
    pub(in crate::native_artifact::program::runtime) fn has_utf8_read(&self) -> bool {
        self.native_read.is_some()
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_utf8_read(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &Region,
        prior: ResourceDemand,
    ) -> Result<Option<u64>, Error> {
        if self.native_read.is_some() {
            return Err(invalid("overlapping native String lookup collection"));
        }
        let descriptor = ctx.descriptor();
        let metadata = descriptor.metadata();
        let plan = &metadata.plan;
        let id = region.collection as usize;
        let iteration = region.iteration as usize;
        if self.current != Some(id as u32) || ctx.ledger().current != Some(region.iteration) {
            return Err(invalid("native String lookup outside exact active builder"));
        }
        let source = self
            .sources
            .get(iteration)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("native String lookup missing source"))?;
        if ctx.progress().get(iteration * 2) != Some(&source.count)
            || ctx.progress().get(iteration * 2 + 1) != Some(&1)
            || source.index_offset != 0
            || source.element.is_some()
        {
            return Ok(None);
        }
        let source_count = source.count;
        let SourceBacking::Value(source_owner) = &source.backing else {
            return Ok(None);
        };
        let source = source_owner.clone();
        let ValueData::Matrix(matrix) = source.data() else {
            return Ok(None);
        };
        let SequenceView::String(strings) = matrix.elements() else {
            return Ok(None);
        };
        let schema = plan.values()[region.source as usize].definition();
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = schema.body()
        else {
            return Ok(None);
        };
        if element.as_ref() != &SchemaBody::String
            || dimensions.len() != 2
            || strings.len() as u64 != source_count
        {
            return Ok(None);
        }
        if !ctx.arena_entry_is_current(region.source)?
            || !ctx.arena_entry_is_current(region.map_root)?
        {
            return Ok(None);
        }
        let source_handle = ctx.arena_current_handle(region.source)?;
        let current = ctx.arena_get(source_handle)?;
        if !current.shares_frozen_storage(&source)
            || !std::ptr::eq(current.shape(), source.shape())
            || current.schema() != source.schema()
            || current.schema_key() != source.schema_key()
        {
            return Ok(None);
        }
        let map_root = ctx.arena_copy_current(region.map_root)?;
        let map_handle = ctx.arena_current_handle(region.map_root)?;
        let source_type = &metadata.types[region.source as usize];
        let map_type = &metadata.types[region.map_root as usize];
        if source.schema() != source_type.schema
            || source.schema_key() != source_type.key
            || map_root.schema() != map_type.schema
            || map_root.schema_key() != map_type.key
        {
            return Ok(None);
        }
        if source.validate_against(&metadata.schemas).is_err()
            || map_root.validate_against(&metadata.schemas).is_err()
        {
            return Ok(None);
        }
        let Some(table) = map(&map_root, region.map_ordinal) else {
            return Ok(None);
        };
        let memory = ctx.library().memory_budget();
        let Some(source_bytes) = source.memory_budget_retained_bytes(&memory) else {
            return Ok(None);
        };
        let Some(map_bytes) = map_root.memory_budget_retained_bytes(&memory) else {
            return Ok(None);
        };
        let Some(source_nodes) = source.memory_budget_retained_nodes(&memory) else {
            return Ok(None);
        };
        let Some(map_nodes) = map_root.memory_budget_retained_nodes(&memory) else {
            return Ok(None);
        };
        let count = strings.len();
        let Some(capacity) = table
            .entries()
            .len()
            .checked_mul(2)
            .and_then(|n| n.max(4).checked_next_power_of_two())
        else {
            return Ok(None);
        };
        let Some(output_bytes) = (count as u64).checked_mul(8) else {
            return Ok(None);
        };
        let metadata_bytes = (capacity as u64)
            .checked_mul(size_of::<Slot>() as u64)
            .and_then(|n| n.checked_add((count as u64).checked_mul(size_of::<Cell>() as u64)?))
            .and_then(|n| n.checked_add((size_of::<Loan>() + size_of::<Frame>()) as u64));
        let Some(metadata_bytes) = metadata_bytes else {
            return Ok(None);
        };
        let setup = map_bytes
            .checked_add(source_bytes)
            .and_then(|n| n.checked_mul(64))
            .and_then(|n| n.checked_add(metadata_bytes.checked_add(output_bytes)?.checked_mul(16)?))
            .and_then(|n| {
                n.checked_add(
                    (table.entries().len() as u64)
                        .checked_mul(capacity as u64)?
                        .checked_mul(64)?,
                )
            })
            .and_then(|n| n.checked_add(metadata.observation_schema_bytes.checked_mul(16)?))
            .and_then(|n| n.checked_add(2048));
        let Some(setup) = setup else { return Ok(None) };
        if ctx
            .library()
            .unscoped_remaining()
            .is_none_or(|(remaining, _)| remaining < setup)
        {
            return Ok(None);
        }
        charge(&mut ctx.work(), setup)?;
        let Action::LibraryCall {
            template: read_template,
            ..
        } = plan.blocks()[region.read_site as usize].action()
        else {
            return Ok(None);
        };
        let Some(read) = metadata
            .calls
            .get(region.read_site as usize)
            .and_then(Option::as_ref)
        else {
            return Ok(None);
        };
        if read.template() != read_template || !read.supports_borrowed_record_map_read() {
            return Ok(None);
        }
        if let Some(site) = region.project_site {
            let Action::LibraryCall { template, .. } = plan.blocks()[site as usize].action() else {
                return Ok(None);
            };
            let Some(project) = metadata.calls.get(site as usize).and_then(Option::as_ref) else {
                return Ok(None);
            };
            if project.template() != template || !project.supports_borrowed_record_map_projection()
            {
                return Ok(None);
            }
        }
        // Only O(1) count/retained metadata was read above. Every full
        // inventory/hash scan below is independently covered by this setup.
        let rows = source
            .shape()
            .resolve_dimension(&dimensions[0])
            .map_err(|_| invalid("native String lookup rows"))?;
        let columns = source
            .shape()
            .resolve_dimension(&dimensions[1])
            .map_err(|_| invalid("native String lookup columns"))?;
        if rows.checked_mul(columns) != Some(source_count) {
            return Ok(None);
        }
        let Some(stored_bytes) = table.entries().iter().try_fold(0u64, |sum, entry| {
            let (ValueData::String(key), ValueData::U64(_)) = (entry.key().data(), entry.value())
            else {
                return None;
            };
            sum.checked_add(key.len() as u64)
        }) else {
            return Ok(None);
        };
        let Some(key_bytes) = strings
            .iter()
            .try_fold(0u64, |sum, key| sum.checked_add(key.len() as u64))
        else {
            return Ok(None);
        };
        let maximum_key = strings
            .iter()
            .map(|key| key.len() as u64)
            .max()
            .unwrap_or(0);
        // This whole-loop worst-case preflight is paid even on an early miss.
        // It includes every hash byte and a complete bounded collision chain.
        let preflight_compare = (capacity as u64)
            .checked_mul(4)
            .and_then(|n| n.checked_add(32))
            .and_then(|n| n.checked_add(stored_bytes))
            .and_then(|n| n.checked_mul(count as u64))
            .and_then(|n| n.checked_add(key_bytes));
        let Some(preflight_compare) = preflight_compare else {
            return Ok(None);
        };
        let Some(preflight_compute) = preflight_compare
            .checked_mul(64)
            .and_then(|n| n.checked_add((count as u64).checked_mul(1024)?))
        else {
            return Ok(None);
        };
        // Retain the source producer and packed push's original logical floors
        // while removing actual canonical String/scalar construction work.
        let Some(commit_compute) = key_bytes
            .checked_mul(16)
            .and_then(|n| n.checked_add((count as u64).checked_mul(416)?))
        else {
            return Ok(None);
        };
        let Some(commit_cloned) = key_bytes.checked_add(output_bytes) else {
            return Ok(None);
        };
        let builder = self.builder(id as u32)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.values.packed_width() != Some(8)
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.join.is_some()
            || builder.histogram.is_some()
            || builder.query.is_some()
        {
            return Ok(None);
        }
        let envelope = output_bytes
            .checked_mul(4)
            .and_then(|bytes| metadata.scalar_bytes.checked_add(bytes))
            .and_then(|n| n.checked_add(4096));
        let Some(envelope) = envelope else {
            return Ok(None);
        };
        let Some(finish) = envelope.checked_mul(16).and_then(|n| n.checked_add(256)) else {
            return Ok(None);
        };
        let Some(compute) = builder
            .compute
            .checked_add(commit_compute)
            .and_then(|n| n.checked_add(finish))
        else {
            return Ok(None);
        };
        let Some(cloned) = builder.cloned_bytes.checked_add(commit_cloned) else {
            return Ok(None);
        };
        let Some(output_bound) = output_bytes
            .checked_add((size_of::<Value>() + size_of::<ValueData>()) as u64)
            .and_then(|n| {
                n.checked_add(
                    (plan.values()[region.output as usize]
                        .definition()
                        .dimension_parameters()
                        .len() as u64)
                        .checked_mul(8)?,
                )
            })
        else {
            return Ok(None);
        };
        let temp = prior
            .turn_peak_bytes
            .checked_add(source_bytes)
            .and_then(|n| n.checked_add(map_bytes))
            .and_then(|n| n.checked_add(metadata_bytes))
            .and_then(|n| n.checked_add(output_bytes))
            .and_then(|n| n.checked_add(envelope))
            .and_then(|n| {
                n.checked_add(
                    metadata
                        .scalar_bytes
                        .checked_add(maximum_key.checked_mul(4)?)?,
                )
            });
        let nodes = prior
            .retained_nodes
            .checked_add(source_nodes)
            .and_then(|n| n.checked_add(map_nodes))
            .and_then(|n| n.checked_add(16));
        let Some(temp) = temp else { return Ok(None) };
        let Some(nodes) = nodes else { return Ok(None) };
        let Some(max_map_compute) = (capacity as u64)
            .checked_mul(4)
            .and_then(|n| n.checked_add(32))
            .and_then(|n| n.checked_add(stored_bytes))
            .and_then(|n| n.checked_add(maximum_key))
            .and_then(|n| n.checked_mul(64))
        else {
            return Ok(None);
        };
        let Some(max_call_clone) = map_bytes.checked_add(maximum_key) else {
            return Ok(None);
        };
        let limits = ctx.limits();
        if !fits(count as u64, limits.max_output_elements)
            || !fits(compute, limits.max_compute_work)
            || !fits(max_map_compute, limits.max_compute_work)
            || !fits(cloned.max(max_call_clone), limits.max_cloned_bytes)
            || !fits(output_bound.max(map_bytes), limits.max_output_bytes)
            || !fits(temp, limits.max_temporary_bytes)
            || !fits(nodes, limits.max_retained_nodes)
            || !fits(preflight_compare, limits.max_comparison_work)
        {
            return Ok(None);
        }
        let Some(total) = preflight_compute
            .checked_add(commit_compute)
            .and_then(|n| n.checked_add(finish))
            .and_then(|n| n.checked_add(4096))
        else {
            return Ok(None);
        };
        if ctx
            .library()
            .unscoped_remaining()
            .is_none_or(|(c, p)| c < total || p < preflight_compare)
        {
            return Ok(None);
        }
        let Some(physical) = metadata_bytes
            .checked_add(output_bytes)
            .and_then(|n| n.checked_add(envelope))
        else {
            return Ok(None);
        };
        let Ok(probe) = memory.reserve_capacity(physical) else {
            return Ok(None);
        };
        drop(probe);
        let Ok(probe) = memory.reserve_nodes(16) else {
            return Ok(None);
        };
        drop(probe);
        let Ok(metadata_reservation) = memory.reserve_capacity(metadata_bytes) else {
            return Ok(None);
        };
        let Ok(output_reservation) = memory.reserve_capacity(output_bytes) else {
            return Ok(None);
        };
        let Ok(nodes) = memory.reserve_nodes(5) else {
            return Ok(None);
        };
        let Some(mut slots) = fixed(capacity, Slot::default()) else {
            return Ok(None);
        };
        let Some(mut cells) = fixed(count, Cell::default()) else {
            return Ok(None);
        };
        let mut output = Vec::<u64>::new();
        if output.try_reserve_exact(count).is_err() || output.capacity() != count {
            return Ok(None);
        };
        output.resize(count, 0);
        for entry in table.entries() {
            let (ValueData::String(key), ValueData::U64(value)) =
                (entry.key().data(), entry.value())
            else {
                return Ok(None);
            };
            let code = hash(key.as_bytes());
            let mut at = code as usize & (capacity - 1);
            while slots[at].occupied != 0 {
                at = (at + 1) & (capacity - 1)
            }
            slots[at] = Slot {
                address: key.as_ptr() as u64,
                bytes: key.len() as u64,
                value: *value,
                hash: code,
                occupied: 1,
            };
        }
        for (cell, key) in cells.iter_mut().zip(strings) {
            *cell = Cell {
                address: key.as_ptr() as u64,
                bytes: key.len() as u64,
                logical_cloned_bytes: key.len() as u64,
                selection_compute_work: (key.len() as u64) * 16 + 160,
                logical_temporary_bytes: metadata.scalar_bytes + (key.len() as u64) * 4,
                ..Cell::default()
            };
        }
        // An invocation visit never wraps. No pointer can survive this exact
        // prepare/finish pair, so its fresh checked visit is the loan epoch.
        let Some(epoch) = ctx.ledger().visits.checked_add(1) else {
            return Ok(None);
        };
        ctx.library()
            .charge(preflight_compute, preflight_compare)
            .map_err(
                |error| crate::native_artifact::NativeExecutionError::LibraryWork {
                    phase: Phase::Setup,
                    error,
                },
            )?;
        let Some(frame) = boxed(Frame {
            slots: slots.as_ptr() as u64,
            capacity: capacity as u64,
            cells: cells.as_ptr() as u64,
            count: count as u64,
            // CollectionElement visits physical source order, unlike the
            // one-based column-major MatrixScalarAccess producer.
            rows: 1,
            columns: count as u64,
            output: output.as_mut_ptr() as u64,
            stored_bytes,
            fuel: preflight_compute,
            comparison: preflight_compare,
            epoch,
            issued: epoch,
            next: 0,
        }) else {
            return Ok(None);
        };
        let pointer = (&*frame as *const Frame) as u64;
        let Some(loan) = boxed(Loan {
            header: region.header as usize,
            collection: id,
            iteration: region.iteration,
            source,
            map_root,
            source_handle,
            map_handle,
            slots,
            cells,
            output,
            frame,
            commit_compute,
            commit_cloned,
            output_reservation,
            _metadata: metadata_reservation,
            _nodes: nodes,
        }) else {
            return Ok(None);
        };
        self.native_read = Some(loan);
        Ok(Some(pointer))
    }
    pub(in crate::native_artifact::program::runtime) fn finish_utf8_read(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        pointer: u64,
        status: u64,
    ) -> Result<bool, Error> {
        let loan = self
            .native_read
            .take()
            .ok_or_else(|| invalid("native String lookup completion has no loan"))?;
        if loan.header != site
            || pointer != (&*loan.frame as *const Frame) as u64
            || self.current != Some(loan.collection as u32)
            || ctx.ledger().current != Some(loan.iteration)
            || loan.frame.issued != loan.frame.epoch
            || loan.frame.epoch == 0
        {
            return Err(invalid(
                "native String lookup completion changed its exact owner",
            ));
        }
        if !ctx
            .arena_get(loan.source_handle)?
            .shares_frozen_storage(&loan.source)
            || !ctx
                .arena_get(loan.map_handle)?
                .shares_frozen_storage(&loan.map_root)
        {
            return Err(invalid("native String lookup source generation changed"));
        }
        if status != 0 {
            return Ok(false);
        }
        if loan.frame.next != loan.frame.count
            || loan.frame.count != loan.output.len() as u64
            || loan.frame.slots != loan.slots.as_ptr() as u64
            || loan.frame.cells != loan.cells.as_ptr() as u64
            || loan.frame.output != loan.output.as_ptr() as u64
        {
            return Err(invalid("incomplete native String lookup success"));
        }
        self.account(
            ctx,
            loan.collection as u32,
            loan.commit_cloned,
            loan.commit_compute,
        )?;
        let Loan {
            collection,
            output,
            output_reservation,
            ..
        } = *loan;
        let bytes = output_reservation.capacity_bytes();
        let old = self.live_bytes;
        let builder = self.builder(collection as u32)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
        {
            return Err(invalid("native String lookup builder changed during loan"));
        }
        builder.values = data::CollectedValues::U64(output);
        builder.reservation = output_reservation;
        builder.payload_bytes = bytes;
        self.live_bytes = add(old, bytes)?;
        Ok(true)
    }
}
