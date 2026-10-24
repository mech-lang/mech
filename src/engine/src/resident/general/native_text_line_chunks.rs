//! Owning incremental str::lines storage, not an I/O route or FSM evaluator.
//! Private chunks are NOT partial canonical Values. Only completed lines can
//! be borrowed; only EOF produces descriptive final geometry. The caller must
//! preserve source admissions/error order before executing any final consumer.
use super::{InvokeError, NativeLibraryWorkBudget};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, MemoryBudgetLimits};
use std::{collections::VecDeque, rc::Rc, sync::Arc};

#[path = "native_text_line_chunk_allocation.rs"]
mod allocation;
use allocation::{Account, Bytes, ChunkData, Claim, Offsets};
#[path = "native_text_line_window.rs"]
mod window;
pub use window::NativeTextLineWindow;
#[cfg(test)]
#[path = "native_text_line_chunk_tests.rs"]
mod native_text_line_chunk_tests;
#[cfg(all(test, unix, feature = "native-capture-pages"))]
#[path = "native_text_line_page_capture_tests.rs"]
mod page_capture_tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeTextLineCaptureFacts {
    bytes: u64,
    lines: u64,
    payload: u64,
    ascii: bool,
}
impl NativeTextLineCaptureFacts {
    pub const fn input_bytes(self) -> u64 {
        self.bytes
    }
    pub const fn lines(self) -> u64 {
        self.lines
    }
    pub const fn payload_bytes(self) -> u64 {
        self.payload
    }
    pub const fn is_ascii(self) -> bool {
        self.ascii
    }
}

struct Chunk {
    first: u64,
    bytes: ChunkData,
    offsets: Option<Offsets>,
}
// Rust does not guarantee Box/Vec/Option aggregate layouts. Preserve the
// existing directory-capacity and source-work debits on EACH compiled target;
// a layout change needs build-time review, never a new runtime capture panic.
const _: () = {
    #[allow(
        dead_code,
        reason = "legacy field layouts are inspected at compile time to preserve admitted directory capacity"
    )]
    struct UncachedOffsets {
        values: Vec<u32>,
        _claim: Claim,
    }
    #[allow(
        dead_code,
        reason = "legacy field layouts are inspected at compile time to preserve admitted directory capacity"
    )]
    struct UncachedChunk {
        first: u64,
        bytes: ChunkData,
        offsets: Option<UncachedOffsets>,
    }
    assert!(core::mem::size_of::<Offsets>() == core::mem::size_of::<UncachedOffsets>());
    assert!(core::mem::align_of::<Offsets>() == core::mem::align_of::<UncachedOffsets>());
    assert!(
        core::mem::size_of::<Option<Offsets>>() == core::mem::size_of::<Option<UncachedOffsets>>()
    );
    assert!(
        core::mem::align_of::<Option<Offsets>>()
            == core::mem::align_of::<Option<UncachedOffsets>>()
    );
    assert!(core::mem::size_of::<Chunk>() == core::mem::size_of::<UncachedChunk>());
    assert!(core::mem::align_of::<Chunk>() == core::mem::align_of::<UncachedChunk>());
};
impl Chunk {
    fn count(&self) -> u64 {
        self.offsets
            .as_ref()
            .map_or(1, |offsets| offsets.values.len() as u64)
    }
    fn end(&self) -> u64 {
        self.first + self.count()
    }
    fn line(&self, ordinal: u64) -> Option<&str> {
        let index = usize::try_from(ordinal.checked_sub(self.first)?).ok()?;
        if let Some(offsets) = &self.offsets {
            let end = *offsets.values.get(index)? as usize;
            let start = if index == 0 {
                0
            } else {
                offsets.values[index - 1] as usize + 1
            };
            let end =
                end - usize::from(end > start && self.bytes.as_str().as_bytes()[end - 1] == b'\r');
            self.bytes.as_str().get(start..end)
        } else {
            (index == 0).then_some(self.bytes.as_str())
        }
    }
    fn payload_bytes(&self, first: u64, end: u64) -> Option<u64> {
        if first == self.first && end == self.end() {
            return Some(
                self.offsets
                    .as_ref()
                    .map_or(self.bytes.as_str().len(), |offsets| offsets.payload_bytes)
                    as u64,
            );
        }
        // A partial edge chunk keeps the original exact cell inspection.
        // No delimiter-inclusive upper bound changes the work debit.
        (first..end).try_fold(0u64, |payload, ordinal| {
            payload.checked_add(self.line(ordinal)?.len() as u64)
        })
    }
}

#[derive(Default)]
struct Pending {
    bytes: u64,
    last_cr: bool,
    backing: Option<Bytes>,
}

/// Storage for one selected producer and one cumulative library account.
/// Retention operations accept coordinates, not source/state names. They can
/// only advance the lower bound or seal a completed range, never resurrect a
/// discarded prefix. Backing is released before its memory reservation.
pub struct NativeTextLineChunks {
    chunks: VecDeque<Chunk>,
    #[cfg(all(unix, feature = "native-capture-pages"))]
    pages: allocation::PagePool,
    pending: Pending,
    slots: Option<Claim>,
    account: Rc<Account>,
    producer: Option<Arc<()>>,
    limits: MemoryBudgetLimits,
    input_limit: u64,
    max_fragment: usize,
    facts: NativeTextLineCaptureFacts,
    lower: u64,
    upper: Option<u64>,
    eof: bool,
    failed: bool,
    _fixed: ManagedMemoryReservation,
}

pub(super) fn prepare(
    input_limit: u64,
    max_fragment: usize,
    limits: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
    lookup_work: u64,
    producer: Option<Arc<()>>,
) -> Result<NativeTextLineChunks, InvokeError> {
    if input_limit >= usize::MAX as u64 || max_fragment == 0 || max_fragment > u32::MAX as usize {
        return Err(InvokeError::InvalidInput);
    }
    let (account, fixed) = Account::new(
        limits,
        work,
        core::mem::size_of::<NativeTextLineChunks>() as u64,
        lookup_work,
    )?;
    Ok(NativeTextLineChunks {
        chunks: VecDeque::new(),
        #[cfg(all(unix, feature = "native-capture-pages"))]
        pages: allocation::PagePool::default(),
        pending: Pending::default(),
        slots: None,
        account,
        producer,
        limits,
        input_limit,
        max_fragment,
        facts: NativeTextLineCaptureFacts {
            bytes: 0,
            lines: 0,
            payload: 0,
            ascii: true,
        },
        lower: 1,
        upper: None,
        eof: false,
        failed: false,
        _fixed: fixed,
    })
}

/// Worst-case cumulative source work when nonempty fragments trigger at most
/// one row loan/bind/retention update each, and each completed cell appears in
/// one loan. Include an EOF loan and one sealed-range update. The host must
/// prove those scheduling conditions before using this setup eligibility bound.
pub(super) fn capture_work_bound(
    input_limit: u64,
    lookup_work: u64,
    binding_work: u64,
    matrix_metadata: u64,
) -> Option<u64> {
    if input_limit >= usize::MAX as u64 {
        return None;
    }
    let fixed = (core::mem::size_of::<NativeTextLineChunks>() as u64)
        .checked_add(core::mem::size_of::<Account>() as u64)?
        .checked_add(512)?
        .checked_mul(8)?
        .checked_add(128)?
        .checked_add(lookup_work)?;
    // Byte-buffer geometric capacities sum to <=4 input bytes. Delimiter
    // offsets use <=4 bytes/input byte. Directory capacities sum to <=4 slots
    // per completed chunk; every chunk contains at least one input byte.
    // Each of these three allocation families makes <=input_limit claims.
    let allocations = 4u64
        .checked_mul(8)?
        .checked_add(4 * 8)?
        .checked_add((core::mem::size_of::<Chunk>() as u64).checked_mul(4 * 8)?)?
        .checked_add(128 * 8)?
        .checked_add(3 * 128)?;
    let per_byte = allocations
        .checked_add(16)? // framing
        .checked_add(4)? // row-loan ASCII scan
        .checked_add(256)? // row geometry and retirement
        .checked_add(16 * 8)? // row-table allocation
        .checked_add(2 * 64)? // both retention boundaries
        .checked_add(128)?; // final producer admission
    let per_callback = (core::mem::size_of::<NativeTextLineWindow<'_>>() as u64)
        .checked_add(128)?
        .checked_mul(8)?
        .checked_add(128)? // row-loan claim
        .checked_add(512)? // framing callback
        .checked_add(4096)? // row directory searches
        .checked_add(256)? // selected producer binding
        .checked_add(4096)?; // retention search
    #[cfg(all(unix, feature = "native-capture-pages"))]
    let preferred_allocations = allocation::page_allocation_work_bound(input_limit)?;
    #[cfg(not(all(unix, feature = "native-capture-pages")))]
    let preferred_allocations = 0;
    fixed
        .checked_add(preferred_allocations)?
        .checked_add(input_limit.checked_mul(per_byte)?)?
        .checked_add(input_limit.checked_add(2)?.checked_mul(per_callback)?)?
        .checked_add(8192 + 512)? // sealed-range searches and EOF
        .checked_add(binding_work)?
        .checked_add(matrix_metadata.checked_mul(8)?)
}

impl NativeTextLineChunks {
    /// Consume the only owner's proven contiguous selection. The selected join
    /// has already prepaid its entire selector/copy/retirement traversal and
    /// allocated the exact output; no source-call allowance is renewed here.
    /// Drop each physical chunk after its final selected cell is appended.
    pub(super) fn append_joined_consuming(
        &mut self,
        first: u64,
        end: u64,
        separator: &str,
        target: &mut String,
    ) -> Result<(), InvokeError> {
        if self.final_facts().is_none()
            || first == 0
            || first > end
            || end > self.facts.lines + 1
            || (first < end && (first < self.lower || self.upper.is_some_and(|upper| end > upper)))
        {
            return Err(InvokeError::InvalidInput);
        }
        let mut next = first;
        let mut has_previous = false;
        while next < end {
            while self.chunks.front().is_some_and(|chunk| chunk.end() <= next) {
                self.chunks.pop_front();
            }
            let chunk = self.chunks.front().ok_or(InvokeError::InvalidInput)?;
            let stop = end.min(chunk.end());
            for ordinal in next..stop {
                let text = chunk.line(ordinal).ok_or(InvokeError::InvalidInput)?;
                crate::resident::text_ops::append_joined(target, separator, has_previous, text);
                has_previous = true;
            }
            next = stop;
            self.chunks.pop_front();
        }
        self.chunks.clear();
        #[cfg(all(unix, feature = "native-capture-pages"))]
        self.pages.clear();
        self.pending.backing = None;
        Ok(())
    }

    pub(super) fn bind_final(
        &self,
        producer: &Arc<()>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<NativeTextLineCaptureFacts, InvokeError> {
        if self.limits != limits
            || !self.account.shares_work(work)
            || !self
                .producer
                .as_ref()
                .is_some_and(|actual| Arc::ptr_eq(actual, producer))
        {
            return Err(InvokeError::InvalidInput);
        }
        self.final_facts().ok_or(InvokeError::InvalidInput)
    }
    pub(super) fn retained_bytes(&self, memory: &ManagedMemoryBudget) -> Result<u64, InvokeError> {
        if !self.account.shares_memory(memory) {
            return Err(InvokeError::UnadmittedValue);
        }
        Ok(self.account.used())
    }
    pub(super) fn debit(&self, work: u64) -> Result<(), InvokeError> {
        self.account.debit(work)
    }
    /// A physical completed-prefix extent, not a language length result.
    /// No final count or canonical shape is available before successful EOF.
    pub fn available_lines(&self) -> Option<u64> {
        (!self.failed).then_some(self.facts.lines)
    }
    pub fn final_facts(&self) -> Option<NativeTextLineCaptureFacts> {
        (self.eof && !self.failed).then_some(self.facts)
    }
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.account.used()
    }
    pub fn peak_capacity_bytes(&self) -> u64 {
        self.account.peak()
    }
    pub fn line(&self, ordinal: u64) -> Option<&str> {
        if self.failed
            || ordinal < self.lower
            || ordinal > self.facts.lines
            || self.upper.is_some_and(|end| ordinal >= end)
        {
            return None;
        }
        let index = self
            .chunks
            .partition_point(|chunk| chunk.first <= ordinal)
            .checked_sub(1)?;
        self.chunks.get(index)?.line(ordinal)
    }

    /// A traversal-local directory hint, never an owner or an access permit.
    /// Sequential cells stay in the current chunk or its immediate successor;
    /// arbitrary gathers retain the same checked directory-search fallback.
    pub(super) fn line_cached(&self, ordinal: u64, cursor: &mut usize) -> Option<&str> {
        if self.failed
            || ordinal < self.lower
            || ordinal > self.facts.lines
            || self.upper.is_some_and(|end| ordinal >= end)
        {
            return None;
        }
        if let Some(text) = self
            .chunks
            .get(*cursor)
            .and_then(|chunk| chunk.line(ordinal))
        {
            return Some(text);
        }
        if let Some(next) = cursor.checked_add(1)
            && let Some(text) = self.chunks.get(next).and_then(|chunk| chunk.line(ordinal))
        {
            *cursor = next;
            return Some(text);
        }
        let index = self
            .chunks
            .partition_point(|chunk| chunk.first <= ordinal)
            .checked_sub(1)?;
        *cursor = index;
        self.chunks.get(index)?.line(ordinal)
    }

    /// Copy one valid UTF-8 fragment into private admitted backing, framing
    /// many lines in one allocation with four-byte delimiter offsets. A line
    /// crossing fragments uses one amortized growing buffer, not one buffer
    /// per character or a repeated copy of its full prefix on every fragment.
    pub fn push(&mut self, text: &str) -> Result<(), InvokeError> {
        if self.failed || self.eof {
            return Err(InvokeError::InvalidInput);
        }
        self.failed = true;
        let result = self.push_inner(text);
        if result.is_ok() {
            self.failed = false;
        }
        result
    }
    fn push_inner(&mut self, mut text: &str) -> Result<(), InvokeError> {
        let total = self
            .facts
            .bytes
            .checked_add(text.len() as u64)
            .ok_or(InvokeError::InvalidInput)?;
        if text.len() > self.max_fragment || total > self.input_limit {
            return Err(InvokeError::InvalidInput);
        }
        self.account.debit(
            (text.len() as u64)
                .checked_mul(16)
                .and_then(|bytes| bytes.checked_add(512))
                .ok_or(InvokeError::InvalidInput)?,
        )?;
        self.facts.bytes = total;
        self.facts.ascii &= text.is_ascii();
        if self.pending.bytes != 0 {
            if let Some(end) = text.find('\n') {
                self.append_pending(&text[..end])?;
                self.complete_pending(true)?;
                text = &text[end + 1..];
            } else {
                return self.append_pending(text);
            }
        }
        if let Some(last) = text.rfind('\n') {
            let complete = &text[..last + 1];
            let count = crate::intrinsics::text_ops::line_count(complete);
            // A single-byte delimiter search avoids a general two-character
            // substring scan. CRLF cannot overlap itself; inspect the checked
            // following byte only at actual CR positions, including CRCRLF.
            let crlf = complete
                .match_indices('\r')
                .filter(|(index, _)| complete.as_bytes().get(*index + 1) == Some(&b'\n'))
                .count();
            let first = self
                .facts
                .lines
                .checked_add(1)
                .ok_or(InvokeError::InvalidInput)?;
            self.facts.lines = self
                .facts
                .lines
                .checked_add(count as u64)
                .ok_or(InvokeError::InvalidInput)?;
            let payload_bytes = complete.len() - count - crlf;
            self.facts.payload = self
                .facts
                .payload
                .checked_add(payload_bytes as u64)
                .ok_or(InvokeError::InvalidInput)?;
            // Fund every mandatory backing in this fragment before preferring
            // a slab. Otherwise its unused capacity can crowd out offsets,
            // directory growth or the unfinished tail even though heap storage
            // would fit. Pending bytes stay private until their line completes.
            if self.upper.is_none() {
                // Release the old directory before acquiring any new fragment
                // backing, keeping its replacement peak independent of the tail.
                self.reserve_chunk_slot()?;
                let offsets = Offsets::new(complete, count, payload_bytes, &self.account)?;
                self.append_pending(&text[last + 1..])?;
                #[cfg(all(unix, feature = "native-capture-pages"))]
                let bytes = match self.pages.copy(complete, self.input_limit, &self.account)? {
                    Some(text) => ChunkData::Pages(text),
                    None => Bytes::copy(complete, &self.account)?.into(),
                };
                #[cfg(not(all(unix, feature = "native-capture-pages")))]
                let bytes = Bytes::copy(complete, &self.account)?;
                // The slot and all other fragment backing are already funded.
                self.chunks.push_back(Chunk {
                    first,
                    bytes,
                    offsets: Some(offsets),
                });
            } else {
                self.append_pending(&text[last + 1..])?;
            }
            return Ok(());
        }
        self.append_pending(text)
    }
    fn append_pending(&mut self, text: &str) -> Result<(), InvokeError> {
        self.pending.bytes = self
            .pending
            .bytes
            .checked_add(text.len() as u64)
            .ok_or(InvokeError::InvalidInput)?;
        if let Some(byte) = text.as_bytes().last() {
            self.pending.last_cr = *byte == b'\r';
        }
        if self.upper.is_none() && !text.is_empty() {
            if let Some(bytes) = &mut self.pending.backing {
                bytes.append(text, self.input_limit as usize, &self.account)?;
            } else {
                self.pending.backing = Some(Bytes::copy(text, &self.account)?);
            }
        }
        Ok(())
    }
    fn complete_pending(&mut self, terminated: bool) -> Result<(), InvokeError> {
        let first = self
            .facts
            .lines
            .checked_add(1)
            .ok_or(InvokeError::InvalidInput)?;
        let trim = terminated && self.pending.last_cr;
        self.facts.lines = first;
        self.facts.payload = self
            .facts
            .payload
            .checked_add(self.pending.bytes - u64::from(trim))
            .ok_or(InvokeError::InvalidInput)?;
        if self.upper.is_none() {
            let mut bytes = self
                .pending
                .backing
                .take()
                .ok_or(InvokeError::InvalidInput)?;
            if trim {
                bytes.text.truncate(bytes.text.len() - 1);
            }
            self.push_chunk(Chunk {
                first,
                bytes: bytes.into(),
                offsets: None,
            })?;
        }
        self.pending = Pending::default();
        Ok(())
    }
    fn push_chunk(&mut self, chunk: Chunk) -> Result<(), InvokeError> {
        self.reserve_chunk_slot()?;
        self.chunks.push_back(chunk);
        Ok(())
    }
    fn reserve_chunk_slot(&mut self) -> Result<(), InvokeError> {
        if self.chunks.len() == self.chunks.capacity() {
            let capacity = self
                .chunks
                .capacity()
                .checked_mul(2)
                .ok_or(InvokeError::InvalidInput)?
                .max(4);
            let bytes = capacity
                .checked_mul(core::mem::size_of::<Chunk>())
                .and_then(|bytes| (bytes as u64).checked_add(128))
                .ok_or(InvokeError::InvalidInput)?;
            let claim = Account::reserve(&self.account, bytes)?;
            let mut replacement = VecDeque::new();
            replacement
                .try_reserve_exact(capacity)
                .map_err(|_| InvokeError::InvalidInput)?;
            if replacement.capacity() != capacity {
                return Err(InvokeError::InvalidInput);
            }
            while let Some(chunk) = self.chunks.pop_front() {
                replacement.push_back(chunk);
            }
            self.chunks = replacement;
            // The allocator has retired the old backing; its old reservation
            // releases only after the complete replacement was held at once.
            self.slots = Some(claim);
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<NativeTextLineCaptureFacts, InvokeError> {
        if self.failed || self.eof {
            return Err(InvokeError::InvalidInput);
        }
        self.failed = true;
        self.account.debit(512)?;
        if self.pending.bytes != 0 {
            self.complete_pending(false)?;
        }
        self.eof = true;
        self.failed = false;
        Ok(self.facts)
    }

    /// The original generated carrier supplies this frontier. The storage
    /// layer never chooses a state, looks for a header, or interprets Mech.
    pub fn retain_from(&mut self, first: u64) -> Result<(), InvokeError> {
        if self.failed || self.upper.is_some() || first < self.lower || first > self.facts.lines + 1
        {
            return Err(InvokeError::InvalidInput);
        }
        // Fund a bounded directory search before finding actual retirements.
        // Charging every live chunk on every fragment would grow quadratically
        // while a large retained range is still receiving input.
        self.account.debit(4096)?;
        let retired = self.chunks.partition_point(|chunk| chunk.end() <= first);
        self.account.debit(
            (retired as u64)
                .checked_mul(64)
                .ok_or(InvokeError::InvalidInput)?,
        )?;
        self.lower = first;
        for _ in 0..retired {
            self.chunks.pop_front();
        }
        Ok(())
    }
    pub fn retain_range(&mut self, first: u64, end: u64) -> Result<(), InvokeError> {
        if self.failed
            || self.upper.is_some()
            || first > end
            || end > self.facts.lines + 1
            || (first < end && first < self.lower)
        {
            return Err(InvokeError::InvalidInput);
        }
        self.account.debit(8192)?;
        // An empty returned range cannot resurrect a previously dead prefix.
        let lower = if first == end {
            self.facts.lines + 1
        } else {
            first
        };
        let upper = if first == end { lower } else { end };
        let front = self.chunks.partition_point(|chunk| chunk.end() <= lower);
        let back = self.chunks.len() - self.chunks.partition_point(|chunk| chunk.first < upper);
        self.account.debit(
            ((front + back) as u64)
                .checked_mul(64)
                .ok_or(InvokeError::InvalidInput)?,
        )?;
        self.lower = lower;
        self.upper = Some(upper);
        for _ in 0..front {
            self.chunks.pop_front();
        }
        for _ in 0..back {
            self.chunks.pop_back();
        }
        #[cfg(all(unix, feature = "native-capture-pages"))]
        if self.chunks.is_empty() {
            self.pages.clear();
        }
        self.pending.backing = None;
        Ok(())
    }
}
