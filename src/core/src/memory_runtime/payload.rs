//! Reservation-backed canonical payload ownership.

#[cfg(feature = "no_std")]
use alloc::{
    alloc::{AllocError, Allocator, Global, Layout},
    boxed::Box,
    rc::{Rc, Weak},
    sync::Arc,
};
#[cfg(not(feature = "no_std"))]
use std::{
    alloc::{AllocError, Allocator, Global, Layout},
    boxed::Box,
    rc::{Rc, Weak},
    sync::Arc,
};

use core::{
    cell::RefCell,
    mem::MaybeUninit,
    ptr::NonNull,
    str,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{
    AllocationHandle, DomainState, MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult,
    PlanObjectKey, RealizedMemoryPlan, RuntimeBinding,
};

pub(crate) struct PayloadBlockRecord {
    pub pointer: NonNull<u8>,
    pub layout: Layout,
}

#[derive(Default)]
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
#[derive(Clone)]
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
    domain: Weak<RefCell<DomainState>>,
    handle: AllocationHandle,
    object: PlanObjectKey,
    realized: RealizedMemoryPlan,
}

impl Drop for PlannedAllocationAuthority {
    fn drop(&mut self) {
        let Some(domain) = self.domain.upgrade() else {
            return;
        };
        let Ok(mut state) = domain.try_borrow_mut() else {
            return;
        };
        let Ok(record) = state.record_mut(self.handle) else {
            return;
        };
        if let Some(next) = record.payload_owner_pins.checked_sub(1) {
            record.payload_owner_pins = next;
        }
    }
}

/// Sealed allocator backed by one realized indirect-payload envelope.
///
/// Public callers cannot construct this value. Clones share the same finite
/// authority and never become the process global allocator.
#[derive(Clone)]
pub struct PlannedAllocator {
    authority: Rc<PlannedAllocationAuthority>,
}

impl PlannedAllocator {
    fn check_layout(&self, layout: Layout) -> MemoryRuntimeResult<()> {
        let domain = self
            .authority
            .domain
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        let state = domain.borrow();
        let record = state.record(self.authority.handle)?;
        let live = record
            .payload_blocks
            .iter()
            .try_fold(0_u64, |total, block| {
                total.checked_add(block.layout.size() as u64).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "live payload bytes",
                        current: total,
                        change: block.layout.size() as u64,
                    },
                )
            })?;
        let requested = live.checked_add(layout.size() as u64).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "requested payload bytes",
                current: live,
                change: layout.size() as u64,
            },
        )?;
        if requested > record.capacity_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: self.authority.object.object(),
                requested,
                capacity: record.capacity_bytes,
            });
        }
        if layout.align() > record.alignment as usize {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(self.authority.object.object()),
                size: layout.size() as u64,
                alignment: u32::try_from(layout.align()).unwrap_or(u32::MAX),
                reason: "payload alignment exceeds its planned envelope",
            });
        }
        if record.payload_blocks.len() == record.payload_blocks.capacity() {
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: Some(self.authority.object.object()),
                requested: layout.size() as u64,
            });
        }
        Ok(())
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
        let domain = self
            .authority
            .domain
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        let mut state = domain.borrow_mut();
        let record = state.record_mut(self.authority.handle)?;
        if record.payload_blocks.len() == record.payload_blocks.capacity() {
            // SAFETY: Global returned this pointer for this exact layout and
            // ownership has not escaped this function.
            unsafe { Global.deallocate(pointer, layout) };
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: Some(self.authority.object.object()),
                requested: layout.size() as u64,
            });
        }
        record
            .payload_blocks
            .push(PayloadBlockRecord { pointer, layout });
        Ok(allocation)
    }

    pub fn capacity_bytes(&self) -> MemoryRuntimeResult<u64> {
        let domain = self
            .authority
            .domain
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        Ok(domain
            .borrow()
            .record(self.authority.handle)?
            .capacity_bytes)
    }

    pub fn allocated_bytes(&self) -> MemoryRuntimeResult<u64> {
        let domain = self
            .authority
            .domain
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        domain
            .borrow()
            .record(self.authority.handle)?
            .payload_blocks
            .iter()
            .try_fold(0_u64, |total, block| {
                total.checked_add(block.layout.size() as u64).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "allocated payload bytes",
                        current: total,
                        change: block.layout.size() as u64,
                    },
                )
            })
    }

    fn record_initialized(&self, bytes: u64) -> MemoryRuntimeResult<()> {
        self.authority
            .realized
            .record_initialized(self.authority.object, bytes)
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
        let Some(domain) = self.authority.domain.upgrade() else {
            return;
        };
        let Ok(mut state) = domain.try_borrow_mut() else {
            return;
        };
        let Ok(record) = state.record_mut(self.authority.handle) else {
            return;
        };
        let Some(index) = record
            .payload_blocks
            .iter()
            .position(|block| block.pointer == pointer && block.layout == layout)
        else {
            return;
        };
        drop(record.payload_blocks.swap_remove(index));
    }
}

impl MemoryDomain {
    pub fn retain_payload_charge(&self, bytes: u64) -> MemoryRuntimeResult<RetainedPayloadTicket> {
        let accounting = self.state.borrow().payload_accounting.clone();
        accounting.add(bytes)?;
        Ok(RetainedPayloadTicket {
            charge: Arc::new(RetainedPayloadCharge { accounting, bytes }),
        })
    }

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
        let mut state = self.state.borrow_mut();
        let record = state.record_mut(handle)?;
        record.payload_owner_pins = record.payload_owner_pins.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "payload owner pin count",
            },
        )?;
        Ok(PlannedAllocator {
            authority: Rc::new(PlannedAllocationAuthority {
                domain: Rc::downgrade(&self.state),
                handle,
                object,
                realized: realized.clone(),
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
