//! Copy already validated UTF8 line payloads into an exactly admitted join.
//! This changes no selection, source traversal, work charge or allocation.
use super::{Chunk, InvokeError};

impl Chunk {
    pub(super) fn append_payload(
        &self,
        first: u64,
        end: u64,
        target: &mut String,
    ) -> Result<(), InvokeError> {
        let offsets = self.offsets.as_ref().ok_or(InvokeError::InvalidInput)?;
        let lower = usize::try_from(
            first
                .checked_sub(self.first)
                .ok_or(InvokeError::InvalidInput)?,
        )
        .map_err(|_| InvokeError::InvalidInput)?;
        let upper = usize::try_from(
            end.checked_sub(self.first)
                .ok_or(InvokeError::InvalidInput)?,
        )
        .map_err(|_| InvokeError::InvalidInput)?;
        let selected = offsets
            .values
            .get(lower..upper)
            .ok_or(InvokeError::InvalidInput)?;
        let payload = usize::try_from(
            self.payload_bytes(first, end)
                .ok_or(InvokeError::InvalidInput)?,
        )
        .map_err(|_| InvokeError::InvalidInput)?;
        let old_len = target.len();
        let new_len = old_len
            .checked_add(payload)
            .ok_or(InvokeError::InvalidInput)?;
        if new_len > target.capacity() {
            return Err(InvokeError::InvalidInput);
        }
        let source = self.bytes.as_str().as_bytes();
        let mut start = if lower == 0 {
            0
        } else {
            offsets.values[lower - 1] as usize + 1
        };
        // SAFETY: the caller exclusively owns target, whose existing allocation
        // was admitted at the original TextJoin. Private, constructor-validated
        // LF offsets delimit UTF8 boundaries; stripping an immediately prior
        // ASCII CR preserves those boundaries. payload is their exact sum (a
        // cached constructor fact for a complete chunk, checked row geometry
        // for partial edges), and new_len <= capacity was checked above.
        // Every nonnegative prefix therefore fits this SAME allocation. Source
        // backing is a distinct immutable owner and cannot overlap target.
        // No mutable borrow, growth, helper callback or source retirement occurs
        // inside this loop. Publish the new length only after all bytes exist;
        // an early error leaves the original String length and UTF8 intact.
        let output = unsafe { target.as_mut_vec() };
        let destination = output.as_mut_ptr();
        let mut written = old_len;
        for &lf in selected {
            let lf = lf as usize;
            let stop = lf - usize::from(lf > start && source[lf - 1] == b'\r');
            let line = source.get(start..stop).ok_or(InvokeError::InvalidInput)?;
            unsafe {
                std::ptr::copy_nonoverlapping(line.as_ptr(), destination.add(written), line.len());
            }
            written += line.len();
            start = lf + 1;
        }
        if written != new_len {
            return Err(InvokeError::InvalidInput);
        }
        unsafe { output.set_len(new_len) };
        Ok(())
    }
}
