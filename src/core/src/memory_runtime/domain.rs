use crate::{
    AllocationPlan, ArenaBackingKind, ArenaPlan, MemoryArenaId, MemoryBudgetLimits,
    MemoryBudgetViolation, MemoryLifetime, MemoryObjectId, MemoryPlanPoint, MemorySpace,
    ResourceDemand, ReuseGroupId,
};

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
    sync::Arc,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
    sync::Arc,
    vec::Vec,
};

use core::cell::{Cell, RefCell};

use super::{
    AllocationHandle, HostBlock, MemoryDomainId, MemoryPlanRevision, MemoryRuntimeError,
    MemoryRuntimeResult, PlanObjectKey, PublishedValueVersion, RegionIncarnation,
};

/// Borrowed adapter over one existing R5 plan.
#[derive(Clone, Copy, Debug)]
pub struct RuntimePlanView<'a> {
    revision: MemoryPlanRevision,
    allocations: &'a [AllocationPlan],
    arenas: &'a [ArenaPlan],
    admitted_demand: ResourceDemand,
    limits: MemoryBudgetLimits,
    violations: &'a [MemoryBudgetViolation],
}

impl<'a> RuntimePlanView<'a> {
    pub fn new(
        revision: MemoryPlanRevision,
        allocations: &'a [AllocationPlan],
        arenas: &'a [ArenaPlan],
        admitted_demand: ResourceDemand,
        limits: MemoryBudgetLimits,
        violations: &'a [MemoryBudgetViolation],
    ) -> Self {
        Self {
            revision,
            allocations,
            arenas,
            admitted_demand,
            limits,
            violations,
        }
    }

    pub const fn revision(self) -> MemoryPlanRevision {
        self.revision
    }

    pub const fn admitted_demand(self) -> ResourceDemand {
        self.admitted_demand
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnedAllocationState {
    Reserved,
    Initialized,
    Live,
    Retired,
    Free,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeBinding {
    Empty {
        space: MemorySpace,
        alignment: u32,
        incarnation: RegionIncarnation,
    },
    ManagedHostRegion {
        handle: AllocationHandle,
        offset_bytes: u64,
        capacity_bytes: u64,
        required_initialization_bytes: u64,
        initialized_bytes: u64,
        incarnation: RegionIncarnation,
    },
    ManagedCanonicalPayload {
        handle: AllocationHandle,
        capacity_bytes: u64,
        required_initialization_bytes: u64,
        initialized_bytes: u64,
        incarnation: RegionIncarnation,
    },
    Device {
        handle: AllocationHandle,
        offset_bytes: u64,
        capacity_bytes: u64,
        required_initialization_bytes: u64,
        initialized_bytes: u64,
        incarnation: RegionIncarnation,
    },
    PinnedExternal {
        capacity_bytes: u64,
        incarnation: RegionIncarnation,
    },
}

impl RuntimeBinding {
    pub const fn handle(&self) -> Option<AllocationHandle> {
        match self {
            Self::ManagedHostRegion { handle, .. }
            | Self::ManagedCanonicalPayload { handle, .. }
            | Self::Device { handle, .. } => Some(*handle),
            Self::Empty { .. } | Self::PinnedExternal { .. } => None,
        }
    }

    pub const fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::ManagedHostRegion { capacity_bytes, .. }
            | Self::ManagedCanonicalPayload { capacity_bytes, .. }
            | Self::Device { capacity_bytes, .. }
            | Self::PinnedExternal { capacity_bytes, .. } => *capacity_bytes,
        }
    }

    pub const fn initialized_bytes(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::ManagedHostRegion {
                initialized_bytes, ..
            }
            | Self::ManagedCanonicalPayload {
                initialized_bytes, ..
            }
            | Self::Device {
                initialized_bytes, ..
            } => *initialized_bytes,
            Self::PinnedExternal { capacity_bytes, .. } => *capacity_bytes,
        }
    }

    pub const fn required_initialization_bytes(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::ManagedHostRegion {
                required_initialization_bytes,
                ..
            }
            | Self::ManagedCanonicalPayload {
                required_initialization_bytes,
                ..
            }
            | Self::Device {
                required_initialization_bytes,
                ..
            } => *required_initialization_bytes,
            Self::PinnedExternal { capacity_bytes, .. } => *capacity_bytes,
        }
    }

    pub const fn incarnation(&self) -> RegionIncarnation {
        match self {
            Self::Empty { incarnation, .. }
            | Self::ManagedHostRegion { incarnation, .. }
            | Self::ManagedCanonicalPayload { incarnation, .. }
            | Self::Device { incarnation, .. }
            | Self::PinnedExternal { incarnation, .. } => *incarnation,
        }
    }

    fn set_incarnation(&mut self, next: RegionIncarnation) {
        match self {
            Self::Empty { incarnation, .. }
            | Self::ManagedHostRegion { incarnation, .. }
            | Self::ManagedCanonicalPayload { incarnation, .. }
            | Self::Device { incarnation, .. }
            | Self::PinnedExternal { incarnation, .. } => *incarnation = next,
        }
    }

    fn set_initialized_bytes(&mut self, next: u64) {
        match self {
            Self::ManagedHostRegion {
                initialized_bytes, ..
            }
            | Self::ManagedCanonicalPayload {
                initialized_bytes, ..
            }
            | Self::Device {
                initialized_bytes, ..
            } => *initialized_bytes = next,
            Self::Empty { .. } | Self::PinnedExternal { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryLedgerSnapshot {
    pub reserved_bytes: u64,
    pub committed_bytes: u64,
    pub retired_pinned_bytes: u64,
    pub exported_snapshot_bytes: u64,
    pub in_flight_device_bytes: u64,
    pub in_flight_transfer_bytes: u64,
    pub active_reservations: u32,
    pub live_allocations: u32,
    pub retired_allocations: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedAllocationObservation {
    pub handle: AllocationHandle,
    pub state: OwnedAllocationState,
    pub capacity_bytes: u64,
    pub actual_block_bytes: u64,
    pub alignment: u32,
    pub actual_block_alignment: u32,
    pub space: MemorySpace,
    pub active_leases: u32,
    pub snapshot_pins: u32,
    pub submission_pins: u32,
    pub payload_owner_pins: u32,
    pub device_owner_pins: u32,
    pub device_lost: bool,
}

#[derive(Clone, Debug)]
struct ObjectAuthorization {
    key: PlanObjectKey,
    arena: MemoryArenaId,
    space: MemorySpace,
    current_bytes: u64,
    capacity_bytes: u64,
    alignment: u32,
    offset: u64,
    lifetime: MemoryLifetime,
    reuse_group: Option<ReuseGroupId>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeRegionRecord {
    pub(crate) handle: Option<AllocationHandle>,
    lifetime: MemoryLifetime,
    reuse_group: Option<ReuseGroupId>,
    pub(crate) incarnation: RegionIncarnation,
    pub(crate) initialized_bytes: u64,
}

#[derive(Clone, Debug)]
struct ArenaAuthorization {
    id: MemoryArenaId,
    space: MemorySpace,
    backing: ArenaBackingKind,
    alignment: u32,
    capacity_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlanRevisionLifecycle {
    Candidate,
    Admitted,
    Realized,
    Active,
    Retired,
}

/// Non-forgeable authority to realize exactly one validated R5 plan view.
pub struct MemoryReservation {
    domain: Rc<RefCell<DomainState>>,
    domain_id: MemoryDomainId,
    revision: MemoryPlanRevision,
    admission: u64,
    bytes: u64,
    active: bool,
    arenas: Box<[ArenaAuthorization]>,
    objects: Box<[ObjectAuthorization]>,
}

impl MemoryReservation {
    pub const fn revision(&self) -> MemoryPlanRevision {
        self.revision
    }

    pub const fn reserved_bytes(&self) -> u64 {
        self.bytes
    }

    fn release(&mut self) {
        if !self.active {
            return;
        }
        let mut state = self.domain.borrow_mut();
        let result = state.release_reservation(self.bytes);
        debug_assert!(result.is_ok());
        state.admissions.remove(&self.admission);
        if state.revisions.get(&self.revision) == Some(&PlanRevisionLifecycle::Admitted) {
            state
                .revisions
                .insert(self.revision, PlanRevisionLifecycle::Retired);
        }
        self.active = false;
        self.bytes = 0;
    }
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        self.release();
    }
}

/// RAII authority for one executable point in a realized memory plan.
///
/// Program- and activation-lifetime objects are always eligible for access.
/// Turn-, transaction-, and transfer-lifetime objects can only be leased while
/// a scope covering their declared interval is active.
pub struct MemoryPlanScope {
    domain: Weak<RefCell<DomainState>>,
    active_point: Rc<Cell<Option<MemoryPlanPoint>>>,
    cleanup_pending: Rc<Cell<bool>>,
    point: MemoryPlanPoint,
    active: bool,
}

impl MemoryPlanScope {
    pub const fn point(&self) -> MemoryPlanPoint {
        self.point
    }
}

impl Drop for MemoryPlanScope {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if self.active_point.get() == Some(self.point) {
            self.active_point.set(None);
            self.cleanup_pending.set(true);
            if let Some(domain) = self.domain.upgrade()
                && let Ok(mut state) = domain.try_borrow_mut()
            {
                state.finish_plan_scope_cleanup();
            }
        }
        self.active = false;
    }
}

/// Owner-thread registration tying one backend device allocation to its
/// admitted domain record. The backend owns the actual buffer; this token
/// prevents the domain record from being reclaimed before that buffer drops.
pub struct DeviceAllocationOwner {
    domain: Rc<RefCell<DomainState>>,
    handle: AllocationHandle,
    active: bool,
}

impl DeviceAllocationOwner {
    pub const fn handle(&self) -> AllocationHandle {
        self.handle
    }

    pub fn mark_lost(&self) -> MemoryRuntimeResult<()> {
        let mut state = self.domain.borrow_mut();
        let record = state.record_mut(self.handle)?;
        record.device_lost = true;
        Ok(())
    }

    fn release(&mut self) -> MemoryRuntimeResult<()> {
        if !self.active {
            return Ok(());
        }
        let mut state = self.domain.borrow_mut();
        let record = state.record_mut(self.handle)?;
        let next_pins = record.device_owner_pins.checked_sub(1).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "device owner pins",
                current: u64::from(record.device_owner_pins),
                change: 1,
            },
        )?;
        record.device_owner_pins = next_pins;
        if next_pins == 0 {
            record.device_actual_bytes = 0;
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for DeviceAllocationOwner {
    fn drop(&mut self) {
        let result = self.release();
        debug_assert!(result.is_ok());
    }
}

/// Pins every allocation participating in one submitted device operation.
/// Completion is observed by the backend and released on the owner thread.
pub struct DeviceSubmissionHold {
    domain: Rc<RefCell<DomainState>>,
    handles: Arc<[AllocationHandle]>,
    device_bytes: u64,
    transfer_bytes: u64,
    active: bool,
}

/// Prevalidated, allocation-free submission metadata for one fixed backend
/// operation. The plan revision and physical handles remain process-local.
pub struct PreparedDeviceSubmission {
    revision: MemoryPlanRevision,
    handles: Arc<[AllocationHandle]>,
    device_handles: Arc<[AllocationHandle]>,
    device_bytes: u64,
    transfer_bytes: u64,
}

impl DeviceSubmissionHold {
    pub fn complete(mut self) -> MemoryRuntimeResult<()> {
        self.release()
    }

    fn release(&mut self) -> MemoryRuntimeResult<()> {
        if !self.active {
            return Ok(());
        }
        let mut state = self.domain.borrow_mut();
        for handle in self.handles.iter().copied() {
            let record = state.record(handle)?;
            if record.submission_pins == 0 {
                return Err(MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "device submission pins",
                    current: 0,
                    change: 1,
                });
            }
        }
        let next_device_bytes = state
            .ledger
            .in_flight_device_bytes
            .checked_sub(self.device_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "in-flight device bytes",
                current: state.ledger.in_flight_device_bytes,
                change: self.device_bytes,
            })?;
        let next_transfer_bytes = state
            .ledger
            .in_flight_transfer_bytes
            .checked_sub(self.transfer_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "in-flight transfer bytes",
                current: state.ledger.in_flight_transfer_bytes,
                change: self.transfer_bytes,
            })?;
        for handle in self.handles.iter().copied() {
            state.record_mut(handle)?.submission_pins -= 1;
        }
        state.ledger.in_flight_device_bytes = next_device_bytes;
        state.ledger.in_flight_transfer_bytes = next_transfer_bytes;
        self.active = false;
        Ok(())
    }
}

impl Drop for DeviceSubmissionHold {
    fn drop(&mut self) {
        let result = self.release();
        debug_assert!(result.is_ok());
    }
}

/// Receipt mapping every R5 plan object to its current live runtime binding.
#[derive(Clone)]
pub struct RealizedMemoryPlan {
    domain: MemoryDomainId,
    revision: MemoryPlanRevision,
    domain_state: Weak<RefCell<DomainState>>,
    bindings: Rc<RefCell<Box<[(PlanObjectKey, RuntimeBinding)]>>>,
    lifetimes: Rc<Box<[(PlanObjectKey, MemoryLifetime)]>>,
}

impl RealizedMemoryPlan {
    pub const fn domain(&self) -> MemoryDomainId {
        self.domain
    }

    pub const fn revision(&self) -> MemoryPlanRevision {
        self.revision
    }

    pub fn binding(&self, key: PlanObjectKey) -> MemoryRuntimeResult<RuntimeBinding> {
        if key.domain() != self.domain {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.domain,
                actual: key.domain(),
            });
        }
        if key.revision() != self.revision {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: self.revision,
                actual: key.revision(),
            });
        }
        let bindings = self.bindings.borrow();
        let mut binding = bindings
            .binary_search_by_key(&key, |(candidate, _)| *candidate)
            .ok()
            .map(|index| bindings[index].1.clone())
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key })?;
        let state = self
            .domain_state
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        let state = state.borrow();
        if state.id != self.domain {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.domain,
                actual: state.id,
            });
        }
        let region = state
            .regions
            .get(&key)
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key })?;
        binding.set_incarnation(region.incarnation);
        binding.set_initialized_bytes(region.initialized_bytes);
        Ok(binding)
    }

    pub fn bindings(&self) -> Box<[(PlanObjectKey, RuntimeBinding)]> {
        self.bindings.borrow().to_vec().into_boxed_slice()
    }

    pub(crate) fn lifetime(&self, key: PlanObjectKey) -> MemoryRuntimeResult<MemoryLifetime> {
        self.lifetimes
            .binary_search_by_key(&key, |(candidate, _)| *candidate)
            .ok()
            .map(|index| self.lifetimes[index].1)
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key })
    }

    pub(crate) fn record_initialized(
        &self,
        key: PlanObjectKey,
        initialized_bytes: u64,
    ) -> MemoryRuntimeResult<()> {
        let mut bindings = self.bindings.borrow_mut();
        let index = bindings
            .binary_search_by_key(&key, |(candidate, _)| *candidate)
            .map_err(|_| MemoryRuntimeError::UnknownPlanObject { key })?;
        let binding = &mut bindings[index].1;
        if initialized_bytes > binding.capacity_bytes() {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: key.object(),
                requested: initialized_bytes,
                capacity: binding.capacity_bytes(),
            });
        }
        match binding {
            RuntimeBinding::ManagedHostRegion {
                initialized_bytes: current,
                ..
            }
            | RuntimeBinding::ManagedCanonicalPayload {
                initialized_bytes: current,
                ..
            }
            | RuntimeBinding::Device {
                initialized_bytes: current,
                ..
            } => *current = (*current).max(initialized_bytes),
            RuntimeBinding::Empty { .. } if initialized_bytes == 0 => {}
            RuntimeBinding::Empty { .. } | RuntimeBinding::PinnedExternal { .. } => {
                return Err(MemoryRuntimeError::UnplannedAllocation {
                    object: Some(key.object()),
                    requested: initialized_bytes,
                });
            }
        }
        let state = self
            .domain_state
            .upgrade()
            .ok_or(MemoryRuntimeError::DomainClosed)?;
        let mut state = state.borrow_mut();
        let region = state
            .regions
            .get_mut(&key)
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key })?;
        region.initialized_bytes = region.initialized_bytes.max(initialized_bytes);
        Ok(())
    }
}

pub(crate) struct ActiveLeaseRecord {
    pub token: u64,
    pub start: u64,
    pub end: u64,
    pub write: bool,
}

pub(crate) struct AllocationRecord {
    pub state: OwnedAllocationState,
    pub block: Option<HostBlock>,
    pub capacity_bytes: u64,
    pub alignment: u32,
    pub space: MemorySpace,
    pub leases: Vec<ActiveLeaseRecord>,
    pub snapshot_pins: u32,
    pub submission_pins: u32,
    pub payload_owner: Option<Rc<super::PayloadEnvelopeOwner>>,
    pub device_owner_pins: u32,
    pub device_actual_bytes: u64,
    pub device_lost: bool,
}

pub(crate) struct AllocationSlot {
    pub generation: u64,
    pub permanently_retired: bool,
    pub record: Option<AllocationRecord>,
}

pub(crate) struct DomainState {
    pub id: MemoryDomainId,
    pub closed: bool,
    pub next_revision: MemoryPlanRevision,
    revisions: BTreeMap<MemoryPlanRevision, PlanRevisionLifecycle>,
    pub(crate) active_revision: Option<MemoryPlanRevision>,
    next_admission: u64,
    admissions: BTreeMap<u64, MemoryPlanRevision>,
    pub next_publication: PublishedValueVersion,
    pub next_lease_token: u64,
    pub ledger: MemoryLedgerSnapshot,
    pub payload_accounting: Arc<super::RetainedPayloadAccounting>,
    pub allocations: Vec<AllocationSlot>,
    pub(crate) regions: BTreeMap<PlanObjectKey, RuntimeRegionRecord>,
    active_reuse_regions: BTreeMap<(MemoryPlanRevision, ReuseGroupId), Option<PlanObjectKey>>,
    pub active_point: Rc<Cell<Option<MemoryPlanPoint>>>,
    scope_cleanup_pending: Rc<Cell<bool>>,
}

impl DomainState {
    fn finish_plan_scope_cleanup(&mut self) {
        if !self.scope_cleanup_pending.get() {
            return;
        }
        for active in self.active_reuse_regions.values_mut() {
            *active = None;
        }
        self.scope_cleanup_pending.set(false);
    }

    fn release_reservation(&mut self, bytes: u64) -> MemoryRuntimeResult<()> {
        self.ledger.reserved_bytes = self.ledger.reserved_bytes.checked_sub(bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "reserved bytes",
                current: self.ledger.reserved_bytes,
                change: bytes,
            },
        )?;
        self.ledger.active_reservations = self.ledger.active_reservations.checked_sub(1).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "active reservations",
                current: u64::from(self.ledger.active_reservations),
                change: 1,
            },
        )?;
        Ok(())
    }

    pub(crate) fn record(
        &self,
        handle: AllocationHandle,
    ) -> MemoryRuntimeResult<&AllocationRecord> {
        self.validate_domain(handle)?;
        let slot = self
            .allocations
            .get(handle.slot() as usize)
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
        if slot.generation != handle.generation() {
            return Err(MemoryRuntimeError::StaleAllocationGeneration {
                handle,
                current: slot.generation,
            });
        }
        slot.record
            .as_ref()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })
    }

    pub(crate) fn record_mut(
        &mut self,
        handle: AllocationHandle,
    ) -> MemoryRuntimeResult<&mut AllocationRecord> {
        self.validate_domain(handle)?;
        let slot = self
            .allocations
            .get_mut(handle.slot() as usize)
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
        if slot.generation != handle.generation() {
            return Err(MemoryRuntimeError::StaleAllocationGeneration {
                handle,
                current: slot.generation,
            });
        }
        slot.record
            .as_mut()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })
    }

    fn validate_domain(&self, handle: AllocationHandle) -> MemoryRuntimeResult<()> {
        if handle.domain() != self.id {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id,
                actual: handle.domain(),
            });
        }
        Ok(())
    }

    fn insert_record(&mut self, record: AllocationRecord) -> MemoryRuntimeResult<AllocationHandle> {
        let candidate = self
            .allocations
            .iter()
            .position(|slot| slot.record.is_none() && !slot.permanently_retired);
        let slot = if let Some(slot) = candidate {
            slot
        } else {
            self.allocations.push(AllocationSlot {
                generation: 1,
                permanently_retired: false,
                record: None,
            });
            self.allocations.len() - 1
        };
        let raw_slot = u32::try_from(slot).map_err(|_| MemoryRuntimeError::IdentityExhausted {
            identity: "allocation slot",
        })?;
        let generation = self.allocations[slot].generation;
        self.allocations[slot].record = Some(record);
        self.ledger.live_allocations = self.ledger.live_allocations.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "live allocation count",
            },
        )?;
        Ok(AllocationHandle::new(self.id, raw_slot, generation))
    }
}

/// Owner-thread-confined managed memory domain.
#[derive(Clone)]
pub struct MemoryDomain {
    pub(crate) state: Rc<RefCell<DomainState>>,
}

impl MemoryDomain {
    pub fn new() -> MemoryRuntimeResult<Self> {
        let id = MemoryDomainId::issue()?;
        Ok(Self {
            state: Rc::new(RefCell::new(DomainState {
                id,
                closed: false,
                next_revision: MemoryPlanRevision::initial(),
                revisions: BTreeMap::new(),
                active_revision: None,
                next_admission: 1,
                admissions: BTreeMap::new(),
                next_publication: PublishedValueVersion::initial(),
                next_lease_token: 1,
                ledger: MemoryLedgerSnapshot::default(),
                payload_accounting: Arc::new(super::RetainedPayloadAccounting::default()),
                allocations: Vec::new(),
                regions: BTreeMap::new(),
                active_reuse_regions: BTreeMap::new(),
                active_point: Rc::new(Cell::new(None)),
                scope_cleanup_pending: Rc::new(Cell::new(false)),
            })),
        })
    }

    pub fn id(&self) -> MemoryDomainId {
        self.state.borrow().id
    }

    pub fn issue_plan_revision(&self) -> MemoryRuntimeResult<MemoryPlanRevision> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let revision = state.next_revision;
        state.next_revision = revision.checked_successor("memory plan revision")?;
        state
            .revisions
            .insert(revision, PlanRevisionLifecycle::Candidate);
        Ok(revision)
    }

    pub fn plan_object_key(
        &self,
        revision: MemoryPlanRevision,
        object: MemoryObjectId,
    ) -> MemoryRuntimeResult<PlanObjectKey> {
        let state = self.state.borrow();
        if !matches!(
            state.revisions.get(&revision),
            Some(PlanRevisionLifecycle::Realized | PlanRevisionLifecycle::Active)
        ) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(state.next_revision),
                actual: revision,
            });
        }
        Ok(PlanObjectKey::new(state.id, revision, object))
    }

    /// Promotes one fully realized candidate to the domain's active revision.
    /// Issuing or rejecting later candidates never changes this authority.
    pub fn activate_realization(&self, realized: &RealizedMemoryPlan) -> MemoryRuntimeResult<()> {
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
        if state.active_point.get().is_some()
            || state.allocations.iter().any(|slot| {
                slot.record
                    .as_ref()
                    .is_some_and(|record| !record.leases.is_empty())
            })
        {
            return Err(MemoryRuntimeError::TurnInFlight);
        }
        if !matches!(
            state.revisions.get(&realized.revision()),
            Some(PlanRevisionLifecycle::Realized | PlanRevisionLifecycle::Active)
        ) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(state.next_revision),
                actual: realized.revision(),
            });
        }
        if let Some(previous) = state.active_revision
            && previous != realized.revision()
            && state.revisions.get(&previous) == Some(&PlanRevisionLifecycle::Active)
        {
            state
                .revisions
                .insert(previous, PlanRevisionLifecycle::Realized);
        }
        state
            .revisions
            .insert(realized.revision(), PlanRevisionLifecycle::Active);
        state.active_revision = Some(realized.revision());
        Ok(())
    }

    pub fn prepare_realization(
        &self,
        view: RuntimePlanView<'_>,
    ) -> MemoryRuntimeResult<MemoryReservation> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if state.revisions.get(&view.revision) != Some(&PlanRevisionLifecycle::Candidate) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(state.next_revision),
                actual: view.revision,
            });
        }
        if let Some(violation) = view.violations.first() {
            return Err(MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: violation.required,
                limit: violation.limit,
            });
        }
        let (arenas, objects, bytes) = validate_plan_view(state.id, view)?;
        let record_count = arenas
            .iter()
            .filter(|arena| arena.backing == ArenaBackingKind::ContiguousBytes)
            .count()
            + objects
                .iter()
                .filter(|object| {
                    arenas.iter().any(|arena| {
                        arena.id == object.arena
                            && arena.backing == ArenaBackingKind::IndirectOwnedPayloads
                    })
                })
                .count();
        state.allocations.try_reserve(record_count).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: u64::try_from(record_count).unwrap_or(u64::MAX),
                alignment: 1,
                space: MemorySpace::Host,
            }
        })?;
        state.ledger.reserved_bytes = state.ledger.reserved_bytes.checked_add(bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "reserved bytes",
                current: state.ledger.reserved_bytes,
                change: bytes,
            },
        )?;
        state.ledger.active_reservations = state.ledger.active_reservations.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "active reservation count",
            },
        )?;
        let admission = state.next_admission;
        state.next_admission =
            admission
                .checked_add(1)
                .ok_or(MemoryRuntimeError::IdentityExhausted {
                    identity: "memory admission",
                })?;
        state.admissions.insert(admission, view.revision);
        state
            .revisions
            .insert(view.revision, PlanRevisionLifecycle::Admitted);
        Ok(MemoryReservation {
            domain: Rc::clone(&self.state),
            domain_id: state.id,
            revision: view.revision,
            admission,
            bytes,
            active: true,
            arenas,
            objects,
        })
    }

    pub fn materialize(
        &self,
        mut reservation: MemoryReservation,
    ) -> MemoryRuntimeResult<RealizedMemoryPlan> {
        if !reservation.active {
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: None,
                requested: 0,
            });
        }
        if reservation.domain_id != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: reservation.domain_id,
            });
        }

        struct PendingArena {
            authorization: ArenaAuthorization,
            block: Option<HostBlock>,
        }
        struct PendingObject {
            authorization: ObjectAuthorization,
            payload_owner: Rc<super::PayloadEnvelopeOwner>,
        }

        let mut contiguous = Vec::new();
        let mut indirect = Vec::new();
        contiguous
            .try_reserve_exact(reservation.arenas.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: reservation.bytes,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        indirect
            .try_reserve_exact(reservation.objects.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: reservation.bytes,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        for arena in reservation.arenas.iter().cloned() {
            match arena.backing {
                ArenaBackingKind::ContiguousBytes => {
                    let block = match arena.space {
                        MemorySpace::Device { .. } => None,
                        MemorySpace::Host | MemorySpace::ResidentCpu => Some(HostBlock::allocate(
                            None,
                            arena.capacity_bytes,
                            arena.alignment,
                            arena.space,
                        )?),
                    };
                    contiguous.push(PendingArena {
                        authorization: arena,
                        block,
                    });
                }
                ArenaBackingKind::IndirectOwnedPayloads => {}
            }
        }
        for object in reservation.objects.iter().cloned() {
            let backing = reservation
                .arenas
                .iter()
                .find(|arena| arena.id == object.arena)
                .map(|arena| arena.backing)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?;
            if backing == ArenaBackingKind::IndirectOwnedPayloads {
                let payload_owner = super::PayloadEnvelopeOwner::new(
                    object.key,
                    object.capacity_bytes,
                    object.alignment,
                    2,
                )?;
                indirect.push(PendingObject {
                    authorization: object,
                    payload_owner,
                });
            }
        }

        let mut bindings = Vec::new();
        bindings
            .try_reserve_exact(reservation.objects.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: reservation.bytes,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if state.admissions.get(&reservation.admission) != Some(&reservation.revision)
            || state.revisions.get(&reservation.revision) != Some(&PlanRevisionLifecycle::Admitted)
        {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(state.next_revision),
                actual: reservation.revision,
            });
        }
        let mut arena_handles = BTreeMap::new();
        for pending in contiguous {
            let capacity = pending.authorization.capacity_bytes;
            let handle = state.insert_record(AllocationRecord {
                state: OwnedAllocationState::Live,
                block: pending.block,
                capacity_bytes: capacity,
                alignment: pending.authorization.alignment,
                space: pending.authorization.space,
                leases: Vec::new(),
                snapshot_pins: 0,
                submission_pins: 0,
                payload_owner: None,
                device_owner_pins: 0,
                device_actual_bytes: 0,
                device_lost: false,
            })?;
            arena_handles.insert(pending.authorization.id, handle);
        }
        let mut indirect_handles = BTreeMap::new();
        for pending in indirect {
            let capacity = pending.authorization.capacity_bytes;
            let handle = state.insert_record(AllocationRecord {
                state: OwnedAllocationState::Live,
                block: None,
                capacity_bytes: capacity,
                alignment: pending.authorization.alignment,
                space: pending.authorization.space,
                leases: Vec::new(),
                snapshot_pins: 0,
                submission_pins: 0,
                payload_owner: Some(pending.payload_owner),
                device_owner_pins: 0,
                device_actual_bytes: 0,
                device_lost: false,
            })?;
            indirect_handles.insert(pending.authorization.key, handle);
        }
        for object in &reservation.objects {
            let arena = reservation
                .arenas
                .iter()
                .find(|arena| arena.id == object.arena)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?;
            let incarnation = RegionIncarnation::initial();
            let binding = if object.capacity_bytes == 0 {
                RuntimeBinding::Empty {
                    space: object.space,
                    alignment: object.alignment,
                    incarnation,
                }
            } else {
                match arena.backing {
                    ArenaBackingKind::ContiguousBytes => {
                        let handle = *arena_handles
                            .get(&object.arena)
                            .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?;
                        match object.space {
                            MemorySpace::Device { .. } => RuntimeBinding::Device {
                                handle,
                                offset_bytes: object.offset,
                                capacity_bytes: object.capacity_bytes,
                                required_initialization_bytes: object.current_bytes,
                                initialized_bytes: 0,
                                incarnation,
                            },
                            MemorySpace::Host | MemorySpace::ResidentCpu => {
                                RuntimeBinding::ManagedHostRegion {
                                    handle,
                                    offset_bytes: object.offset,
                                    capacity_bytes: object.capacity_bytes,
                                    required_initialization_bytes: object.current_bytes,
                                    initialized_bytes: 0,
                                    incarnation,
                                }
                            }
                        }
                    }
                    ArenaBackingKind::IndirectOwnedPayloads => {
                        RuntimeBinding::ManagedCanonicalPayload {
                            handle: *indirect_handles
                                .get(&object.key)
                                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?,
                            capacity_bytes: object.capacity_bytes,
                            required_initialization_bytes: object.current_bytes,
                            initialized_bytes: 0,
                            incarnation,
                        }
                    }
                }
            };
            if state
                .regions
                .insert(
                    object.key,
                    RuntimeRegionRecord {
                        handle: binding.handle(),
                        lifetime: object.lifetime,
                        reuse_group: object.reuse_group,
                        incarnation,
                        initialized_bytes: 0,
                    },
                )
                .is_some()
            {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(object.key.object()),
                    size: object.capacity_bytes,
                    alignment: object.alignment,
                    reason: "runtime region was already realized",
                });
            }
            if let Some(group) = object.reuse_group {
                state
                    .active_reuse_regions
                    .entry((reservation.revision, group))
                    .or_insert(None);
            }
            bindings.push((object.key, binding));
        }
        bindings.sort_by_key(|(key, _)| *key);
        let mut lifetimes = reservation
            .objects
            .iter()
            .map(|object| (object.key, object.lifetime))
            .collect::<Vec<_>>();
        lifetimes.sort_by_key(|(key, _)| *key);
        state.ledger.committed_bytes = state
            .ledger
            .committed_bytes
            .checked_add(reservation.bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "committed bytes",
                current: state.ledger.committed_bytes,
                change: reservation.bytes,
            })?;
        state.release_reservation(reservation.bytes)?;
        state.admissions.remove(&reservation.admission);
        let lifecycle = if state.active_revision.is_none() {
            state.active_revision = Some(reservation.revision);
            PlanRevisionLifecycle::Active
        } else {
            PlanRevisionLifecycle::Realized
        };
        state.revisions.insert(reservation.revision, lifecycle);
        reservation.active = false;
        reservation.bytes = 0;
        Ok(RealizedMemoryPlan {
            domain: state.id,
            revision: reservation.revision,
            domain_state: Rc::downgrade(&self.state),
            bindings: Rc::new(RefCell::new(bindings.into_boxed_slice())),
            lifetimes: Rc::new(lifetimes.into_boxed_slice()),
        })
    }

    pub fn retire(&self, handle: AllocationHandle) -> MemoryRuntimeResult<()> {
        let mut state = self.state.borrow_mut();
        let capacity = {
            let record = state.record_mut(handle)?;
            if record.state == OwnedAllocationState::Retired {
                return Ok(());
            }
            record.state = OwnedAllocationState::Retired;
            if let Some(owner) = &record.payload_owner {
                owner.revoke();
            }
            record.capacity_bytes
        };
        state.ledger.live_allocations = state.ledger.live_allocations.checked_sub(1).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "live allocations",
                current: u64::from(state.ledger.live_allocations),
                change: 1,
            },
        )?;
        state.ledger.retired_allocations = state.ledger.retired_allocations.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "retired allocation count",
            },
        )?;
        state.ledger.retired_pinned_bytes = state
            .ledger
            .retired_pinned_bytes
            .checked_add(capacity)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retired pinned bytes",
                current: state.ledger.retired_pinned_bytes,
                change: capacity,
            })?;
        Ok(())
    }

    pub fn collect_retired(&self) -> MemoryRuntimeResult<u32> {
        let mut state = self.state.borrow_mut();
        let mut reclaimed = 0_u32;
        let mut reclaimed_bytes = 0_u64;
        for slot in &mut state.allocations {
            let reclaimable = slot.record.as_ref().is_some_and(|record| {
                record.state == OwnedAllocationState::Retired
                    && record.leases.is_empty()
                    && record.snapshot_pins == 0
                    && record.submission_pins == 0
                    && record
                        .payload_owner
                        .as_ref()
                        .is_none_or(|owner| Rc::strong_count(owner) == 1)
                    && record.device_owner_pins == 0
            });
            if !reclaimable {
                continue;
            }
            let record = slot.record.take().expect("reclaimable record exists");
            reclaimed_bytes = reclaimed_bytes.checked_add(record.capacity_bytes).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "reclaimed bytes",
                    current: reclaimed_bytes,
                    change: record.capacity_bytes,
                },
            )?;
            if let Some(next) = slot.generation.checked_add(1) {
                slot.generation = next;
            } else {
                slot.permanently_retired = true;
            }
            reclaimed = reclaimed
                .checked_add(1)
                .ok_or(MemoryRuntimeError::IdentityExhausted {
                    identity: "reclaimed allocation count",
                })?;
        }
        state.ledger.committed_bytes = state
            .ledger
            .committed_bytes
            .checked_sub(reclaimed_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "committed bytes",
                current: state.ledger.committed_bytes,
                change: reclaimed_bytes,
            })?;
        state.ledger.retired_pinned_bytes = state
            .ledger
            .retired_pinned_bytes
            .checked_sub(reclaimed_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retired pinned bytes",
                current: state.ledger.retired_pinned_bytes,
                change: reclaimed_bytes,
            })?;
        state.ledger.retired_allocations = state
            .ledger
            .retired_allocations
            .checked_sub(reclaimed)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retired allocations",
                current: u64::from(state.ledger.retired_allocations),
                change: u64::from(reclaimed),
            })?;
        Ok(reclaimed)
    }

    pub fn close(&self) -> MemoryRuntimeResult<()> {
        let handles = {
            let mut state = self.state.borrow_mut();
            state.finish_plan_scope_cleanup();
            if state.closed {
                return Ok(());
            }
            if state.ledger.active_reservations != 0 || state.active_point.get().is_some() {
                return Err(MemoryRuntimeError::TurnInFlight);
            }
            state.closed = true;
            state
                .allocations
                .iter()
                .enumerate()
                .filter_map(|(slot, entry)| {
                    entry.record.as_ref().and_then(|record| {
                        (record.state == OwnedAllocationState::Live)
                            .then(|| AllocationHandle::new(state.id, slot as u32, entry.generation))
                    })
                })
                .collect::<Vec<_>>()
        };
        for handle in handles {
            self.retire(handle)?;
        }
        self.collect_retired()?;
        Ok(())
    }

    pub fn ledger(&self) -> MemoryLedgerSnapshot {
        let state = self.state.borrow();
        MemoryLedgerSnapshot {
            exported_snapshot_bytes: state.payload_accounting.bytes(),
            ..state.ledger
        }
    }

    pub fn enter_plan_point(&self, point: MemoryPlanPoint) -> MemoryRuntimeResult<MemoryPlanScope> {
        let mut state = self.state.borrow_mut();
        state.finish_plan_scope_cleanup();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if state.active_point.get().is_some() {
            return Err(MemoryRuntimeError::TurnInFlight);
        }
        if let Some((slot, entry)) = state.allocations.iter().enumerate().find(|(_, entry)| {
            entry
                .record
                .as_ref()
                .is_some_and(|record| !record.leases.is_empty())
        }) {
            return Err(MemoryRuntimeError::OutstandingLease {
                handle: AllocationHandle::new(state.id, slot as u32, entry.generation),
            });
        }
        let revision = state
            .active_revision
            .ok_or(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.next_revision,
                actual: state.next_revision,
            })?;
        {
            let DomainState {
                regions,
                active_reuse_regions,
                ..
            } = &mut *state;
            for ((group_revision, group), active) in active_reuse_regions.iter() {
                if *group_revision != revision {
                    continue;
                }
                if active.is_some() {
                    return Err(MemoryRuntimeError::TurnInFlight);
                }
                let mut candidates = regions.iter().filter(|(_, region)| {
                    region.reuse_group == Some(*group)
                        && runtime_lifetime_is_active(region.lifetime, Some(point))
                });
                let Some((key, region)) = candidates.next() else {
                    continue;
                };
                if candidates.next().is_some() {
                    return Err(MemoryRuntimeError::InvalidReuse {
                        object: key.object(),
                        reason: "multiple reuse-group members are live at one plan point",
                    });
                }
                let _ = region.incarnation.checked_successor()?;
            }
            for ((group_revision, group), active) in active_reuse_regions.iter_mut() {
                if *group_revision != revision {
                    continue;
                }
                let candidate = regions
                    .iter()
                    .find(|(_, region)| {
                        region.reuse_group == Some(*group)
                            && runtime_lifetime_is_active(region.lifetime, Some(point))
                    })
                    .map(|(key, _)| *key);
                if let Some(key) = candidate {
                    let region = regions
                        .get_mut(&key)
                        .expect("validated reuse-group region exists");
                    region.incarnation = region
                        .incarnation
                        .checked_successor()
                        .expect("region successor was prevalidated");
                    region.initialized_bytes = 0;
                    *active = Some(key);
                }
            }
        }
        state.active_point.set(Some(point));
        Ok(MemoryPlanScope {
            domain: Rc::downgrade(&self.state),
            active_point: Rc::clone(&state.active_point),
            cleanup_pending: Rc::clone(&state.scope_cleanup_pending),
            point,
            active: true,
        })
    }

    /// Registers the actual device allocation created for one planned device
    /// arena. The capacity is exact; a backend cannot attach an oversized or
    /// undersized buffer and call it planned.
    pub fn register_device_allocation(
        &self,
        realized: &RealizedMemoryPlan,
        object: PlanObjectKey,
        actual_capacity_bytes: u64,
        initialized_bytes: u64,
    ) -> MemoryRuntimeResult<DeviceAllocationOwner> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        let binding = realized.binding(object)?;
        let RuntimeBinding::Device {
            handle,
            capacity_bytes,
            ..
        } = binding
        else {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(object.object()),
                size: actual_capacity_bytes,
                alignment: 1,
                reason: "device registration requires a device plan object",
            });
        };
        if actual_capacity_bytes != capacity_bytes || initialized_bytes > capacity_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: object.object(),
                requested: actual_capacity_bytes.max(initialized_bytes),
                capacity: capacity_bytes,
            });
        }
        {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return Err(MemoryRuntimeError::DomainClosed);
            }
            let record = state.record_mut(handle)?;
            if record.state != OwnedAllocationState::Live || record.device_owner_pins != 0 {
                return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                    object: Some(object.object()),
                    from: "unavailable device record",
                    to: "attached device allocation",
                });
            }
            if record.capacity_bytes != actual_capacity_bytes
                || !matches!(record.space, MemorySpace::Device { .. })
            {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(object.object()),
                    size: actual_capacity_bytes,
                    alignment: record.alignment,
                    reason: "device allocation differs from its planned arena",
                });
            }
            record.device_owner_pins = 1;
            record.device_actual_bytes = actual_capacity_bytes;
            record.device_lost = false;
        }
        realized.record_initialized(object, initialized_bytes)?;
        Ok(DeviceAllocationOwner {
            domain: Rc::clone(&self.state),
            handle,
            active: true,
        })
    }

    pub fn record_device_initialized(
        &self,
        realized: &RealizedMemoryPlan,
        object: PlanObjectKey,
        initialized_bytes: u64,
    ) -> MemoryRuntimeResult<()> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        let binding = realized.binding(object)?;
        let RuntimeBinding::Device { handle, .. } = binding else {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(object.object()),
                size: initialized_bytes,
                alignment: 1,
                reason: "device initialization requires a device plan object",
            });
        };
        let state = self.state.borrow();
        let record = state.record(handle)?;
        if record.state != OwnedAllocationState::Live || record.device_owner_pins == 0 {
            return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                object: Some(object.object()),
                from: "unavailable device record",
                to: "initialized device allocation",
            });
        }
        if record.device_lost {
            return Err(MemoryRuntimeError::DeviceLost {
                object: Some(object.object()),
            });
        }
        drop(state);
        realized.record_initialized(object, initialized_bytes)
    }

    /// Resolves the fixed object set for one backend operation once during
    /// activation. Reusing the returned authority performs no metadata
    /// allocation on the submission path.
    pub fn prepare_device_submission(
        &self,
        realized: &RealizedMemoryPlan,
        device_objects: &[PlanObjectKey],
        transfer_objects: &[PlanObjectKey],
    ) -> MemoryRuntimeResult<PreparedDeviceSubmission> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        let mut requested = BTreeMap::<AllocationHandle, (bool, bool)>::new();
        for object in device_objects {
            let binding = realized.binding(*object)?;
            let RuntimeBinding::Device {
                handle,
                capacity_bytes,
                ..
            } = binding
            else {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(object.object()),
                    size: binding.capacity_bytes(),
                    alignment: 1,
                    reason: "device submission references a non-device object",
                });
            };
            if capacity_bytes == 0 {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(object.object()),
                    size: 0,
                    alignment: 1,
                    reason: "device submission references an empty binding",
                });
            }
            requested.entry(handle).or_default().0 = true;
        }
        for object in transfer_objects {
            if !matches!(realized.lifetime(*object)?, MemoryLifetime::Transfer { .. }) {
                return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                    object: Some(object.object()),
                    from: "non-transfer lifetime",
                    to: "device transfer",
                });
            }
            let binding = realized.binding(*object)?;
            let handle = binding
                .handle()
                .ok_or(MemoryRuntimeError::UnplannedAllocation {
                    object: Some(object.object()),
                    requested: binding.capacity_bytes(),
                })?;
            requested.entry(handle).or_default().1 = true;
        }
        let state = self.state.borrow();
        let mut device_bytes = 0_u64;
        let mut transfer_bytes = 0_u64;
        let mut handles = Vec::new();
        let mut device_handles = Vec::new();
        handles.try_reserve_exact(requested.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: requested.len() as u64,
                alignment: 1,
                space: MemorySpace::Host,
            }
        })?;
        device_handles
            .try_reserve_exact(requested.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: requested.len() as u64,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        for (handle, (device, transfer)) in &requested {
            let record = state.record(*handle)?;
            handles.push(*handle);
            if *device {
                device_handles.push(*handle);
                device_bytes = device_bytes.checked_add(record.capacity_bytes).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "in-flight device bytes",
                        current: device_bytes,
                        change: record.capacity_bytes,
                    },
                )?;
            }
            if *transfer {
                transfer_bytes = transfer_bytes.checked_add(record.capacity_bytes).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "in-flight transfer bytes",
                        current: transfer_bytes,
                        change: record.capacity_bytes,
                    },
                )?;
            }
        }
        Ok(PreparedDeviceSubmission {
            revision: realized.revision(),
            handles: Arc::from(handles.into_boxed_slice()),
            device_handles: Arc::from(device_handles.into_boxed_slice()),
            device_bytes,
            transfer_bytes,
        })
    }

    /// Pins a prevalidated backend operation until completion without
    /// allocating or rebuilding its object set.
    pub fn begin_prepared_device_submission(
        &self,
        realized: &RealizedMemoryPlan,
        prepared: &PreparedDeviceSubmission,
    ) -> MemoryRuntimeResult<DeviceSubmissionHold> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        if prepared.revision != realized.revision() {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: realized.revision(),
                actual: prepared.revision,
            });
        }
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        for handle in prepared.handles.iter().copied() {
            let record = state.record(handle)?;
            if record.state != OwnedAllocationState::Live {
                return Err(MemoryRuntimeError::InvalidLifetimeTransition {
                    object: None,
                    from: "retired allocation",
                    to: "device submission",
                });
            }
            if prepared.device_handles.binary_search(&handle).is_ok() {
                if record.device_lost {
                    return Err(MemoryRuntimeError::DeviceLost { object: None });
                }
                if record.device_owner_pins == 0 {
                    return Err(MemoryRuntimeError::UnplannedAllocation {
                        object: None,
                        requested: record.capacity_bytes,
                    });
                }
            }
            record
                .submission_pins
                .checked_add(1)
                .ok_or(MemoryRuntimeError::IdentityExhausted {
                    identity: "device submission pin count",
                })?;
        }
        let next_device_bytes = state
            .ledger
            .in_flight_device_bytes
            .checked_add(prepared.device_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "in-flight device bytes",
                current: state.ledger.in_flight_device_bytes,
                change: prepared.device_bytes,
            })?;
        let next_transfer_bytes = state
            .ledger
            .in_flight_transfer_bytes
            .checked_add(prepared.transfer_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "in-flight transfer bytes",
                current: state.ledger.in_flight_transfer_bytes,
                change: prepared.transfer_bytes,
            })?;
        for handle in prepared.handles.iter().copied() {
            state.record_mut(handle)?.submission_pins += 1;
        }
        state.ledger.in_flight_device_bytes = next_device_bytes;
        state.ledger.in_flight_transfer_bytes = next_transfer_bytes;
        Ok(DeviceSubmissionHold {
            domain: Rc::clone(&self.state),
            handles: Arc::clone(&prepared.handles),
            device_bytes: prepared.device_bytes,
            transfer_bytes: prepared.transfer_bytes,
            active: true,
        })
    }

    /// Compatibility facade for one-off backend operations. Resident paths
    /// prepare the authority at activation and use
    /// [`Self::begin_prepared_device_submission`].
    pub fn begin_device_submission(
        &self,
        realized: &RealizedMemoryPlan,
        device_objects: &[PlanObjectKey],
        transfer_objects: &[PlanObjectKey],
    ) -> MemoryRuntimeResult<DeviceSubmissionHold> {
        let prepared =
            self.prepare_device_submission(realized, device_objects, transfer_objects)?;
        self.begin_prepared_device_submission(realized, &prepared)
    }

    pub fn allocation_observations(&self) -> Box<[ManagedAllocationObservation]> {
        let state = self.state.borrow();
        state
            .allocations
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                entry.record.as_ref().map(|record| {
                    let (fixed_bytes, fixed_alignment) = record
                        .block
                        .as_ref()
                        .map(|block| {
                            (
                                u64::try_from(block.bytes()).unwrap_or(u64::MAX),
                                u32::try_from(block.alignment()).unwrap_or(u32::MAX),
                            )
                        })
                        .unwrap_or((0, 1));
                    let payload_bytes = record
                        .payload_owner
                        .as_ref()
                        .and_then(|owner| owner.allocated_bytes().ok())
                        .unwrap_or(0);
                    let payload_alignment = record
                        .payload_owner
                        .as_ref()
                        .map(|owner| owner.max_alignment())
                        .unwrap_or(1);
                    let device_alignment = (record.device_actual_bytes != 0)
                        .then_some(record.alignment)
                        .unwrap_or(1);
                    ManagedAllocationObservation {
                        handle: AllocationHandle::new(state.id, slot as u32, entry.generation),
                        state: record.state,
                        capacity_bytes: record.capacity_bytes,
                        actual_block_bytes: fixed_bytes
                            .saturating_add(payload_bytes)
                            .saturating_add(record.device_actual_bytes),
                        alignment: record.alignment,
                        actual_block_alignment: fixed_alignment
                            .max(payload_alignment)
                            .max(device_alignment),
                        space: record.space,
                        active_leases: u32::try_from(record.leases.len()).unwrap_or(u32::MAX),
                        snapshot_pins: record.snapshot_pins,
                        submission_pins: record.submission_pins,
                        payload_owner_pins: record
                            .payload_owner
                            .as_ref()
                            .map(|owner| Rc::strong_count(owner).saturating_sub(1))
                            .and_then(|pins| u32::try_from(pins).ok())
                            .unwrap_or(0),
                        device_owner_pins: record.device_owner_pins,
                        device_lost: record.device_lost,
                    }
                })
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

fn validate_plan_view(
    domain: MemoryDomainId,
    view: RuntimePlanView<'_>,
) -> MemoryRuntimeResult<(Box<[ArenaAuthorization]>, Box<[ObjectAuthorization]>, u64)> {
    let mut arena_ids = BTreeSet::new();
    let mut arena_members = BTreeMap::<MemoryArenaId, BTreeSet<MemoryObjectId>>::new();
    let mut arenas = Vec::new();
    arenas.try_reserve_exact(view.arenas.len()).map_err(|_| {
        MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: view.arenas.len() as u64,
            alignment: 1,
            space: MemorySpace::Host,
        }
    })?;
    for arena in view.arenas {
        if !arena_ids.insert(arena.id) {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "duplicate arena id",
            });
        }
        if arena.alignment == 0 || !arena.alignment.is_power_of_two() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "arena alignment is not a nonzero power of two",
            });
        }
        if let Some(limit) = view.limits.max_storage_buffer_bytes
            && arena.backing == ArenaBackingKind::ContiguousBytes
            && arena.capacity_bytes > limit
        {
            return Err(MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: arena.capacity_bytes,
                limit,
            });
        }
        let members = arena.members.iter().copied().collect::<BTreeSet<_>>();
        if members.len() != arena.members.len() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "arena repeats a member object",
            });
        }
        arena_members.insert(arena.id, members);
        arenas.push(ArenaAuthorization {
            id: arena.id,
            space: arena.space,
            backing: arena.backing,
            alignment: arena.alignment,
            capacity_bytes: arena.capacity_bytes,
        });
    }

    let mut object_ids = BTreeSet::new();
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(view.allocations.len())
        .map_err(|_| MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: view.allocations.len() as u64,
            alignment: 1,
            space: MemorySpace::Host,
        })?;
    for allocation in view.allocations {
        if !object_ids.insert(allocation.id) {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "duplicate memory object id",
            });
        }
        let arena = view
            .arenas
            .iter()
            .find(|arena| arena.id == allocation.placement.arena)
            .ok_or(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation references an unknown arena",
            })?;
        if !arena_members
            .get(&arena.id)
            .is_some_and(|members| members.contains(&allocation.id))
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation is absent from its arena member list",
            });
        }
        if arena.space != allocation.space {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation and arena use different memory spaces",
            });
        }
        if allocation.current_bytes > allocation.capacity_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: allocation.id,
                requested: allocation.current_bytes,
                capacity: allocation.capacity_bytes,
            });
        }
        if allocation.alignment == 0 || !allocation.alignment.is_power_of_two() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation alignment is not a nonzero power of two",
            });
        }
        if arena.backing == ArenaBackingKind::ContiguousBytes {
            if arena.alignment < allocation.alignment {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(allocation.id),
                    size: allocation.capacity_bytes,
                    alignment: allocation.alignment,
                    reason: "arena base alignment is weaker than its member alignment",
                });
            }
            if allocation.placement.offset % u64::from(allocation.alignment) != 0 {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(allocation.id),
                    size: allocation.capacity_bytes,
                    alignment: allocation.alignment,
                    reason: "allocation offset violates its alignment",
                });
            }
            let end = allocation
                .placement
                .offset
                .checked_add(allocation.capacity_bytes)
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(allocation.id),
                    size: allocation.capacity_bytes,
                    alignment: allocation.alignment,
                    reason: "allocation end overflows",
                })?;
            if end > arena.capacity_bytes {
                return Err(MemoryRuntimeError::CapacityExceeded {
                    object: allocation.id,
                    requested: end,
                    capacity: arena.capacity_bytes,
                });
            }
        }
        objects.push(ObjectAuthorization {
            key: PlanObjectKey::new(domain, view.revision, allocation.id),
            arena: arena.id,
            space: allocation.space,
            current_bytes: allocation.current_bytes,
            capacity_bytes: allocation.capacity_bytes,
            alignment: allocation.alignment,
            offset: allocation.placement.offset,
            lifetime: allocation.lifetime,
            reuse_group: allocation.reuse_group,
        });
    }
    for arena in view.arenas {
        if arena
            .members
            .iter()
            .any(|member| !object_ids.contains(member))
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "arena member references an unknown allocation",
            });
        }
    }
    validate_overlaps(view.allocations, view.arenas)?;

    let mut bytes = 0_u64;
    for arena in view.arenas {
        if arena.backing == ArenaBackingKind::ContiguousBytes {
            bytes = bytes.checked_add(arena.capacity_bytes).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "reservation bytes",
                    current: bytes,
                    change: arena.capacity_bytes,
                },
            )?;
        }
    }
    for object in &objects {
        if arenas.iter().any(|arena| {
            arena.id == object.arena && arena.backing == ArenaBackingKind::IndirectOwnedPayloads
        }) {
            bytes = bytes.checked_add(object.capacity_bytes).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "reservation bytes",
                    current: bytes,
                    change: object.capacity_bytes,
                },
            )?;
        }
    }
    Ok((arenas.into_boxed_slice(), objects.into_boxed_slice(), bytes))
}

fn validate_overlaps(
    allocations: &[AllocationPlan],
    arenas: &[ArenaPlan],
) -> MemoryRuntimeResult<()> {
    for arena in arenas {
        if arena.backing == ArenaBackingKind::IndirectOwnedPayloads {
            continue;
        }
        let members = allocations
            .iter()
            .filter(|allocation| allocation.placement.arena == arena.id)
            .collect::<Vec<_>>();
        for (position, left) in members.iter().enumerate() {
            let left_end = left
                .placement
                .offset
                .checked_add(left.capacity_bytes)
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(left.id),
                    size: left.capacity_bytes,
                    alignment: left.alignment,
                    reason: "allocation end overflows",
                })?;
            for right in &members[position + 1..] {
                let right_end = right
                    .placement
                    .offset
                    .checked_add(right.capacity_bytes)
                    .ok_or(MemoryRuntimeError::InvalidLayout {
                        object: Some(right.id),
                        size: right.capacity_bytes,
                        alignment: right.alignment,
                        reason: "allocation end overflows",
                    })?;
                let overlaps = left.placement.offset < right_end
                    && right.placement.offset < left_end
                    && left.capacity_bytes != 0
                    && right.capacity_bytes != 0;
                if !overlaps {
                    continue;
                }
                let allowed = left.reuse_group.is_some()
                    && left.reuse_group == right.reuse_group
                    && lifetimes_disjoint(left.lifetime, right.lifetime);
                if !allowed {
                    return Err(MemoryRuntimeError::InvalidReuse {
                        object: right.id,
                        reason: "overlapping live regions are not one disjoint-lifetime reuse group",
                    });
                }
            }
        }
    }
    Ok(())
}

fn lifetimes_disjoint(left: MemoryLifetime, right: MemoryLifetime) -> bool {
    fn interval(lifetime: MemoryLifetime) -> Option<(MemoryPlanPoint, MemoryPlanPoint)> {
        match lifetime {
            MemoryLifetime::Turn { first, last }
            | MemoryLifetime::Transaction { first, last }
            | MemoryLifetime::Transfer { first, last } => Some((first, last)),
            MemoryLifetime::Program | MemoryLifetime::Activation => None,
        }
    }
    match (interval(left), interval(right)) {
        (Some((left_first, left_last)), Some((right_first, right_last))) => {
            left_last < right_first || right_last < left_first
        }
        _ => false,
    }
}

fn runtime_lifetime_is_active(lifetime: MemoryLifetime, active: Option<MemoryPlanPoint>) -> bool {
    match lifetime {
        MemoryLifetime::Program | MemoryLifetime::Activation => true,
        MemoryLifetime::Turn { first, last }
        | MemoryLifetime::Transaction { first, last }
        | MemoryLifetime::Transfer { first, last } => {
            active.is_some_and(|point| first <= point && point <= last)
        }
    }
}
