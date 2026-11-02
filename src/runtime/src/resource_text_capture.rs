//! Shared bounded, single-acquisition UTF-8 delivery for native capture.
//!
//! This is an I/O adapter, NOT a resource/permission route or a Mech evaluator.
//! The caller must retain the ordinary provider/runtime authorization and the
//! one-shot source lifetime. Existing cached/mixed CLI reads are unchanged.
//! No partial canonical String/Matrix/Value is constructed here. A receiver
//! borrows valid text only for its synchronous call and must separately admit
//! any backing it retains and any compiled computation it performs.
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, MemoryRuntimeError, Utf8Fragment};
use std::io::{self, Read};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

const UTF8_CARRY: usize = 3;

#[derive(Debug)]
pub enum RuntimeTextCaptureSetupError {
    InvalidLimit,
    InvalidChunkSize,
    Memory(MemoryRuntimeError),
    Allocation(std::collections::TryReserveError),
    UnexpectedCapacity,
}
impl std::fmt::Display for RuntimeTextCaptureSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "text capture setup: {self:?}")
    }
}
impl std::error::Error for RuntimeTextCaptureSetupError {}

#[derive(Debug)]
pub enum RuntimeTextCaptureError<E> {
    Acquisition(io::Error),
    InputLimit {
        limit: u64,
    },
    InvalidUtf8 {
        valid_up_to: u64,
        error_len: Option<usize>,
    },
    Consumer(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for RuntimeTextCaptureError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "text capture: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for RuntimeTextCaptureError<E> {}

/// Descriptive EOF result, not a resource grant, canonical Value, or permission
/// to publish the receiver's speculative computation. Capture must succeed
/// before the ordinary native turn/effect transaction can complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeTextCaptureSummary {
    bytes: u64,
}
impl RuntimeTextCaptureSummary {
    pub const fn input_bytes(self) -> u64 {
        self.bytes
    }
}

/// One fixed-size, pre-admitted scratch owner. Consuming this object prevents
/// replay of this acquisition; it does not change the provider's cache policy.
/// Fields drop in order: scratch backing retires before its capacity charge.
#[derive(Debug)]
pub struct RuntimeTextCaptureBuffer {
    buffer: Vec<u8>,
    reservation: ManagedMemoryReservation,
    chunk_bytes: usize,
    limit: u64,
}
impl RuntimeTextCaptureBuffer {
    /// Prepare before observing the source. Capacity is independent of total
    /// input size; the extra three bytes hold one split UTF-8 code point.
    pub fn prepare(
        limit: u64,
        chunk_bytes: usize,
        memory: &ManagedMemoryBudget,
    ) -> Result<Self, RuntimeTextCaptureSetupError> {
        let acquisition = limit
            .checked_add(1)
            .ok_or(RuntimeTextCaptureSetupError::InvalidLimit)?;
        let chunk = u64::try_from(chunk_bytes)
            .ok()
            .filter(|n| *n > 0)
            .ok_or(RuntimeTextCaptureSetupError::InvalidChunkSize)?;
        let chunk_bytes = usize::try_from(chunk.min(acquisition))
            .map_err(|_| RuntimeTextCaptureSetupError::InvalidChunkSize)?;
        let capacity = chunk_bytes
            .checked_add(UTF8_CARRY)
            .ok_or(RuntimeTextCaptureSetupError::InvalidChunkSize)?;
        let bytes =
            u64::try_from(capacity).map_err(|_| RuntimeTextCaptureSetupError::InvalidChunkSize)?;
        let reservation = memory
            .reserve_capacity(bytes)
            .map_err(RuntimeTextCaptureSetupError::Memory)?;
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(capacity)
            .map_err(RuntimeTextCaptureSetupError::Allocation)?;
        // Exact reservation is a minimum. Do not silently retain unfunded
        // allocator-selected Vec capacity, even on the acquisition boundary.
        if buffer.capacity() != capacity {
            return Err(RuntimeTextCaptureSetupError::UnexpectedCapacity);
        }
        buffer.resize(capacity, 0);
        Ok(Self {
            buffer,
            reservation,
            chunk_bytes,
            limit,
        })
    }

    pub fn retained_capacity_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }

    pub const fn input_limit(&self) -> u64 {
        self.limit
    }

    /// Deliver only valid UTF-8 fragments, including characters split across
    /// arbitrary short reads. No content/line/header/language logic is used.
    ///
    /// Acquisition/cap errors precede UTF-8 errors, which precede consumer
    /// failures. After a consumer error/panic or bad UTF-8, keep draining the
    /// SAME reader through EOF (or the first capture failure), without invoking
    /// the consumer again. This matches capture-before-computation ordering.
    /// A denied cap reads at most limit+1 bytes, never later I/O. A deferred
    /// consumer panic resumes its original payload only after successful EOF.
    pub fn capture<R: Read, E>(
        self,
        reader: R,
        consumer: impl FnMut(&str) -> Result<(), E>,
    ) -> Result<RuntimeTextCaptureSummary, RuntimeTextCaptureError<E>> {
        match self.capture_deferred(reader, consumer) {
            Ok(summary) => Ok(summary),
            Err(DeferredCaptureFailure::Error(error)) => Err(error),
            Err(DeferredCaptureFailure::Panic(payload)) => resume_unwind(payload),
        }
    }

    // Resource delivery releases the provider/reader loan before propagating a
    // consumer panic. A speculative host panic must not poison the provider's
    // lock, unlike a panic in that provider's OWN acquisition implementation.
    pub(crate) fn capture_deferred<R: Read, E>(
        self,
        reader: R,
        mut consumer: impl FnMut(&str) -> Result<(), E>,
    ) -> Result<RuntimeTextCaptureSummary, DeferredCaptureFailure<E>> {
        self.capture_fragments_deferred(reader, |fragment| consumer(fragment.text()))
    }

    pub(crate) fn capture_fragments_deferred<R: Read, E>(
        mut self,
        mut reader: R,
        mut consumer: impl FnMut(Utf8Fragment<'_>) -> Result<(), E>,
    ) -> Result<RuntimeTextCaptureSummary, DeferredCaptureFailure<E>> {
        let acquisition = self.limit + 1; // checked by prepare
        let mut total = 0u64;
        let mut carry = 0usize;
        let mut utf8_error = None;
        let mut consumer_failure = None;
        loop {
            let wanted = usize::try_from((acquisition - total).min(self.chunk_bytes as u64))
                .expect("capture extent is bounded by the prepared usize chunk");
            let read = match reader.read(&mut self.buffer[carry..carry + wanted]) {
                Ok(read) if read <= wanted => read,
                Ok(_) => {
                    return Err(RuntimeTextCaptureError::Acquisition(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "reader exceeded the supplied capture buffer",
                    ))
                    .into());
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(RuntimeTextCaptureError::Acquisition(error).into()),
            };
            if read == 0 {
                if carry != 0 {
                    utf8_error = Some((total - carry as u64, None));
                }
                break;
            }
            total += read as u64;
            if total > self.limit {
                return Err(RuntimeTextCaptureError::InputLimit { limit: self.limit }.into());
            }
            if utf8_error.is_some() {
                continue;
            }
            let length = carry + read;
            match Utf8Fragment::decode(&self.buffer[..length]) {
                Ok(fragment) => {
                    offer(fragment, &mut consumer, &mut consumer_failure);
                    carry = 0;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    let fragment = Utf8Fragment::decode(&self.buffer[..valid])
                        .expect("Utf8Error identifies a valid prefix");
                    offer(fragment, &mut consumer, &mut consumer_failure);
                    match error.error_len() {
                        Some(error_len) => {
                            utf8_error =
                                Some((total - length as u64 + valid as u64, Some(error_len)));
                            carry = 0;
                        }
                        None => {
                            carry = length - valid;
                            debug_assert!(carry <= UTF8_CARRY);
                            self.buffer.copy_within(valid..length, 0);
                        }
                    }
                }
            }
        }
        if let Some((valid_up_to, error_len)) = utf8_error {
            return Err(RuntimeTextCaptureError::InvalidUtf8 {
                valid_up_to,
                error_len,
            }
            .into());
        }
        match consumer_failure {
            Some(ConsumerFailure::Error(error)) => {
                Err(RuntimeTextCaptureError::Consumer(error).into())
            }
            Some(ConsumerFailure::Panic(payload)) => Err(DeferredCaptureFailure::Panic(payload)),
            None => Ok(RuntimeTextCaptureSummary { bytes: total }),
        }
    }
}

pub(crate) enum DeferredCaptureFailure<E> {
    Error(RuntimeTextCaptureError<E>),
    Panic(Box<dyn std::any::Any + Send>),
}
impl<E> From<RuntimeTextCaptureError<E>> for DeferredCaptureFailure<E> {
    fn from(error: RuntimeTextCaptureError<E>) -> Self {
        Self::Error(error)
    }
}

pub(crate) enum ConsumerFailure<E> {
    Error(E),
    Panic(Box<dyn std::any::Any + Send>),
}
fn offer<E>(
    fragment: Utf8Fragment<'_>,
    consumer: &mut impl FnMut(Utf8Fragment<'_>) -> Result<(), E>,
    failure: &mut Option<ConsumerFailure<E>>,
) {
    if fragment.text().is_empty() || failure.is_some() {
        return;
    }
    *failure = match catch_unwind(AssertUnwindSafe(|| consumer(fragment))) {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(ConsumerFailure::Error(error)),
        Err(payload) => Some(ConsumerFailure::Panic(payload)),
    };
}

#[cfg(test)]
#[path = "resource_text_capture_tests.rs"]
mod tests;
