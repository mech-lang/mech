//! Bounded synchronous row loans from the actual owning packed line store.
//! The source borrow statically prevents mutation/discard while pointers live.
//! No biased pointer, canonical partial Matrix, or language count is invented.
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
        let mut payload = 0u64;
        self.visit_window_cells(first, end, |text| {
            payload = payload
                .checked_add(text.len() as u64)
                .ok_or(InvokeError::InvalidInput)?;
            Ok(())
        })?;
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
        let mut rows = Vec::new();
        rows.try_reserve_exact(count)
            .map_err(|_| InvokeError::InvalidInput)?;
        if rows.capacity() != count {
            return Err(InvokeError::InvalidInput);
        }
        let mut ascii = true;
        // The immutable producer already inspected every delivered byte. An
        // all-ASCII source therefore proves all borrowed cells ASCII without
        // rescanning each short line. Genuine str::lines cells contain no LF,
        // so they cannot contain the sole multi-byte ASCII grapheme, CRLF.
        // For a mixed source, inspect only these cells to preserve the exact
        // window qualification (ASCII cells may follow discarded Unicode).
        // Keep the same payload precharge above on BOTH representation paths.
        if self.facts.ascii {
            self.visit_window_cells(first, end, |text| {
                rows.push([text.as_ptr() as u64, text.len() as u64]);
                Ok(())
            })?;
        } else {
            self.visit_window_cells(first, end, |text| {
                ascii &= text.is_ascii();
                rows.push([text.as_ptr() as u64, text.len() as u64]);
                Ok(())
            })?;
        }
        Ok(NativeTextLineWindow {
            rows,
            owner: self,
            first,
            end,
            ascii,
            _claim: claim,
        })
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
    pub fn rows(&self) -> &[[u64; 2]] {
        &self.rows
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
