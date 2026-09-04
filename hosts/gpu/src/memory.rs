use std::collections::BTreeMap;

use crate::{
    GpuExecutionBindingRole, GpuExecutionPlan, GpuExecutionPlanError, GpuKernelPlanSource,
    GpuPlanScalar,
};
use mech_core::{
    AllocationPlan, AllocationRole, ArenaPlacement, ArenaPlan, GpuMemoryLimits, MemoryArenaId,
    MemoryBudgetViolation, MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanError,
    MemoryPlanPoint, MemorySpace, ResourceDemand, TargetMemoryProfile, TransferDirection,
    TransferPlan, evaluate_memory_budget,
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
            let id = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let ordinal =
                u32::try_from(ordinal).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                    field: "GPU readback ordinal",
                })?;
            let lifetime = MemoryLifetime::Transfer {
                first: MemoryPlanPoint::new(0),
                last: MemoryPlanPoint::new(0),
            };
            push_gpu_allocation(
                &mut allocations,
                &mut arenas,
                id,
                MemoryObjectOwner::Transfer { ordinal },
                AllocationRole::TransferStage,
                MemorySpace::Host,
                bytes,
                lifetime,
                limits.min_storage_buffer_offset_alignment,
            )?;
            readback_objects.insert(slot, id);
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
        space,
        current_bytes: bytes,
        capacity_bytes: bytes,
        alignment,
        lifetime,
        placement: ArenaPlacement { arena, offset: 0 },
        reuse_group: None,
    });
    arenas.push(ArenaPlan {
        id: arena,
        space,
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
}
