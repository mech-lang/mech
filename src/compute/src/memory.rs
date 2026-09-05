use std::collections::BTreeMap;

use mech_core::{
    AllocationPlan, AllocationRole, ArenaPlacement, BoundCall, CallMemoryPlan, MemoryArenaId,
    MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanError, MemoryPlanPoint,
    MemorySpace, TargetMemoryProfile, TransferDirection as PlannedTransferDirection, TransferPlan,
};
use mech_engine::memory_planner::{
    ActivationMemoryFacts, ProgramMemoryPlan, ProgramMemoryPlanTemplate,
    instantiate_program_memory_plan_with_target_overrides, plan_program_memory_template,
};
use mech_engine::{ComputeRegionDeclaration, ProgramArtifact};

use crate::{ComputePhysicalPlan, TransferBoundary, TransferDirection, plan_compute_artifact};

/// CPU/GPU placement plus the process-local R5 memory template derived from
/// that exact placement. The artifact and wire plans remain unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedComputeArtifact {
    pub placement: ComputePhysicalPlan,
    pub memory: ProgramMemoryPlanTemplate,
}

pub fn plan_compute_memory(
    artifact: &ProgramArtifact,
    explicit_regions: &[ComputeRegionDeclaration],
    instruction_nodes: &[mech_core::NodeId],
    instruction_bindings: &[Option<BoundCall>],
    instruction_memory_plans: &[Option<CallMemoryPlan>],
) -> Result<PlannedComputeArtifact, MemoryPlanError> {
    let placement = plan_compute_artifact(artifact, explicit_regions);
    let mut memory = plan_program_memory_template(
        artifact,
        instruction_nodes,
        instruction_bindings,
        instruction_memory_plans,
    )?;
    let elements = placement
        .slots
        .iter()
        .map(|slot| (slot.slot, slot.elements))
        .collect::<BTreeMap<_, _>>();
    let mut transfers = placement
        .transfers
        .iter()
        .map(|boundary| {
            transfer_plan(
                artifact,
                &placement,
                &memory.node_positions,
                &elements,
                boundary,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    transfers.sort_by(|left, right| {
        (
            left.direction,
            left.slot,
            left.consumer,
            &left.interface_name,
        )
            .cmp(&(
                right.direction,
                right.slot,
                right.consumer,
                &right.interface_name,
            ))
    });
    transfers.dedup_by(|left, right| {
        left.direction == right.direction
            && left.slot == right.slot
            && left.consumer == right.consumer
            && left.interface_name == right.interface_name
    });
    let allocations = transfers
        .iter()
        .enumerate()
        .map(|(ordinal, transfer)| {
            let ordinal =
                u32::try_from(ordinal).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                    field: "compute transfer ordinal",
                })?;
            Ok(AllocationPlan {
                id: MemoryObjectId::new(ordinal),
                owner: MemoryObjectOwner::Transfer { ordinal },
                role: AllocationRole::TransferStage,
                space: transfer.destination,
                current_bytes: transfer.current_bytes,
                capacity_bytes: transfer.capacity_bytes,
                alignment: 4,
                lifetime: transfer.lifetime,
                placement: ArenaPlacement {
                    arena: MemoryArenaId::new(0),
                    offset: 0,
                },
                reuse_group: None,
            })
        })
        .collect::<Result<Vec<_>, MemoryPlanError>>()?;
    memory.allocations = allocations.into_boxed_slice();
    memory.transfers = transfers.into_boxed_slice();
    Ok(PlannedComputeArtifact { placement, memory })
}

pub fn instantiate_compute_memory(
    artifact: &PlannedComputeArtifact,
    target: &TargetMemoryProfile,
    facts: &ActivationMemoryFacts,
) -> Result<ProgramMemoryPlan, MemoryPlanError> {
    let host = TargetMemoryProfile::current_native_host()?;
    let target_overrides = BTreeMap::from([(MemorySpace::Host, host)]);
    instantiate_program_memory_plan_with_target_overrides(
        &artifact.memory,
        target,
        &target_overrides,
        facts,
    )
}

fn transfer_plan(
    artifact: &ProgramArtifact,
    placement: &ComputePhysicalPlan,
    positions: &BTreeMap<mech_core::NodeId, u32>,
    elements: &BTreeMap<mech_core::CellSlotId, Option<u64>>,
    boundary: &TransferBoundary,
) -> Result<TransferPlan, MemoryPlanError> {
    let count = elements.get(&boundary.slot).copied().flatten().ok_or(
        MemoryPlanError::MissingFootprintWitness {
            stage: mech_core::MemoryWitnessStage::Activation,
        },
    )?;
    let bytes = count
        .checked_mul(4)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "compute transfer bytes",
        })?;
    let device = MemorySpace::Device {
        region: boundary
            .consumer
            .and_then(|consumer| placement.nodes.get(consumer.get() as usize))
            .and_then(|node| node.region)
            .or_else(|| producer_region(placement, artifact, boundary.slot))
            .unwrap_or(0),
    };
    let (direction, source, destination) = match boundary.direction {
        TransferDirection::Upload => (PlannedTransferDirection::Upload, MemorySpace::Host, device),
        TransferDirection::Readback => (
            PlannedTransferDirection::Readback,
            device,
            MemorySpace::Host,
        ),
    };
    let position = match boundary.consumer {
        Some(node) => positions
            .get(&node)
            .copied()
            .ok_or(MemoryPlanError::LifetimeOrderInvalid)?,
        None => {
            u32::try_from(positions.len()).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                field: "compute transfer terminal position",
            })?
        }
    };
    let first = position
        .checked_mul(2)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "compute transfer start point",
        })?;
    let last = first
        .checked_add(u32::from(boundary.consumer.is_some()))
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "compute transfer end point",
        })?;
    Ok(TransferPlan {
        slot: boundary.slot,
        direction,
        source,
        destination,
        current_bytes: bytes,
        capacity_bytes: bytes,
        lifetime: MemoryLifetime::Transfer {
            first: MemoryPlanPoint::new(first),
            last: MemoryPlanPoint::new(last),
        },
        consumer: boundary.consumer,
        interface_name: boundary.interface_name.clone(),
    })
}

fn producer_region(
    placement: &ComputePhysicalPlan,
    artifact: &ProgramArtifact,
    slot: mech_core::CellSlotId,
) -> Option<u32> {
    let declaration = artifact.slots().get(slot.get() as usize)?;
    let mech_engine::ProducerReference::NodeOutput { node, .. } = declaration.producer else {
        return None;
    };
    placement
        .nodes
        .get(node.get() as usize)
        .and_then(|node| node.region)
}
