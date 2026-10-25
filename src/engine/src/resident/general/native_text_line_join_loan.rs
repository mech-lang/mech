//! Unique physical source ownership for a prepaid generated join/map copy.
//! The chunk, its offset allocation and every original memory claim travel
//! together; no source pointer outlives this opaque owner.
use super::{Chunk, InvokeError, NativeTextLineChunks};

pub struct NativeTextLineJoinChunk {
    chunk: Chunk,
    words: [u64; 6],
    following: u64,
    payload: u64,
}

impl NativeTextLineJoinChunk {
    /// Source pointer/length, LF pointer, first LF index, selected row count,
    /// and complete LF allocation length. A zero LF pointer denotes one lone
    /// EOF row, whose trailing CR is not a CRLF terminator and stays payload.
    /// These addresses are valid only while this owning object remains live.
    pub fn geometry(&self) -> &[u64; 6] {
        &self.words
    }
    pub const fn following(&self) -> u64 {
        self.following
    }
    pub const fn payload_bytes(&self) -> u64 {
        self.payload
    }
}

impl NativeTextLineChunks {
    /// Move one actual chunk out of the sole-use final selection, without
    /// copying any payload. The caller has already admitted the original join
    /// and must hold the returned owner until its generated loan has ended.
    pub(in super::super) fn take_join_chunk(
        &mut self,
        first: u64,
        end: u64,
    ) -> Result<Option<NativeTextLineJoinChunk>, InvokeError> {
        if self.final_facts().is_none()
            || !self.facts.ascii
            || first == 0
            || first > end
            || end > self.facts.lines + 1
            || (first < end && (first < self.lower || self.upper.is_some_and(|upper| end > upper)))
        {
            return Err(InvokeError::InvalidInput);
        }
        while self
            .chunks
            .front()
            .is_some_and(|chunk| chunk.end() <= first)
        {
            self.chunks.pop_front();
        }
        if first == end {
            self.chunks.clear();
            #[cfg(all(unix, feature = "native-capture-pages"))]
            self.pages.clear();
            self.pending.backing = None;
            return Ok(None);
        }
        let chunk = self.chunks.front().ok_or(InvokeError::InvalidInput)?;
        if first < chunk.first || first >= chunk.end() {
            return Err(InvokeError::InvalidInput);
        }
        let stop = end.min(chunk.end());
        let payload = chunk
            .payload_bytes(first, stop)
            .ok_or(InvokeError::InvalidInput)?;
        let text = chunk.bytes.as_str();
        let words = [
            text.as_ptr() as u64,
            text.len() as u64,
            chunk
                .offsets
                .as_ref()
                .map_or(0, |o| o.values.as_ptr() as u64),
            first - chunk.first,
            stop - first,
            chunk.offsets.as_ref().map_or(0, |o| o.values.len() as u64),
        ];
        // Both data and offsets are out-of-line owning allocations. Moving
        // their headers changes neither address nor account identity. Moving
        // the chunk also keeps its original admission live until the loan ends.
        let chunk = self.chunks.pop_front().ok_or(InvokeError::InvalidInput)?;
        Ok(Some(NativeTextLineJoinChunk {
            chunk,
            words,
            following: stop,
            payload,
        }))
    }
}

impl Drop for NativeTextLineJoinChunk {
    fn drop(&mut self) {
        // The owning field remains live through all geometry access, including
        // the caller's final native completion checks, rather than just until
        // its last explicit Rust read.
        let _ = &self.chunk;
    }
}

#[cfg(test)]
#[path = "native_text_line_join_loan_tests.rs"]
mod native_text_line_join_loan_tests;
