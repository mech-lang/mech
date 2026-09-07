use crate::{CanonicalCellId, MemoryObjectId};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

use core::{marker::PhantomData, mem, mem::MaybeUninit, slice};

use super::{
    ActiveLeaseRecord, AllocationHandle, MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult,
    OwnedAllocationState, PlanObjectKey, RealizedMemoryPlan, RuntimeBinding,
};
use crate::{MemoryLifetime, MemoryPlanPoint};

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
pub enum ManagedPortRole {
    Input(usize),
    Output(usize),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CallAccessRequest {
    pub object: PlanObjectKey,
    pub mode: MemoryAccessMode,
    pub region: MemoryAccessRegion,
}

/// A relocatable typed capability naming one logical cell. It deliberately
/// contains neither a physical allocation handle nor an owning payload.
#[derive(Debug)]
pub struct ManagedPort<T> {
    cell: CanonicalCellId,
    role: ManagedPortRole,
    marker: PhantomData<fn() -> T>,
}

impl<T> Copy for ManagedPort<T> {}

impl<T> Clone for ManagedPort<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> ManagedPort<T> {
    #[cfg(feature = "functions")]
    pub(crate) const fn input(cell: CanonicalCellId, index: usize) -> Self {
        Self {
            cell,
            role: ManagedPortRole::Input(index),
            marker: PhantomData,
        }
    }

    #[cfg(feature = "functions")]
    pub(crate) const fn output(cell: CanonicalCellId) -> Self {
        Self {
            cell,
            role: ManagedPortRole::Output(0),
            marker: PhantomData,
        }
    }

    pub const fn logical_cell_id(self) -> CanonicalCellId {
        self.cell
    }

    pub const fn role(self) -> ManagedPortRole {
        self.role
    }
}

/// One planned access associated with a relocatable logical port.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ManagedCallAccessRequest {
    cell: CanonicalCellId,
    role: ManagedPortRole,
    object: PlanObjectKey,
    mode: MemoryAccessMode,
    region: MemoryAccessRegion,
}

impl ManagedCallAccessRequest {
    pub const fn new<T>(
        port: ManagedPort<T>,
        object: PlanObjectKey,
        mode: MemoryAccessMode,
        region: MemoryAccessRegion,
    ) -> Self {
        Self {
            cell: port.logical_cell_id(),
            role: port.role(),
            object,
            mode,
            region,
        }
    }

    pub const fn for_input_cell(
        cell: CanonicalCellId,
        index: usize,
        object: PlanObjectKey,
        mode: MemoryAccessMode,
        region: MemoryAccessRegion,
    ) -> Self {
        Self {
            cell,
            role: ManagedPortRole::Input(index),
            object,
            mode,
            region,
        }
    }

    pub const fn for_output_cell(
        cell: CanonicalCellId,
        index: usize,
        object: PlanObjectKey,
        mode: MemoryAccessMode,
        region: MemoryAccessRegion,
    ) -> Self {
        Self {
            cell,
            role: ManagedPortRole::Output(index),
            object,
            mode,
            region,
        }
    }
}

mod managed_element_sealed {
    pub trait Sealed {}
}

/// A fixed-width initialized element that may be viewed inside a managed
/// lease. The sealed set excludes owning or recursively allocated values.
pub trait ManagedElement: managed_element_sealed::Sealed + Copy + 'static {}

impl<T> ManagedElement for T where T: managed_element_sealed::Sealed + Copy + 'static {}

macro_rules! managed_elements {
    ($($type:ty),+ $(,)?) => {$(
        impl managed_element_sealed::Sealed for $type {}
    )+};
}

managed_elements!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, usize
);

/// Sealed sequential constructor for fresh planned storage. It exposes only
/// `MaybeUninit<T>` writes and records how many leading elements were actually
/// constructed; it never permits reading an uninitialized slot.
pub struct InitWriter<'a, T: ManagedElement> {
    slots: &'a mut [MaybeUninit<T>],
    initialized: usize,
}

impl<T: ManagedElement> InitWriter<'_, T> {
    pub fn remaining(&self) -> usize {
        self.slots.len().saturating_sub(self.initialized)
    }

    pub fn write_next(&mut self, value: T) -> MemoryRuntimeResult<()> {
        let capacity = self.slots.len();
        let slot =
            self.slots
                .get_mut(self.initialized)
                .ok_or(MemoryRuntimeError::CapacityExceeded {
                    object: MemoryObjectId::new(0),
                    requested: self.initialized.saturating_add(1) as u64,
                    capacity: capacity as u64,
                })?;
        slot.write(value);
        self.initialized += 1;
        Ok(())
    }

    pub fn copy_from_slice(&mut self, values: &[T]) -> MemoryRuntimeResult<()> {
        if values.len() > self.remaining() {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: MemoryObjectId::new(0),
                requested: values.len() as u64,
                capacity: self.remaining() as u64,
            });
        }
        for value in values {
            self.write_next(*value)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ResolvedAccessRequest {
    cell: Option<CanonicalCellId>,
    role: Option<ManagedPortRole>,
    object: PlanObjectKey,
    handle: Option<AllocationHandle>,
    mode: MemoryAccessMode,
    start: u64,
    end: u64,
    relative_end: u64,
    lifetime: MemoryLifetime,
    region: MemoryAccessRegion,
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
                cell: None,
                role: None,
                object: request.object,
                handle: binding.handle(),
                mode: request.mode,
                start,
                end,
                relative_end,
                lifetime: realized.lifetime(request.object)?,
                region: request.region,
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

    pub fn prepare_managed_call(
        &self,
        realized: &RealizedMemoryPlan,
        requests: &[ManagedCallAccessRequest],
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
                cell: Some(request.cell),
                role: Some(request.role),
                object: request.object,
                handle: binding.handle(),
                mode: request.mode,
                start,
                end,
                relative_end,
                lifetime: realized.lifetime(request.object)?,
                region: request.region,
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
                request.cell,
                request.role,
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
        if state.active_revision != Some(realized.revision()) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(realized.revision()),
                actual: realized.revision(),
            });
        }
        for (position, request) in prepared.requests.iter().enumerate() {
            let region = state.regions.get(&request.object).ok_or(
                MemoryRuntimeError::UnknownPlanObject {
                    key: request.object,
                },
            )?;
            if region.handle != request.handle {
                let handle = request.handle.ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(request.object.object()),
                    size: request.relative_end,
                    alignment: 1,
                    reason: "empty plan object unexpectedly acquired physical storage",
                })?;
                return Err(MemoryRuntimeError::StaleAllocationGeneration {
                    handle,
                    current: region.handle.map(AllocationHandle::generation).unwrap_or(0),
                });
            }
            let Some(handle) = request.handle else {
                if request.start != request.end || request.relative_end != 0 {
                    return Err(MemoryRuntimeError::InvalidLayout {
                        object: Some(request.object.object()),
                        size: request.relative_end,
                        alignment: 1,
                        reason: "empty plan object has a nonempty access span",
                    });
                }
                if !lifetime_is_active(request.lifetime, state.active_point.get()) {
                    return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                        object: Some(request.object.object()),
                        from: "inactive plan interval",
                        to: "leased empty allocation",
                    });
                }
                continue;
            };
            let record = state.record(handle)?;
            if record.state != OwnedAllocationState::Live {
                return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                    object: Some(request.object.object()),
                    from: "retired allocation",
                    to: "leased allocation",
                });
            }
            if matches!(record.space, crate::MemorySpace::Device { .. }) {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(request.object.object()),
                    size: request.relative_end,
                    alignment: record.alignment,
                    reason: "device allocation requires a backend submission hold",
                });
            }
            if !lifetime_is_active(request.lifetime, state.active_point.get()) {
                return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                    object: Some(request.object.object()),
                    from: "inactive plan interval",
                    to: "leased allocation",
                });
            }
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
            let token = if let Some(handle) = request.handle {
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
                Some(token)
            } else {
                None
            };
            held.push(HeldLease {
                token,
                handle: request.handle,
                object: request.object,
                cell: request.cell,
                role: request.role,
                mode: request.mode,
                start: request.start,
                end: request.end,
                relative_end: request.relative_end,
                region: request.region,
                incarnation: state
                    .regions
                    .get(&request.object)
                    .ok_or(MemoryRuntimeError::UnknownPlanObject {
                        key: request.object,
                    })?
                    .incarnation,
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
    token: Option<u64>,
    handle: Option<AllocationHandle>,
    object: PlanObjectKey,
    cell: Option<CanonicalCellId>,
    role: Option<ManagedPortRole>,
    mode: MemoryAccessMode,
    start: u64,
    end: u64,
    relative_end: u64,
    region: MemoryAccessRegion,
    incarnation: super::RegionIncarnation,
}

/// Complete call-scoped authority for one managed kernel invocation.
///
/// Leased storage can be observed only inside the accessor closure, so a
/// borrowed slice cannot escape the frame:
///
/// ```compile_fail
/// use mech_core::{KernelMemoryFrame, ManagedPort};
///
/// fn escape<'a>(frame: &'a KernelMemoryFrame<'_>, port: ManagedPort<u64>) -> &'a [u64] {
///     frame.with_port_slice(port, |values| values).unwrap()
/// }
/// ```
///
/// A read accessor cannot be used to obtain write authority:
///
/// ```compile_fail
/// use mech_core::{KernelMemoryFrame, ManagedPort};
///
/// fn unauthorized_write(frame: &KernelMemoryFrame<'_>, port: ManagedPort<u64>) {
///     frame.with_port_slice(port, |values| values[0] = 1).unwrap();
/// }
/// ```
pub struct KernelMemoryFrame<'a> {
    domain: &'a MemoryDomain,
    realized: &'a RealizedMemoryPlan,
    leases: Vec<HeldLease>,
}

impl KernelMemoryFrame<'_> {
    pub fn with_port_slice<T: ManagedElement, R>(
        &self,
        port: ManagedPort<T>,
        access: impl FnOnce(&[T]) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self.port_lease(port.logical_cell_id(), port.role(), false)?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.with_bytes(lease.object, |bytes| {
            if bytes.is_empty() {
                return Ok(access(&[]));
            }
            validate_typed_bytes::<T>(
                lease.object.object(),
                lease.start,
                bytes.as_ptr() as usize,
                bytes.len(),
            )?;
            let count = bytes.len() / mem::size_of::<T>();
            // SAFETY: ManagedElement is sealed to fixed-width Copy values,
            // the realized arena and offset satisfy T's alignment, and the
            // lease restricts this initialized region to shared reads.
            let values = unsafe { slice::from_raw_parts(bytes.as_ptr().cast::<T>(), count) };
            Ok(access(values))
        })?
    }

    pub fn with_port_slice_mut<T: ManagedElement, R>(
        &mut self,
        port: ManagedPort<T>,
        access: impl FnOnce(&mut [T]) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self.port_lease(port.logical_cell_id(), port.role(), true)?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.with_bytes_mut(lease.object, |bytes| {
            if bytes.is_empty() {
                return Ok(access(&mut []));
            }
            validate_typed_bytes::<T>(
                lease.object.object(),
                lease.start,
                bytes.as_ptr() as usize,
                bytes.len(),
            )?;
            let count = bytes.len() / mem::size_of::<T>();
            // SAFETY: ManagedElement is sealed to fixed-width Copy values,
            // alignment and length were checked, and the exclusive lease
            // guarantees no overlapping live reference for this closure.
            let values =
                unsafe { slice::from_raw_parts_mut(bytes.as_mut_ptr().cast::<T>(), count) };
            Ok(access(values))
        })?
    }

    pub fn with_port_init_writer<T: ManagedElement, R>(
        &mut self,
        port: ManagedPort<T>,
        access: impl FnOnce(&mut InitWriter<'_, T>) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self.port_lease(port.logical_cell_id(), port.role(), true)?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.with_init_writer(lease, access)
    }

    pub fn with_object_init_writer<T: ManagedElement, R>(
        &mut self,
        object: PlanObjectKey,
        access: impl FnOnce(&mut InitWriter<'_, T>) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self
            .leases
            .iter()
            .find(|lease| lease.object == object && lease.mode.writes())
            .copied()
            .ok_or(MemoryRuntimeError::BorrowConflict {
                object: object.object(),
            })?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.with_init_writer(lease, access)
    }

    fn with_init_writer<T: ManagedElement, R>(
        &mut self,
        lease: HeldLease,
        access: impl FnOnce(&mut InitWriter<'_, T>) -> R,
    ) -> MemoryRuntimeResult<R> {
        let length = lease.end.checked_sub(lease.start).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "initialization lease span",
                current: lease.start,
                change: lease.end,
            },
        )?;
        if length == 0 {
            let mut writer = InitWriter {
                slots: &mut [],
                initialized: 0,
            };
            return Ok(access(&mut writer));
        }
        let (pointer, capacity) = {
            let state = self.domain.state.borrow();
            let region = state
                .regions
                .get(&lease.object)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: lease.object })?;
            if region.incarnation != lease.incarnation {
                return Err(MemoryRuntimeError::StaleRegionIncarnation {
                    key: lease.object,
                    expected: lease.incarnation,
                    actual: region.incarnation,
                });
            }
            let handle = lease.handle.ok_or(MemoryRuntimeError::InvalidLayout {
                object: Some(lease.object.object()),
                size: length,
                alignment: mem::align_of::<T>() as u32,
                reason: "nonempty initialization has no allocation",
            })?;
            let record = state.record(handle)?;
            let block = record
                .block
                .as_ref()
                .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
            let pointer = block
                .pointer()
                .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
            (pointer, record.capacity_bytes)
        };
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: lease.object.object(),
                requested: lease.start,
                capacity,
            })?;
        let length_usize =
            usize::try_from(length).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: lease.object.object(),
                requested: length,
                capacity,
            })?;
        let address = unsafe { pointer.as_ptr().add(start) } as usize;
        validate_typed_bytes::<T>(lease.object.object(), lease.start, address, length_usize)?;
        let count = length_usize / mem::size_of::<T>();
        // SAFETY: the lease owns this uninitialized range exclusively; the
        // resulting writer exposes only MaybeUninit writes and cannot escape.
        let slots = unsafe {
            slice::from_raw_parts_mut(pointer.as_ptr().add(start).cast::<MaybeUninit<T>>(), count)
        };
        let mut writer = InitWriter {
            slots,
            initialized: 0,
        };
        let result = access(&mut writer);
        let initialized_bytes = u64::try_from(writer.initialized)
            .ok()
            .and_then(|count| count.checked_mul(mem::size_of::<T>() as u64))
            .and_then(|bytes| {
                lease
                    .relative_end
                    .checked_sub(length)
                    .and_then(|start| start.checked_add(bytes))
            })
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "initialized element bytes",
                current: writer.initialized as u64,
                change: mem::size_of::<T>() as u64,
            })?;
        self.realized
            .record_initialized(lease.object, initialized_bytes)?;
        Ok(result)
    }

    fn port_lease(
        &self,
        cell: CanonicalCellId,
        role: ManagedPortRole,
        write: bool,
    ) -> MemoryRuntimeResult<HeldLease> {
        self.leases
            .iter()
            .find(|lease| {
                lease.cell == Some(cell)
                    && lease.role == Some(role)
                    && if write {
                        lease.mode.writes()
                    } else {
                        matches!(
                            lease.mode,
                            MemoryAccessMode::Read | MemoryAccessMode::ExclusiveInPlace
                        )
                    }
            })
            .copied()
            .ok_or(MemoryRuntimeError::UnplannedAllocation {
                object: None,
                requested: 0,
            })
    }

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
        let (base, capacity) = {
            let state = self.domain.state.borrow();
            let region = state
                .regions
                .get(&lease.object)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: lease.object })?;
            if region.incarnation != lease.incarnation {
                return Err(MemoryRuntimeError::StaleRegionIncarnation {
                    key: lease.object,
                    expected: lease.incarnation,
                    actual: region.incarnation,
                });
            }
            let Some(handle) = lease.handle else {
                return Ok(access(&[]));
            };
            let record = state.record(handle)?;
            let block = record
                .block
                .as_ref()
                .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
            let base = block
                .pointer()
                .ok_or(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: lease.relative_end,
                    initialized: 0,
                })?;
            (base, record.capacity_bytes)
        };
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
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: lease.start,
                capacity,
            })?;
        let length = usize::try_from(length).map_err(|_| MemoryRuntimeError::CapacityExceeded {
            object: object.object(),
            requested: length,
            capacity,
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
        let (base, capacity, initialized_bytes) = {
            let state = self.domain.state.borrow();
            let region = state
                .regions
                .get(&lease.object)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: lease.object })?;
            if region.incarnation != lease.incarnation {
                return Err(MemoryRuntimeError::StaleRegionIncarnation {
                    key: lease.object,
                    expected: lease.incarnation,
                    actual: region.incarnation,
                });
            }
            let Some(handle) = lease.handle else {
                return Ok(access(&mut []));
            };
            let record = state.record(handle)?;
            let block = record
                .block
                .as_ref()
                .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
            let base = block
                .pointer()
                .ok_or(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: lease.relative_end,
                    initialized: region.initialized_bytes,
                })?;
            (base, record.capacity_bytes, region.initialized_bytes)
        };
        let length = lease.end.checked_sub(lease.start).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "lease span",
                current: lease.start,
                change: lease.end,
            },
        )?;
        if lease.relative_end > initialized_bytes {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: object.object(),
                requested: lease.relative_end,
                initialized: initialized_bytes,
            });
        }
        if length == 0 {
            return Ok(access(&mut []));
        }
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: lease.start,
                capacity,
            })?;
        let length = usize::try_from(length).map_err(|_| MemoryRuntimeError::CapacityExceeded {
            object: object.object(),
            requested: length,
            capacity,
        })?;
        // SAFETY: complete acquisition proved this write span has no live
        // overlapping read or write lease; the mutable slice cannot escape
        // the closure and remains within the validated block.
        let bytes = unsafe { slice::from_raw_parts_mut(base.as_ptr().add(start), length) };
        Ok(access(bytes))
    }

    /// Writes an initialized prefix of an already leased planned region.
    /// This is used by bounded transfers whose per-turn payload may be
    /// smaller than their activation-time capacity.
    pub fn with_bytes_mut_prefix<R>(
        &mut self,
        object: PlanObjectKey,
        length_bytes: u64,
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
        let leased_length = lease.end.checked_sub(lease.start).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "lease span",
                current: lease.start,
                change: lease.end,
            },
        )?;
        if length_bytes > leased_length {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: length_bytes,
                capacity: leased_length,
            });
        }
        let relative_end = lease
            .relative_end
            .checked_sub(leased_length)
            .and_then(|start| start.checked_add(length_bytes))
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "initialized transfer prefix",
                current: lease.relative_end,
                change: length_bytes,
            })?;
        let (base, capacity, initialized_bytes) = {
            let state = self.domain.state.borrow();
            let region = state
                .regions
                .get(&lease.object)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: lease.object })?;
            if region.incarnation != lease.incarnation {
                return Err(MemoryRuntimeError::StaleRegionIncarnation {
                    key: lease.object,
                    expected: lease.incarnation,
                    actual: region.incarnation,
                });
            }
            let Some(handle) = lease.handle else {
                if length_bytes != 0 {
                    return Err(MemoryRuntimeError::CapacityExceeded {
                        object: object.object(),
                        requested: length_bytes,
                        capacity: 0,
                    });
                }
                return Ok(access(&mut []));
            };
            let record = state.record(handle)?;
            let block = record
                .block
                .as_ref()
                .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
            let base = block
                .pointer()
                .ok_or(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: relative_end,
                    initialized: region.initialized_bytes,
                })?;
            (base, record.capacity_bytes, region.initialized_bytes)
        };
        if relative_end > initialized_bytes {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: object.object(),
                requested: relative_end,
                initialized: initialized_bytes,
            });
        }
        if length_bytes == 0 {
            return Ok(access(&mut []));
        }
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: lease.start,
                capacity,
            })?;
        let length =
            usize::try_from(length_bytes).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: length_bytes,
                capacity,
            })?;
        // SAFETY: the exclusive planned lease owns the complete enclosing
        // region, and the checked prefix remains within that region.
        let bytes = unsafe { slice::from_raw_parts_mut(base.as_ptr().add(start), length) };
        Ok(access(bytes))
    }
}

fn validate_typed_bytes<T: ManagedElement>(
    object: MemoryObjectId,
    absolute_offset: u64,
    address: usize,
    bytes: usize,
) -> MemoryRuntimeResult<()> {
    let element_bytes = mem::size_of::<T>();
    let alignment = mem::align_of::<T>();
    if element_bytes == 0 {
        return Err(MemoryRuntimeError::InvalidLayout {
            object: Some(object),
            size: bytes as u64,
            alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
            reason: "zero-sized managed elements are unsupported",
        });
    }
    if bytes % element_bytes != 0
        || absolute_offset % alignment as u64 != 0
        || address % alignment != 0
    {
        return Err(MemoryRuntimeError::InvalidLayout {
            object: Some(object),
            size: bytes as u64,
            alignment: u32::try_from(alignment).unwrap_or(u32::MAX),
            reason: "managed typed view has incompatible length or alignment",
        });
    }
    Ok(())
}

fn validate_contiguous_typed_region(
    object: MemoryObjectId,
    region: MemoryAccessRegion,
) -> MemoryRuntimeResult<()> {
    if matches!(
        region,
        MemoryAccessRegion::WholeInitialized | MemoryAccessRegion::Contiguous { .. }
    ) {
        return Ok(());
    }
    Err(MemoryRuntimeError::InvalidLayout {
        object: Some(object),
        size: 0,
        alignment: 1,
        reason: "noncontiguous planned access requires a geometry-preserving managed view",
    })
}

impl Drop for KernelMemoryFrame<'_> {
    fn drop(&mut self) {
        let mut state = self.domain.state.borrow_mut();
        for held in self.leases.drain(..) {
            let (Some(handle), Some(token)) = (held.handle, held.token) else {
                continue;
            };
            let record = state
                .record_mut(handle)
                .expect("a live frame retains every leased allocation record");
            let position = record
                .leases
                .iter()
                .position(|lease| lease.token == token)
                .expect("a live frame retains every installed lease token");
            record.leases.remove(position);
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
        RuntimeBinding::Device { offset_bytes, .. } => *offset_bytes,
        RuntimeBinding::ManagedCanonicalPayload { .. }
        | RuntimeBinding::PinnedExternal { .. }
        | RuntimeBinding::Empty { .. } => 0,
    };
    let accessible = if mode.writes() {
        binding.capacity_bytes()
    } else {
        binding.initialized_bytes()
    };
    if !mode.writes()
        && matches!(region, MemoryAccessRegion::WholeInitialized)
        && binding.initialized_bytes() < binding.required_initialization_bytes()
    {
        return Err(MemoryRuntimeError::UninitializedAccess {
            object,
            requested: binding.required_initialization_bytes(),
            initialized: binding.initialized_bytes(),
        });
    }
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

const fn lifetime_is_active(
    lifetime: MemoryLifetime,
    active_point: Option<MemoryPlanPoint>,
) -> bool {
    match lifetime {
        MemoryLifetime::Program | MemoryLifetime::Activation => true,
        MemoryLifetime::Turn { first, last }
        | MemoryLifetime::Transaction { first, last }
        | MemoryLifetime::Transfer { first, last } => match active_point {
            Some(point) => point.get() >= first.get() && point.get() <= last.get(),
            None => false,
        },
    }
}
