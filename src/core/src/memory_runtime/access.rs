use crate::{CanonicalCellId, MemoryObjectId};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, vec::Vec};

use core::{
    cell::{RefCell, RefMut},
    marker::PhantomData,
    mem,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
};

use super::{
    ActiveLeaseRecord, AllocationHandle, MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult,
    OwnedAllocationState, PlanObjectKey, RealizedMemoryPlan, RuntimeBinding,
};
use crate::{MemoryLifetime, MemoryPlanPoint};

/// Sealed copy primitive for a pair of already validated owned host regions.
/// Policy and ownership checks remain in the safe domain module; raw access is
/// confined to this module with the other typed-view primitives.
pub(crate) fn copy_prevalidated_host_bytes(
    source: NonNull<u8>,
    source_offset: usize,
    destination: NonNull<u8>,
    destination_offset: usize,
    bytes: usize,
) {
    // SAFETY: the caller has retained both allocation owners and validated
    // each offset plus the common byte count against the corresponding layout.
    // `ptr::copy` deliberately permits two regions in one arena to overlap.
    unsafe {
        core::ptr::copy(
            source.as_ptr().add(source_offset),
            destination.as_ptr().add(destination_offset),
            bytes,
        );
    }
}

pub(crate) fn copy_prevalidated_initialization(
    regions: &mut BTreeMap<PlanObjectKey, super::RuntimeRegionRecord>,
    source: PlanObjectKey,
    destination: PlanObjectKey,
) {
    let source = regions.get(&source).expect("undo source was prevalidated")
        as *const super::RuntimeRegionRecord;
    let destination = regions
        .get_mut(&destination)
        .expect("undo destination was prevalidated");
    // SAFETY: source and destination are distinct validated plan keys and the
    // map is not structurally modified while the two records are accessed.
    let source = unsafe { &*source };
    destination
        .initialization
        .copy_from_prevalidated(&source.initialization);
    destination.initialized_bytes = source.initialized_bytes;
}

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
pub struct ManagedPort<T> {
    cell: crate::ValueCell,
    role: ManagedPortRole,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for ManagedPort<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
            role: self.role,
            marker: PhantomData,
        }
    }
}

impl<T> core::fmt::Debug for ManagedPort<T> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ManagedPort")
            .field("cell", &self.cell.reactive_cell_id())
            .field("role", &self.role)
            .finish()
    }
}

impl<T> ManagedPort<T> {
    #[cfg(feature = "functions")]
    pub(crate) fn input(cell: crate::ValueCell, index: usize) -> Self {
        Self {
            cell,
            role: ManagedPortRole::Input(index),
            marker: PhantomData,
        }
    }

    #[cfg(feature = "functions")]
    pub(crate) fn output(cell: crate::ValueCell) -> Self {
        Self {
            cell,
            role: ManagedPortRole::Output(0),
            marker: PhantomData,
        }
    }

    pub fn logical_cell_id(&self) -> CanonicalCellId {
        self.cell.reactive_cell_id()
    }

    pub const fn role(&self) -> ManagedPortRole {
        self.role
    }

    /// Stable logical cell retained for compilation, diagnostics, and
    /// publication metadata. Physical storage is always resolved by a live
    /// managed frame.
    pub const fn cell(&self) -> &crate::ValueCell {
        &self.cell
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
    pub fn new<T>(
        port: &ManagedPort<T>,
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
pub trait ManagedElement: managed_element_sealed::Sealed + Copy + 'static {
    const SLOT: crate::PlannedSlotKind;
}

/// Physical lane for semantic identifiers. Keeping this distinct from `u64`
/// prevents an identifier plan slot from being opened through the unsigned
/// integer codec merely because both lanes have the same size and alignment.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct ManagedId(pub(crate) u64);

macro_rules! managed_elements {
    ($($type:ty => $slot:expr),+ $(,)?) => {$(
        impl managed_element_sealed::Sealed for $type {}
        impl ManagedElement for $type {
            const SLOT: crate::PlannedSlotKind = $slot;
        }
    )+};
}

managed_elements!(
    u8 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Unsigned(crate::IntegerWidth::W8)),
    u16 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Unsigned(crate::IntegerWidth::W16)),
    u32 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Unsigned(crate::IntegerWidth::W32)),
    u64 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Unsigned(crate::IntegerWidth::W64)),
    u128 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Unsigned(crate::IntegerWidth::W128)),
    i8 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Signed(crate::IntegerWidth::W8)),
    i16 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Signed(crate::IntegerWidth::W16)),
    i32 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Signed(crate::IntegerWidth::W32)),
    i64 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Signed(crate::IntegerWidth::W64)),
    i128 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Signed(crate::IntegerWidth::W128)),
    f32 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Floating(crate::FloatWidth::W32)),
    f64 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Floating(crate::FloatWidth::W64)),
    usize => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Index)
);

managed_elements!(
    ManagedId => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Id)
);

#[cfg(feature = "bool")]
managed_elements!(
    bool => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Bool)
);

#[cfg(feature = "complex")]
managed_elements!(
    crate::C64 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Complex(crate::FloatWidth::W64))
);

#[cfg(feature = "rational")]
managed_elements!(
    crate::R64 => crate::PlannedSlotKind::FixedScalar(crate::ScalarMemoryKind::Rational64)
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
    alias_cell: Option<CanonicalCellId>,
    alias_role: Option<ManagedPortRole>,
    object: PlanObjectKey,
    mode: MemoryAccessMode,
    lifetime: MemoryLifetime,
    region: MemoryAccessRegion,
}

#[derive(Debug)]
pub struct PreparedCallAccess {
    revision: super::MemoryPlanRevision,
    requests: Box<[ResolvedAccessRequest]>,
    logical_ports: Box<[PreparedLogicalPort]>,
    authority: CallAccessAuthority,
    undo: Option<PreparedUndoAccess>,
    workspace: RefCell<CallAccessWorkspace>,
}

#[derive(Clone, Copy, Debug)]
struct PreparedUndoAccess {
    input: usize,
    undo: PlanObjectKey,
}

/// Armed rollback authority for one admitted in-place transaction. Dropping
/// it before publication restores the exact bytes and initialization map.
pub(crate) struct PreparedUndoSnapshot {
    realized: RealizedMemoryPlan,
    target: PlanObjectKey,
    undo: PlanObjectKey,
    armed: bool,
    retained_lease: Option<RetainedPublicationLease>,
}

impl PreparedUndoSnapshot {
    pub(crate) fn commit(&mut self) {
        self.armed = false;
    }

    pub(crate) fn matches(&self, realized: &RealizedMemoryPlan, target: PlanObjectKey) -> bool {
        self.realized.domain() == realized.domain()
            && self.realized.revision() == realized.revision()
            && self.target == target
    }

    fn restore(&mut self) {
        if !self.armed {
            return;
        }
        self.realized
            .copy_undo_image(self.undo, self.target)
            .expect("an armed undo snapshot retains its prevalidated storage");
        self.armed = false;
    }
}

struct RetainedPublicationLease {
    domain: MemoryDomain,
    _realized: RealizedMemoryPlan,
    handle: AllocationHandle,
    token: u64,
}

impl Drop for RetainedPublicationLease {
    fn drop(&mut self) {
        let mut state = self.domain.state.borrow_mut();
        let record = state
            .record_mut(self.handle)
            .expect("retained publication lease keeps its allocation live");
        let position = record
            .leases
            .iter()
            .position(|lease| lease.token == self.token)
            .expect("retained publication lease keeps its token installed");
        record.leases.remove(position);
    }
}

impl Drop for PreparedUndoSnapshot {
    fn drop(&mut self) {
        self.restore();
    }
}

#[derive(Debug)]
enum CallAccessAuthority {
    ActivePlan,
    OwnedInitialization,
    PublishedCell {
        cell: crate::ValueCell,
        update: bool,
    },
}

/// Only logical cells and planner object coordinates survive preparation.
/// Acquisition resolves these against the current published bindings.
#[derive(Debug)]
struct PreparedLogicalPort {
    cell: crate::ValueCell,
    role: ManagedPortRole,
    transaction_pair: Option<(MemoryObjectId, MemoryObjectId)>,
}

#[derive(Debug)]
struct CallAccessWorkspace {
    leases: Vec<HeldLease>,
}

impl Deref for CallAccessWorkspace {
    type Target = [HeldLease];

    fn deref(&self) -> &Self::Target {
        &self.leases
    }
}

impl DerefMut for CallAccessWorkspace {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.leases
    }
}

impl MemoryDomain {
    #[cfg(feature = "functions")]
    pub fn prepare_function_call(
        &self,
        realized: &RealizedMemoryPlan,
        plan: &crate::CallMemoryPlan,
        invocation: &crate::FunctionInvocation,
    ) -> MemoryRuntimeResult<PreparedCallAccess> {
        if !(plan.inputs.len() == invocation.input_cells().len()
            || plan.inputs.len() == invocation.input_cells().len().saturating_add(1))
            || plan.outputs.len() > 1
            || plan.transactions.len() != plan.outputs.len()
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "function invocation, outputs, and transaction authorities differ in arity"
                    .into(),
            });
        }
        let mut resolved = Vec::new();
        let mut logical_ports = Vec::new();
        let mut prepared_undo = None;
        logical_ports
            .try_reserve_exact(plan.inputs.len() + plan.outputs.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: (plan.inputs.len() + plan.outputs.len()) as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            })?;
        resolved
            .try_reserve_exact(plan.allocations.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: plan.allocations.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            })?;
        for (index, input) in plan.inputs.iter().enumerate() {
            let cell = invocation.planned_input_cell(plan, index).map_err(|_| {
                MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(input.object),
                    reason: "semantic input cannot be mapped to the physical invocation".into(),
                }
            })?;
            let planned_object = self.plan_object_key(realized.revision(), input.object)?;
            let (object, binding, region, lifetime) = if let Some(live) = cell
                .managed_host_binding()
                .map_err(|_| MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(planned_object.object()),
                    reason: "logical input storage could not be resolved".into(),
                })?
                .filter(|live| {
                    live.realized.domain() == self.id()
                        && live.realized.revision() == realized.revision()
                }) {
                let binding = realized.binding(live.object)?;
                let lifetime = realized.lifetime(live.object)?;
                (live.object, binding, live.region, lifetime)
            } else {
                let binding = realized.binding(planned_object)?;
                (
                    planned_object,
                    binding,
                    planned_value_access_region(&input.value)?,
                    realized.lifetime(planned_object)?,
                )
            };
            // Binding validates geometry before the input's planned import
            // is initialized. Read initialization is checked at acquisition.
            let _ = enclosing_span(object.object(), &binding, MemoryAccessMode::Write, region)?;
            resolved.push(ResolvedAccessRequest {
                cell: Some(cell.reactive_cell_id()),
                role: Some(ManagedPortRole::Input(index)),
                alias_cell: None,
                alias_role: None,
                object,
                mode: MemoryAccessMode::Read,
                lifetime,
                region,
            });
            // Retain every logical input, including values currently copied
            // through an admitted import slot. A later publication may move
            // that same cell into this realization; acquisition must resolve
            // the live binding instead of preserving the preparation-time
            // physical address.
            logical_ports.push(PreparedLogicalPort {
                cell: cell.clone(),
                role: ManagedPortRole::Input(index),
                transaction_pair: None,
            });
        }
        for (index, output) in plan.outputs.iter().enumerate() {
            let transaction = plan.transactions[index];
            let target = match transaction {
                crate::TransactionRequirement::StageAndSwap { staged, .. } => staged,
                crate::TransactionRequirement::DoubleBuffer { next, .. } => next,
                crate::TransactionRequirement::UndoSnapshot { target, .. } => target,
                crate::TransactionRequirement::None => output.object,
            };
            let object = self.plan_object_key(realized.revision(), target)?;
            let binding = realized.binding(object)?;
            let region = match transaction {
                // A staged read-modify-write candidate must first become a
                // complete initialized value before publication. The kernel
                // still applies only its resolved ordered indices; this
                // exclusive lease is authority over the unpublished stage,
                // not authority to broaden mutation of the published value.
                crate::TransactionRequirement::StageAndSwap { .. }
                | crate::TransactionRequirement::DoubleBuffer { .. } => {
                    planned_value_access_region(&output.value)?
                }
                _ if matches!(output.region, crate::RegionAccessPlan::WholeValue) => {
                    planned_value_access_region(&output.value)?
                }
                _ => region_access_for_port(
                    &output.region,
                    output.value.current_address_span_bytes,
                    output.value.slot.bytes,
                )?,
            };
            let _ = enclosing_span(object.object(), &binding, MemoryAccessMode::Write, region)?;
            if let crate::TransactionRequirement::UndoSnapshot { undo, .. } = transaction {
                let input = match plan.aliases.get(index) {
                    Some(crate::AliasDecision::InPlaceRequired { input }) => *input as usize,
                    _ => {
                        return Err(MemoryRuntimeError::CandidateValidationFailed {
                            object: Some(object.object()),
                            reason: "undo transaction has no required in-place alias".into(),
                        });
                    }
                };
                let input_request = resolved
                    .iter_mut()
                    .find(|request| request.role == Some(ManagedPortRole::Input(input)))
                    .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                        object: Some(object.object()),
                        reason: "undo transaction target is not a planned logical input".into(),
                    })?;
                input_request.mode = MemoryAccessMode::ExclusiveInPlace;
                input_request.alias_cell = Some(invocation.output_cell().reactive_cell_id());
                input_request.alias_role = Some(ManagedPortRole::Output(index));
                let undo = self.plan_object_key(realized.revision(), undo)?;
                let undo_binding = realized.binding(undo)?;
                let undo_region = planned_value_access_region(&output.value)?;
                let _ = enclosing_span(
                    undo.object(),
                    &undo_binding,
                    MemoryAccessMode::Write,
                    undo_region,
                )?;
                resolved.push(ResolvedAccessRequest {
                    cell: None,
                    role: None,
                    alias_cell: None,
                    alias_role: None,
                    object: undo,
                    mode: MemoryAccessMode::Write,
                    lifetime: realized.lifetime(undo)?,
                    region: undo_region,
                });
                prepared_undo = Some(PreparedUndoAccess { input, undo });
            } else {
                resolved.push(ResolvedAccessRequest {
                    cell: Some(invocation.output_cell().reactive_cell_id()),
                    role: Some(ManagedPortRole::Output(index)),
                    alias_cell: None,
                    alias_role: None,
                    object,
                    mode: MemoryAccessMode::Write,
                    lifetime: realized.lifetime(object)?,
                    region,
                });
            }
            let transaction_pair = match transaction {
                crate::TransactionRequirement::StageAndSwap { current, staged } => {
                    Some((current, staged))
                }
                crate::TransactionRequirement::DoubleBuffer { current, next } => {
                    Some((current, next))
                }
                _ => None,
            };
            logical_ports.push(PreparedLogicalPort {
                cell: invocation.output_cell().clone(),
                role: ManagedPortRole::Output(index),
                transaction_pair,
            });
        }
        for allocation in plan.allocations.iter().filter(|allocation| {
            matches!(
                allocation.role,
                crate::AllocationRole::Scratch
                    | crate::AllocationRole::SelectorPlan
                    | crate::AllocationRole::OrderedIndex
            )
        }) {
            let object = self.plan_object_key(realized.revision(), allocation.id)?;
            let binding = realized.binding(object)?;
            let region = MemoryAccessRegion::Contiguous {
                offset_bytes: 0,
                length_bytes: allocation.current_bytes,
            };
            let _ = enclosing_span(object.object(), &binding, MemoryAccessMode::Write, region)?;
            resolved.push(ResolvedAccessRequest {
                cell: None,
                role: None,
                alias_cell: None,
                alias_role: None,
                object,
                mode: MemoryAccessMode::Write,
                lifetime: realized.lifetime(object)?,
                region,
            });
        }
        resolved.sort_by_key(|request| {
            (
                request.object,
                request.cell,
                request.role,
                request.mode,
                request.region,
            )
        });
        resolved.dedup();
        let mut leases = Vec::new();
        leases.try_reserve_exact(resolved.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: resolved.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        Ok(PreparedCallAccess {
            revision: realized.revision(),
            requests: resolved.into_boxed_slice(),
            logical_ports: logical_ports.into_boxed_slice(),
            authority: CallAccessAuthority::ActivePlan,
            undo: prepared_undo,
            workspace: RefCell::new(CallAccessWorkspace { leases }),
        })
    }

    #[cfg(feature = "functions")]
    pub(crate) fn prepare_function_input_initialization(
        &self,
        realized: &RealizedMemoryPlan,
        plan: &crate::CallMemoryPlan,
        invocation: &crate::FunctionInvocation,
    ) -> MemoryRuntimeResult<PreparedCallAccess> {
        if !(plan.inputs.len() == invocation.input_cells().len()
            || plan.inputs.len() == invocation.input_cells().len().saturating_add(1))
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "function invocation and input plan arity differ".into(),
            });
        }
        let mut requests = Vec::new();
        requests.try_reserve_exact(plan.inputs.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: plan.inputs.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        for (index, input) in plan.inputs.iter().enumerate() {
            let cell = invocation.planned_input_cell(plan, index).map_err(|_| {
                MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(input.object),
                    reason: "semantic input cannot be mapped to the physical invocation".into(),
                }
            })?;
            if !cell.requires_planned_import(realized).map_err(|_| {
                MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(input.object),
                    reason: "logical input storage could not be resolved".into(),
                }
            })? {
                continue;
            }
            requests.push(CallAccessRequest {
                object: self.plan_object_key(realized.revision(), input.object)?,
                mode: MemoryAccessMode::Write,
                region: planned_value_access_region(&input.value)?,
            });
        }
        self.prepare_call(realized, &requests)
    }

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
            let _ = enclosing_span(
                request.object.object(),
                &binding,
                request.mode,
                request.region,
            )?;
            resolved.push(ResolvedAccessRequest {
                cell: None,
                role: None,
                alias_cell: None,
                alias_role: None,
                object: request.object,
                mode: request.mode,
                lifetime: realized.lifetime(request.object)?,
                region: request.region,
            });
        }
        resolved.sort_by_key(|request| (request.object, request.mode, request.region));
        resolved.dedup();
        let mut leases = Vec::new();
        leases.try_reserve_exact(resolved.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: resolved.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        Ok(PreparedCallAccess {
            revision: realized.revision(),
            requests: resolved.into_boxed_slice(),
            logical_ports: Box::default(),
            authority: CallAccessAuthority::ActivePlan,
            undo: None,
            workspace: RefCell::new(CallAccessWorkspace { leases }),
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
            let _ = enclosing_span(
                request.object.object(),
                &binding,
                request.mode,
                request.region,
            )?;
            resolved.push(ResolvedAccessRequest {
                cell: Some(request.cell),
                role: Some(request.role),
                alias_cell: None,
                alias_role: None,
                object: request.object,
                mode: request.mode,
                lifetime: realized.lifetime(request.object)?,
                region: request.region,
            });
        }
        resolved.sort_by_key(|request| {
            (
                request.object,
                request.cell,
                request.role,
                request.mode,
                request.region,
            )
        });
        resolved.dedup();
        let mut leases = Vec::new();
        leases.try_reserve_exact(resolved.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: resolved.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        Ok(PreparedCallAccess {
            revision: realized.revision(),
            requests: resolved.into_boxed_slice(),
            logical_ports: Box::default(),
            authority: CallAccessAuthority::ActivePlan,
            undo: None,
            workspace: RefCell::new(CallAccessWorkspace { leases }),
        })
    }

    pub fn acquire_call<'a>(
        &'a self,
        realized: &'a RealizedMemoryPlan,
        prepared: &'a PreparedCallAccess,
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

        let mut workspace = prepared.workspace.try_borrow_mut().map_err(|_| {
            MemoryRuntimeError::BorrowConflict {
                object: prepared
                    .requests
                    .first()
                    .map(|request| request.object.object())
                    .unwrap_or(MemoryObjectId::new(0)),
            }
        })?;
        workspace.leases.clear();
        for request in &prepared.requests {
            let mut request = *request;
            if let Some(logical) = prepared.logical_ports.iter().find(|logical| {
                Some(logical.cell.reactive_cell_id()) == request.cell
                    && Some(logical.role) == request.role
            }) {
                let live = logical.cell.managed_host_binding().map_err(|_| {
                    MemoryRuntimeError::CandidateValidationFailed {
                        object: Some(request.object.object()),
                        reason: "logical port binding is unavailable at acquisition".into(),
                    }
                })?;
                if let Some(live) = live {
                    match logical.role {
                        ManagedPortRole::Input(_)
                            if live.realized.domain() == self.id()
                                && live.realized.revision() == realized.revision() =>
                        {
                            request.object = live.object;
                            request.region = live.region;
                            request.lifetime = realized.lifetime(request.object)?;
                        }
                        // Cross-session and revised-plan inputs remain in the
                        // call's admitted import object. initialize_inputs has
                        // refreshed that object from the current logical value
                        // immediately before this acquisition.
                        ManagedPortRole::Input(_) => {}
                        ManagedPortRole::Output(_) => {
                            if live.realized.domain() != self.id() {
                                return Err(MemoryRuntimeError::WrongMemoryDomain {
                                    expected: self.id(),
                                    actual: live.realized.domain(),
                                });
                            }
                            if live.realized.revision() == realized.revision()
                                && let Some((current, staged)) = logical.transaction_pair
                            {
                                let candidate = if live.object.object() == staged {
                                    current
                                } else if live.object.object() == current {
                                    staged
                                } else {
                                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                                        object: Some(live.object.object()),
                                        reason: "published output is outside its planned transaction pair".into(),
                                    });
                                };
                                request.object =
                                    self.plan_object_key(realized.revision(), candidate)?;
                                request.lifetime = realized.lifetime(request.object)?;
                            }
                        }
                    }
                }
            }
            let binding = realized.binding(request.object)?;
            let (start, end, relative_end) = enclosing_span(
                request.object.object(),
                &binding,
                request.mode,
                request.region,
            )?;
            workspace.leases.push(HeldLease {
                token: None,
                handle: binding.handle(),
                object: request.object,
                cell: request.cell,
                role: request.role,
                alias_cell: request.alias_cell,
                alias_role: request.alias_role,
                mode: request.mode,
                start,
                end,
                relative_end,
                region: request.region,
                lifetime: request.lifetime,
                incarnation: binding.incarnation(),
                owns_lease: true,
            });
        }

        // An in-place output and every repeated input role for the same
        // logical cell share one physical exclusive lease. Keep lightweight
        // role entries for typed port lookup, but install exactly one token.
        for exclusive in 0..workspace.leases.len() {
            if workspace.leases[exclusive].mode != MemoryAccessMode::ExclusiveInPlace {
                continue;
            }
            let owner = workspace.leases[exclusive];
            for other in 0..workspace.leases.len() {
                if other == exclusive
                    || workspace.leases[other].cell != owner.cell
                    || workspace.leases[other].mode != MemoryAccessMode::Read
                    || workspace.leases[other].object != owner.object
                {
                    continue;
                }
                if workspace.leases[other].start < owner.start
                    || workspace.leases[other].end > owner.end
                {
                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                        object: Some(owner.object.object()),
                        reason: "repeated in-place input exceeds the exclusive output region"
                            .into(),
                    });
                }
                workspace.leases[other].owns_lease = false;
            }
        }

        let retained_owner = match &prepared.authority {
            CallAccessAuthority::ActivePlan => false,
            CallAccessAuthority::OwnedInitialization => {
                if workspace.leases.iter().any(|lease| {
                    !lease.mode.writes()
                        || !realized
                            .binding(lease.object)
                            .is_ok_and(|binding| binding.initialized_bytes() == 0)
                }) {
                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                        object: None,
                        reason: "construction authority cannot read storage".into(),
                    });
                }
                true
            }
            CallAccessAuthority::PublishedCell { cell, update } => {
                let live = cell
                    .managed_host_binding()
                    .map_err(|_| MemoryRuntimeError::DomainClosed)?
                    .ok_or(MemoryRuntimeError::CandidateValidationFailed {
                        object: None,
                        reason: "published cell no longer has managed host storage".into(),
                    })?;
                if live.realized.domain() != self.id()
                    || live.realized.revision() != realized.revision()
                {
                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                        object: Some(live.object.object()),
                        reason: "published cell binding was replaced after preparation".into(),
                    });
                }
                for lease in workspace.leases.iter() {
                    let valid = if *update {
                        lease.mode == MemoryAccessMode::Write
                            && realized.transactions().iter().any(|transaction| {
                                let (current, next) = match transaction {
                                    crate::TransactionRequirement::StageAndSwap {
                                        current,
                                        staged,
                                    } => (*current, *staged),
                                    crate::TransactionRequirement::DoubleBuffer {
                                        current,
                                        next,
                                    } => (*current, *next),
                                    _ => return false,
                                };
                                (live.object.object() == current && lease.object.object() == next)
                                    || (live.object.object() == next
                                        && lease.object.object() == current)
                            })
                    } else {
                        lease.mode == MemoryAccessMode::Read
                            && lease.object == live.object
                            && lease.region == live.region
                    };
                    if !valid {
                        return Err(MemoryRuntimeError::CandidateValidationFailed {
                            object: Some(lease.object.object()),
                            reason: "cell access differs from its live publication authority"
                                .into(),
                        });
                    }
                }
                true
            }
        };
        let undo_coordinates = if let Some(undo) = prepared.undo {
            let target = workspace
                .leases
                .iter()
                .find(|lease| lease.role == Some(ManagedPortRole::Input(undo.input)))
                .map(|lease| lease.object)
                .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(undo.undo.object()),
                    reason: "undo transaction lost its resolved in-place target".into(),
                })?;
            Some((target, undo.undo))
        } else {
            None
        };
        let state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if !retained_owner
            && state.execution_revision.or(state.active_revision) != Some(realized.revision())
        {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(realized.revision()),
                actual: realized.revision(),
            });
        }
        for (position, request) in workspace.leases.iter().enumerate() {
            let region = state.regions.get(&request.object).ok_or(
                MemoryRuntimeError::UnknownPlanObject {
                    key: request.object,
                },
            )?;
            if region.handle != request.handle || region.incarnation != request.incarnation {
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
            if request.mode != MemoryAccessMode::Write
                && !region
                    .initialization
                    .contains_region(request.region, region.initialized_bytes)
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: request.object.object(),
                    requested: request.relative_end,
                    initialized: region.initialized_bytes,
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
            if record.arena_projection_owner.upgrade().is_some() {
                return Err(MemoryRuntimeError::BorrowConflict {
                    object: request.object.object(),
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
            if request.owns_lease
                && record.leases.iter().any(|lease| {
                    overlaps(request.start, request.end, lease.start, lease.end)
                        && (request.mode.writes() || lease.write)
                })
            {
                return Err(MemoryRuntimeError::BorrowConflict {
                    object: request.object.object(),
                });
            }
            if request.owns_lease
                && workspace.leases[..position].iter().any(|other| {
                    other.owns_lease
                        && other.handle == Some(handle)
                        && overlaps(request.start, request.end, other.start, other.end)
                        && (request.mode.writes() || other.mode.writes())
                })
            {
                return Err(MemoryRuntimeError::BorrowConflict {
                    object: request.object.object(),
                });
            }
        }
        let physical_count = workspace
            .leases
            .iter()
            .filter(|request| request.owns_lease && request.handle.is_some())
            .count();
        let token_increment =
            u64::try_from(physical_count).map_err(|_| MemoryRuntimeError::IdentityExhausted {
                identity: "lease token",
            })?;
        let next_token = state.next_lease_token.checked_add(token_increment).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "lease token",
            },
        )?;
        for request in &workspace.leases {
            if !request.owns_lease {
                continue;
            }
            let Some(handle) = request.handle else {
                continue;
            };
            let requested = workspace
                .leases
                .iter()
                .filter(|candidate| candidate.owns_lease && candidate.handle == Some(handle))
                .count();
            let record = state.record(handle)?;
            if record.leases.len().saturating_add(requested) > record.leases.capacity() {
                return Err(MemoryRuntimeError::UnplannedAllocation {
                    object: Some(request.object.object()),
                    requested: requested as u64,
                });
            }
        }
        drop(state);
        if let Some((target, undo)) = undo_coordinates {
            // Every conflict, lifetime, capacity, metadata-slot and token check
            // has completed. Snapshot construction changes only the admitted
            // undo object and cannot expose a partially acquired call.
            realized.copy_undo_image(target, undo)?;
        }
        let mut state = self.state.borrow_mut();
        let mut token = state.next_lease_token;
        for request in &mut workspace.leases {
            let token = if request.owns_lease {
                request.handle.map(|handle| {
                    let installed = token;
                    token += 1;
                    state
                        .record_mut(handle)
                        .expect("lease installation was completely prevalidated")
                        .leases
                        .push(ActiveLeaseRecord {
                            token: installed,
                            start: request.start,
                            end: request.end,
                            write: request.mode.writes(),
                        });
                    installed
                })
            } else {
                None
            };
            request.token = token;
        }
        state.next_lease_token = next_token;
        drop(state);
        let undo_snapshot = if let Some((target, undo)) = undo_coordinates {
            Some(PreparedUndoSnapshot {
                realized: realized.clone(),
                target,
                undo,
                armed: true,
                retained_lease: None,
            })
        } else {
            None
        };
        Ok(KernelMemoryFrame {
            domain: self,
            realized,
            leases: workspace,
            #[cfg(feature = "functions")]
            staged_canonical_output: None,
            undo_snapshot,
        })
    }

    pub(crate) fn prepare_owned_initialization(
        &self,
        realized: &RealizedMemoryPlan,
        request: CallAccessRequest,
    ) -> MemoryRuntimeResult<PreparedCallAccess> {
        if realized.binding(request.object)?.initialized_bytes() != 0 {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(request.object.object()),
                reason: "owned initialization requires fresh storage".into(),
            });
        }
        let mut prepared = self.prepare_call(realized, &[request])?;
        prepared.authority = CallAccessAuthority::OwnedInitialization;
        Ok(prepared)
    }

    pub(crate) fn prepare_cell_access(
        &self,
        realized: &RealizedMemoryPlan,
        cell: &crate::ValueCell,
        request: CallAccessRequest,
        update: bool,
    ) -> MemoryRuntimeResult<PreparedCallAccess> {
        let mut prepared = self.prepare_call(realized, &[request])?;
        prepared.authority = CallAccessAuthority::PublishedCell {
            cell: cell.clone(),
            update,
        };
        Ok(prepared)
    }
}

#[derive(Clone, Copy, Debug)]
struct HeldLease {
    token: Option<u64>,
    handle: Option<AllocationHandle>,
    object: PlanObjectKey,
    cell: Option<CanonicalCellId>,
    role: Option<ManagedPortRole>,
    alias_cell: Option<CanonicalCellId>,
    alias_role: Option<ManagedPortRole>,
    mode: MemoryAccessMode,
    start: u64,
    end: u64,
    relative_end: u64,
    region: MemoryAccessRegion,
    lifetime: MemoryLifetime,
    incarnation: super::RegionIncarnation,
    owns_lease: bool,
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
    leases: RefMut<'a, CallAccessWorkspace>,
    #[cfg(feature = "functions")]
    staged_canonical_output: Option<(PlanObjectKey, crate::Value)>,
    undo_snapshot: Option<PreparedUndoSnapshot>,
}

/// Borrowed fixed-width scalar/matrix view whose geometry comes from the
/// active call plan. It never treats capacity stride gaps as initialized
/// elements.
pub struct ManagedValueView<'a, T> {
    base: NonNull<T>,
    rows: usize,
    columns: usize,
    row_stride: usize,
    column_stride: usize,
    marker: PhantomData<&'a T>,
}

impl<T: Copy> ManagedValueView<'_, T> {
    pub const fn rows(&self) -> usize {
        self.rows
    }

    pub const fn columns(&self) -> usize {
        self.columns
    }

    pub fn len(&self) -> usize {
        self.rows.saturating_mul(self.columns)
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.columns == 0
    }

    pub fn get(&self, row: usize, column: usize) -> Option<T> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        let offset = row
            .checked_mul(self.row_stride)?
            .checked_add(column.checked_mul(self.column_stride)?)?;
        // SAFETY: construction validates the complete planned geometry and
        // the frame retains a shared lease for this view's lifetime.
        Some(unsafe { *self.base.as_ptr().add(offset) })
    }

    pub fn get_column_major(&self, index: usize) -> Option<T> {
        if self.rows == 0 {
            return None;
        }
        self.get(index % self.rows, index / self.rows)
    }
}

/// Exclusive counterpart to [`ManagedValueView`]. Only element-wise methods
/// are exposed, so strided gaps can never become Rust references.
pub struct ManagedValueViewMut<'a, T> {
    base: NonNull<MaybeUninit<T>>,
    rows: usize,
    columns: usize,
    row_stride: usize,
    column_stride: usize,
    fully_initialized: bool,
    marker: PhantomData<&'a mut T>,
}

impl<T: Copy> ManagedValueViewMut<'_, T> {
    pub const fn rows(&self) -> usize {
        self.rows
    }

    pub const fn columns(&self) -> usize {
        self.columns
    }

    pub fn len(&self) -> usize {
        self.rows.saturating_mul(self.columns)
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.columns == 0
    }

    fn write(&mut self, row: usize, column: usize, value: T) -> MemoryRuntimeResult<()> {
        if row >= self.rows || column >= self.columns {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: MemoryObjectId::new(0),
                requested: row
                    .saturating_mul(self.columns)
                    .saturating_add(column)
                    .saturating_add(1) as u64,
                capacity: self.len() as u64,
            });
        }
        let offset = row
            .checked_mul(self.row_stride)
            .and_then(|row| {
                column
                    .checked_mul(self.column_stride)
                    .and_then(|column| row.checked_add(column))
            })
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "managed view element offset",
                current: row as u64,
                change: column as u64,
            })?;
        // SAFETY: construction validates the complete geometry and the frame
        // retains one exclusive lease covering every addressed element.
        unsafe {
            self.base
                .as_ptr()
                .add(offset)
                .write(MaybeUninit::new(value))
        };
        Ok(())
    }

    fn write_column_major(&mut self, index: usize, value: T) -> MemoryRuntimeResult<()> {
        if self.rows == 0 {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: MemoryObjectId::new(0),
                requested: index.saturating_add(1) as u64,
                capacity: 0,
            });
        }
        self.write(index % self.rows, index / self.rows, value)
    }

    /// Reads one already-initialized logical element from an exclusive view.
    /// This is used by ordered read-modify-write kernels after they have
    /// initialized the complete unpublished candidate. It never exposes a
    /// second Rust reference to the region.
    pub fn get_column_major(&self, index: usize) -> Option<T> {
        if !self.fully_initialized || self.rows == 0 || index >= self.len() {
            return None;
        }
        let row = index % self.rows;
        let column = index / self.rows;
        let offset = row
            .checked_mul(self.row_stride)?
            .checked_add(column.checked_mul(self.column_stride)?)?;
        // SAFETY: construction validated the view geometry, the frame owns
        // one exclusive lease, and `fully_initialized` is set only after all
        // logical lanes were written successfully.
        Some(unsafe { self.base.as_ptr().add(offset).read().assume_init() })
    }

    /// Replaces one logical lane in an already-initialized unpublished view.
    /// Ordered callers can therefore preserve duplicate-destination semantics
    /// without allocating a temporary destination list.
    pub fn try_set_column_major(&mut self, index: usize, value: T) -> crate::MResult<()> {
        if !self.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: MemoryObjectId::new(0),
                requested: index.saturating_add(1) as u64,
                initialized: 0,
            }
            .into());
        }
        self.write_column_major(index, value)?;
        Ok(())
    }

    /// Initializes every logical output element in canonical column-major
    /// order. Success is proof of complete initialization; an error leaves
    /// the region unpublished and its initialization map unchanged.
    pub fn try_fill_column_major(
        &mut self,
        mut element: impl FnMut(usize) -> crate::MResult<T>,
    ) -> crate::MResult<()> {
        for index in 0..self.len() {
            let value = element(index)?;
            self.write_column_major(index, value)?;
        }
        self.fully_initialized = true;
        Ok(())
    }
}

impl KernelMemoryFrame<'_> {
    /// Opens two semantic invocation inputs and one staged output as sealed
    /// fixed-width views. This is the migration boundary for catalog
    /// implementations that retain [`FunctionValueInput`] identities rather
    /// than concrete matrix owners: physical storage is still resolved from
    /// the live call frame on every invocation.
    #[cfg(feature = "functions")]
    pub fn with_binary_function_value_views<
        A: ManagedElement,
        B: ManagedElement,
        O: ManagedElement,
        R,
    >(
        &mut self,
        first: &crate::FunctionValueInput,
        second: &crate::FunctionValueInput,
        output: &crate::FunctionValueOutput,
        access: impl FnOnce(
            ManagedValueView<'_, A>,
            ManagedValueView<'_, B>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let first = ManagedPort {
            cell: first.cell().clone(),
            role: first.managed_role(),
            marker: PhantomData,
        };
        let second = ManagedPort {
            cell: second.cell().clone(),
            role: second.managed_role(),
            marker: PhantomData,
        };
        let output = ManagedPort {
            cell: output.cell().clone(),
            role: ManagedPortRole::Output(0),
            marker: PhantomData,
        };
        self.with_binary_typed_port_views(&first, &second, &output, access)
    }

    /// Three-input counterpart to [`Self::with_binary_function_value_views`].
    #[cfg(feature = "functions")]
    pub fn with_ternary_function_value_views<
        A: ManagedElement,
        B: ManagedElement,
        C: ManagedElement,
        O: ManagedElement,
        R,
    >(
        &mut self,
        first: &crate::FunctionValueInput,
        second: &crate::FunctionValueInput,
        third: &crate::FunctionValueInput,
        output: &crate::FunctionValueOutput,
        access: impl FnOnce(
            ManagedValueView<'_, A>,
            ManagedValueView<'_, B>,
            ManagedValueView<'_, C>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let first = ManagedPort {
            cell: first.cell().clone(),
            role: first.managed_role(),
            marker: PhantomData,
        };
        let second = ManagedPort {
            cell: second.cell().clone(),
            role: second.managed_role(),
            marker: PhantomData,
        };
        let third = ManagedPort {
            cell: third.cell().clone(),
            role: third.managed_role(),
            marker: PhantomData,
        };
        let output = ManagedPort {
            cell: output.cell().clone(),
            role: ManagedPortRole::Output(0),
            marker: PhantomData,
        };
        self.with_ternary_typed_port_views(&first, &second, &third, &output, access)
    }

    /// Opens one semantic invocation input and its staged output without
    /// retaining an allocation handle in the implementation.
    #[cfg(feature = "functions")]
    pub fn with_unary_function_value_views<A: ManagedElement, O: ManagedElement, R>(
        &mut self,
        input: &crate::FunctionValueInput,
        output: &crate::FunctionValueOutput,
        access: impl FnOnce(
            ManagedValueView<'_, A>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let input = ManagedPort {
            cell: input.cell().clone(),
            role: input.managed_role(),
            marker: PhantomData,
        };
        let output = ManagedPort {
            cell: output.cell().clone(),
            role: ManagedPortRole::Output(0),
            marker: PhantomData,
        };
        self.with_unary_typed_port_views(&input, &output, access)
    }

    /// Opens the published base value, assignment source, selector, and
    /// unpublished output stage for one read-modify-write assignment. The
    /// roles are semantic call-plan ordinals, so a coalesced base input never
    /// shifts the source or selector onto the wrong physical invocation port.
    #[cfg(feature = "functions")]
    pub fn with_assignment_selection_views<T: ManagedElement, S: ManagedElement, R>(
        &mut self,
        sink: &crate::ValueCell,
        source: &crate::ValueCell,
        selector: &crate::ValueCell,
        access: impl FnOnce(
            ManagedValueView<'_, T>,
            ManagedValueView<'_, T>,
            ManagedValueView<'_, S>,
            &mut ManagedValueViewMut<'_, T>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let sink = self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Input(0), false)?;
        let source =
            self.port_lease(source.reactive_cell_id(), ManagedPortRole::Input(1), false)?;
        let selector = self.port_lease(
            selector.reactive_cell_id(),
            ManagedPortRole::Input(2),
            false,
        )?;
        let output = self.port_lease(
            sink.cell.expect("logical assignment sink"),
            ManagedPortRole::Output(0),
            true,
        )?;
        self.validate_managed_element::<T>(sink, false)?;
        self.validate_managed_element::<T>(source, false)?;
        self.validate_managed_element::<S>(selector, false)?;
        self.validate_managed_element::<T>(output, false)?;
        let sink_view = self.read_view::<T>(sink)?;
        let source_view = self.read_view::<T>(source)?;
        let selector_view = self.read_view::<S>(selector)?;
        let mut output_view = self.write_view::<T>(output)?;
        let result = access(sink_view, source_view, selector_view, &mut output_view)?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output.object.object(),
                requested: output.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<T>(output)?;
        Ok(result)
    }

    /// Opens a read-modify-write base, its source, and the unpublished stage
    /// for a whole-value assignment.
    #[cfg(feature = "functions")]
    pub fn with_assignment_whole_views<T: ManagedElement, R>(
        &mut self,
        sink: &crate::ValueCell,
        source: &crate::ValueCell,
        access: impl FnOnce(
            ManagedValueView<'_, T>,
            ManagedValueView<'_, T>,
            &mut ManagedValueViewMut<'_, T>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let sink_lease =
            self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Input(0), false)?;
        let source_lease =
            self.port_lease(source.reactive_cell_id(), ManagedPortRole::Input(1), false)?;
        let output_lease =
            self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Output(0), true)?;
        self.validate_managed_element::<T>(sink_lease, false)?;
        self.validate_managed_element::<T>(source_lease, false)?;
        self.validate_managed_element::<T>(output_lease, false)?;
        let sink_view = self.read_view::<T>(sink_lease)?;
        let source_view = self.read_view::<T>(source_lease)?;
        let mut output_view = self.write_view::<T>(output_lease)?;
        let result = access(sink_view, source_view, &mut output_view)?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output_lease.object.object(),
                requested: output_lease.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<T>(output_lease)?;
        Ok(result)
    }

    /// Two-selector assignment counterpart to
    /// [`Self::with_assignment_selection_views`].
    #[cfg(feature = "functions")]
    pub fn with_assignment_rectangle_views<
        T: ManagedElement,
        R: ManagedElement,
        C: ManagedElement,
        O,
    >(
        &mut self,
        sink: &crate::ValueCell,
        source: &crate::ValueCell,
        rows: &crate::ValueCell,
        columns: &crate::ValueCell,
        access: impl FnOnce(
            ManagedValueView<'_, T>,
            ManagedValueView<'_, T>,
            ManagedValueView<'_, R>,
            ManagedValueView<'_, C>,
            &mut ManagedValueViewMut<'_, T>,
        ) -> crate::MResult<O>,
    ) -> crate::MResult<O> {
        let sink_lease =
            self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Input(0), false)?;
        let source_lease =
            self.port_lease(source.reactive_cell_id(), ManagedPortRole::Input(1), false)?;
        let rows_lease =
            self.port_lease(rows.reactive_cell_id(), ManagedPortRole::Input(2), false)?;
        let columns_lease =
            self.port_lease(columns.reactive_cell_id(), ManagedPortRole::Input(3), false)?;
        let output_lease =
            self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Output(0), true)?;
        self.validate_managed_element::<T>(sink_lease, false)?;
        self.validate_managed_element::<T>(source_lease, false)?;
        self.validate_managed_element::<R>(rows_lease, false)?;
        self.validate_managed_element::<C>(columns_lease, false)?;
        self.validate_managed_element::<T>(output_lease, false)?;
        let sink_view = self.read_view::<T>(sink_lease)?;
        let source_view = self.read_view::<T>(source_lease)?;
        let rows_view = self.read_view::<R>(rows_lease)?;
        let columns_view = self.read_view::<C>(columns_lease)?;
        let mut output_view = self.write_view::<T>(output_lease)?;
        let result = access(
            sink_view,
            source_view,
            rows_view,
            columns_view,
            &mut output_view,
        )?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output_lease.object.object(),
                requested: output_lease.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<T>(output_lease)?;
        Ok(result)
    }

    /// Snapshots one immutable canonical input after validating that the live
    /// logical port still names this call's admitted object and incarnation.
    #[cfg(feature = "functions")]
    pub fn snapshot_canonical_port_value<T>(
        &self,
        input: &ManagedPort<T>,
    ) -> crate::MResult<crate::Value> {
        let lease = self.port_lease(input.logical_cell_id(), input.role(), false)?;
        if !input.cell().has_managed_canonical_storage()? {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(lease.object.object()),
                size: lease.relative_end,
                alignment: 1,
                reason: "canonical kernel input is not backed by its admitted payload envelope",
            }
            .into());
        }
        input.cell().snapshot()
    }

    /// Snapshots one semantically typed value input through this call's live
    /// logical-port authority. Fixed-width imports are reconstructed from the
    /// admitted call object; immutable canonical inputs share their frozen
    /// published root.
    #[cfg(feature = "functions")]
    pub fn snapshot_function_value_input(
        &self,
        input: &crate::FunctionValueInput,
    ) -> crate::MResult<crate::Value> {
        let lease =
            self.port_lease(input.cell().reactive_cell_id(), input.managed_role(), false)?;
        if input.cell().has_managed_canonical_storage()? {
            return input.snapshot();
        }
        let shape = input.cell().shape().clone();
        let schemas = input.cell().schema_table();
        crate::cell_binding::value_from_managed_object(
            self.domain,
            self.realized,
            lease.object,
            lease.region,
            input.representation(),
            input.schema(),
            &shape,
            schemas.as_ref(),
        )
    }

    /// Compares two semantic values only after both logical input leases have
    /// been validated against this call's current plan and incarnations.
    #[cfg(feature = "functions")]
    pub fn function_value_inputs_equal(
        &self,
        lhs: &crate::FunctionValueInput,
        rhs: &crate::FunctionValueInput,
    ) -> crate::MResult<bool> {
        let _ = self.snapshot_function_value_input(lhs)?;
        let _ = self.snapshot_function_value_input(rhs)?;
        lhs.cell().snapshot_eq(rhs.cell())
    }

    /// Constructs one maintained canonical output only after its prospective
    /// footprint has been admitted by the active candidate plan. The build
    /// closure executes while the charge is held and cannot publish directly.
    #[cfg(feature = "functions")]
    pub fn with_admitted_canonical_output<R>(
        &mut self,
        output: &crate::ValueCell,
        footprint: crate::CurrentMemoryFootprint,
        build: impl FnOnce(&mut Self) -> crate::MResult<(R, crate::Value)>,
    ) -> crate::MResult<R> {
        let (object, _) = self.output_target(output, 0)?;
        if self.staged_canonical_output.is_some() {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(object.object()),
                reason: "canonical output was staged more than once in one invocation".into(),
            }
            .into());
        }
        let plan = self.realized.call_plan().ok_or_else(|| {
            MemoryRuntimeError::CandidateValidationFailed {
                object: Some(object.object()),
                reason: "canonical output construction requires its authoritative call plan".into(),
            }
        })?;
        let expected_shape = plan.outputs[0].descriptor.shape().clone();
        let header = plan
            .allocations
            .iter()
            .find(|allocation| allocation.id == object.object())
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object })?;
        let payload = plan
            .allocations
            .iter()
            .find(|allocation| {
                allocation.role == crate::AllocationRole::VariablePayload
                    && allocation.owner == header.owner
                    && allocation.lifetime == header.lifetime
            })
            .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                object: Some(object.object()),
                reason: "canonical output has no planned payload envelope".into(),
            })?;
        let payload = self
            .domain
            .plan_object_key(self.realized.revision(), payload.id)?;
        let allocator = self.domain.planned_allocator(self.realized, payload)?;
        let admission =
            allocator.prepare_frozen_snapshot(footprint.payload_bytes, footprint.retained_nodes)?;
        let (result, next) = build(self)?;
        let actual = next
            .retained_footprint(output.schema_table().as_ref())
            .map_err(|_| MemoryRuntimeError::CandidateValidationFailed {
                object: Some(payload.object()),
                reason: "canonical builder produced an invalid footprint".into(),
            })?;
        if actual.retained_bytes > footprint.payload_bytes
            || actual.node_count > footprint.retained_nodes
            || actual.encoded_bytes > footprint.encoded_bytes
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(payload.object()),
                reason: format!(
                    "canonical builder exceeded its admitted prospective footprint: actual bytes/nodes/encoded {}/{}/{}, admitted {}/{}/{}",
                    actual.retained_bytes,
                    actual.node_count,
                    actual.encoded_bytes,
                    footprint.payload_bytes,
                    footprint.retained_nodes,
                    footprint.encoded_bytes,
                ),
            }
            .into());
        }
        if next.schema_key() != output.schema_key()
            || (next.shape() != &expected_shape && !output.accepts_published_shape(next.shape()))
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(object.object()),
                reason: "canonical builder output differs from its closed schema or shape".into(),
            }
            .into());
        }
        let ownership = admission.complete()?;
        let staged = next.into_retained_payload_ticket(ownership);
        let initialized = self
            .realized
            .binding(object)?
            .required_initialization_bytes();
        self.realized.record_initialized(object, initialized)?;
        self.staged_canonical_output = Some((object, staged));
        Ok(result)
    }

    /// Opens two immutable canonical inputs and one canonical staged output
    /// under the already acquired complete-call leases. Canonical payloads
    /// are immutable frozen roots; the builder returns the next root, which
    /// is retained by this frame until the atomic publication boundary.
    #[cfg(feature = "functions")]
    pub fn with_admitted_canonical_binary_port_values<T, R>(
        &mut self,
        first: &ManagedPort<T>,
        second: &ManagedPort<T>,
        output: &ManagedPort<T>,
        requirements: impl FnOnce(
            &crate::Value,
            &crate::Value,
            &crate::ValueCell,
        ) -> crate::MResult<crate::CurrentMemoryFootprint>,
        build: impl FnOnce(
            &crate::Value,
            &crate::Value,
            &crate::ValueCell,
        ) -> crate::MResult<(R, crate::Value)>,
    ) -> crate::MResult<R> {
        let first_lease = self.port_lease(first.logical_cell_id(), first.role(), false)?;
        let second_lease = self.port_lease(second.logical_cell_id(), second.role(), false)?;
        let output_lease = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        for (lease, canonical) in [
            (first_lease, first.cell().has_managed_canonical_storage()?),
            (second_lease, second.cell().has_managed_canonical_storage()?),
            (output_lease, output.cell().has_managed_canonical_storage()?),
        ] {
            if !canonical {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(lease.object.object()),
                    size: lease.relative_end,
                    alignment: 1,
                    reason: "canonical kernel port is not backed by its admitted payload envelope",
                }
                .into());
            }
        }
        let first_value = first.cell().snapshot()?;
        let second_value = second.cell().snapshot()?;
        let footprint = requirements(&first_value, &second_value, output.cell())?;
        if self.staged_canonical_output.is_some() {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(output_lease.object.object()),
                reason: "canonical output was staged more than once in one invocation".into(),
            }
            .into());
        }
        let plan = self.realized.call_plan().ok_or_else(|| {
            MemoryRuntimeError::CandidateValidationFailed {
                object: Some(output_lease.object.object()),
                reason: "canonical output construction requires its authoritative call plan".into(),
            }
        })?;
        let header = plan
            .allocations
            .iter()
            .find(|allocation| allocation.id == output_lease.object.object())
            .ok_or(MemoryRuntimeError::UnknownPlanObject {
                key: output_lease.object,
            })?;
        let payload = plan
            .allocations
            .iter()
            .find(|allocation| {
                allocation.role == crate::AllocationRole::VariablePayload
                    && allocation.owner == header.owner
                    && allocation.lifetime == header.lifetime
            })
            .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                object: Some(output_lease.object.object()),
                reason: "canonical output has no planned payload envelope".into(),
            })?;
        let payload = self
            .domain
            .plan_object_key(self.realized.revision(), payload.id)?;
        let allocator = self.domain.planned_allocator(self.realized, payload)?;
        // This is the fail-closed boundary: the result-building closure is
        // unreachable until its exact retained bytes and recursive nodes have
        // been reserved by the candidate call plan.
        let admission =
            allocator.prepare_frozen_snapshot(footprint.payload_bytes, footprint.retained_nodes)?;
        let (result, next) = build(&first_value, &second_value, output.cell())?;
        let actual = next
            .retained_footprint(output.cell().schema_table().as_ref())
            .map_err(|_| MemoryRuntimeError::CandidateValidationFailed {
                object: Some(payload.object()),
                reason: "canonical builder produced an invalid footprint".into(),
            })?;
        if actual.retained_bytes != footprint.payload_bytes
            || actual.node_count != footprint.retained_nodes
            || actual.encoded_bytes != footprint.encoded_bytes
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(payload.object()),
                reason: "canonical builder output differs from its admitted prospective footprint"
                    .into(),
            }
            .into());
        }
        let expected_shape = plan.outputs[0].descriptor.shape();
        if next.schema_key() != output.cell().schema_key()
            || (next.shape() != expected_shape
                && !output.cell().accepts_published_shape(next.shape()))
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: Some(output_lease.object.object()),
                reason: "canonical builder output differs from its closed schema or shape".into(),
            }
            .into());
        }
        let ownership = admission.complete()?;
        let staged = next.into_retained_payload_ticket(ownership);
        let initialized = self
            .realized
            .binding(output_lease.object)?
            .required_initialization_bytes();
        self.realized
            .record_initialized(output_lease.object, initialized)?;
        self.staged_canonical_output = Some((output_lease.object, staged));
        Ok(result)
    }

    #[cfg(feature = "functions")]
    pub(crate) fn output_target(
        &self,
        cell: &crate::ValueCell,
        index: usize,
    ) -> MemoryRuntimeResult<(PlanObjectKey, MemoryAccessRegion)> {
        let lease = self.port_lease(
            cell.reactive_cell_id(),
            ManagedPortRole::Output(index),
            true,
        )?;
        Ok((lease.object, lease.region))
    }

    pub fn with_object_value_view<T: ManagedElement, R>(
        &self,
        object: PlanObjectKey,
        access: impl FnOnce(ManagedValueView<'_, T>) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self
            .leases
            .iter()
            .find(|lease| {
                lease.object == object
                    && matches!(
                        lease.mode,
                        MemoryAccessMode::Read
                            | MemoryAccessMode::Write
                            | MemoryAccessMode::ExclusiveInPlace
                    )
            })
            .copied()
            .ok_or(MemoryRuntimeError::BorrowConflict {
                object: object.object(),
            })?;
        self.validate_managed_element::<T>(lease, false)?;
        {
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
            if !region
                .initialization
                .contains_region(lease.region, region.initialized_bytes)
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: lease.relative_end,
                    initialized: region.initialized_bytes,
                });
            }
        }
        Ok(access(self.read_view::<T>(lease)?))
    }

    pub fn with_object_init_view<T: ManagedElement, R>(
        &mut self,
        object: PlanObjectKey,
        access: impl FnOnce(&mut ManagedValueViewMut<'_, T>) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let lease = self
            .leases
            .iter()
            .find(|lease| lease.object == object && lease.mode.writes())
            .copied()
            .ok_or(MemoryRuntimeError::BorrowConflict {
                object: object.object(),
            })?;
        self.validate_managed_element::<T>(lease, false)?;
        let mut output = self.write_view::<T>(lease)?;
        let result = access(&mut output)?;
        if !output.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: object.object(),
                requested: lease.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<T>(lease)?;
        Ok(result)
    }

    /// Opens one staged logical output when the operation's semantic inputs
    /// are canonical payloads inspected through their own managed snapshot
    /// authority. This remains an output-only physical capability; callers
    /// cannot obtain an unrestricted object handle from it.
    pub fn with_output_port_view<T: ManagedElement, R>(
        &mut self,
        output: &ManagedPort<T>,
        access: impl FnOnce(&mut ManagedValueViewMut<'_, T>) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let output = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        self.validate_managed_element::<T>(output, false)?;
        let mut output_view = self.write_view::<T>(output)?;
        let result = access(&mut output_view)?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output.object.object(),
                requested: output.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<T>(output)?;
        Ok(result)
    }

    /// Opens two read ports and one staged output simultaneously. All three
    /// physical bindings were conflict-checked as one acquisition, so no
    /// DomainState borrow remains live while the kernel runs.
    pub fn with_binary_port_views<T: ManagedElement, R>(
        &mut self,
        first: &ManagedPort<T>,
        second: &ManagedPort<T>,
        output: &ManagedPort<T>,
        access: impl FnOnce(
            ManagedValueView<'_, T>,
            ManagedValueView<'_, T>,
            &mut ManagedValueViewMut<'_, T>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        self.with_binary_typed_port_views(first, second, output, access)
    }

    /// Opens the three sealed MatrixSolve scratch ordinals together with its
    /// inputs and transaction output. Scratch element identity comes from the
    /// selected implementation contract and validated ports, never from a
    /// caller-provided object handle. Only the logical scratch prefix is
    /// exposed; spare admitted capacity is not initialized as a side effect.
    #[cfg(feature = "functions")]
    pub fn with_matrix_solve_port_views<T: ManagedElement, R>(
        &mut self,
        coefficients: &ManagedPort<T>,
        rhs: &ManagedPort<T>,
        output: &ManagedPort<T>,
        access: impl FnOnce(
            ManagedValueView<'_, T>,
            ManagedValueView<'_, T>,
            &mut ManagedValueViewMut<'_, T>,
            &mut ManagedValueViewMut<'_, T>,
            &mut ManagedValueViewMut<'_, T>,
            &mut ManagedValueViewMut<'_, usize>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let coefficients =
            self.port_lease(coefficients.logical_cell_id(), coefficients.role(), false)?;
        let rhs = self.port_lease(rhs.logical_cell_id(), rhs.role(), false)?;
        let output = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        self.validate_managed_element::<T>(coefficients, false)?;
        self.validate_managed_element::<T>(rhs, false)?;
        self.validate_managed_element::<T>(output, false)?;
        let coefficients_view = self.read_view::<T>(coefficients)?;
        let rhs_view = self.read_view::<T>(rhs)?;
        let mut output_view = self.write_view::<T>(output)?;
        if coefficients_view.rows() != coefficients_view.columns()
            || coefficients_view.rows() != rhs_view.rows()
            || rhs_view.rows() != output_view.rows()
            || rhs_view.columns() != output_view.columns()
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(output.object.object()),
                size: output_view.len() as u64,
                alignment: mem::align_of::<T>() as u32,
                reason: "matrix solve input/output dimensions disagree",
            }
            .into());
        }
        let coefficient_scratch = self.matrix_solve_scratch::<T>(0, coefficients_view.len())?;
        let solution_scratch = self.matrix_solve_scratch::<T>(1, output_view.len())?;
        let pivot_scratch = self.matrix_solve_scratch::<usize>(2, coefficients_view.rows())?;
        let mut coefficient_workspace = self.write_view::<T>(coefficient_scratch)?;
        let mut solution_workspace = self.write_view::<T>(solution_scratch)?;
        let mut pivots = self.write_view::<usize>(pivot_scratch)?;
        let result = access(
            coefficients_view,
            rhs_view,
            &mut output_view,
            &mut coefficient_workspace,
            &mut solution_workspace,
            &mut pivots,
        )?;
        for (initialized, lease) in [
            (output_view.fully_initialized, output),
            (coefficient_workspace.fully_initialized, coefficient_scratch),
            (solution_workspace.fully_initialized, solution_scratch),
            (pivots.fully_initialized, pivot_scratch),
        ] {
            if !initialized {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: lease.object.object(),
                    requested: lease.relative_end,
                    initialized: 0,
                }
                .into());
            }
        }
        self.record_view_initialized::<T>(output)?;
        self.record_view_initialized::<T>(coefficient_scratch)?;
        self.record_view_initialized::<T>(solution_scratch)?;
        self.record_view_initialized::<usize>(pivot_scratch)?;
        Ok(result)
    }

    #[cfg(feature = "functions")]
    fn matrix_solve_scratch<T: ManagedElement>(
        &self,
        ordinal: u16,
        elements: usize,
    ) -> MemoryRuntimeResult<HeldLease> {
        let invalid = || MemoryRuntimeError::InvalidLayout {
            object: None,
            size: elements as u64,
            alignment: mem::align_of::<T>() as u32,
            reason: "matrix solve scratch does not match its admitted implementation plan",
        };
        let plan = self.realized.call_plan().ok_or_else(invalid)?;
        if plan.implementation_memory != crate::ImplementationMemoryClass::MatrixSolve {
            return Err(invalid());
        }
        let expected_slot = match ordinal {
            0 => plan.inputs.first().and_then(|port| {
                plan.allocations
                    .iter()
                    .find(|allocation| allocation.id == port.object)
                    .and_then(|allocation| allocation.slot)
            }),
            1 => plan.outputs.first().and_then(|port| {
                plan.allocations
                    .iter()
                    .find(|allocation| allocation.id == port.object)
                    .and_then(|allocation| allocation.slot)
            }),
            2 => Some(<usize as ManagedElement>::SLOT),
            _ => None,
        };
        if expected_slot != Some(T::SLOT) {
            return Err(invalid());
        }
        let allocation = plan
            .allocations
            .iter()
            .find(|allocation| {
                matches!(
                    allocation.owner,
                    crate::MemoryObjectOwner::NodeScratch { ordinal: found, .. } if found == ordinal
                )
            })
            .ok_or_else(invalid)?;
        let bytes = elements
            .checked_mul(mem::size_of::<T>())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or_else(invalid)?;
        if allocation.alignment < mem::align_of::<T>() as u32
            || bytes > allocation.current_bytes
            || (ordinal == 2 && allocation.role != crate::AllocationRole::OrderedIndex)
            || (ordinal < 2 && allocation.role != crate::AllocationRole::Scratch)
        {
            return Err(invalid());
        }
        let mut lease = self
            .leases
            .iter()
            .find(|lease| {
                lease.object.object() == allocation.id
                    && lease.object.revision() == self.realized.revision()
                    && lease.mode == MemoryAccessMode::Write
                    && lease.cell.is_none()
            })
            .copied()
            .ok_or_else(invalid)?;
        if !matches!(
            lease.region,
            MemoryAccessRegion::Contiguous {
                offset_bytes: 0,
                ..
            }
        ) || bytes > lease.end.checked_sub(lease.start).ok_or_else(invalid)?
        {
            return Err(invalid());
        }
        if let Some(handle) = lease.handle {
            let state = self.domain.state.borrow();
            if state.record(handle)?.alignment < mem::align_of::<T>() as u32 {
                return Err(invalid());
            }
        }
        lease.end = lease.start.checked_add(bytes).ok_or_else(invalid)?;
        lease.relative_end = bytes;
        lease.region = MemoryAccessRegion::Contiguous {
            offset_bytes: 0,
            length_bytes: bytes,
        };
        Ok(lease)
    }

    pub fn with_binary_typed_port_views<
        A: ManagedElement,
        B: ManagedElement,
        O: ManagedElement,
        R,
    >(
        &mut self,
        first: &ManagedPort<A>,
        second: &ManagedPort<B>,
        output: &ManagedPort<O>,
        access: impl FnOnce(
            ManagedValueView<'_, A>,
            ManagedValueView<'_, B>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let first = self.port_lease(first.logical_cell_id(), first.role(), false)?;
        let second = self.port_lease(second.logical_cell_id(), second.role(), false)?;
        let output = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        self.validate_managed_element::<A>(first, false)?;
        self.validate_managed_element::<B>(second, false)?;
        self.validate_managed_element::<O>(output, false)?;
        let first_view = self.read_view::<A>(first)?;
        let second_view = self.read_view::<B>(second)?;
        let mut output_view = self.write_view::<O>(output)?;
        let result = access(first_view, second_view, &mut output_view)?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output.object.object(),
                requested: output.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<O>(output)?;
        Ok(result)
    }

    /// Opens three logical inputs and one staged output under the leases that
    /// were acquired for the complete call. This is the canonical fixed-width
    /// indexed-kernel entry: selector inspection and output staging happen in
    /// one allocation-free frame without caching physical addresses.
    pub fn with_ternary_typed_port_views<
        A: ManagedElement,
        B: ManagedElement,
        C: ManagedElement,
        O: ManagedElement,
        R,
    >(
        &mut self,
        first: &ManagedPort<A>,
        second: &ManagedPort<B>,
        third: &ManagedPort<C>,
        output: &ManagedPort<O>,
        access: impl FnOnce(
            ManagedValueView<'_, A>,
            ManagedValueView<'_, B>,
            ManagedValueView<'_, C>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let first = self.port_lease(first.logical_cell_id(), first.role(), false)?;
        let second = self.port_lease(second.logical_cell_id(), second.role(), false)?;
        let third = self.port_lease(third.logical_cell_id(), third.role(), false)?;
        let output = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        self.validate_managed_element::<A>(first, false)?;
        self.validate_managed_element::<B>(second, false)?;
        self.validate_managed_element::<C>(third, false)?;
        self.validate_managed_element::<O>(output, false)?;
        let first_view = self.read_view::<A>(first)?;
        let second_view = self.read_view::<B>(second)?;
        let third_view = self.read_view::<C>(third)?;
        let mut output_view = self.write_view::<O>(output)?;
        let result = access(first_view, second_view, third_view, &mut output_view)?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output.object.object(),
                requested: output.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<O>(output)?;
        Ok(result)
    }

    /// Opens four logical inputs and one staged output under one complete-call
    /// lease set. No port may introduce a second acquisition or cache a
    /// physical address across plan revisions.
    pub fn with_quaternary_typed_port_views<
        A: ManagedElement,
        B: ManagedElement,
        C: ManagedElement,
        D: ManagedElement,
        O: ManagedElement,
        R,
    >(
        &mut self,
        first: &ManagedPort<A>,
        second: &ManagedPort<B>,
        third: &ManagedPort<C>,
        fourth: &ManagedPort<D>,
        output: &ManagedPort<O>,
        access: impl FnOnce(
            ManagedValueView<'_, A>,
            ManagedValueView<'_, B>,
            ManagedValueView<'_, C>,
            ManagedValueView<'_, D>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let first = self.port_lease(first.logical_cell_id(), first.role(), false)?;
        let second = self.port_lease(second.logical_cell_id(), second.role(), false)?;
        let third = self.port_lease(third.logical_cell_id(), third.role(), false)?;
        let fourth = self.port_lease(fourth.logical_cell_id(), fourth.role(), false)?;
        let output = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        self.validate_managed_element::<A>(first, false)?;
        self.validate_managed_element::<B>(second, false)?;
        self.validate_managed_element::<C>(third, false)?;
        self.validate_managed_element::<D>(fourth, false)?;
        self.validate_managed_element::<O>(output, false)?;
        let first_view = self.read_view::<A>(first)?;
        let second_view = self.read_view::<B>(second)?;
        let third_view = self.read_view::<C>(third)?;
        let fourth_view = self.read_view::<D>(fourth)?;
        let mut output_view = self.write_view::<O>(output)?;
        let result = access(
            first_view,
            second_view,
            third_view,
            fourth_view,
            &mut output_view,
        )?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output.object.object(),
                requested: output.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<O>(output)?;
        Ok(result)
    }

    /// Opens one read port and one staged output simultaneously.
    pub fn with_unary_port_views<T: ManagedElement, R>(
        &mut self,
        input: &ManagedPort<T>,
        output: &ManagedPort<T>,
        access: impl FnOnce(
            ManagedValueView<'_, T>,
            &mut ManagedValueViewMut<'_, T>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        self.with_unary_typed_port_views(input, output, access)
    }

    /// Opens a typed conversion's source and staged destination together.
    /// Each lane is validated against its own sealed planned element kind.
    pub fn with_unary_typed_port_views<I: ManagedElement, O: ManagedElement, R>(
        &mut self,
        input: &ManagedPort<I>,
        output: &ManagedPort<O>,
        access: impl FnOnce(
            ManagedValueView<'_, I>,
            &mut ManagedValueViewMut<'_, O>,
        ) -> crate::MResult<R>,
    ) -> crate::MResult<R> {
        let input = self.port_lease(input.logical_cell_id(), input.role(), false)?;
        let output = self.port_lease(output.logical_cell_id(), output.role(), true)?;
        self.validate_managed_element::<I>(input, false)?;
        self.validate_managed_element::<O>(output, false)?;
        let input_view = self.read_view::<I>(input)?;
        let mut output_view = self.write_view::<O>(output)?;
        let result = access(input_view, &mut output_view)?;
        if !output_view.fully_initialized {
            return Err(MemoryRuntimeError::UninitializedAccess {
                object: output.object.object(),
                requested: output.relative_end,
                initialized: 0,
            }
            .into());
        }
        self.record_view_initialized::<O>(output)?;
        Ok(result)
    }

    /// Copies one already-resolved fixed-width logical input into its staged
    /// output. Semantic routing has already selected the operation; this
    /// adapter only chooses the sealed physical lane codec after binding.
    #[cfg(feature = "functions")]
    pub fn copy_fixed_port_value(
        &mut self,
        input: &crate::ValueCell,
        output: &crate::ValueCell,
        representation: crate::FunctionValueRepresentation,
    ) -> crate::MResult<()> {
        macro_rules! copy {
            ($type:ty) => {
                return self.copy_fixed_port_lanes::<$type>(input, output)
            };
        }
        match representation {
            #[cfg(feature = "u8")]
            crate::FunctionValueRepresentation::U8 => copy!(u8),
            #[cfg(feature = "u16")]
            crate::FunctionValueRepresentation::U16 => copy!(u16),
            #[cfg(feature = "u32")]
            crate::FunctionValueRepresentation::U32 => copy!(u32),
            #[cfg(feature = "u64")]
            crate::FunctionValueRepresentation::U64 => copy!(u64),
            #[cfg(feature = "u128")]
            crate::FunctionValueRepresentation::U128 => copy!(u128),
            #[cfg(feature = "i8")]
            crate::FunctionValueRepresentation::I8 => copy!(i8),
            #[cfg(feature = "i16")]
            crate::FunctionValueRepresentation::I16 => copy!(i16),
            #[cfg(feature = "i32")]
            crate::FunctionValueRepresentation::I32 => copy!(i32),
            #[cfg(feature = "i64")]
            crate::FunctionValueRepresentation::I64 => copy!(i64),
            #[cfg(feature = "i128")]
            crate::FunctionValueRepresentation::I128 => copy!(i128),
            #[cfg(feature = "f32")]
            crate::FunctionValueRepresentation::F32 => copy!(f32),
            #[cfg(feature = "f64")]
            crate::FunctionValueRepresentation::F64 => copy!(f64),
            #[cfg(feature = "bool")]
            crate::FunctionValueRepresentation::Bool => copy!(bool),
            crate::FunctionValueRepresentation::Index => copy!(usize),
            #[cfg(feature = "complex")]
            crate::FunctionValueRepresentation::C64 => copy!(crate::C64),
            #[cfg(feature = "rational")]
            crate::FunctionValueRepresentation::R64 => copy!(crate::R64),
            #[cfg(feature = "matrix")]
            crate::FunctionValueRepresentation::Matrix { element, .. } => match element {
                #[cfg(feature = "u8")]
                crate::FunctionMatrixElement::U8 => copy!(u8),
                #[cfg(feature = "u16")]
                crate::FunctionMatrixElement::U16 => copy!(u16),
                #[cfg(feature = "u32")]
                crate::FunctionMatrixElement::U32 => copy!(u32),
                #[cfg(feature = "u64")]
                crate::FunctionMatrixElement::U64 => copy!(u64),
                #[cfg(feature = "u128")]
                crate::FunctionMatrixElement::U128 => copy!(u128),
                #[cfg(feature = "i8")]
                crate::FunctionMatrixElement::I8 => copy!(i8),
                #[cfg(feature = "i16")]
                crate::FunctionMatrixElement::I16 => copy!(i16),
                #[cfg(feature = "i32")]
                crate::FunctionMatrixElement::I32 => copy!(i32),
                #[cfg(feature = "i64")]
                crate::FunctionMatrixElement::I64 => copy!(i64),
                #[cfg(feature = "i128")]
                crate::FunctionMatrixElement::I128 => copy!(i128),
                #[cfg(feature = "f32")]
                crate::FunctionMatrixElement::F32 => copy!(f32),
                #[cfg(feature = "f64")]
                crate::FunctionMatrixElement::F64 => copy!(f64),
                #[cfg(feature = "bool")]
                crate::FunctionMatrixElement::Bool => copy!(bool),
                crate::FunctionMatrixElement::Index => copy!(usize),
                #[cfg(feature = "complex")]
                crate::FunctionMatrixElement::C64 => copy!(crate::C64),
                #[cfg(feature = "rational")]
                crate::FunctionMatrixElement::R64 => copy!(crate::R64),
                _ => {}
            },
            _ => {}
        }
        Err(MemoryRuntimeError::InvalidLayout {
            object: None,
            size: 0,
            alignment: 1,
            reason: "logical value requires the managed canonical payload stage",
        }
        .into())
    }

    /// Applies a resolved matrix selection to the staged output for a
    /// fixed-width value. `positions` are physical column-major lane indices
    /// in semantic selector order. The published input is copied first, so a
    /// failed call never mutates the current cell binding.
    #[cfg(feature = "functions")]
    pub fn assign_fixed_port_selection(
        &mut self,
        sink: &crate::ValueCell,
        source: &crate::ValueCell,
        representation: crate::FunctionValueRepresentation,
        positions: &[usize],
    ) -> crate::MResult<()> {
        macro_rules! assign {
            ($type:ty) => {
                return self.assign_fixed_port_lanes::<$type>(sink, source, positions)
            };
        }
        match representation {
            #[cfg(feature = "u8")]
            crate::FunctionValueRepresentation::U8 => assign!(u8),
            #[cfg(feature = "u16")]
            crate::FunctionValueRepresentation::U16 => assign!(u16),
            #[cfg(feature = "u32")]
            crate::FunctionValueRepresentation::U32 => assign!(u32),
            #[cfg(feature = "u64")]
            crate::FunctionValueRepresentation::U64 => assign!(u64),
            #[cfg(feature = "u128")]
            crate::FunctionValueRepresentation::U128 => assign!(u128),
            #[cfg(feature = "i8")]
            crate::FunctionValueRepresentation::I8 => assign!(i8),
            #[cfg(feature = "i16")]
            crate::FunctionValueRepresentation::I16 => assign!(i16),
            #[cfg(feature = "i32")]
            crate::FunctionValueRepresentation::I32 => assign!(i32),
            #[cfg(feature = "i64")]
            crate::FunctionValueRepresentation::I64 => assign!(i64),
            #[cfg(feature = "i128")]
            crate::FunctionValueRepresentation::I128 => assign!(i128),
            #[cfg(feature = "f32")]
            crate::FunctionValueRepresentation::F32 => assign!(f32),
            #[cfg(feature = "f64")]
            crate::FunctionValueRepresentation::F64 => assign!(f64),
            #[cfg(feature = "bool")]
            crate::FunctionValueRepresentation::Bool => assign!(bool),
            crate::FunctionValueRepresentation::Index => assign!(usize),
            #[cfg(feature = "complex")]
            crate::FunctionValueRepresentation::C64 => assign!(crate::C64),
            #[cfg(feature = "rational")]
            crate::FunctionValueRepresentation::R64 => assign!(crate::R64),
            #[cfg(feature = "matrix")]
            crate::FunctionValueRepresentation::Matrix { element, .. } => match element {
                #[cfg(feature = "u8")]
                crate::FunctionMatrixElement::U8 => assign!(u8),
                #[cfg(feature = "u16")]
                crate::FunctionMatrixElement::U16 => assign!(u16),
                #[cfg(feature = "u32")]
                crate::FunctionMatrixElement::U32 => assign!(u32),
                #[cfg(feature = "u64")]
                crate::FunctionMatrixElement::U64 => assign!(u64),
                #[cfg(feature = "u128")]
                crate::FunctionMatrixElement::U128 => assign!(u128),
                #[cfg(feature = "i8")]
                crate::FunctionMatrixElement::I8 => assign!(i8),
                #[cfg(feature = "i16")]
                crate::FunctionMatrixElement::I16 => assign!(i16),
                #[cfg(feature = "i32")]
                crate::FunctionMatrixElement::I32 => assign!(i32),
                #[cfg(feature = "i64")]
                crate::FunctionMatrixElement::I64 => assign!(i64),
                #[cfg(feature = "i128")]
                crate::FunctionMatrixElement::I128 => assign!(i128),
                #[cfg(feature = "f32")]
                crate::FunctionMatrixElement::F32 => assign!(f32),
                #[cfg(feature = "f64")]
                crate::FunctionMatrixElement::F64 => assign!(f64),
                #[cfg(feature = "bool")]
                crate::FunctionMatrixElement::Bool => assign!(bool),
                crate::FunctionMatrixElement::Index => assign!(usize),
                #[cfg(feature = "complex")]
                crate::FunctionMatrixElement::C64 => assign!(crate::C64),
                #[cfg(feature = "rational")]
                crate::FunctionMatrixElement::R64 => assign!(crate::R64),
                _ => {}
            },
            _ => {}
        }
        Err(MemoryRuntimeError::InvalidLayout {
            object: None,
            size: 0,
            alignment: 1,
            reason: "selected assignment requires the managed canonical payload stage",
        }
        .into())
    }

    /// Initializes the transaction-selected output object from an externally
    /// produced immutable value. Schema and shape remain the logical cell's
    /// authority; the sealed physical codec performs the actual staged copy.
    #[cfg(feature = "functions")]
    pub fn stage_output_value(
        &mut self,
        output: &crate::ValueCell,
        value: crate::Value,
    ) -> crate::MResult<()> {
        let (object, _) = self.output_target(output, 0)?;
        let expected_shape = self
            .realized
            .call_plan()
            .and_then(|plan| plan.outputs.first().map(|output| output.descriptor.shape()))
            .cloned()
            .unwrap_or_else(|| output.shape().clone());
        if value.schema_key() != output.schema_key()
            || (value.shape() != &expected_shape && !output.accepts_published_shape(value.shape()))
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "staged output value differs from its closed schema or shape".into(),
            }
            .into());
        }
        let plan = self.realized.call_plan().ok_or_else(|| {
            MemoryRuntimeError::CandidateValidationFailed {
                object: Some(object.object()),
                reason: "output adoption requires its authoritative call plan".into(),
            }
        })?;
        let output_plan =
            plan.outputs
                .first()
                .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(object.object()),
                    reason: "output adoption has no planned output".into(),
                })?;
        if matches!(
            output_plan.value.storage.planned_slot(),
            crate::PlannedSlotKind::StringHeader | crate::PlannedSlotKind::CanonicalValueHandle
        ) {
            if self.staged_canonical_output.is_some() {
                return Err(MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(object.object()),
                    reason: "canonical output was staged more than once in one invocation".into(),
                }
                .into());
            }
            let header = plan
                .allocations
                .iter()
                .find(|allocation| allocation.id == object.object())
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object })?;
            let payload = plan
                .allocations
                .iter()
                .find(|allocation| {
                    allocation.role == crate::AllocationRole::VariablePayload
                        && allocation.owner == header.owner
                        && allocation.lifetime == header.lifetime
                })
                .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(object.object()),
                    reason: "canonical output has no planned payload envelope".into(),
                })?;
            let payload = self
                .domain
                .plan_object_key(self.realized.revision(), payload.id)?;
            let footprint = value
                .retained_footprint(output.schema_table().as_ref())
                .map_err(|_| MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(payload.object()),
                    reason: "canonical output footprint is invalid for its declared schema".into(),
                })?;
            let allocator = self.domain.planned_allocator(self.realized, payload)?;
            let ownership =
                allocator.admit_frozen_snapshot(footprint.retained_bytes, footprint.node_count)?;
            let staged = value.into_retained_payload_ticket(ownership);
            let initialized = self
                .realized
                .binding(object)?
                .required_initialization_bytes();
            self.realized.record_initialized(object, initialized)?;
            self.staged_canonical_output = Some((object, staged));
            return Ok(());
        }
        crate::cell_binding::initialize_managed_object_from_value(
            self,
            object,
            output.representation(),
            &value,
        )
    }

    #[cfg(feature = "functions")]
    pub(crate) fn take_staged_output_value(
        &mut self,
        object: PlanObjectKey,
    ) -> Option<crate::Value> {
        match self.staged_canonical_output.take() {
            Some((staged, value)) if staged == object => Some(value),
            Some(staged) => {
                self.staged_canonical_output = Some(staged);
                None
            }
            None => None,
        }
    }

    #[cfg(feature = "functions")]
    pub(crate) fn take_undo_snapshot(
        &mut self,
    ) -> MemoryRuntimeResult<Option<PreparedUndoSnapshot>> {
        let Some(mut undo) = self.undo_snapshot.take() else {
            return Ok(None);
        };
        let position = self
            .leases
            .iter()
            .position(|lease| {
                lease.object == undo.target
                    && lease.mode == MemoryAccessMode::ExclusiveInPlace
                    && lease.owns_lease
            })
            .ok_or_else(|| MemoryRuntimeError::CandidateValidationFailed {
                object: Some(undo.target.object()),
                reason: "undo publication lost its exclusive execution lease".into(),
            })?;
        let held = self.leases.leases.remove(position);
        match (held.handle, held.token) {
            (Some(handle), Some(token)) => {
                undo.retained_lease = Some(RetainedPublicationLease {
                    domain: self.domain.clone(),
                    _realized: self.realized.clone(),
                    handle,
                    token,
                });
            }
            // Empty objects have real logical transaction authority but no
            // physical allocation to pin. Keep the armed undo record so the
            // same publication protocol applies without inventing a handle.
            (None, None) if held.start == held.end => {}
            _ => {
                return Err(MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(undo.target.object()),
                    reason: "undo publication has incomplete physical lease authority".into(),
                });
            }
        }
        Ok(Some(undo))
    }

    #[cfg(feature = "functions")]
    fn copy_fixed_port_lanes<T: ManagedElement>(
        &mut self,
        input: &crate::ValueCell,
        output: &crate::ValueCell,
    ) -> crate::MResult<()> {
        let input_lease =
            self.port_lease(input.reactive_cell_id(), ManagedPortRole::Input(0), false)?;
        let output_lease =
            self.port_lease(output.reactive_cell_id(), ManagedPortRole::Output(0), true)?;
        self.validate_managed_element::<T>(input_lease, false)?;
        self.validate_managed_element::<T>(output_lease, false)?;
        let input_view = self.read_view::<T>(input_lease)?;
        let mut output_view = self.write_view::<T>(output_lease)?;
        if input_view.len() != output_view.len()
            || input_view.rows() != output_view.rows()
            || input_view.columns() != output_view.columns()
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(output_lease.object.object()),
                size: output_view.len() as u64,
                alignment: core::mem::align_of::<T>() as u32,
                reason: "assignment source and staged output geometry disagree",
            }
            .into());
        }
        output_view.try_fill_column_major(|index| {
            input_view.get_column_major(index).ok_or_else(|| {
                MemoryRuntimeError::InvalidLayout {
                    object: Some(input_lease.object.object()),
                    size: input_view.len() as u64,
                    alignment: core::mem::align_of::<T>() as u32,
                    reason: "assignment source index is outside its managed view",
                }
                .into()
            })
        })?;
        self.record_view_initialized::<T>(output_lease)?;
        Ok(())
    }

    #[cfg(feature = "functions")]
    fn assign_fixed_port_lanes<T: ManagedElement>(
        &mut self,
        sink: &crate::ValueCell,
        source: &crate::ValueCell,
        positions: &[usize],
    ) -> crate::MResult<()> {
        let sink_lease =
            self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Input(0), false)?;
        let source_lease =
            self.port_lease(source.reactive_cell_id(), ManagedPortRole::Input(1), false)?;
        let output_lease =
            self.port_lease(sink.reactive_cell_id(), ManagedPortRole::Output(0), true)?;
        self.validate_managed_element::<T>(sink_lease, false)?;
        self.validate_managed_element::<T>(source_lease, false)?;
        self.validate_managed_element::<T>(output_lease, false)?;
        let sink_view = self.read_view::<T>(sink_lease)?;
        let source_view = self.read_view::<T>(source_lease)?;
        let mut output_view = self.write_view::<T>(output_lease)?;
        if sink_view.len() != output_view.len()
            || sink_view.rows() != output_view.rows()
            || sink_view.columns() != output_view.columns()
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(output_lease.object.object()),
                size: output_view.len() as u64,
                alignment: core::mem::align_of::<T>() as u32,
                reason: "selected assignment stage geometry disagrees with its published input",
            }
            .into());
        }
        output_view.try_fill_column_major(|index| {
            sink_view.get_column_major(index).ok_or_else(|| {
                MemoryRuntimeError::InvalidLayout {
                    object: Some(sink_lease.object.object()),
                    size: sink_view.len() as u64,
                    alignment: core::mem::align_of::<T>() as u32,
                    reason: "selected assignment base index is outside its managed view",
                }
                .into()
            })
        })?;
        let source_len = source_view.len();
        for (ordinal, &destination) in positions.iter().enumerate() {
            if destination >= output_view.len() {
                return Err(MemoryRuntimeError::CapacityExceeded {
                    object: output_lease.object.object(),
                    requested: destination as u64 + 1,
                    capacity: output_view.len() as u64,
                }
                .into());
            }
            let source_index = if source_len == 1 {
                0
            } else if source_len == output_view.len() && source_len != positions.len() {
                destination
            } else if source_len == positions.len() {
                let row = ordinal / source_view.columns();
                let column = ordinal % source_view.columns();
                column
                    .checked_mul(source_view.rows())
                    .and_then(|base| base.checked_add(row))
                    .ok_or(MemoryRuntimeError::CapacityExceeded {
                        object: source_lease.object.object(),
                        requested: ordinal as u64,
                        capacity: source_len as u64,
                    })?
            } else {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(source_lease.object.object()),
                    size: source_len as u64,
                    alignment: core::mem::align_of::<T>() as u32,
                    reason: "selected assignment source cardinality does not match its destinations",
                }
                .into());
            };
            let value = source_view.get_column_major(source_index).ok_or(
                MemoryRuntimeError::CapacityExceeded {
                    object: source_lease.object.object(),
                    requested: source_index as u64 + 1,
                    capacity: source_len as u64,
                },
            )?;
            output_view.write_column_major(destination, value)?;
        }
        self.record_view_initialized::<T>(output_lease)?;
        Ok(())
    }

    fn read_view<T: ManagedElement>(
        &self,
        lease: HeldLease,
    ) -> MemoryRuntimeResult<ManagedValueView<'_, T>> {
        let (base, rows, columns, row_stride, column_stride) =
            self.view_geometry::<T>(lease, false)?;
        Ok(ManagedValueView {
            base: base.cast::<T>(),
            rows,
            columns,
            row_stride,
            column_stride,
            marker: PhantomData,
        })
    }

    fn write_view<T: ManagedElement>(
        &self,
        lease: HeldLease,
    ) -> MemoryRuntimeResult<ManagedValueViewMut<'_, T>> {
        let (base, rows, columns, row_stride, column_stride) =
            self.view_geometry::<T>(lease, true)?;
        Ok(ManagedValueViewMut {
            base: base.cast::<MaybeUninit<T>>(),
            rows,
            columns,
            row_stride,
            column_stride,
            fully_initialized: rows == 0 || columns == 0,
            marker: PhantomData,
        })
    }

    fn view_geometry<T: ManagedElement>(
        &self,
        lease: HeldLease,
        write: bool,
    ) -> MemoryRuntimeResult<(NonNull<u8>, usize, usize, usize, usize)> {
        if (write && !lease.mode.writes())
            || (!write
                && !matches!(
                    lease.mode,
                    MemoryAccessMode::Read
                        | MemoryAccessMode::Write
                        | MemoryAccessMode::ExclusiveInPlace
                ))
        {
            return Err(MemoryRuntimeError::BorrowConflict {
                object: lease.object.object(),
            });
        }
        let (rows, columns, row_stride_bytes, column_stride_bytes) = match lease.region {
            MemoryAccessRegion::WholeInitialized | MemoryAccessRegion::Contiguous { .. } => {
                let bytes = lease.end.checked_sub(lease.start).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "managed contiguous view span",
                        current: lease.start,
                        change: lease.end,
                    },
                )?;
                (
                    1,
                    bytes / mem::size_of::<T>() as u64,
                    0,
                    mem::size_of::<T>() as u64,
                )
            }
            MemoryAccessRegion::Strided {
                count,
                stride_bytes,
                element_bytes,
                ..
            } if element_bytes == mem::size_of::<T>() as u64 => (count, 1, stride_bytes, 0),
            MemoryAccessRegion::Rectangle {
                rows,
                columns,
                row_stride_bytes,
                column_stride_bytes,
                element_bytes,
                ..
            } if element_bytes == mem::size_of::<T>() as u64 => {
                (rows, columns, row_stride_bytes, column_stride_bytes)
            }
            _ => {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(lease.object.object()),
                    size: lease.relative_end,
                    alignment: mem::align_of::<T>() as u32,
                    reason: "managed typed view requires scalar, strided, or rectangle geometry",
                });
            }
        };
        if row_stride_bytes % mem::size_of::<T>() as u64 != 0
            || column_stride_bytes % mem::size_of::<T>() as u64 != 0
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(lease.object.object()),
                size: lease.relative_end,
                alignment: mem::align_of::<T>() as u32,
                reason: "managed view stride is not aligned to its element type",
            });
        }
        let Some(handle) = lease.handle else {
            if rows != 0 && columns != 0 {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(lease.object.object()),
                    size: lease.relative_end,
                    alignment: mem::align_of::<T>() as u32,
                    reason: "nonempty managed view has no allocation",
                });
            }
            return Ok((
                NonNull::<T>::dangling().cast(),
                usize::try_from(rows).unwrap_or(0),
                usize::try_from(columns).unwrap_or(0),
                0,
                0,
            ));
        };
        let state = self.domain.state.borrow();
        let record = state.record(handle)?;
        let block = record
            .block
            .as_ref()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
        let pointer = block
            .pointer()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
        let start =
            usize::try_from(lease.start).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: lease.object.object(),
                requested: lease.start,
                capacity: record.capacity_bytes,
            })?;
        let address = unsafe { pointer.as_ptr().add(start) } as usize;
        validate_typed_bytes::<T>(
            lease.object.object(),
            lease.start,
            address,
            mem::size_of::<T>(),
        )?;
        Ok((
            unsafe { NonNull::new_unchecked(pointer.as_ptr().add(start)) },
            usize::try_from(rows).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: lease.object.object(),
                requested: rows,
                capacity: usize::MAX as u64,
            })?,
            usize::try_from(columns).map_err(|_| MemoryRuntimeError::CapacityExceeded {
                object: lease.object.object(),
                requested: columns,
                capacity: usize::MAX as u64,
            })?,
            usize::try_from(row_stride_bytes / mem::size_of::<T>() as u64).map_err(|_| {
                MemoryRuntimeError::CapacityExceeded {
                    object: lease.object.object(),
                    requested: row_stride_bytes,
                    capacity: usize::MAX as u64,
                }
            })?,
            usize::try_from(column_stride_bytes / mem::size_of::<T>() as u64).map_err(|_| {
                MemoryRuntimeError::CapacityExceeded {
                    object: lease.object.object(),
                    requested: column_stride_bytes,
                    capacity: usize::MAX as u64,
                }
            })?,
        ))
    }

    fn record_view_initialized<T: ManagedElement>(
        &self,
        lease: HeldLease,
    ) -> MemoryRuntimeResult<()> {
        let element = mem::size_of::<T>() as u64;
        match lease.region {
            MemoryAccessRegion::WholeInitialized | MemoryAccessRegion::Contiguous { .. } => {
                let length = lease.end.checked_sub(lease.start).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "managed initialized view span",
                        current: lease.start,
                        change: lease.end,
                    },
                )?;
                self.realized.record_initialized_range(
                    lease.object,
                    lease.relative_end.saturating_sub(length),
                    length,
                )
            }
            MemoryAccessRegion::Strided {
                offset_bytes,
                count,
                stride_bytes,
                ..
            } => {
                for index in 0..count {
                    let start = index
                        .checked_mul(stride_bytes)
                        .and_then(|delta| offset_bytes.checked_add(delta))
                        .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                            dimension: "strided initialized element offset",
                            current: offset_bytes,
                            change: stride_bytes,
                        })?;
                    self.realized
                        .record_initialized_range(lease.object, start, element)?;
                }
                Ok(())
            }
            MemoryAccessRegion::Rectangle {
                offset_bytes,
                rows,
                columns,
                row_stride_bytes,
                column_stride_bytes,
                ..
            } => {
                for column in 0..columns {
                    for row in 0..rows {
                        let start = row
                            .checked_mul(row_stride_bytes)
                            .and_then(|row| {
                                column
                                    .checked_mul(column_stride_bytes)
                                    .and_then(|column| row.checked_add(column))
                            })
                            .and_then(|delta| offset_bytes.checked_add(delta))
                            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                                dimension: "rectangular initialized element offset",
                                current: offset_bytes,
                                change: row_stride_bytes.max(column_stride_bytes),
                            })?;
                        self.realized
                            .record_initialized_range(lease.object, start, element)?;
                    }
                }
                Ok(())
            }
        }
    }
    pub fn with_port_slice<T: ManagedElement, R>(
        &self,
        port: &ManagedPort<T>,
        access: impl FnOnce(&[T]) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self.port_lease(port.logical_cell_id(), port.role(), false)?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.validate_managed_element::<T>(lease, false)?;
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
        port: &ManagedPort<T>,
        access: impl FnOnce(&mut [T]) -> R,
    ) -> MemoryRuntimeResult<R> {
        let lease = self.port_lease(port.logical_cell_id(), port.role(), true)?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.validate_managed_element::<T>(lease, false)?;
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

    pub fn with_port_init_writer<T: ManagedElement>(
        &mut self,
        port: &ManagedPort<T>,
        access: impl FnOnce(&mut InitWriter<'_, T>) -> MemoryRuntimeResult<()>,
    ) -> MemoryRuntimeResult<()> {
        let lease = self.port_lease(port.logical_cell_id(), port.role(), true)?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.validate_managed_element::<T>(lease, false)?;
        self.with_init_writer(lease, access)
    }

    pub fn with_object_init_writer<T: ManagedElement>(
        &mut self,
        object: PlanObjectKey,
        access: impl FnOnce(&mut InitWriter<'_, T>) -> MemoryRuntimeResult<()>,
    ) -> MemoryRuntimeResult<()> {
        let lease = self
            .leases
            .iter()
            .find(|lease| lease.object == object && lease.mode.writes())
            .copied()
            .ok_or(MemoryRuntimeError::BorrowConflict {
                object: object.object(),
            })?;
        validate_contiguous_typed_region(lease.object.object(), lease.region)?;
        self.validate_managed_element::<T>(lease, true)?;
        self.with_init_writer(lease, access)
    }

    fn validate_managed_element<T: ManagedElement>(
        &self,
        lease: HeldLease,
        allow_raw_bytes: bool,
    ) -> MemoryRuntimeResult<()> {
        let state = self.domain.state.borrow();
        let region = state
            .regions
            .get(&lease.object)
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key: lease.object })?;
        let byte_codec = allow_raw_bytes
            && mem::size_of::<T>() == 1
            && matches!(
                region.slot,
                None | Some(crate::PlannedSlotKind::FixedScalar(_))
            );
        if region.slot != Some(T::SLOT) && !byte_codec {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(lease.object.object()),
                size: mem::size_of::<T>() as u64,
                alignment: mem::align_of::<T>() as u32,
                reason: "typed view does not match the planned slot identity",
            });
        }
        if let Some(handle) = lease.handle {
            let record = state.record(handle)?;
            if record.alignment < mem::align_of::<T>() as u32 {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(lease.object.object()),
                    size: mem::size_of::<T>() as u64,
                    alignment: mem::align_of::<T>() as u32,
                    reason: "allocation base alignment is weaker than the typed view",
                });
            }
        }
        Ok(())
    }

    fn with_init_writer<T: ManagedElement>(
        &mut self,
        lease: HeldLease,
        access: impl FnOnce(&mut InitWriter<'_, T>) -> MemoryRuntimeResult<()>,
    ) -> MemoryRuntimeResult<()> {
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
            return access(&mut writer);
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
        access(&mut writer)?;
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
        let initialized_start = lease.relative_end.checked_sub(length).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "initialized range start",
                current: lease.relative_end,
                change: length,
            },
        )?;
        self.realized.record_initialized_range(
            lease.object,
            initialized_start,
            initialized_bytes.saturating_sub(initialized_start),
        )
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
                ((lease.cell == Some(cell) && lease.role == Some(role))
                    || (lease.alias_cell == Some(cell) && lease.alias_role == Some(role)))
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
        validate_contiguous_region(object.object(), lease.region)?;
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
            if !region
                .initialization
                .contains_region(lease.region, region.initialized_bytes)
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: lease.relative_end,
                    initialized: region.initialized_bytes,
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
        validate_contiguous_region(object.object(), lease.region)?;
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
            if !region
                .initialization
                .contains_region(lease.region, region.initialized_bytes)
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: lease.relative_end,
                    initialized: region.initialized_bytes,
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
        validate_contiguous_region(object.object(), lease.region)?;
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
            let prefix_region = MemoryAccessRegion::Contiguous {
                offset_bytes: lease.relative_end.checked_sub(leased_length).ok_or(
                    MemoryRuntimeError::AccountingInvariantViolation {
                        dimension: "lease relative start",
                        current: lease.relative_end,
                        change: leased_length,
                    },
                )?,
                length_bytes,
            };
            if !region
                .initialization
                .contains_region(prefix_region, region.initialized_bytes)
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: object.object(),
                    requested: relative_end,
                    initialized: region.initialized_bytes,
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

fn validate_contiguous_region(
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
        reason: "raw byte access cannot expose gaps inside a noncontiguous planned region",
    })
}

pub(crate) fn planned_value_access_region(
    value: &crate::ValueLayoutPlan,
) -> MemoryRuntimeResult<MemoryAccessRegion> {
    if matches!(
        value.storage,
        crate::StorageLayoutClass::CanonicalSnapshot { .. }
    ) {
        // Canonical aggregates may carry semantic cardinality/rank axes, but
        // their fixed call object is one sealed root handle. Child payload
        // geometry belongs to the separately admitted indirect envelope.
        return Ok(MemoryAccessRegion::Contiguous {
            offset_bytes: 0,
            length_bytes: value.current_address_span_bytes,
        });
    }
    match value.axes.as_ref() {
        [] => Ok(MemoryAccessRegion::Contiguous {
            offset_bytes: 0,
            length_bytes: value.current_address_span_bytes,
        }),
        [rows, columns] => {
            let [row_stride_bytes, column_stride_bytes] = value.strides_bytes.as_ref() else {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: None,
                    size: value.current_address_span_bytes,
                    alignment: value.slot.alignment,
                    reason: "rank-two planned value does not carry two physical strides",
                });
            };
            Ok(MemoryAccessRegion::Rectangle {
                offset_bytes: 0,
                rows: rows.current,
                columns: columns.current,
                row_stride_bytes: *row_stride_bytes,
                column_stride_bytes: *column_stride_bytes,
                element_bytes: value.slot.bytes,
            })
        }
        _ => Err(MemoryRuntimeError::InvalidLayout {
            object: None,
            size: value.current_address_span_bytes,
            alignment: value.slot.alignment,
            reason: "managed runtime supports scalar and rank-two value geometry",
        }),
    }
}

#[cfg(feature = "functions")]
fn region_access_for_port(
    region: &crate::RegionAccessPlan,
    whole_bytes: u64,
    element_bytes: u64,
) -> MemoryRuntimeResult<MemoryAccessRegion> {
    match region {
        crate::RegionAccessPlan::WholeValue => Ok(MemoryAccessRegion::Contiguous {
            offset_bytes: 0,
            length_bytes: whole_bytes,
        }),
        crate::RegionAccessPlan::Contiguous {
            offset_bytes,
            length_bytes,
        } => Ok(MemoryAccessRegion::Contiguous {
            offset_bytes: *offset_bytes,
            length_bytes: *length_bytes,
        }),
        crate::RegionAccessPlan::Strided {
            offset_bytes,
            count,
            stride_bytes,
            element_bytes,
        } => Ok(MemoryAccessRegion::Strided {
            offset_bytes: *offset_bytes,
            count: *count,
            stride_bytes: *stride_bytes,
            element_bytes: *element_bytes,
        }),
        crate::RegionAccessPlan::Rectangle {
            base_offset_bytes,
            rows,
            columns,
            row_stride_bytes,
            column_stride_bytes,
        } => Ok(MemoryAccessRegion::Rectangle {
            offset_bytes: *base_offset_bytes,
            rows: *rows,
            columns: *columns,
            row_stride_bytes: *row_stride_bytes,
            column_stride_bytes: *column_stride_bytes,
            element_bytes,
        }),
        crate::RegionAccessPlan::Gather { .. }
        | crate::RegionAccessPlan::CollectionEntry { .. }
        | crate::RegionAccessPlan::Deferred(_) => Err(MemoryRuntimeError::InvalidLayout {
            object: None,
            size: whole_bytes,
            alignment: 1,
            reason: "selector/key/deferred access requires its concrete bounded plan",
        }),
    }
}

impl Drop for KernelMemoryFrame<'_> {
    fn drop(&mut self) {
        let mut state = self.domain.state.borrow_mut();
        for held in self.leases.leases.drain(..) {
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

pub(super) fn enclosing_span(
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
