use std::collections::BTreeMap;
#[cfg(feature = "native")]
use std::{
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use crate::{
    GpuExecutionBindingRole, GpuExecutionPlan, GpuExecutionPlanError, GpuKernelPlanSource,
    GpuPlanScalar,
};
#[cfg(feature = "native")]
use mech_core::AllocationHandle;
use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, CallAccessRequest,
    DeviceAllocationOwner, DeviceSubmissionHold, GpuMemoryLimits, ManagedAllocationObservation,
    MemoryAccessMode, MemoryAccessRegion, MemoryArenaId, MemoryBudgetViolation, MemoryDomain,
    MemoryLedgerSnapshot, MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanError,
    MemoryPlanPoint, MemoryRuntimeError, MemorySpace, PlanObjectKey, PreparedCallAccess,
    PreparedDeviceSubmission, RealizedMemoryPlan, ResourceDemand, RuntimePlanView,
    TargetMemoryProfile, TransferDirection, TransferPlan, evaluate_memory_budget,
};

/// Existing GPU execution plan paired with the process-local, non-wire R5
/// allocation plan used to admit every backing before device creation.
#[derive(Clone, Debug)]
pub struct PlannedGpuExecution {
    pub execution: GpuExecutionPlan,
    /// A physical backing projection subordinate to `execution`. It is not a
    /// semantic `ProgramMemoryPlan` and therefore cannot become an alternate
    /// operation, alias, transaction, or lifetime authority.
    pub memory: GpuBackingMemoryPlan,
    binding_objects: BTreeMap<u32, MemoryObjectId>,
    state_objects: BTreeMap<mech_core::CellSlotId, [MemoryObjectId; 2]>,
    readback_objects: BTreeMap<mech_core::CellSlotId, MemoryObjectId>,
    readback_device_objects: BTreeMap<mech_core::CellSlotId, MemoryObjectId>,
    integrity_readback_objects: Option<[MemoryObjectId; 2]>,
    device_objects: Box<[MemoryObjectId]>,
    writable_device_objects: Box<[(MemoryObjectId, u64)]>,
    transfer_objects: Box<[MemoryObjectId]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuBackingMemoryPlan {
    pub allocations: Box<[AllocationPlan]>,
    pub arenas: Box<[ArenaPlan]>,
    pub transfers: Box<[TransferPlan]>,
    pub budget_limits: mech_core::MemoryBudgetLimits,
    pub demand: ResourceDemand,
    pub budget_violations: Box<[MemoryBudgetViolation]>,
}

/// Process-local realization of the subordinate GPU backing plan. It keeps
/// logical plan objects separate from backend buffers while making every
/// buffer registration and submission pin pass through the R6 domain.
pub struct ManagedGpuMemory {
    domain: MemoryDomain,
    realized: RealizedMemoryPlan,
    objects: Box<[(MemoryObjectId, PlanObjectKey)]>,
    device_owners: Box<[(MemoryObjectId, DeviceAllocationRegistration)]>,
    content_versions: Box<[(MemoryObjectId, u64)]>,
    host_transfer_writes: Box<[(MemoryObjectId, PreparedCallAccess)]>,
}

enum DeviceAllocationRegistration {
    Vacant,
    /// Low-level unit-test authority used without constructing a platform
    /// device. Production registrations always couple the real buffer owner.
    #[cfg(test)]
    AccountingOnly(DeviceAllocationOwner),
    #[cfg(feature = "native")]
    Buffer(Weak<DeviceAllocationOwner>),
}

impl DeviceAllocationRegistration {
    fn is_registered(&self) -> bool {
        !matches!(self, Self::Vacant)
    }
}

/// One native backend buffer coupled to the allocation identity and
/// accounting owner that admitted it. The field order is intentional: the
/// wgpu handle is released before the accounting owner can be released.
#[cfg(feature = "native")]
pub struct RegisteredDeviceBuffer {
    buffer: wgpu::Buffer,
    owner: Rc<DeviceAllocationOwner>,
    object: MemoryObjectId,
}

#[cfg(feature = "native")]
impl RegisteredDeviceBuffer {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub const fn object(&self) -> MemoryObjectId {
        self.object
    }

    pub fn allocation_handle(&self) -> AllocationHandle {
        self.owner.handle()
    }
}

impl ManagedGpuMemory {
    pub fn realize(plan: &GpuBackingMemoryPlan) -> Result<Self, GpuMemoryPlanError> {
        let domain = MemoryDomain::new()?;
        let revision = domain.issue_plan_revision()?;
        let reservation = domain.prepare_realization(RuntimePlanView::new(
            revision,
            &plan.allocations,
            &plan.arenas,
            plan.demand,
            0,
            plan.budget_limits,
            &[],
            1,
            &plan.budget_violations,
        ))?;
        let realized = domain.materialize(reservation)?;
        let mut objects = Vec::new();
        objects
            .try_reserve_exact(plan.allocations.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: plan.allocations.len() as u64,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        let device_count = plan
            .allocations
            .iter()
            .filter(|allocation| matches!(allocation.space, MemorySpace::Device { .. }))
            .count();
        let mut device_owners = Vec::new();
        device_owners.try_reserve_exact(device_count).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: device_count as u64,
                alignment: 1,
                space: MemorySpace::Host,
            }
        })?;
        for allocation in &plan.allocations {
            let key = domain.plan_object_key(revision, allocation.id)?;
            objects.push((allocation.id, key));
            if matches!(allocation.space, MemorySpace::Device { .. }) {
                device_owners.push((allocation.id, DeviceAllocationRegistration::Vacant));
            }
        }
        objects.sort_by_key(|(object, _)| *object);
        device_owners.sort_by_key(|(object, _)| *object);
        let content_versions = device_owners
            .iter()
            .map(|(object, _)| (*object, 0))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut host_transfer_writes = Vec::new();
        host_transfer_writes
            .try_reserve_exact(plan.transfers.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: plan.transfers.len() as u64,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        for allocation in plan.allocations.iter().filter(|allocation| {
            allocation.space == MemorySpace::Host
                && matches!(allocation.lifetime, MemoryLifetime::Transfer { .. })
        }) {
            let key = domain.plan_object_key(revision, allocation.id)?;
            let prepared = domain.prepare_call(
                &realized,
                &[CallAccessRequest {
                    object: key,
                    mode: MemoryAccessMode::Write,
                    region: MemoryAccessRegion::Contiguous {
                        offset_bytes: 0,
                        length_bytes: allocation.capacity_bytes,
                    },
                }],
            )?;
            host_transfer_writes.push((allocation.id, prepared));
        }
        host_transfer_writes.sort_by_key(|(object, _)| *object);
        Ok(Self {
            domain,
            realized,
            objects: objects.into_boxed_slice(),
            device_owners: device_owners.into_boxed_slice(),
            content_versions,
            host_transfer_writes: host_transfer_writes.into_boxed_slice(),
        })
    }

    pub fn object_key(&self, object: MemoryObjectId) -> Result<PlanObjectKey, GpuMemoryPlanError> {
        self.objects
            .binary_search_by_key(&object, |(candidate, _)| *candidate)
            .ok()
            .map(|index| self.objects[index].1)
            .ok_or(GpuMemoryPlanError::MissingPlanObject { object })
    }

    #[cfg(test)]
    pub(crate) fn attach_device_allocation(
        &mut self,
        object: MemoryObjectId,
        actual_capacity_bytes: u64,
        initialized_bytes: u64,
    ) -> Result<(), GpuMemoryPlanError> {
        let key = self.object_key(object)?;
        let index = self
            .device_owners
            .binary_search_by_key(&object, |(candidate, _)| *candidate)
            .map_err(|_| GpuMemoryPlanError::MissingPlanObject { object })?;
        if self.device_owners[index].1.is_registered() {
            return Err(GpuMemoryPlanError::DuplicateDeviceAllocation { object });
        }
        let owner = self.domain.register_device_allocation(
            &self.realized,
            key,
            actual_capacity_bytes,
            initialized_bytes,
        )?;
        self.device_owners[index].1 = DeviceAllocationRegistration::AccountingOnly(owner);
        if initialized_bytes != 0 {
            self.content_versions[index].1 = 1;
        }
        Ok(())
    }

    /// Registers the exact native buffer which realizes one planned object.
    /// The returned wrapper, rather than this logical registry, owns the
    /// allocation pin so accounting cannot outlive or precede the real buffer.
    #[cfg(feature = "native")]
    pub fn register_device_buffer(
        &mut self,
        object: MemoryObjectId,
        buffer: wgpu::Buffer,
        initialized_bytes: u64,
    ) -> Result<RegisteredDeviceBuffer, GpuMemoryPlanError> {
        let key = self.object_key(object)?;
        let index = self
            .device_owners
            .binary_search_by_key(&object, |(candidate, _)| *candidate)
            .map_err(|_| GpuMemoryPlanError::MissingPlanObject { object })?;
        if self.device_owners[index].1.is_registered() {
            return Err(GpuMemoryPlanError::DuplicateDeviceAllocation { object });
        }
        let owner = Rc::new(self.domain.register_device_allocation(
            &self.realized,
            key,
            buffer.size(),
            initialized_bytes,
        )?);
        self.device_owners[index].1 = DeviceAllocationRegistration::Buffer(Rc::downgrade(&owner));
        if initialized_bytes != 0 {
            self.content_versions[index].1 = 1;
        }
        Ok(RegisteredDeviceBuffer {
            buffer,
            owner,
            object,
        })
    }

    pub fn record_device_write(
        &mut self,
        object: MemoryObjectId,
        initialized_bytes: u64,
    ) -> Result<u64, GpuMemoryPlanError> {
        let key = self.object_key(object)?;
        let index = self
            .content_versions
            .binary_search_by_key(&object, |(candidate, _)| *candidate)
            .map_err(|_| GpuMemoryPlanError::MissingPlanObject { object })?;
        let next = self.content_versions[index]
            .1
            .checked_add(1)
            .ok_or(GpuMemoryPlanError::ContentVersionExhausted { object })?;
        self.domain
            .record_device_initialized(&self.realized, key, initialized_bytes)?;
        self.content_versions[index].1 = next;
        Ok(next)
    }

    pub fn content_version(&self, object: MemoryObjectId) -> Option<u64> {
        self.content_versions
            .binary_search_by_key(&object, |(candidate, _)| *candidate)
            .ok()
            .map(|index| self.content_versions[index].1)
    }

    pub fn begin_submission(
        &self,
        device_objects: &[MemoryObjectId],
        transfer_objects: &[MemoryObjectId],
    ) -> Result<DeviceSubmissionHold, GpuMemoryPlanError> {
        let device_keys = device_objects
            .iter()
            .copied()
            .map(|object| self.object_key(object))
            .collect::<Result<Vec<_>, _>>()?;
        let transfer_keys = transfer_objects
            .iter()
            .copied()
            .map(|object| self.object_key(object))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .domain
            .begin_device_submission(&self.realized, &device_keys, &transfer_keys)?)
    }

    pub fn prepare_submission(
        &self,
        device_objects: &[MemoryObjectId],
        transfer_objects: &[MemoryObjectId],
    ) -> Result<PreparedDeviceSubmission, GpuMemoryPlanError> {
        let device_keys = device_objects
            .iter()
            .copied()
            .map(|object| self.object_key(object))
            .collect::<Result<Vec<_>, _>>()?;
        let transfer_keys = transfer_objects
            .iter()
            .copied()
            .map(|object| self.object_key(object))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .domain
            .prepare_device_submission(&self.realized, &device_keys, &transfer_keys)?)
    }

    pub fn begin_prepared_submission(
        &self,
        prepared: &PreparedDeviceSubmission,
    ) -> Result<DeviceSubmissionHold, GpuMemoryPlanError> {
        Ok(self
            .domain
            .begin_prepared_device_submission(&self.realized, prepared)?)
    }

    /// Copies a completed backend mapping into its planned host transfer
    /// region before exposing a call-scoped borrowed view to the decoder.
    pub fn with_staged_host_transfer<R>(
        &self,
        object: MemoryObjectId,
        source: &[u8],
        consume: impl FnOnce(&[u8]) -> R,
    ) -> Result<R, GpuMemoryPlanError> {
        let key = self.object_key(object)?;
        let index = self
            .host_transfer_writes
            .binary_search_by_key(&object, |(candidate, _)| *candidate)
            .map_err(|_| GpuMemoryPlanError::MissingPlanObject { object })?;
        let capacity = self.realized.binding(key)?.capacity_bytes();
        if source.len() as u64 > capacity {
            return Err(GpuMemoryPlanError::Runtime(
                MemoryRuntimeError::CapacityExceeded {
                    object,
                    requested: source.len() as u64,
                    capacity,
                },
            ));
        }
        let _scope = self.domain.enter_plan_point(MemoryPlanPoint::new(0))?;
        let mut frame = self
            .domain
            .acquire_call(&self.realized, &self.host_transfer_writes[index].1)?;
        frame.with_object_init_writer::<u8>(key, |writer| writer.copy_from_slice(source))?;
        Ok(frame.with_bytes_mut_prefix(key, source.len() as u64, |target| consume(target))?)
    }

    pub fn ledger(&self) -> MemoryLedgerSnapshot {
        self.domain.ledger()
    }

    pub fn allocations(&self) -> Box<[ManagedAllocationObservation]> {
        self.domain.allocation_observations()
    }

    pub fn mark_device_lost(&self) -> Result<(), GpuMemoryPlanError> {
        for (_, registration) in self.device_owners.iter() {
            match registration {
                DeviceAllocationRegistration::Vacant => {}
                #[cfg(test)]
                DeviceAllocationRegistration::AccountingOnly(owner) => owner.mark_lost()?,
                #[cfg(feature = "native")]
                DeviceAllocationRegistration::Buffer(owner) => {
                    if let Some(owner) = owner.upgrade() {
                        owner.mark_lost()?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), GpuMemoryPlanError> {
        for (_, registration) in self.device_owners.iter_mut() {
            *registration = DeviceAllocationRegistration::Vacant;
        }
        self.domain.close()?;
        Ok(())
    }
}

#[cfg(feature = "native")]
pub(crate) struct DeviceSubmissionTracker {
    next_serial: u64,
    completed: Arc<AtomicU64>,
    device_lost: Arc<AtomicBool>,
    pending: Vec<(u64, DeviceSubmissionHold)>,
    submitted: Vec<(u64, Option<wgpu::SubmissionIndex>)>,
    planned_capacity: usize,
}

/// A batch whose serial, metadata slots, and ownership holds all exist before
/// the backend submission boundary. Dropping it before `record_submitted`
/// cancels the unsubmitted holds. Once recorded, the tracker owns those holds
/// until the exact submission completes or the device is lost.
#[cfg(feature = "native")]
pub(crate) struct UnsubmittedDeviceBatch<'a> {
    tracker: &'a mut DeviceSubmissionTracker,
    serial: u64,
    pending_start: usize,
    submitted_start: usize,
}

#[cfg(feature = "native")]
impl DeviceSubmissionTracker {
    pub(crate) fn new(capacity: usize) -> Result<Self, String> {
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(capacity)
            .map_err(|_| "GPU submission tracker allocation was not available".to_owned())?;
        let mut submitted = Vec::new();
        submitted
            .try_reserve_exact(capacity)
            .map_err(|_| "GPU submission completion metadata was not available".to_owned())?;
        Ok(Self {
            next_serial: 1,
            completed: Arc::new(AtomicU64::new(0)),
            device_lost: Arc::new(AtomicBool::new(false)),
            pending,
            submitted,
            planned_capacity: capacity,
        })
    }

    pub(crate) fn observe_device_loss(&self, device: &wgpu::Device) {
        let device_lost = Arc::clone(&self.device_lost);
        device.set_device_lost_callback(move |_, _| {
            device_lost.store(true, Ordering::Release);
        });
    }

    /// Verifies that the next backend operation can be represented without
    /// growing tracker metadata or discovering serial exhaustion after a
    /// queue write has already been issued.
    pub(crate) fn preflight_batch(&self, hold_count: usize) -> Result<(), String> {
        if hold_count == 0 {
            return Err("GPU submission batch has no ownership holds".to_owned());
        }
        let pending_end = self
            .pending
            .len()
            .checked_add(hold_count)
            .ok_or_else(|| "GPU submission tracker capacity overflowed".to_owned())?;
        if pending_end > self.planned_capacity {
            return Err("GPU submission tracker exceeded its planned capacity".to_owned());
        }
        if self.submitted.len() == self.planned_capacity {
            return Err("GPU submission metadata exceeded its planned capacity".to_owned());
        }
        self.next_serial
            .checked_add(1)
            .ok_or_else(|| "GPU submission serial exhausted".to_owned())?;
        Ok(())
    }

    pub(crate) fn reserve_batch<I>(
        &mut self,
        holds: I,
    ) -> Result<UnsubmittedDeviceBatch<'_>, String>
    where
        I: Iterator<Item = DeviceSubmissionHold>,
    {
        let (hold_count, upper_bound) = holds.size_hint();
        if upper_bound != Some(hold_count) {
            return Err("GPU submission batch has no exact ownership-hold count".to_owned());
        }
        self.preflight_batch(hold_count)?;
        let pending_end = self.pending.len() + hold_count;
        let serial = self.next_serial;
        self.next_serial = serial
            .checked_add(1)
            .ok_or_else(|| "GPU submission serial exhausted".to_owned())?;
        let pending_start = self.pending.len();
        let submitted_start = self.submitted.len();
        // This slot is filled by moving the `SubmissionIndex` returned from
        // `Queue::submit`; no tracker allocation or validation remains after
        // the backend has accepted the command buffers.
        self.submitted.push((serial, None));
        for hold in holds {
            if self.pending.len() == pending_end {
                self.pending.truncate(pending_start);
                self.submitted.truncate(submitted_start);
                return Err("GPU submission batch exceeded its reserved hold count".to_owned());
            }
            // `new` reserves `planned_capacity` once, and the complete batch
            // was bounded above before this loop. These pushes cannot grow.
            self.pending.push((serial, hold));
        }
        if self.pending.len() != pending_end {
            self.pending.truncate(pending_start);
            self.submitted.truncate(submitted_start);
            return Err("GPU submission batch did not fill its reserved hold count".to_owned());
        }
        Ok(UnsubmittedDeviceBatch {
            tracker: self,
            serial,
            pending_start,
            submitted_start,
        })
    }

    pub(crate) fn reap(&mut self) -> Result<(), String> {
        let completed = self.completed.load(Ordering::Acquire);
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].0 <= completed {
                let (_, hold) = self.pending.remove(index);
                hold.complete()
                    .map_err(|error| format!("GPU completion accounting failed: {error}"))?;
            } else {
                index += 1;
            }
        }
        self.submitted.retain(|(serial, _)| *serial > completed);
        Ok(())
    }

    pub(crate) fn wait_for_submitted(&mut self, device: &wgpu::Device) -> Result<(), String> {
        let Some((serial, submission)) = self.submitted.last() else {
            return Ok(());
        };
        let submission = submission.clone().ok_or_else(|| {
            "GPU submission completion was requested before queue acceptance".to_owned()
        })?;
        let serial = *serial;
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(submission));
        if !self.device_was_lost() {
            // Queues complete in submission order, so waiting for the newest
            // reserved batch completes every earlier serial as well.
            self.completed.fetch_max(serial, Ordering::Release);
            self.submitted.clear();
        }
        Ok(())
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(crate) fn device_was_lost(&self) -> bool {
        self.device_lost.load(Ordering::Acquire)
    }

    /// Releases accepted work only after the caller has established device
    /// loss as the terminal backend boundary.
    pub(crate) fn release_after_device_loss(&mut self) -> Result<(), String> {
        self.submitted.clear();
        while let Some((_, hold)) = self.pending.pop() {
            hold.complete()
                .map_err(|error| format!("GPU device-loss accounting failed: {error}"))?;
        }
        Ok(())
    }
}

#[cfg(feature = "native")]
impl UnsubmittedDeviceBatch<'_> {
    /// Records the already accepted queue operation. All fallible tracker work
    /// and all tracker-owned storage were completed by `reserve_batch`.
    pub(crate) fn record_submitted(mut self, submission: wgpu::SubmissionIndex) -> u64 {
        let serial = self.serial;
        self.tracker.submitted[self.submitted_start].1 = Some(submission);
        // From these assignments onward Drop must never interpret the batch as
        // a cancellation. Recording is now complete and cannot fail or grow.
        self.pending_start = usize::MAX;
        self.submitted_start = usize::MAX;
        serial
    }
}

#[cfg(feature = "native")]
impl Drop for UnsubmittedDeviceBatch<'_> {
    fn drop(&mut self) {
        if self.pending_start == usize::MAX {
            return;
        }
        debug_assert!(self.pending_start <= self.tracker.pending.len());
        // Nothing can append while this token exclusively borrows the tracker,
        // so the suffix is exactly this not-yet-submitted batch.
        self.tracker.pending.truncate(self.pending_start);
        self.tracker.submitted.truncate(self.submitted_start);
    }
}

impl PlannedGpuExecution {
    pub fn build(
        source: GpuKernelPlanSource<'_>,
        input_values: &BTreeMap<String, Vec<f32>>,
        limits: GpuMemoryLimits,
    ) -> Result<Self, GpuMemoryPlanError> {
        let execution = GpuExecutionPlan::build(source, input_values)?;
        Self::from_execution(execution, limits)
    }

    pub fn from_execution(
        execution: GpuExecutionPlan,
        limits: GpuMemoryLimits,
    ) -> Result<Self, GpuMemoryPlanError> {
        execution.validate()?;
        let target = TargetMemoryProfile::gpu(limits)?;
        let workgroups = execution
            .dispatch_elements
            .div_ceil(execution.workgroup_size);
        if execution.workgroup_size > limits.max_compute_workgroup_size_x
            || execution.workgroup_size > limits.max_compute_invocations_per_workgroup
            || workgroups > limits.max_compute_workgroups_per_dimension
        {
            return Err(GpuMemoryPlanError::WorkgroupLimit {
                workgroup_size: execution.workgroup_size,
                workgroups,
            });
        }

        let mut allocations = Vec::new();
        let mut arenas = Vec::new();
        let mut binding_objects = BTreeMap::new();
        let mut state_objects = BTreeMap::new();
        let mut readback_objects = BTreeMap::new();
        let mut readback_device_objects = BTreeMap::new();
        let mut integrity_readback_objects = None;
        let mut next_id = 0_u32;
        for state in &execution.states {
            let slot = mech_core::CellSlotId::new(state.slot);
            let bytes = checked_binding_bytes(state.elements, GpuPlanScalar::F32)?;
            let current = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let next = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                current,
                MemoryObjectOwner::Slot(slot),
                AllocationRole::FixedStorage,
                MemorySpace::Device { region: 0 },
                bytes,
                MemoryLifetime::Activation,
                limits.min_storage_buffer_offset_alignment,
            )?;
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                next,
                MemoryObjectOwner::Slot(slot),
                AllocationRole::TransactionStage,
                MemorySpace::Device { region: 0 },
                bytes,
                MemoryLifetime::Activation,
                limits.min_storage_buffer_offset_alignment,
            )?;
            state_objects.insert(slot, [current, next]);
        }

        let mut bindings = execution.bindings.iter().collect::<Vec<_>>();
        bindings.sort_by_key(|binding| binding.binding);
        for binding in bindings {
            let slot = mech_core::CellSlotId::new(binding.slot);
            if matches!(
                binding.role,
                GpuExecutionBindingRole::StateRead | GpuExecutionBindingRole::StateWrite
            ) {
                let [current, next] = state_objects
                    .get(&slot)
                    .copied()
                    .ok_or(MemoryPlanError::DescriptorMismatch)?;
                binding_objects.insert(
                    binding.binding,
                    if binding.role == GpuExecutionBindingRole::StateRead {
                        current
                    } else {
                        next
                    },
                );
                continue;
            }
            let bytes = checked_binding_bytes(binding.elements, binding.scalar)?;
            let id = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let role = if binding.role == GpuExecutionBindingRole::IntegrityFault {
                AllocationRole::Scratch
            } else {
                AllocationRole::FixedStorage
            };
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                id,
                MemoryObjectOwner::Slot(slot),
                role,
                MemorySpace::Device { region: 0 },
                bytes,
                MemoryLifetime::Activation,
                limits.min_storage_buffer_offset_alignment,
            )?;
            binding_objects.insert(binding.binding, id);
        }

        let mut readback_elements = BTreeMap::<mech_core::CellSlotId, u64>::new();
        for output in &execution.outputs {
            readback_elements
                .entry(mech_core::CellSlotId::new(output.slot))
                .and_modify(|elements| *elements = (*elements).max(output.elements))
                .or_insert(output.elements);
        }
        let mut transfers = Vec::new();
        for (ordinal, (slot, elements)) in readback_elements.into_iter().enumerate() {
            let bytes = checked_binding_bytes(elements, GpuPlanScalar::F32)?;
            let device_stage = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let host_stage = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let ordinal =
                u32::try_from(ordinal).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                    field: "GPU readback ordinal",
                })?;
            let device_ordinal =
                ordinal
                    .checked_mul(2)
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "GPU readback device ordinal",
                    })?;
            let host_ordinal =
                device_ordinal
                    .checked_add(1)
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "GPU readback host ordinal",
                    })?;
            let lifetime = MemoryLifetime::Transfer {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(0),
            };
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                device_stage,
                MemoryObjectOwner::Transfer {
                    ordinal: device_ordinal,
                },
                AllocationRole::TransferStage,
                MemorySpace::Device { region: 0 },
                bytes,
                lifetime,
                limits.min_storage_buffer_offset_alignment,
            )?;
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                host_stage,
                MemoryObjectOwner::Transfer {
                    ordinal: host_ordinal,
                },
                AllocationRole::TransferStage,
                MemorySpace::Host,
                bytes,
                lifetime,
                limits.min_storage_buffer_offset_alignment,
            )?;
            readback_device_objects.insert(slot, device_stage);
            readback_objects.insert(slot, host_stage);
            transfers.push(TransferPlan {
                slot,
                direction: TransferDirection::Readback,
                source: MemorySpace::Device { region: 0 },
                destination: MemorySpace::Host,
                current_bytes: bytes,
                capacity_bytes: bytes,
                lifetime,
                consumer: None,
                interface_name: execution
                    .outputs
                    .iter()
                    .find(|output| output.slot == slot.get())
                    .map(|output| output.name.clone()),
            });
        }
        if let Some(binding) = execution
            .bindings
            .iter()
            .find(|binding| binding.role == GpuExecutionBindingRole::IntegrityFault)
        {
            let bytes = checked_binding_bytes(binding.elements, binding.scalar)?;
            let device_stage = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let host_stage = MemoryObjectId::new(next_id);
            let lifetime = MemoryLifetime::Transfer {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(0),
            };
            let ordinal = u32::try_from(readback_objects.len())
                .map_err(|_| MemoryPlanError::ArithmeticOverflow {
                    field: "GPU integrity readback ordinal",
                })?
                .checked_mul(2)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "GPU integrity readback ordinal",
                })?;
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                device_stage,
                MemoryObjectOwner::Transfer { ordinal },
                AllocationRole::TransferStage,
                MemorySpace::Device { region: 0 },
                bytes,
                lifetime,
                limits.min_storage_buffer_offset_alignment,
            )?;
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                host_stage,
                MemoryObjectOwner::Transfer {
                    ordinal: ordinal
                        .checked_add(1)
                        .ok_or(MemoryPlanError::ArithmeticOverflow {
                            field: "GPU integrity host ordinal",
                        })?,
                },
                AllocationRole::TransferStage,
                MemorySpace::Host,
                bytes,
                lifetime,
                limits.min_storage_buffer_offset_alignment,
            )?;
            transfers.push(TransferPlan {
                slot: mech_core::CellSlotId::new(binding.slot),
                direction: TransferDirection::Readback,
                source: MemorySpace::Device { region: 0 },
                destination: MemorySpace::Host,
                current_bytes: bytes,
                capacity_bytes: bytes,
                lifetime,
                consumer: None,
                interface_name: Some("integrity-fault".to_owned()),
            });
            integrity_readback_objects = Some([device_stage, host_stage]);
        }

        let mut demand = ResourceDemand {
            storage_bindings: u32::try_from(execution.bindings.len()).map_err(|_| {
                MemoryPlanError::ArithmeticOverflow {
                    field: "GPU storage binding count",
                }
            })?,
            ..ResourceDemand::default()
        };
        for allocation in &allocations {
            let field = match allocation.lifetime {
                MemoryLifetime::Transfer { .. } => &mut demand.transfer_bytes,
                _ => &mut demand.activation_bytes,
            };
            *field = field.checked_add(allocation.capacity_bytes).ok_or(
                MemoryPlanError::ArithmeticOverflow {
                    field: "GPU planned bytes",
                },
            )?;
        }
        let mut budget_violations = Vec::<MemoryBudgetViolation>::new();
        for allocation in &allocations {
            budget_violations.extend(evaluate_memory_budget(
                allocation.owner.clone(),
                demand_for_gpu_allocation(allocation, demand.storage_bindings),
                allocation.capacity_bytes,
                allocation.capacity_bytes,
                target.limits,
            ));
        }
        budget_violations.sort();
        budget_violations.dedup();
        if let Some(violation) = budget_violations.first().cloned() {
            return Err(GpuMemoryPlanError::Plan(
                MemoryPlanError::TargetLimitExceeded { violation },
            ));
        }
        allocations.sort_by_key(|allocation| allocation.id);
        arenas.sort_by_key(|arena| arena.id);
        let device_objects = allocations
            .iter()
            .filter(|allocation| matches!(allocation.space, MemorySpace::Device { .. }))
            .map(|allocation| allocation.id)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut writable_device_objects = execution
            .bindings
            .iter()
            .filter(|binding| binding.access == crate::GpuBindingAccess::ReadWrite)
            .map(|binding| {
                let object = binding_objects.get(&binding.binding).copied().ok_or(
                    GpuMemoryPlanError::MissingBinding {
                        binding: binding.binding,
                    },
                )?;
                Ok((
                    object,
                    checked_binding_bytes(binding.elements, binding.scalar)?,
                ))
            })
            .collect::<Result<Vec<_>, GpuMemoryPlanError>>()?;
        writable_device_objects.sort_by_key(|(object, _)| *object);
        writable_device_objects.dedup_by_key(|(object, _)| *object);
        let transfer_objects = allocations
            .iter()
            .filter(|allocation| matches!(allocation.lifetime, MemoryLifetime::Transfer { .. }))
            .map(|allocation| allocation.id)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            execution,
            memory: GpuBackingMemoryPlan {
                allocations: allocations.into_boxed_slice(),
                arenas: arenas.into_boxed_slice(),
                transfers: transfers.into_boxed_slice(),
                budget_limits: target.limits,
                demand,
                budget_violations: budget_violations.into_boxed_slice(),
            },
            binding_objects,
            state_objects,
            readback_objects,
            readback_device_objects,
            integrity_readback_objects,
            device_objects,
            writable_device_objects: writable_device_objects.into_boxed_slice(),
            transfer_objects,
        })
    }

    pub fn binding_bytes(&self, binding: u32) -> Option<u64> {
        let object = self.binding_objects.get(&binding)?;
        self.memory
            .allocations
            .iter()
            .find(|allocation| allocation.id == *object)
            .map(|allocation| allocation.capacity_bytes)
    }

    pub fn binding_object(&self, binding: u32) -> Option<MemoryObjectId> {
        self.binding_objects.get(&binding).copied()
    }

    pub fn state_objects(&self, slot: mech_core::CellSlotId) -> Option<[MemoryObjectId; 2]> {
        self.state_objects.get(&slot).copied()
    }

    pub fn readback_object(&self, slot: mech_core::CellSlotId) -> Option<MemoryObjectId> {
        self.readback_objects.get(&slot).copied()
    }

    pub fn readback_device_object(&self, slot: mech_core::CellSlotId) -> Option<MemoryObjectId> {
        self.readback_device_objects.get(&slot).copied()
    }

    pub const fn integrity_readback_objects(&self) -> Option<[MemoryObjectId; 2]> {
        self.integrity_readback_objects
    }

    pub fn managed_memory(&self) -> Result<ManagedGpuMemory, GpuMemoryPlanError> {
        ManagedGpuMemory::realize(&self.memory)
    }

    pub fn device_objects(&self) -> &[MemoryObjectId] {
        &self.device_objects
    }

    pub fn writable_device_objects(&self) -> &[(MemoryObjectId, u64)] {
        &self.writable_device_objects
    }

    pub fn transfer_objects(&self) -> &[MemoryObjectId] {
        &self.transfer_objects
    }

    pub fn state_bytes(&self, slot: mech_core::CellSlotId) -> Option<u64> {
        let [current, next] = self.state_objects.get(&slot)?;
        let current = self
            .memory
            .allocations
            .iter()
            .find(|allocation| allocation.id == *current)?;
        let next = self
            .memory
            .allocations
            .iter()
            .find(|allocation| allocation.id == *next)?;
        (current.capacity_bytes == next.capacity_bytes).then_some(current.capacity_bytes)
    }

    pub fn assert_binding_bytes(
        &self,
        binding: u32,
        actual: u64,
    ) -> Result<(), GpuMemoryPlanError> {
        let planned = self
            .binding_bytes(binding)
            .ok_or(GpuMemoryPlanError::MissingBinding { binding })?;
        if planned != actual {
            return Err(GpuMemoryPlanError::BufferSizeMismatch {
                binding,
                planned,
                actual,
            });
        }
        Ok(())
    }

    pub fn assert_readback_bytes(
        &self,
        slot: mech_core::CellSlotId,
        actual: u64,
    ) -> Result<(), GpuMemoryPlanError> {
        let object = self
            .readback_objects
            .get(&slot)
            .ok_or(GpuMemoryPlanError::MissingReadback { slot })?;
        let planned = self
            .memory
            .allocations
            .iter()
            .find(|allocation| allocation.id == *object)
            .ok_or(GpuMemoryPlanError::MissingReadback { slot })?
            .capacity_bytes;
        if actual > planned {
            return Err(GpuMemoryPlanError::ReadbackSizeExceeded {
                slot,
                planned,
                actual,
            });
        }
        Ok(())
    }

    pub fn readback_bytes(&self, slot: mech_core::CellSlotId) -> Option<u64> {
        let object = self.readback_objects.get(&slot)?;
        self.memory
            .allocations
            .iter()
            .find(|allocation| allocation.id == *object)
            .map(|allocation| allocation.capacity_bytes)
    }
}

#[derive(Clone, Debug)]
pub enum GpuMemoryPlanError {
    Execution(GpuExecutionPlanError),
    Plan(MemoryPlanError),
    WorkgroupLimit {
        workgroup_size: u32,
        workgroups: u32,
    },
    MissingBinding {
        binding: u32,
    },
    MissingReadback {
        slot: mech_core::CellSlotId,
    },
    BufferSizeMismatch {
        binding: u32,
        planned: u64,
        actual: u64,
    },
    ReadbackSizeExceeded {
        slot: mech_core::CellSlotId,
        planned: u64,
        actual: u64,
    },
    MissingPlanObject {
        object: MemoryObjectId,
    },
    DuplicateDeviceAllocation {
        object: MemoryObjectId,
    },
    ContentVersionExhausted {
        object: MemoryObjectId,
    },
    Runtime(MemoryRuntimeError),
}

impl core::fmt::Display for GpuMemoryPlanError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GpuMemoryPlanError {}

impl From<GpuExecutionPlanError> for GpuMemoryPlanError {
    fn from(error: GpuExecutionPlanError) -> Self {
        Self::Execution(error)
    }
}

impl From<MemoryPlanError> for GpuMemoryPlanError {
    fn from(error: MemoryPlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<MemoryRuntimeError> for GpuMemoryPlanError {
    fn from(error: MemoryRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

fn checked_binding_bytes(elements: u64, scalar: GpuPlanScalar) -> Result<u64, MemoryPlanError> {
    let bytes = match scalar {
        GpuPlanScalar::F32 | GpuPlanScalar::U32 => 4,
    };
    elements
        .checked_mul(bytes)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "GPU binding bytes",
        })
}

fn checked_next(value: u32) -> Result<u32, MemoryPlanError> {
    value
        .checked_add(1)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "GPU memory-object id",
        })
}

fn push_gpu_allocation(
    allocations: &mut Vec<AllocationPlan>,
    arenas: &mut Vec<ArenaPlan>,
    id: MemoryObjectId,
    owner: MemoryObjectOwner,
    role: AllocationRole,
    space: MemorySpace,
    bytes: u64,
    lifetime: MemoryLifetime,
    adapter_alignment: u32,
) -> Result<(), MemoryPlanError> {
    if bytes == 0 {
        return Err(MemoryPlanError::ZeroSizedGpuBinding);
    }
    let alignment = adapter_alignment.max(4);
    if !alignment.is_power_of_two() {
        return Err(MemoryPlanError::InvalidAlignment { alignment });
    }
    let arena = MemoryArenaId::new(id.get());
    allocations.push(AllocationPlan {
        id,
        owner,
        role,
        slot: None,
        space,
        current_bytes: bytes,
        capacity_bytes: bytes,
        payload_block_capacity: 0,
        alignment,
        lifetime,
        placement: ArenaPlacement { arena, offset: 0 },
        reuse_group: None,
    });
    arenas.push(ArenaPlan {
        id: arena,
        space,
        backing: ArenaBackingKind::ContiguousBytes,
        alignment,
        capacity_bytes: bytes,
        members: vec![id].into_boxed_slice(),
    });
    Ok(())
}

fn demand_for_gpu_allocation(allocation: &AllocationPlan, bindings: u32) -> ResourceDemand {
    let mut demand = ResourceDemand {
        storage_bindings: bindings,
        ..ResourceDemand::default()
    };
    match allocation.lifetime {
        MemoryLifetime::Transfer { .. } => demand.transfer_bytes = allocation.capacity_bytes,
        _ => demand.activation_bytes = allocation.capacity_bytes,
    }
    demand
}

#[cfg(feature = "native")]
pub fn gpu_memory_limits(limits: &wgpu::Limits) -> GpuMemoryLimits {
    GpuMemoryLimits {
        max_buffer_size: limits.max_buffer_size,
        max_storage_buffer_binding_size: u64::from(limits.max_storage_buffer_binding_size),
        max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
        max_bindings_per_bind_group: limits.max_bindings_per_bind_group,
        max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
        max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
        max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
        min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_plan::test_execution_plan;

    fn limits(max_buffer_size: u64) -> GpuMemoryLimits {
        GpuMemoryLimits {
            max_buffer_size,
            max_storage_buffer_binding_size: max_buffer_size,
            max_storage_buffers_per_shader_stage: 8,
            max_bindings_per_bind_group: 8,
            max_compute_workgroups_per_dimension: 65_535,
            max_compute_invocations_per_workgroup: 256,
            max_compute_workgroup_size_x: 256,
            min_storage_buffer_offset_alignment: 4,
        }
    }

    #[test]
    fn exact_gpu_binding_bytes_are_planned_before_creation() {
        let planned =
            PlannedGpuExecution::from_execution(test_execution_plan(2), limits(1024)).unwrap();
        assert_eq!(planned.binding_bytes(0), Some(8));
        assert!(planned.assert_binding_bytes(0, 8).is_ok());
        assert!(planned.assert_binding_bytes(0, 4).is_err());
    }

    #[test]
    fn adapter_buffer_limit_is_a_structured_plan_rejection() {
        assert!(matches!(
            PlannedGpuExecution::from_execution(test_execution_plan(2), limits(7)),
            Err(GpuMemoryPlanError::Plan(
                MemoryPlanError::TargetLimitExceeded { .. }
            ))
        ));
    }

    #[cfg(feature = "native")]
    #[test]
    fn unsubmitted_tracker_batch_cancels_its_reserved_holds() {
        let planned =
            PlannedGpuExecution::from_execution(test_execution_plan(2), limits(1024)).unwrap();
        let object = planned.binding_object(0).unwrap();
        let mut memory = planned.managed_memory().unwrap();
        memory.attach_device_allocation(object, 8, 8).unwrap();
        let hold = memory.begin_submission(&[object], &[]).unwrap();
        let mut tracker = DeviceSubmissionTracker::new(1).unwrap();

        {
            let _unsubmitted = tracker.reserve_batch(core::iter::once(hold)).unwrap();
            assert_eq!(memory.ledger().in_flight_device_bytes, 8);
            assert_eq!(memory.allocations()[0].submission_pins, 1);
        }

        assert!(!tracker.has_pending());
        assert_eq!(memory.ledger().in_flight_device_bytes, 0);
        assert_eq!(memory.allocations()[0].submission_pins, 0);
    }

    #[cfg(feature = "native")]
    #[test]
    fn submitted_batch_stays_pinned_through_mapping_failure_until_completion() {
        let instance = wgpu::Instance::default();
        let Some(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
        else {
            return;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Mech submission ownership test"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )) else {
            return;
        };
        let planned =
            PlannedGpuExecution::from_execution(test_execution_plan(2), limits(1024)).unwrap();
        let object = planned.binding_object(0).unwrap();
        let mut memory = planned.managed_memory().unwrap();
        memory.attach_device_allocation(object, 8, 8).unwrap();
        let hold = memory.begin_submission(&[object], &[]).unwrap();
        let mut tracker = DeviceSubmissionTracker::new(1).unwrap();
        tracker.observe_device_loss(&device);
        let unsubmitted = tracker.reserve_batch(core::iter::once(hold)).unwrap();

        let submission = queue.submit(core::iter::empty());
        unsubmitted.record_submitted(submission);
        let mapping_result: Result<(), &'static str> = Err("injected mapping failure");
        assert!(mapping_result.is_err());
        assert_eq!(memory.ledger().in_flight_device_bytes, 8);
        assert_eq!(memory.allocations()[0].submission_pins, 1);

        tracker.wait_for_submitted(&device).unwrap();
        tracker.reap().unwrap();
        assert!(!tracker.has_pending());
        assert_eq!(memory.ledger().in_flight_device_bytes, 0);
        assert_eq!(memory.allocations()[0].submission_pins, 0);
    }
}
