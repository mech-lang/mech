//! Atomic native preflight for a complete read-only String-key collection.
//! All private output is discarded on a miss; ordinary source visits then
//! retain their original errors. Only complete success commits body fees and
//! transfers the already filled U64 allocation to the maintained builder.
use super::*;
use crate::native_artifact::program::prepare::utf8_reads::Region;
use crate::native_artifact::program::runtime::maps::owner::generic_dictionary::Dictionary;
use crate::native_artifact::program::runtime::maps::string::frozen::FrozenStringMap;
use crate::native_artifact::program::runtime::maps::utf8_read::{
    ADMITTED_CELL_BYTES, Cell, Frame, HASH_OFFSET, HASH_PRIME, Slot,
};
use mech_core::snapshot::{MapValue, SequenceView};
use mech_core::{ResourceDemand, SchemaBody};

/// One invocation-scoped private classification, produced by native hashing.
/// Source generation and exact producer are rechecked at the consumer. This
/// carries no dictionary values, source visit, callback or publication permit.
pub(in crate::native_artifact::program::runtime) struct ClassifiedSource {
    pub source: Value,
    pub handle: NativeValueHandle,
    pub producer: usize,
    pub representatives: Box<[u64]>,
    pub first_rows: Box<[u64]>,
    pub reservation: ManagedMemoryReservation,
    pub _nodes: ManagedNodeReservation,
}
pub(super) enum Storage {
    Classes(ClassifiedSource),
    Loan(Loan),
}
pub(in crate::native_artifact::program::runtime) fn classification_bytes(
    count: usize,
) -> Option<u64> {
    (count as u64)
        .checked_mul(8)?
        .checked_add(size_of::<Storage>() as u64)
}
pub(super) struct Loan {
    header: usize,
    collection: usize,
    iteration: u32,
    source: Value,
    map_root: Option<MapBacking>,
    source_handle: NativeValueHandle,
    map_handle: NativeValueHandle,
    slots: Box<[Slot]>,
    cells: Box<[Cell]>,
    output: data::CollectedValues,
    classes: Option<ClassifiedSource>,
    pub(super) frame: Box<Frame>,
    commit_compute: u64,
    commit_cloned: u64,
    output_reservation: ManagedMemoryReservation,
    _metadata: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
enum MapBacking {
    Canonical(Value),
    Frozen(Dictionary),
}
enum Table<'a> {
    Canonical(&'a MapValue),
    Frozen(&'a FrozenStringMap),
}
impl Table<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Canonical(v) => v.entries().len(),
            Self::Frozen(v) => v.len(),
        }
    }
    fn key_bytes(&self) -> Option<u64> {
        match self {
            Self::Canonical(v) => v.entries().iter().try_fold(0u64, |n, e| {
                let (ValueData::String(k), ValueData::U64(_)) = (e.key().data(), e.value()) else {
                    return None;
                };
                n.checked_add(k.len() as u64)
            }),
            Self::Frozen(v) => Some(v.key_bytes()),
        }
    }
    fn each(&self, mut visit: impl FnMut(&str, u64)) -> Result<bool, Error> {
        match self {
            Self::Canonical(v) => {
                for e in v.entries() {
                    let (ValueData::String(k), ValueData::U64(v)) = (e.key().data(), e.value())
                    else {
                        return Ok(false);
                    };
                    visit(k, *v)
                }
            }
            Self::Frozen(v) => {
                for (k, value) in v
                    .entries()
                    .map_err(super::super::maps::owner::storage_error)?
                {
                    visit(k, value)
                }
            }
        }
        Ok(true)
    }
}
// Tests run the old per-header scan through the exact same dynamic replay
// formulas to compare source errors, required/limit thresholds and retries.
#[cfg(all(test, feature = "canonical-native-program"))]
std::thread_local! {
    static REFERENCE_REPLAY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact) fn with_reference_replay_for_test<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            REFERENCE_REPLAY.with(|v| v.set(self.0));
        }
    }
    let reset = Reset(REFERENCE_REPLAY.with(|v| v.replace(true)));
    let result = run();
    drop(reset);
    result
}
fn replay_facts(
    metadata: &crate::native_artifact::program::prepare::ProgramMetadata,
    region: &Region,
) -> Option<crate::native_artifact::program::prepare::utf8_reads::ReplayFacts> {
    #[cfg(all(test, feature = "canonical-native-program"))]
    if REFERENCE_REPLAY.with(std::cell::Cell::get) {
        return crate::native_artifact::program::prepare::utf8_reads::reference_replay_facts(
            metadata, region,
        );
    }
    #[cfg(not(all(test, feature = "canonical-native-program")))]
    let _ = metadata;
    region.replay_facts()
}

fn fits(need: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|cap| need <= cap)
}
fn hash(key: &[u8]) -> u64 {
    key.iter().fold(HASH_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(HASH_PRIME)
    })
}
pub(in crate::native_artifact::program::runtime) fn fixed<T: Copy>(
    len: usize,
    value: T,
) -> Option<Box<[T]>> {
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
pub(in crate::native_artifact::program::runtime) fn boxed<T>(value: T) -> Option<Box<T>> {
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
fn private_output(width: u8, count: usize) -> Option<data::CollectedValues> {
    macro_rules! allocate {
        ($kind:ident, $ty:ty) => {{
            let mut values = Vec::<$ty>::new();
            if values.try_reserve_exact(count).is_err() || values.capacity() != count {
                return None;
            }
            values.resize(count, 0);
            data::CollectedValues::$kind(values)
        }};
    }
    Some(match width {
        1 => allocate!(U8, u8),
        2 => allocate!(U16, u16),
        4 => allocate!(U32, u32),
        8 => allocate!(U64, u64),
        _ => return None,
    })
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
    /// Pure finite replay envelope, not paid credit or a source visit. Narrow
    /// failure retains paid physical work, but must not displace an ordinary
    /// arithmetic error with a speculative work/comparison denial.
    pub(in crate::native_artifact::program::runtime) fn utf8_replay_credit(
        &self,
        descriptor: &NativeProgramDescriptor,
        region: &Region,
        retirements: &[u64],
        map_bytes: u64,
        map_count: usize,
        memory: &ManagedMemoryBudget,
    ) -> Result<Option<(u64, u64)>, Error> {
        use crate::native_artifact::program::prepare::retirement::RetirementKind;
        let metadata = descriptor.metadata();
        let slots = metadata
            .retirements
            .slots(retirements, region.iteration, RetirementKind::Collection)
            .map_err(invalid)?;
        let Some(source) = self
            .sources
            .get(region.iteration as usize)
            .and_then(Option::as_ref)
        else {
            return Ok(None);
        };
        let SourceBacking::Value(owner) = &source.backing else {
            return Ok(None);
        };
        let Some(source_bytes) = owner.memory_budget_retained_bytes(memory) else {
            return Ok(None);
        };
        let credit = (|| {
            let n = source.count;
            let m = u64::try_from(map_count).ok()?;
            // A table bank has capacity<4*m (or four cells when empty).
            // Each probe compares at most its stored key plus this input key;
            // total stored bytes and any one input key are bounded by the
            // admitted owner envelopes. Repeated/shared strings stay covered.
            let capacity = m.checked_mul(4)?.checked_add(4)?;
            let physical_compare = capacity
                .checked_mul(4)?
                .checked_add(32)?
                .checked_add(map_bytes)?
                .checked_add(source_bytes)?;
            // Ordinary String search prepays candidate(len+8), then the
            // largest candidate/stored key envelope for every binary-search
            // level. The physical hash-bank scan alone does not bound this,
            // especially for one long absent selector among many tiny keys.
            let levels = m.checked_ilog2().map_or(0, |height| u64::from(height) + 1);
            let candidate = source_bytes.checked_add(8)?;
            let largest_key = source_bytes.checked_add(map_bytes)?.checked_add(8)?;
            let compare = physical_compare
                .checked_add(candidate)?
                .checked_add(largest_key.checked_mul(levels)?)?;
            let map_call = compare.checked_mul(64)?;
            let payload = source_bytes
                .checked_add(map_bytes)?
                .checked_add(metadata.scalar_bytes)?
                .checked_add(metadata.observation_schema_bytes)?;
            let facts = replay_facts(metadata, region)?;
            // Managed String/map projection roots can retire in this sealed
            // body. Their whole retained bound is at most payload, per slot.
            let retirement = payload
                .checked_mul(4)?
                .checked_add(64)?
                .checked_mul(slots.len() as u64)?;
            // Covers element construction/import, scalar push/growth, retained
            // map argument/owner queries, schema work and exact source visits.
            let traversal = facts.traversal(map_call, payload, retirement)?;
            // A returned virtual dictionary materializes a plain String/U64
            // Map. Mirror publication::sort_with_debits: floor(m/2)+(m-1)
            // sift calls, at most levels+1 iterations and two comparisons per
            // iteration, each comparing at most two complete key envelopes.
            let sift_calls = (m / 2).checked_add(m.saturating_sub(1))?;
            let sift_steps = sift_calls.checked_mul(levels.checked_add(1)?)?;
            let sort_compares = sift_steps.checked_mul(2)?;
            let compared_bytes = map_bytes.checked_mul(2)?.checked_add(1)?;
            let sort_comparison = sort_compares.checked_mul(compared_bytes)?;
            let sort_compute = sort_compares
                .checked_mul(compared_bytes.checked_mul(8)?.checked_add(32)?)?
                .checked_add(sift_steps.checked_mul(32)?)?
                .checked_add(m.saturating_sub(1).checked_mul(32)?)?;
            let finalization = map_bytes
                .checked_mul(3)?
                .checked_add(m.checked_mul(8)?)?
                .checked_add(8)?;
            // scalar_bytes = 4*(schema + canonical owner + claim) +4096,
            // exactly the fixed owner/workspace prefix in publication_plan.
            // map_bytes includes frozen logical_bytes, hence key payload and
            // published bytes are each <= this O(1) envelope.
            let drafts = m.checked_mul(
                (size_of::<mech_core::snapshot::MapEntryDraft>()
                    + 2 * size_of::<ValueDataDraft>()
                    + size_of::<String>()) as u64,
            )?;
            let order = m.checked_mul(size_of::<(&str, u64)>() as u64)?;
            let workspace = metadata
                .scalar_bytes
                .checked_add(order)?
                .checked_add(drafts.checked_mul(4)?)?
                .checked_add(map_bytes.checked_mul(8)?)?;
            let materialize_compute = workspace
                .checked_mul(16)?
                .checked_add(metadata.observation_schema_bytes)?
                .checked_add(128 + 256)?
                .checked_add(sort_compute)?;
            let materialize_comparison = sort_comparison.checked_add(finalization)?;
            let replay = n
                .checked_add(1)?
                .checked_mul(traversal)?
                .checked_add(materialize_compute)?;
            let metadata_bytes = capacity
                .checked_mul(size_of::<Slot>() as u64)?
                .checked_add(n.checked_mul(ADMITTED_CELL_BYTES)?)?
                .checked_add((size_of::<Storage>() + size_of::<Frame>()) as u64)?;
            // Probe bound is the existing setup formula using only O(1) upper
            // envelopes, plus descriptor/owner/caller inventory queries.
            let setup = map_bytes
                .checked_add(source_bytes)?
                .checked_mul(64)?
                .checked_add(
                    metadata_bytes
                        .checked_add(n.checked_mul(8)?)?
                        .checked_mul(16)?,
                )?
                .checked_add(m.checked_mul(capacity)?.checked_mul(64)?)?
                .checked_add(metadata.observation_schema_bytes.checked_mul(16)?)?
                .checked_add(2048)?;
            let probe = facts.probe(setup)?;
            let comparisons = n
                .checked_add(1)?
                .checked_mul(compare)?
                .checked_add(materialize_comparison)?;
            Some((replay.checked_add(probe)?, comparisons))
        })();
        Ok(credit)
    }
    pub(in crate::native_artifact::program::runtime) fn fresh_utf8_header(
        &self,
        collection: u32,
        iteration: u32,
        progress: &[u64],
    ) -> bool {
        !self.has_utf8_read()
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
        matches!(self.native_read.as_deref(), Some(Storage::Loan(_)))
    }
    pub(in crate::native_artifact::program::runtime::collections) fn class_nodes(&self) -> u64 {
        match self.native_read.as_deref() {
            Some(Storage::Classes(_)) => 3,
            Some(Storage::Loan(loan)) if loan.classes.is_some() => 3,
            _ => 0,
        }
    }
    pub(in crate::native_artifact::program::runtime) fn publish_classes(
        &mut self,
        cache: ClassifiedSource,
    ) -> Result<(), Error> {
        if self.has_utf8_read() {
            return Err(invalid("histogram overlaps active String loan"));
        }
        if let Some(old) = self.native_read.take() {
            let Storage::Classes(old) = *old else {
                unreachable!()
            };
            self.live_bytes = self
                .live_bytes
                .checked_sub(old.reservation.capacity_bytes())
                .ok_or_else(|| invalid("String classification retirement underflow"))?;
        }
        let bytes = cache.reservation.capacity_bytes();
        if let Some(storage) = boxed(Storage::Classes(cache)) {
            self.live_bytes = add(self.live_bytes, bytes)?;
            self.native_read = Some(storage);
        }
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_utf8_read(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &Region,
        prior: ResourceDemand,
        classified: bool,
        dictionary: &mut Option<Dictionary>,
        replay: Option<(u64, u64)>,
    ) -> Result<Option<u64>, Error> {
        #[cfg(test)]
        if FORCE_REFUSAL.with(|v| v.get()) {
            return Ok(None);
        }
        if self.has_utf8_read() {
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
            || (dictionary.is_none() && !ctx.arena_entry_is_current(region.map_root)?)
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
        let map_root = if dictionary.is_none() {
            Some(ctx.arena_copy_current(region.map_root)?)
        } else {
            None
        };
        let map_handle = if let Some(owner) = dictionary.as_ref() {
            if owner.value != region.map_root || owner.header != region.header as usize {
                return Err(invalid("native dictionary differs from lookup source"));
            }
            owner
                .map
                .check_account(ctx.library())
                .map_err(super::super::maps::owner::storage_error)?;
            ctx.arena_check_virtual(owner.handle)?;
            owner.handle
        } else {
            ctx.arena_current_handle(region.map_root)?
        };
        let source_type = &metadata.types[region.source as usize];
        let map_type = &metadata.types[region.map_root as usize];
        if source.schema() != source_type.schema
            || source.schema_key() != source_type.key
            || map_root
                .as_ref()
                .is_some_and(|v| v.schema() != map_type.schema || v.schema_key() != map_type.key)
            || dictionary
                .as_ref()
                .is_some_and(|v| v.map.schema() != map_type.schema)
        {
            return Ok(None);
        }
        if source.validate_against(&metadata.schemas).is_err()
            || map_root
                .as_ref()
                .is_some_and(|v| v.validate_against(&metadata.schemas).is_err())
        {
            return Ok(None);
        }
        let table = if let Some(owner) = dictionary.as_ref() {
            Table::Frozen(&owner.map)
        } else {
            let Some(table) = map(
                map_root
                    .as_ref()
                    .ok_or_else(|| invalid("lookup lost canonical owner"))?,
                region.map_ordinal,
            ) else {
                return Ok(None);
            };
            Table::Canonical(table)
        };
        let memory = ctx.library().memory_budget();
        let Some(source_bytes) = source.memory_budget_retained_bytes(&memory) else {
            return Ok(None);
        };
        let map_bytes = if let Some(owner) = dictionary.as_ref() {
            owner
                .map
                .retained_bytes()
                .map_err(super::super::maps::owner::storage_error)?
        } else {
            let Some(n) = map_root
                .as_ref()
                .and_then(|v| v.memory_budget_retained_bytes(&memory))
            else {
                return Ok(None);
            };
            n
        };
        let map_logical_bytes = dictionary
            .as_ref()
            .map_or(map_bytes, |v| v.map.logical_bytes().max(map_bytes));
        let Some(source_nodes) = source.memory_budget_retained_nodes(&memory) else {
            return Ok(None);
        };
        let map_nodes = if let Some(owner) = dictionary.as_ref() {
            owner.map.retained_nodes()
        } else {
            let Some(n) = map_root
                .as_ref()
                .and_then(|v| v.memory_budget_retained_nodes(&memory))
            else {
                return Ok(None);
            };
            n
        };
        let count = strings.len();
        let classes = match self.native_read.as_deref() {
            Some(Storage::Classes(cache))
                if classified
                    && region.classification_site == Some(cache.producer as u32)
                    && cache.handle == source_handle
                    && cache.source.shares_frozen_storage(&source)
                    && cache.representatives.len() == count =>
            {
                Some(cache)
            }
            _ => None,
        };
        let query_count = classes.map_or(count, |cache| cache.first_rows.len());
        let use_classes = classes.is_some();
        let Some(capacity) = table
            .len()
            .checked_mul(2)
            .and_then(|n| n.max(4).checked_next_power_of_two())
        else {
            return Ok(None);
        };
        let output_width = u64::from(region.output_width);
        let Some(output_bytes) = (count as u64).checked_mul(output_width) else {
            return Ok(None);
        };
        let metadata_bytes = (capacity as u64)
            .checked_mul(size_of::<Slot>() as u64)
            .and_then(|n| n.checked_add((count as u64).checked_mul(ADMITTED_CELL_BYTES)?))
            .and_then(|n| n.checked_add((size_of::<Storage>() + size_of::<Frame>()) as u64));
        let Some(metadata_bytes) = metadata_bytes else {
            return Ok(None);
        };
        let setup = map_bytes
            .checked_add(source_bytes)
            .and_then(|n| n.checked_mul(64))
            .and_then(|n| n.checked_add(metadata_bytes.checked_add(output_bytes)?.checked_mul(16)?))
            .and_then(|n| {
                n.checked_add(
                    (table.len() as u64)
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
        let Some(stored_bytes) = table.key_bytes() else {
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
        let Some(preflight_compute) = preflight_compare.checked_mul(64).and_then(|n| {
            n.checked_add(
                (count as u64).checked_mul(if region.conversion_site.is_some() {
                    2048
                } else {
                    1024
                })?,
            )
        }) else {
            return Ok(None);
        };
        // Retain the source producer and packed push's original logical floors
        // while removing actual canonical String/scalar construction work.
        let Some(commit_compute) = key_bytes.checked_mul(16).and_then(|n| {
            n.checked_add((count as u64).checked_mul(416u64.checked_add(
                if region.conversion_site.is_some() {
                    384
                } else {
                    0
                },
            )?)?)
        }) else {
            return Ok(None);
        };
        let Some(commit_cloned) = key_bytes.checked_add(output_bytes) else {
            return Ok(None);
        };
        let builder = self.builder(id as u32)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.values.packed_width() != Some(output_width)
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
        let Some(max_call_clone) = map_logical_bytes.checked_add(maximum_key) else {
            return Ok(None);
        };
        let limits = ctx.limits();
        if !fits(count as u64, limits.max_output_elements)
            || !fits(compute, limits.max_compute_work)
            || !fits(max_map_compute, limits.max_compute_work)
            || !fits(cloned.max(max_call_clone), limits.max_cloned_bytes)
            || !fits(output_bound.max(map_logical_bytes), limits.max_output_bytes)
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
        let Some(admission) = total.checked_add(replay.map_or(0, |r| r.0)) else {
            return Ok(None);
        };
        let Some(comparisons) = preflight_compare.checked_add(replay.map_or(0, |r| r.1)) else {
            return Ok(None);
        };
        if ctx
            .library()
            .unscoped_remaining()
            .is_none_or(|(c, p)| c < admission || p < comparisons)
            || admission > {
                let work = ctx.work();
                work.limit().saturating_sub(work.used())
            }
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
        let Some(mut cells) = fixed(query_count, Cell::default()) else {
            return Ok(None);
        };
        let Some(mut output) = private_output(region.output_width, count) else {
            return Ok(None);
        };
        if !table.each(|key, value| {
            let code = hash(key.as_bytes());
            let mut at = code as usize & (capacity - 1);
            while slots[at].occupied != 0 {
                at = (at + 1) & (capacity - 1)
            }
            slots[at] = Slot {
                address: key.as_ptr() as u64,
                bytes: key.len() as u64,
                value,
                hash: code,
                occupied: 1,
            };
        })? {
            return Ok(None);
        }
        let classes = match self.native_read.as_deref() {
            Some(Storage::Classes(cache)) if use_classes => Some(cache),
            _ => None,
        };
        for (ordinal, cell) in cells.iter_mut().enumerate() {
            let row = classes.map_or(ordinal as u64, |cache| cache.first_rows[ordinal]);
            let Some(key) = strings.get(row as usize) else {
                return Err(invalid("String class representative outside source"));
            };
            *cell = Cell {
                address: key.as_ptr() as u64,
                bytes: key.len() as u64,
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
            count: query_count as u64,
            // CollectionElement visits physical source order, unlike the
            // one-based column-major MatrixScalarAccess producer.
            rows: 1,
            columns: query_count as u64,
            output: output.unsigned_pointer()?,
            stored_bytes,
            fuel: preflight_compute,
            comparison: preflight_compare,
            epoch,
            issued: epoch,
            next: 0,
            output_rows: classes.map_or(0, |cache| cache.first_rows.as_ptr() as u64),
            representatives: classes.map_or(0, |cache| cache.representatives.as_ptr() as u64),
            output_count: if classes.is_some() { count as u64 } else { 0 },
        }) else {
            return Ok(None);
        };
        let pointer = (&*frame as *const Frame) as u64;
        let used_classes = classes.is_some();
        let classes = match self.native_read.take() {
            Some(cache) => {
                let Storage::Classes(cache) = *cache else {
                    unreachable!()
                };
                if used_classes {
                    Some(cache)
                } else {
                    self.live_bytes = self
                        .live_bytes
                        .checked_sub(cache.reservation.capacity_bytes())
                        .ok_or_else(|| invalid("String classification retirement underflow"))?;
                    None
                }
            }
            None => None,
        };
        // The class claim moves into this loan; all pointers remain pinned.
        let class_bytes = classes
            .as_ref()
            .map_or(0, |cache| cache.reservation.capacity_bytes());
        let Some(mut loan) = boxed(Storage::Loan(Loan {
            header: region.header as usize,
            collection: id,
            iteration: region.iteration,
            source,
            map_root: None,
            source_handle,
            map_handle,
            slots,
            cells,
            output,
            classes,
            frame,
            commit_compute,
            commit_cloned,
            output_reservation,
            _metadata: metadata_reservation,
            _nodes: nodes,
        })) else {
            self.live_bytes = self
                .live_bytes
                .checked_sub(class_bytes)
                .ok_or_else(|| invalid("String classification allocation retirement underflow"))?;
            return Ok(None);
        };
        let Storage::Loan(inner) = loan.as_mut() else {
            unreachable!()
        };
        inner.map_root = Some(match dictionary.take() {
            Some(owner) => MapBacking::Frozen(owner),
            None => {
                MapBacking::Canonical(map_root.ok_or_else(|| invalid("lookup lost admitted map"))?)
            }
        });
        self.native_read = Some(loan);
        Ok(Some(pointer))
    }
    pub(in crate::native_artifact::program::runtime) fn finish_utf8_read(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        pointer: u64,
        status: u64,
        returned: &mut Option<Dictionary>,
    ) -> Result<bool, Error> {
        let loan = self
            .native_read
            .take()
            .ok_or_else(|| invalid("native String lookup completion has no loan"))?;
        let Storage::Loan(mut loan) = *loan else {
            return Err(invalid("String lookup finished an unused classification"));
        };
        // Retire the transferred cache from collection backing on every exit,
        // including a native miss. The loan retains its real claims until drop.
        if let Some(cache) = &loan.classes {
            self.live_bytes = self
                .live_bytes
                .checked_sub(cache.reservation.capacity_bytes())
                .ok_or_else(|| invalid("String classification retirement underflow"))?;
        }
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
        {
            return Err(invalid("native String lookup source generation changed"));
        }
        match loan
            .map_root
            .as_ref()
            .ok_or_else(|| invalid("lookup loan lacks dictionary owner"))?
        {
            MapBacking::Canonical(value) => {
                if !ctx.arena_get(loan.map_handle)?.shares_frozen_storage(value) {
                    return Err(invalid("native lookup map generation changed"));
                }
            }
            MapBacking::Frozen(owner) => {
                if owner.handle != loan.map_handle {
                    return Err(invalid("dictionary handle changed"));
                }
                ctx.arena_check_virtual(loan.map_handle)?;
            }
        }
        if status != 0 {
            let backing = loan.map_root.take();
            // All generated pointers, temporary output and its claims retire
            // before the caller can materialize a returned dictionary owner.
            drop(loan);
            if let Some(MapBacking::Frozen(owner)) = backing {
                *returned = Some(owner)
            }
            return Ok(false);
        }
        if loan.frame.next != loan.frame.count
            || loan.frame.count != loan.cells.len() as u64
            || match &loan.classes {
                Some(cache) => {
                    loan.frame.output_count != loan.output.len() as u64
                        || loan.frame.output_rows != cache.first_rows.as_ptr() as u64
                        || loan.frame.representatives != cache.representatives.as_ptr() as u64
                }
                None => {
                    loan.frame.count != loan.output.len() as u64
                        || loan.frame.output_rows != 0
                        || loan.frame.representatives != 0
                        || loan.frame.output_count != 0
                }
            }
            || loan.frame.slots != loan.slots.as_ptr() as u64
            || loan.frame.cells != loan.cells.as_ptr() as u64
            || loan.frame.output != loan.output.unsigned_pointer()?
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
        } = loan;
        let bytes = output_reservation.capacity_bytes();
        let old = self.live_bytes;
        let builder = self.builder(collection as u32)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
        {
            return Err(invalid("native String lookup builder changed during loan"));
        }
        builder.values = output;
        builder.reservation = output_reservation;
        builder.payload_bytes = bytes;
        self.live_bytes = add(old, bytes)?;
        Ok(true)
    }
}

#[cfg(test)]
thread_local! {static FORCE_REFUSAL:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
#[cfg(test)]
pub(in crate::native_artifact::program::runtime) fn force_dictionary_refusal(value: bool) {
    FORCE_REFUSAL.with(|v| v.set(value));
}
#[cfg(test)]
impl State {
    pub(in crate::native_artifact::program::runtime) fn has_frozen_dictionary_loan(&self) -> bool {
        matches!(self.native_read.as_deref(),Some(Storage::Loan(loan)) if matches!(loan.map_root.as_ref(),Some(MapBacking::Frozen(_))))
    }
}
