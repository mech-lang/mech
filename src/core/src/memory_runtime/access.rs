use crate::MemoryObjectId;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

use core::slice;

use super::{
    ActiveLeaseRecord, AllocationHandle, MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult,
    PlanObjectKey, RealizedMemoryPlan, RuntimeBinding,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryAccessMode {
    Read,
    Write,
    ExclusiveInPlace,
}

impl MemoryAccessMode {
    const fn writes(self) -> bool {
        matches!(self, Self::Write | Self::ExclusiveInPlace)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryAccessRegion {
    WholeInitialized,
    Contiguous {
        offset_bytes: u64,
        length_bytes: u64,
    },
    Strided {
        offset_bytes: u64,
        count: u64,
        stride_bytes: u64,
        element_bytes: u64,
    },
    Rectangle {
        offset_bytes: u64,
        rows: u64,
        columns: u64,
        row_stride_bytes: u64,
        column_stride_bytes: u64,
        element_bytes: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CallAccessRequest {
    pub object: PlanObjectKey,
    pub mode: MemoryAccessMode,
    pub region: MemoryAccessRegion,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ResolvedAccessRequest {
    object: PlanObjectKey,
    handle: Option<AllocationHandle>,
    mode: MemoryAccessMode,
    start: u64,
    end: u64,
    relative_end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedCallAccess {
    revision: super::MemoryPlanRevision,
    requests: Box<[ResolvedAccessRequest]>,
}

impl MemoryDomain {
    pub fn prepare_call(
        &self,
        realized: &RealizedMemoryPlan,
        requests: &[CallAccessRequest],
    ) -> MemoryRuntimeResult<PreparedCallAccess> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        let mut resolved = Vec::new();
        resolved.try_reserve_exact(requests.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: requests.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        for request in requests {
            let binding = realized.binding(request.object)?;
            let (start, end, relative_end) = enclosing_span(
                request.object.object(),
                &binding,
                request.mode,
                request.region,
            )?;
            resolved.push(ResolvedAccessRequest {
                object: request.object,
                handle: binding.handle(),
                mode: request.mode,
                start,
                end,
                relative_end,
            });
        }
        resolved.sort_by_key(|request| {
            (
                request.handle.map(AllocationHandle::domain),
                request.handle.map(AllocationHandle::slot),
                request.start,
                request.end,
                request.mode,
                request.object,
            )
        });
        resolved.dedup();
        Ok(PreparedCallAccess {
            revision: realized.revision(),
            requests: resolved.into_boxed_slice(),
        })
    }

    pub fn acquire_call<'a>(
        &'a self,
        realized: &'a RealizedMemoryPlan,
        prepared: &PreparedCallAccess,
    ) -> MemoryRuntimeResult<KernelMemoryFrame<'a>> {
        if prepared.revision != realized.revision() {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: realized.revision(),
                actual: prepared.revision,
            });
        }
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }

        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        for (position, request) in prepared.requests.iter().enumerate() {
            let Some(handle) = request.handle else {
                continue;
            };
            let record = state.record(handle)?;
            if request.end > record.capacity_bytes {
                return Err(MemoryRuntimeError::CapacityExceeded {
                    object: request.object.object(),
                    requested: request.end,
                    capacity: record.capacity_bytes,
                });
            }
            if record.leases.iter().any(|lease| {
                overlaps(request.start, request.end, lease.start, lease.end)
                    && (request.mode.writes() || lease.write)
            }) {
                return Err(MemoryRuntimeError::BorrowConflict {
                    object: request.object.object(),
                });
            }
            if prepared.requests[..position].iter().any(|other| {
                other.handle == Some(handle)
                    && overlaps(request.start, request.end, other.start, other.end)
                    && (request.mode.writes() || other.mode.writes())
                    && !(request.object == other.object
                        && request.mode == MemoryAccessMode::ExclusiveInPlace
                        && other.mode == MemoryAccessMode::ExclusiveInPlace)
            }) {
                return Err(MemoryRuntimeError::BorrowConflict {
                    object: request.object.object(),
                });
            }
        }

        let mut held = Vec::new();
        held.try_reserve_exact(prepared.requests.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: prepared.requests.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            })?;
        for request in &prepared.requests {
            let Some(handle) = request.handle else {
                continue;
            };
            let token = state.next_lease_token;
            state.next_lease_token =
                token
                    .checked_add(1)
                    .ok_or(MemoryRuntimeError::IdentityExhausted {
                        identity: "lease token",
                    })?;
            state.record_mut(handle)?.leases.push(ActiveLeaseRecord {
                token,
                start: request.start,
                end: request.end,
                write: request.mode.writes(),
            });
            held.push(HeldLease {
                token,
                handle,
                object: request.object,
                mode: request.mode,
                start: request.start,
                end: request.end,
                relative_end: request.relative_end,
            });
        }
        drop(state);
        Ok(KernelMemoryFrame {
            domain: self,
            realized,
            leases: held,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct HeldLease {
    token: u64,
    handle: AllocationHandle,
    object: PlanObjectKey,
    mode: MemoryAccessMode,
    start: u64,
    end: u64,
    relative_end: u64,
}

pub struct KernelMemoryFrame<'a> {
    domain: &'a MemoryDomain,
    realized: &'a RealizedMemoryPlan,
    leases: Vec<HeldLease>,
}

impl KernelMemoryFrame<'_> {
    pub fn with_bytes<R>(
        &self,
        object: PlanObjectKey,
        access: impl FnOnce(&[u8]) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self
            .leases
            .iter()
            .find(|lease| {
                lease.object == object
                    && matches!(
                        lease.mode,
                        MemoryAccessMode::Read | MemoryAccessMode::ExclusiveInPlace
                    )
            })
            .ok_or(MemoryRuntimeError::BorrowConflict {
                object: object.object(),
            })?;
        self.realized.binding(object)?;
        let state = self.domain.state.borrow();
        let record = state.record(lease.handle)?;
        let block = record
            .block
            .as_ref()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle {
                handle: lease.handle,
            })?;
        let length = lease.end.checked_sub(lease.start).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "lease span",
                current: lease.start,
                change: lease.end,
            },
        )?;
        if length == 0 {
            return Ok(access(&[]));
        }
        let base = block
            .pointer()
            .ok_or(MemoryRuntimeError::UninitializedAccess {
                object: object.object(),
                requested: length,
                initialized: 0,
            })?;
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: lease.start,
                capacity: record.capacity_bytes,
            })?;
        let length = usize::try_from(length).map_err(|_| MemoryRuntimeError::CapacityExceeded {
            object: object.object(),
            requested: length,
            capacity: record.capacity_bytes,
        })?;
        // SAFETY: plan validation proved this half-open span belongs to the
        // live block; the read lease keeps it initialized, live, and
        // non-mutably aliased for the closure duration.
        let bytes = unsafe { slice::from_raw_parts(base.as_ptr().add(start), length) };
        Ok(access(bytes))
    }

    pub fn with_bytes_mut<R>(
        &mut self,
        object: PlanObjectKey,
        access: impl FnOnce(&mut [u8]) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self
            .leases
            .iter()
            .find(|lease| lease.object == object && lease.mode.writes())
            .copied()
            .ok_or(MemoryRuntimeError::BorrowConflict {
                object: object.object(),
            })?;
        let mut state = self.domain.state.borrow_mut();
        let record = state.record_mut(lease.handle)?;
        let length = lease.end.checked_sub(lease.start).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "lease span",
                current: lease.start,
                change: lease.end,
            },
        )?;
        if length == 0 {
            return Ok(access(&mut []));
        }
        let block = record
            .block
            .as_mut()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle {
                handle: lease.handle,
            })?;
        let base = block
            .pointer()
            .ok_or(MemoryRuntimeError::UninitializedAccess {
                object: object.object(),
                requested: length,
                initialized: 0,
            })?;
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: lease.start,
                capacity: record.capacity_bytes,
            })?;
        let length = usize::try_from(length).map_err(|_| MemoryRuntimeError::CapacityExceeded {
            object: object.object(),
            requested: length,
            capacity: record.capacity_bytes,
        })?;
        // SAFETY: complete acquisition proved this write span has no live
        // overlapping read or write lease; the mutable slice cannot escape
        // the closure and remains within the validated block.
        let bytes = unsafe { slice::from_raw_parts_mut(base.as_ptr().add(start), length) };
        let result = access(bytes);
        drop(state);
        self.realized
            .record_initialized(object, lease.relative_end)?;
        Ok(result)
    }
}

impl Drop for KernelMemoryFrame<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.domain.state.try_borrow_mut() {
            for held in self.leases.drain(..) {
                if let Ok(record) = state.record_mut(held.handle)
                    && let Some(position) = record
                        .leases
                        .iter()
                        .position(|lease| lease.token == held.token)
                {
                    record.leases.remove(position);
                }
            }
        }
    }
}

fn enclosing_span(
    object: MemoryObjectId,
    binding: &RuntimeBinding,
    mode: MemoryAccessMode,
    region: MemoryAccessRegion,
) -> MemoryRuntimeResult<(u64, u64, u64)> {
    let base = match binding {
        RuntimeBinding::ManagedHostRegion { offset_bytes, .. } => *offset_bytes,
        RuntimeBinding::ManagedCanonicalPayload { .. }
        | RuntimeBinding::Device { .. }
        | RuntimeBinding::PinnedExternal { .. }
        | RuntimeBinding::Empty { .. } => 0,
    };
    let accessible = if mode.writes() {
        binding.capacity_bytes()
    } else {
        binding.initialized_bytes()
    };
    let relative = match region {
        MemoryAccessRegion::WholeInitialized => (0, accessible),
        MemoryAccessRegion::Contiguous {
            offset_bytes,
            length_bytes,
        } => (
            offset_bytes,
            offset_bytes
                .checked_add(length_bytes)
                .ok_or(MemoryRuntimeError::CapacityExceeded {
                    object,
                    requested: u64::MAX,
                    capacity: binding.capacity_bytes(),
                })?,
        ),
        MemoryAccessRegion::Strided {
            offset_bytes,
            count,
            stride_bytes,
            element_bytes,
        } => {
            let end = if count == 0 {
                offset_bytes
            } else {
                offset_bytes
                    .checked_add((count - 1).checked_mul(stride_bytes).ok_or(
                        MemoryRuntimeError::CapacityExceeded {
                            object,
                            requested: u64::MAX,
                            capacity: binding.capacity_bytes(),
                        },
                    )?)
                    .and_then(|end| end.checked_add(element_bytes))
                    .ok_or(MemoryRuntimeError::CapacityExceeded {
                        object,
                        requested: u64::MAX,
                        capacity: binding.capacity_bytes(),
                    })?
            };
            (offset_bytes, end)
        }
        MemoryAccessRegion::Rectangle {
            offset_bytes,
            rows,
            columns,
            row_stride_bytes,
            column_stride_bytes,
            element_bytes,
        } => {
            let end = if rows == 0 || columns == 0 {
                offset_bytes
            } else {
                offset_bytes
                    .checked_add((rows - 1).checked_mul(row_stride_bytes).ok_or(
                        MemoryRuntimeError::CapacityExceeded {
                            object,
                            requested: u64::MAX,
                            capacity: binding.capacity_bytes(),
                        },
                    )?)
                    .and_then(|end| {
                        end.checked_add((columns - 1).checked_mul(column_stride_bytes)?)
                    })
                    .and_then(|end| end.checked_add(element_bytes))
                    .ok_or(MemoryRuntimeError::CapacityExceeded {
                        object,
                        requested: u64::MAX,
                        capacity: binding.capacity_bytes(),
                    })?
            };
            (offset_bytes, end)
        }
    };
    if mode.writes() && relative.0 > binding.initialized_bytes() {
        return Err(MemoryRuntimeError::UninitializedAccess {
            object,
            requested: relative.0,
            initialized: binding.initialized_bytes(),
        });
    }
    if relative.1 > accessible {
        return Err(MemoryRuntimeError::UninitializedAccess {
            object,
            requested: relative.1,
            initialized: accessible,
        });
    }
    let start = base
        .checked_add(relative.0)
        .ok_or(MemoryRuntimeError::CapacityExceeded {
            object,
            requested: u64::MAX,
            capacity: binding.capacity_bytes(),
        })?;
    let end = base
        .checked_add(relative.1)
        .ok_or(MemoryRuntimeError::CapacityExceeded {
            object,
            requested: u64::MAX,
            capacity: binding.capacity_bytes(),
        })?;
    Ok((start, end, relative.1))
}

const fn overlaps(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
    left_start < right_end && right_start < left_end
}
