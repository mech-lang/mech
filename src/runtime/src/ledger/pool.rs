use std::sync::{Arc, Mutex, MutexGuard};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::turn_record::AccountedRecord;

#[derive(Debug)]
struct PoolState {
    available: Vec<Vec<u8>>,
    in_use_segments: usize,
    total_capacity: usize,
    reuses: u64,
    allocations: u64,
    dropped_oversized: u64,
}

#[derive(Debug)]
struct PoolInner {
    max_segments: usize,
    max_total_capacity: usize,
    max_reusable_capacity: usize,
    state: Mutex<PoolState>,
}

/// A bounded owner of reusable byte sections for typed turn records.
#[derive(Clone, Debug)]
pub struct RecordBufferPool {
    inner: Arc<PoolInner>,
}

impl RecordBufferPool {
    pub fn new(
        max_segments: usize,
        max_total_capacity: usize,
        max_reusable_capacity: usize,
    ) -> MResult<Self> {
        if max_segments == 0 {
            return Err(pool_exhausted(
                "segment limit must allow at least one segment",
            ));
        }
        if max_reusable_capacity > max_total_capacity {
            return Err(pool_exhausted(
                "reusable segment limit exceeds total pool capacity",
            ));
        }
        let mut available = Vec::new();
        available.try_reserve_exact(max_segments).map_err(|_| {
            MechError::new(
                RecordBufferPoolExhausted {
                    reason: "failed to allocate pool segment slots",
                },
                None,
            )
        })?;
        Ok(Self {
            inner: Arc::new(PoolInner {
                max_segments,
                max_total_capacity,
                max_reusable_capacity,
                state: Mutex::new(PoolState {
                    available,
                    in_use_segments: 0,
                    total_capacity: 0,
                    reuses: 0,
                    allocations: 0,
                    dropped_oversized: 0,
                }),
            }),
        })
    }

    pub fn acquire(&self, minimum_capacity: usize) -> MResult<PooledRecordBuffer> {
        if minimum_capacity > self.inner.max_total_capacity {
            return Err(pool_exhausted(
                "requested segment exceeds total pool capacity",
            ));
        }
        let mut state = self.lock();
        if let Some(index) = state
            .available
            .iter()
            .enumerate()
            .filter(|(_, buffer)| buffer.capacity() >= minimum_capacity)
            .min_by_key(|(_, buffer)| buffer.capacity())
            .map(|(index, _)| index)
        {
            let mut buffer = state.available.swap_remove(index);
            buffer.clear();
            state.in_use_segments += 1;
            state.reuses += 1;
            return Ok(PooledRecordBuffer {
                pool: Arc::clone(&self.inner),
                buffer: Some(buffer),
            });
        }

        let owned_segments = state
            .available
            .len()
            .checked_add(state.in_use_segments)
            .expect("bounded pool segment accounting overflow");
        let segment_replacement_required = owned_segments >= self.inner.max_segments;

        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(minimum_capacity)
            .map_err(|_| pool_exhausted("failed to allocate requested record buffer capacity"))?;
        let new_capacity = buffer.capacity();
        let capacity_replacement_required = state
            .total_capacity
            .checked_add(new_capacity)
            .is_none_or(|total| total > self.inner.max_total_capacity);
        let replace_index = if segment_replacement_required || capacity_replacement_required {
            state
                .available
                .iter()
                .enumerate()
                .max_by_key(|(_, buffer)| buffer.capacity())
                .map(|(index, _)| index)
                .ok_or_else(|| {
                    pool_exhausted("all bounded pool segments are currently owned by records")
                })?
                .into()
        } else {
            None
        };
        let replaced_capacity = replace_index
            .map(|index| state.available[index].capacity())
            .unwrap_or(0);

        let retained_capacity = state
            .total_capacity
            .checked_sub(replaced_capacity)
            .expect("pool replacement capacity accounting underflow");
        let new_total = retained_capacity
            .checked_add(new_capacity)
            .ok_or_else(|| pool_exhausted("pool capacity accounting overflow"))?;
        if new_total > self.inner.max_total_capacity {
            return Err(pool_exhausted(
                "requested segment would exceed total pool capacity",
            ));
        }

        if let Some(index) = replace_index {
            let replaced = state.available.swap_remove(index);
            drop(replaced);
        }
        state.total_capacity = new_total;
        state.in_use_segments += 1;
        state.allocations += 1;
        Ok(PooledRecordBuffer {
            pool: Arc::clone(&self.inner),
            buffer: Some(buffer),
        })
    }

    pub fn stats(&self) -> RecordBufferPoolStats {
        let state = self.lock();
        RecordBufferPoolStats {
            available_segments: state.available.len(),
            in_use_segments: state.in_use_segments,
            total_capacity: state.total_capacity,
            reuses: state.reuses,
            allocations: state.allocations,
            dropped_oversized: state.dropped_oversized,
        }
    }

    fn lock(&self) -> MutexGuard<'_, PoolState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordBufferPoolStats {
    pub available_segments: usize,
    pub in_use_segments: usize,
    pub total_capacity: usize,
    pub reuses: u64,
    pub allocations: u64,
    pub dropped_oversized: u64,
}

/// An exclusively owned pool segment. It recycles only when this owner drops.
#[derive(Debug)]
pub struct PooledRecordBuffer {
    pool: Arc<PoolInner>,
    buffer: Option<Vec<u8>>,
}

impl PooledRecordBuffer {
    pub fn len(&self) -> usize {
        self.buffer().len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer().is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.buffer().capacity()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.buffer().as_slice()
    }

    pub fn clear(&mut self) {
        self.buffer_mut().clear();
    }

    pub fn try_extend_from_slice(&mut self, bytes: &[u8]) -> MResult<()> {
        let buffer = self.buffer_mut();
        let requested = buffer.len().checked_add(bytes.len()).ok_or_else(|| {
            MechError::new(
                RecordBufferCapacityExceeded {
                    capacity: buffer.capacity(),
                    requested: usize::MAX,
                },
                None,
            )
        })?;
        if requested > buffer.capacity() {
            return Err(MechError::new(
                RecordBufferCapacityExceeded {
                    capacity: buffer.capacity(),
                    requested,
                },
                None,
            ));
        }
        buffer.extend_from_slice(bytes);
        Ok(())
    }

    fn buffer(&self) -> &Vec<u8> {
        self.buffer.as_ref().expect("live pooled record buffer")
    }

    fn buffer_mut(&mut self) -> &mut Vec<u8> {
        self.buffer.as_mut().expect("live pooled record buffer")
    }
}

impl AccountedRecord for PooledRecordBuffer {
    fn retained_bytes(&self) -> usize {
        self.capacity()
    }
}

impl Drop for PooledRecordBuffer {
    fn drop(&mut self) {
        let Some(mut buffer) = self.buffer.take() else {
            return;
        };
        buffer.clear();
        let capacity = buffer.capacity();
        let mut state = self
            .pool
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.in_use_segments = state
            .in_use_segments
            .checked_sub(1)
            .expect("pool in-use segment accounting underflow");
        if capacity > self.pool.max_reusable_capacity {
            state.total_capacity = state
                .total_capacity
                .checked_sub(capacity)
                .expect("pool oversized capacity accounting underflow");
            state.dropped_oversized += 1;
            return;
        }
        state.available.push(buffer);
    }
}

fn pool_exhausted(reason: &'static str) -> MechError {
    MechError::new(RecordBufferPoolExhausted { reason }, None)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordBufferPoolExhausted {
    pub reason: &'static str,
}

impl MechErrorKind for RecordBufferPoolExhausted {
    fn name(&self) -> &str {
        "RecordBufferPoolExhausted"
    }

    fn message(&self) -> String {
        format!("record buffer pool exhausted: {}", self.reason)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordBufferCapacityExceeded {
    pub capacity: usize,
    pub requested: usize,
}

impl MechErrorKind for RecordBufferCapacityExceeded {
    fn name(&self) -> &str {
        "RecordBufferCapacityExceeded"
    }

    fn message(&self) -> String {
        format!(
            "record buffer capacity exceeded: requested {}, capacity {}",
            self.requested, self.capacity
        )
    }
}
