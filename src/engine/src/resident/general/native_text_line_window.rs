//! Bounded synchronous row/offset loans from the owning packed line store.
//! The source borrow statically prevents mutation/discard while addresses live.
//! Global ordinals never bias a dereferenced pointer outside its allocation.
use super::{
    Account, Claim, InvokeError, NativeLibraryWorkBudget, NativeTextLineCaptureFacts,
    NativeTextLineChunks,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "native_text_line_window_tests.rs"]
mod native_text_line_window_tests;

pub struct NativeTextLineWindow<'owner> {
    rows: Vec<[u64; 2]>,
    // At most two physical chunks cover an ordinary capture delivery. Larger
    // or mixed-Unicode windows keep the checked materialized representation.
    // Words: bytes pointer, bytes length, LF-offset pointer (zero for a lone
    // completed line), first offset index, number of intersecting rows.
    segments: [[u64; 5]; 2],
    segment_count: usize,
    direct: bool,
    owner: &'owner NativeTextLineChunks,
    first: u64,
    end: u64,
    ascii: bool,
    _claim: Claim,
}
impl Drop for NativeTextLineWindow<'_> {
    fn drop(&mut self) {
        // Keep drop checking of the source borrow through row-table and claim
        // retirement, not merely through the last explicit getter call.
    }
}
impl NativeTextLineChunks {
    pub fn borrow_window(
        &self,
        first: u64,
        end: u64,
    ) -> Result<NativeTextLineWindow<'_>, InvokeError> {
        if self.failed
            || first == 0
            || first > end
            || end > self.facts.lines + 1
            || (first < end && (first < self.lower || self.upper.is_some_and(|limit| end > limit)))
        {
            return Err(InvokeError::InvalidInput);
        }
        let count = usize::try_from(end - first).map_err(|_| InvokeError::InvalidInput)?;
        // Two directory searches, then linear visits of intersecting chunks.
        // Fund both geometry passes and retirement without per-line debits.
        self.account.debit(
            (count as u64)
                .checked_mul(256)
                .and_then(|work| work.checked_add(4096))
                .ok_or(InvokeError::InvalidInput)?,
        )?;
        let payload = self.window_payload_bytes(first, end)?;
        self.account
            .debit(payload.checked_mul(4).ok_or(InvokeError::InvalidInput)?)?;
        let bytes = (count as u64)
            .checked_mul(core::mem::size_of::<[u64; 2]>() as u64)
            .and_then(|bytes| {
                bytes.checked_add(core::mem::size_of::<NativeTextLineWindow<'_>>() as u64)
            })
            .and_then(|bytes| bytes.checked_add(128))
            .ok_or(InvokeError::InvalidInput)?;
        let claim = Account::reserve(&self.account, bytes)?;
        let start = self.chunks.partition_point(|chunk| chunk.end() <= first);
        let finish = self.chunks.partition_point(|chunk| chunk.first < end);
        let direct = self.facts.ascii && finish - start <= 2;
        let mut segments = [[0; 5]; 2];
        let mut segment_count = 0;
        let mut rows = Vec::new();
        let mut ascii = true;
        if direct {
            let mut seen = 0;
            for chunk in self.chunks.range(start..finish) {
                let lo = first.max(chunk.first);
                let hi = end.min(chunk.end());
                if lo >= hi { continue; }
                segments[segment_count] = [
                    chunk.bytes.as_str().as_ptr() as u64,
                    chunk.bytes.as_str().len() as u64,
                    chunk.offsets.as_ref().map_or(0, |offsets| offsets.values.as_ptr() as u64),
                    lo - chunk.first,
                    hi - lo,
                ];
                segment_count += 1;
                seen += hi - lo;
            }
            if seen != count as u64 { return Err(InvokeError::InvalidInput); }
        } else {
            rows.try_reserve_exact(count).map_err(|_| InvokeError::InvalidInput)?;
            if rows.capacity() != count { return Err(InvokeError::InvalidInput); }
            // Retain the exact per-window qualification when a Unicode prefix
            // exists, even if that prefix was already discarded by the source.
            self.visit_window_cells(first, end, |text| {
                if !self.facts.ascii { ascii &= text.is_ascii(); }
                rows.push([text.as_ptr() as u64, text.len() as u64]);
                Ok(())
            })?;
        }
        Ok(NativeTextLineWindow {
            rows,
            segments,
            segment_count,
            direct,
            owner: self,
            first,
            end,
            ascii,
            _claim: claim,
        })
    }

    /// Reuse immutable full-chunk totals without a row loan or a second work
    /// allowance. Partial edge chunks retain the exact CRLF-aware cell walk.
    /// Only the owning final line Matrix may use this private success proof.
    pub(in super::super) fn final_range_payload(&self, first: u64, end: u64) -> Option<u64> {
        let facts = self.final_facts()?;
        if first == 0
            || first >= end
            || end > facts.lines.checked_add(1)?
            || first < self.lower
            || self.upper.is_some_and(|limit| end > limit)
        {
            return None;
        }
        self.window_payload_bytes(first, end).ok()
    }

    fn window_payload_bytes(&self, first: u64, end: u64) -> Result<u64, InvokeError> {
        let start = self.chunks.partition_point(|chunk| chunk.end() <= first);
        let finish = self.chunks.partition_point(|chunk| chunk.first < end);
        let mut seen = 0u64;
        let mut payload = 0u64;
        for chunk in self.chunks.range(start..finish) {
            let first = first.max(chunk.first);
            let end = end.min(chunk.end());
            payload = payload
                .checked_add(
                    chunk
                        .payload_bytes(first, end)
                        .ok_or(InvokeError::InvalidInput)?,
                )
                .ok_or(InvokeError::InvalidInput)?;
            seen += end - first;
        }
        if seen != end - first {
            return Err(InvokeError::InvalidInput);
        }
        Ok(payload)
    }

    fn visit_window_cells(
        &self,
        first: u64,
        end: u64,
        mut visit: impl FnMut(&str) -> Result<(), InvokeError>,
    ) -> Result<(), InvokeError> {
        let start = self.chunks.partition_point(|chunk| chunk.end() <= first);
        let finish = self.chunks.partition_point(|chunk| chunk.first < end);
        let mut seen = 0u64;
        for chunk in self.chunks.range(start..finish) {
            for ordinal in first.max(chunk.first)..end.min(chunk.end()) {
                visit(chunk.line(ordinal).ok_or(InvokeError::InvalidInput)?)?;
                seen += 1;
            }
        }
        if seen != end - first {
            return Err(InvokeError::InvalidInput);
        }
        Ok(())
    }
}
impl NativeTextLineWindow<'_> {
    pub(crate) fn shares_work(&self, work: &NativeLibraryWorkBudget) -> bool {
        self.owner.account.shares_work(work)
    }
    pub const fn first(&self) -> u64 {
        self.first
    }
    pub const fn end(&self) -> u64 {
        self.end
    }
    pub fn row_count(&self) -> usize {
        (self.end - self.first) as usize
    }
    pub fn rows(&self) -> Option<&[[u64; 2]]> {
        (!self.direct).then_some(self.rows.as_slice())
    }
    /// ABI29 machine-view words. Bit zero distinguishes two inline compact
    /// chunk descriptors from a u64-aligned materialized [pointer,length] table.
    /// This grants no execution authority: the host must first bind the SAME
    /// producer/work identity and pin this exact borrowed window through entry.
    pub fn native_rows(&self) -> (u64, u64) {
        let pointer = if self.direct {
            self.segments.as_ptr() as u64 | 1
        } else {
            self.rows.as_ptr() as u64
        };
        (pointer, self.row_count() as u64)
    }
    pub const fn is_ascii(&self) -> bool {
        self.ascii
    }
    pub fn available_lines(&self) -> u64 {
        self.owner.facts.lines
    }
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.owner.account.used()
    }
    pub fn producer_compute_work(&self) -> u64 {
        self.owner.account.spent()
    }
    pub fn final_facts(&self) -> Option<NativeTextLineCaptureFacts> {
        self.owner.final_facts()
    }
    pub fn line(&self, ordinal: u64) -> Option<&str> {
        (ordinal >= self.first && ordinal < self.end)
            .then(|| self.owner.line(ordinal))
            .flatten()
    }
    pub(crate) fn bind(
        &self,
        producer: &Arc<()>,
        work: &NativeLibraryWorkBudget,
    ) -> Result<u64, InvokeError> {
        if !self
            .owner
            .producer
            .as_ref()
            .is_some_and(|actual| Arc::ptr_eq(actual, producer))
            || !self.owner.account.shares_work(work)
        {
            return Err(InvokeError::InvalidInput);
        }
        self.owner.account.debit(256)?;
        // Includes the real store, this row loan and any other admitted loans.
        Ok(self.owner.account.used())
    }
}
