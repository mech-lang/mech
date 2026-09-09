//! Reservation-backed canonical payload ownership.

#[cfg(feature = "no_std")]
use alloc::{
    alloc::{AllocError, Allocator, Global, Layout},
    boxed::Box,
    rc::{Rc, Weak},
    string::String,
    sync::Arc,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    boxed::Box,
    rc::{Rc, Weak},
    string::String,
    sync::Arc,
    vec::Vec,
};

use core::{
    cell::{Cell, RefCell},
    mem::MaybeUninit,
    ptr::NonNull,
    str,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{
    MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, PlanObjectKey, RealizedMemoryPlan,
    RuntimeBinding,
};

pub(crate) struct PayloadBlockRecord {
    pub pointer: NonNull<u8>,
    pub layout: Layout,
}

/// Independent owner of one admitted indirect-payload envelope. Runtime
/// records authorize growth, but the envelope itself owns every live block so
/// containers remain valid after the creating domain handle is dropped.
pub(crate) struct PayloadEnvelopeOwner {
    object: PlanObjectKey,
    capacity_bytes: u64,
    block_capacity: usize,
    alignment: u32,
    accepting_allocations: Cell<bool>,
    blocks: RefCell<Vec<PayloadBlockRecord>>,
    accounting: Arc<RetainedPayloadAccounting>,
    accounted_bytes: u64,
}

impl PayloadEnvelopeOwner {
    pub(crate) fn new(
        object: PlanObjectKey,
        capacity_bytes: u64,
        alignment: u32,
        block_capacity: usize,
        accounting: Arc<RetainedPayloadAccounting>,
    ) -> MemoryRuntimeResult<Rc<Self>> {
        let mut blocks = Vec::new();
        blocks.try_reserve_exact(block_capacity).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: Some(object.object()),
                requested: block_capacity as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        accounting.add(capacity_bytes)?;
        Ok(Rc::new(Self {
            object,
            capacity_bytes,
            block_capacity,
            alignment,
            accepting_allocations: Cell::new(true),
            blocks: RefCell::new(blocks),
            accounting,
            accounted_bytes: capacity_bytes,
        }))
    }

    pub(crate) fn revoke(&self) {
        self.accepting_allocations.set(false);
    }

    pub(crate) fn allocated_bytes(&self) -> MemoryRuntimeResult<u64> {
        self.blocks.borrow().iter().try_fold(0_u64, |total, block| {
            total.checked_add(block.layout.size() as u64).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "allocated payload bytes",
                    current: total,
                    change: block.layout.size() as u64,
                },
            )
        })
    }

    pub(crate) fn max_alignment(&self) -> u32 {
        self.blocks
            .borrow()
            .iter()
            .map(|block| u32::try_from(block.layout.align()).unwrap_or(u32::MAX))
            .max()
            .unwrap_or(1)
    }

    fn check_layout(&self, layout: Layout) -> MemoryRuntimeResult<()> {
        if !self.accepting_allocations.get() {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let live = self.allocated_bytes()?;
        let requested = live.checked_add(layout.size() as u64).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "requested payload bytes",
                current: live,
                change: layout.size() as u64,
            },
        )?;
        if requested > self.capacity_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: self.object.object(),
                requested,
                capacity: self.capacity_bytes,
            });
        }
        if layout.align() > self.alignment as usize {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(self.object.object()),
                size: layout.size() as u64,
                alignment: u32::try_from(layout.align()).unwrap_or(u32::MAX),
                reason: "payload alignment exceeds its planned envelope",
            });
        }
        if self.blocks.borrow().len() >= self.block_capacity {
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: Some(self.object.object()),
                requested: layout.size() as u64,
            });
        }
        Ok(())
    }
}

impl Drop for PayloadEnvelopeOwner {
    fn drop(&mut self) {
        self.accounting.subtract(self.accounted_bytes);
    }
}

#[derive(Debug, Default)]
pub(crate) struct RetainedPayloadAccounting {
    bytes: AtomicU64,
}

impl RetainedPayloadAccounting {
    pub(crate) fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Acquire)
    }

    fn add(&self, bytes: u64) -> MemoryRuntimeResult<()> {
        self.bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(bytes)
            })
            .map(|_| ())
            .map_err(|_| MemoryRuntimeError::IdentityExhausted {
                identity: "retained payload charge",
            })
    }

    fn subtract(&self, bytes: u64) {
        let _ = self
            .bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_sub(bytes)
            });
    }
}

#[derive(Debug)]
struct RetainedPayloadCharge {
    accounting: Arc<RetainedPayloadAccounting>,
    bytes: u64,
}

impl Drop for RetainedPayloadCharge {
    fn drop(&mut self) {
        self.accounting.subtract(self.bytes);
    }
}

/// Shared immutable accounting ownership that can outlive its originating
/// owner-thread domain without retaining that domain or any runtime object.
#[derive(Clone, Debug)]
pub struct RetainedPayloadTicket {
    charge: Arc<RetainedPayloadCharge>,
}

impl RetainedPayloadTicket {
    pub fn bytes(&self) -> u64 {
        self.charge.bytes
    }
}

impl Drop for PayloadBlockRecord {
    fn drop(&mut self) {
        if self.layout.size() == 0 {
            return;
        }
        // SAFETY: the record exclusively owns the exact pointer/Layout pair
        // returned by Global until it is dropped.
        unsafe { Global.deallocate(self.pointer, self.layout) };
    }
}

struct PlannedAllocationAuthority {
    owner: Rc<PayloadEnvelopeOwner>,
    object: PlanObjectKey,
    // Initialization notification is revocable metadata, never byte
    // ownership. Allocators and live payloads do not retain a mutable domain.
    initialization: Weak<RefCell<super::DomainState>>,
}

/// Sealed allocator backed by one realized indirect-payload envelope.
///
/// Public callers cannot construct this value. Clones share the same finite
/// authority and never become the process global allocator.
#[derive(Clone)]
pub struct PlannedAllocator {
    authority: Rc<PlannedAllocationAuthority>,
}

/// A checked, charged payload reservation held across canonical construction.
/// Dropping it before completion releases the retained charge automatically;
/// initialization is recorded only after a valid frozen value exists.
pub(crate) struct PreparedFrozenSnapshotAdmission {
    allocator: PlannedAllocator,
    ticket: Option<RetainedPayloadTicket>,
    retained_bytes: u64,
    retained_nodes: u64,
}

impl PreparedFrozenSnapshotAdmission {
    pub(crate) fn begin_construction(
        self,
        finalization_bytes: u64,
        temporary_bytes: u64,
    ) -> MemoryRuntimeResult<FrozenSnapshotConstruction> {
        if finalization_bytes > temporary_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: self.allocator.authority.object.object(),
                requested: finalization_bytes,
                capacity: temporary_bytes,
            });
        }
        Ok(FrozenSnapshotConstruction {
            admission: Some(self),
            remaining_temporary_bytes: temporary_bytes - finalization_bytes,
            charged_temporary_bytes: finalization_bytes,
        })
    }
}

/// Sealed call-bound authority for constructing one immutable canonical root.
///
/// The retained candidate and its finalization workspace are admitted before
/// this value can be obtained. Maintained builders request any additional
/// draft/container storage through the fallible helpers below, which debit the
/// remaining R5 scratch authority before asking the global allocator for it.
/// Dropping this value before `complete` releases the retained candidate charge.
pub struct FrozenSnapshotConstruction {
    admission: Option<PreparedFrozenSnapshotAdmission>,
    remaining_temporary_bytes: u64,
    charged_temporary_bytes: u64,
}

impl FrozenSnapshotConstruction {
    pub fn remaining_temporary_bytes(&self) -> u64 {
        self.remaining_temporary_bytes
    }

    fn object(&self) -> crate::MemoryObjectId {
        self.admission
            .as_ref()
            .expect("live construction retains its admission")
            .allocator
            .authority
            .object
            .object()
    }

    fn charge_temporary(&mut self, bytes: u64) -> MemoryRuntimeResult<()> {
        if bytes > self.remaining_temporary_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: self.object(),
                requested: self
                    .charged_temporary_bytes
                    .checked_add(bytes)
                    .unwrap_or(u64::MAX),
                capacity: self
                    .charged_temporary_bytes
                    .checked_add(self.remaining_temporary_bytes)
                    .unwrap_or(u64::MAX),
            });
        }
        let admission = self
            .admission
            .as_ref()
            .expect("live construction retains its admission");
        if let Some(domain) = admission.allocator.authority.initialization.upgrade() {
            domain.borrow_mut().check_failure_injection(
                super::MemoryFailurePoint::HostAllocation,
                bytes,
                1,
                crate::MemorySpace::Host,
            )?;
        } else {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        self.remaining_temporary_bytes -= bytes;
        self.charged_temporary_bytes = self.charged_temporary_bytes.checked_add(bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "canonical construction temporary bytes",
                current: self.charged_temporary_bytes,
                change: bytes,
            },
        )?;
        Ok(())
    }

    /// Allocates an exact-capacity temporary vector after charging its full
    /// element storage against the call's prepared construction workspace.
    pub fn try_vec_with_capacity<T>(&mut self, count: usize) -> MemoryRuntimeResult<Vec<T>> {
        let bytes = u64::try_from(core::mem::size_of::<T>().checked_mul(count).ok_or(
            MemoryRuntimeError::InvalidLayout {
                object: Some(self.object()),
                size: u64::MAX,
                alignment: u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX),
                reason: "canonical temporary vector layout overflows",
            },
        )?)
        .map_err(|_| MemoryRuntimeError::InvalidLayout {
            object: Some(self.object()),
            size: u64::MAX,
            alignment: u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX),
            reason: "canonical temporary vector byte count exceeds u64",
        })?;
        self.charge_temporary(bytes)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: Some(self.object()),
                requested: bytes,
                alignment: u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX),
                space: crate::MemorySpace::Host,
            })?;
        Ok(values)
    }

    /// Builds one exact-capacity String temporary after charging the bytes
    /// before allocation. This is the maintained String concatenation path;
    /// final immutable storage is covered separately by the retained ticket.
    pub fn try_concatenate_string(
        &mut self,
        left: &str,
        right: &str,
    ) -> MemoryRuntimeResult<String> {
        let capacity =
            left.len()
                .checked_add(right.len())
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(self.object()),
                    size: u64::MAX,
                    alignment: 1,
                    reason: "canonical String length overflows",
                })?;
        self.charge_temporary(u64::try_from(capacity).unwrap_or(u64::MAX))?;
        let mut value = String::new();
        value
            .try_reserve_exact(capacity)
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: Some(self.object()),
                requested: u64::try_from(capacity).unwrap_or(u64::MAX),
                alignment: 1,
                space: crate::MemorySpace::Host,
            })?;
        value.push_str(left);
        value.push_str(right);
        Ok(value)
    }

    pub(crate) fn complete(
        mut self,
        actual_retained_bytes: u64,
        actual_retained_nodes: u64,
    ) -> MemoryRuntimeResult<RetainedPayloadTicket> {
        let mut admission = self
            .admission
            .take()
            .expect("live construction retains its admission");
        if actual_retained_bytes > admission.retained_bytes
            || actual_retained_nodes > admission.retained_nodes
        {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: admission.allocator.authority.object.object(),
                requested: actual_retained_bytes.max(actual_retained_nodes),
                capacity: admission.retained_bytes.max(admission.retained_nodes),
            });
        }
        admission
            .allocator
            .record_initialized(actual_retained_bytes)?;
        let mut ticket = admission
            .ticket
            .take()
            .expect("prepared frozen snapshot retains its charge until completion");
        let unused = admission.retained_bytes - actual_retained_bytes;
        if unused != 0 {
            admission
                .allocator
                .authority
                .owner
                .accounting
                .subtract(unused);
            let charge = Arc::get_mut(&mut ticket.charge)
                .expect("prepared retained ticket cannot be shared before completion");
            charge.bytes = actual_retained_bytes;
        }
        Ok(ticket)
    }
}

impl PlannedAllocator {
    fn check_layout(&self, layout: Layout) -> MemoryRuntimeResult<()> {
        self.authority.owner.check_layout(layout)
    }

    fn allocate_checked(&self, layout: Layout, zeroed: bool) -> MemoryRuntimeResult<NonNull<[u8]>> {
        self.check_layout(layout)?;
        let allocation = if zeroed {
            Global.allocate_zeroed(layout)
        } else {
            Global.allocate(layout)
        }
        .map_err(|_| MemoryRuntimeError::AllocationFailed {
            object: Some(self.authority.object.object()),
            requested: layout.size() as u64,
            alignment: u32::try_from(layout.align()).unwrap_or(u32::MAX),
            space: crate::MemorySpace::Host,
        })?;
        let pointer = allocation.cast::<u8>();
        let mut blocks = self.authority.owner.blocks.borrow_mut();
        if blocks.len() >= self.authority.owner.block_capacity {
            // SAFETY: Global returned this pointer for this exact layout and
            // ownership has not escaped this function.
            unsafe { Global.deallocate(pointer, layout) };
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: Some(self.authority.object.object()),
                requested: layout.size() as u64,
            });
        }
        blocks.push(PayloadBlockRecord { pointer, layout });
        Ok(allocation)
    }

    pub fn capacity_bytes(&self) -> MemoryRuntimeResult<u64> {
        Ok(self.authority.owner.capacity_bytes)
    }

    pub fn allocated_bytes(&self) -> MemoryRuntimeResult<u64> {
        self.authority.owner.allocated_bytes()
    }

    /// Admits the immutable canonical tree that will own its concrete Rust
    /// allocations after publication. The payload envelope remains reusable;
    /// the returned pointer-free ticket independently retains the exact
    /// exported charge for as long as the frozen root is alive.
    pub(crate) fn prepare_frozen_snapshot(
        &self,
        retained_bytes: u64,
        retained_nodes: u64,
    ) -> MemoryRuntimeResult<PreparedFrozenSnapshotAdmission> {
        if !self.authority.owner.accepting_allocations.get() {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if retained_bytes > self.authority.owner.capacity_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: self.authority.object.object(),
                requested: retained_bytes,
                capacity: self.authority.owner.capacity_bytes,
            });
        }
        let node_capacity = u64::try_from(self.authority.owner.block_capacity).unwrap_or(u64::MAX);
        if retained_nodes > node_capacity {
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: Some(self.authority.object.object()),
                requested: retained_nodes,
            });
        }
        self.authority.owner.accounting.add(retained_bytes)?;
        let ticket = RetainedPayloadTicket {
            charge: Arc::new(RetainedPayloadCharge {
                accounting: self.authority.owner.accounting.clone(),
                bytes: retained_bytes,
            }),
        };
        Ok(PreparedFrozenSnapshotAdmission {
            allocator: self.clone(),
            ticket: Some(ticket),
            retained_bytes,
            retained_nodes,
        })
    }

    /// Adoption boundary for an immutable value that already exists outside
    /// maintained Mech construction. Internal kernels use
    /// `prepare_frozen_snapshot` before they allocate their candidate.
    pub(crate) fn admit_frozen_snapshot(
        &self,
        retained_bytes: u64,
        retained_nodes: u64,
    ) -> MemoryRuntimeResult<RetainedPayloadTicket> {
        self.prepare_frozen_snapshot(retained_bytes, retained_nodes)?
            .begin_construction(0, 0)?
            .complete(retained_bytes, retained_nodes)
    }

    fn record_initialized(&self, bytes: u64) -> MemoryRuntimeResult<()> {
        let domain = self
            .authority
            .initialization
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        let mut state = domain.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let region = state.regions.get_mut(&self.authority.object).ok_or(
            MemoryRuntimeError::UnknownPlanObject {
                key: self.authority.object,
            },
        )?;
        region.initialization.mark_range(0, bytes)?;
        region.initialized_bytes = region.initialized_bytes.max(bytes);
        Ok(())
    }
}

unsafe impl Allocator for PlannedAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.allocate_checked(layout, false).map_err(|_| AllocError)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.allocate_checked(layout, true).map_err(|_| AllocError)
    }

    unsafe fn deallocate(&self, pointer: NonNull<u8>, layout: Layout) {
        let mut blocks = self.authority.owner.blocks.borrow_mut();
        let index = blocks
            .iter()
            .position(|block| block.pointer == pointer && block.layout == layout)
            .expect("planned allocator deallocation must name a live owned block");
        drop(blocks.swap_remove(index));
    }
}

impl MemoryDomain {
    pub fn planned_allocator(
        &self,
        realized: &RealizedMemoryPlan,
        object: PlanObjectKey,
    ) -> MemoryRuntimeResult<PlannedAllocator> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        let binding = realized.binding(object)?;
        let RuntimeBinding::ManagedCanonicalPayload { handle, .. } = binding else {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(object.object()),
                size: binding.capacity_bytes(),
                alignment: 1,
                reason: "planned allocator requires an indirect payload envelope",
            });
        };
        let state = self.state.borrow();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let record = state.record(handle)?;
        let owner =
            record
                .payload_owner
                .as_ref()
                .cloned()
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(object.object()),
                    size: binding.capacity_bytes(),
                    alignment: 1,
                    reason: "payload envelope has no independent owner",
                })?;
        Ok(PlannedAllocator {
            authority: Rc::new(PlannedAllocationAuthority {
                owner,
                object,
                initialization: Rc::downgrade(&self.state),
            }),
        })
    }
}

/// Fallible exact-capacity sequence whose allocator cannot escape through its
/// mutation API.
pub struct ManagedSequence<T: Copy> {
    values: Box<[T], PlannedAllocator>,
}

impl<T: Copy> ManagedSequence<T> {
    pub fn try_from_slice(allocator: PlannedAllocator, values: &[T]) -> MemoryRuntimeResult<Self> {
        let layout =
            Layout::array::<T>(values.len()).map_err(|_| MemoryRuntimeError::InvalidLayout {
                object: Some(allocator.authority.object.object()),
                size: u64::MAX,
                alignment: u32::try_from(core::mem::align_of::<T>()).unwrap_or(u32::MAX),
                reason: "managed sequence layout overflows",
            })?;
        allocator.check_layout(layout)?;
        let mut storage: Box<[MaybeUninit<T>], PlannedAllocator> =
            Box::try_new_uninit_slice_in(values.len(), allocator.clone()).map_err(|_| {
                MemoryRuntimeError::AllocationFailed {
                    object: Some(allocator.authority.object.object()),
                    requested: layout.size() as u64,
                    alignment: u32::try_from(layout.align()).unwrap_or(u32::MAX),
                    space: crate::MemorySpace::Host,
                }
            })?;
        for (destination, value) in storage.iter_mut().zip(values) {
            destination.write(*value);
        }
        // SAFETY: every element was initialized exactly once above; T is Copy
        // and therefore requires no partial-construction unwind bookkeeping.
        let values = unsafe { storage.assume_init() };
        allocator.record_initialized(layout.size() as u64)?;
        Ok(Self { values })
    }

    pub fn as_slice(&self) -> &[T] {
        &self.values
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// UTF-8 payload stored in an exact reservation-backed owned block.
pub struct ManagedString {
    bytes: ManagedSequence<u8>,
}

impl ManagedString {
    pub fn try_new(allocator: PlannedAllocator, value: &str) -> MemoryRuntimeResult<Self> {
        Ok(Self {
            bytes: ManagedSequence::try_from_slice(allocator, value.as_bytes())?,
        })
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(self.bytes.as_slice())
            .expect("ManagedString is constructed only from validated UTF-8")
    }
}
