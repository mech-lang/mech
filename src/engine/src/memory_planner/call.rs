use mech_core::{
    AllocationPlan, MemoryLifetime, MemoryObjectId, MemoryObjectOwner, MemoryPlanError,
    MemoryPlanPoint, NodeId, PortDirection,
};

pub(crate) fn node_points(
    node: NodeId,
) -> Result<(MemoryPlanPoint, MemoryPlanPoint), MemoryPlanError> {
    let before = node
        .get()
        .checked_mul(2)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "node memory-plan point",
        })?;
    let after = before
        .checked_add(1)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "node memory-plan point",
        })?;
    Ok((MemoryPlanPoint::new(before), MemoryPlanPoint::new(after)))
}

pub(crate) fn remap_call_allocations(
    node: NodeId,
    plan: &mech_core::CallMemoryPlan,
    existing_objects: &std::collections::BTreeMap<MemoryObjectId, MemoryObjectId>,
    next_id: &mut u32,
) -> Result<Vec<(MemoryObjectId, AllocationPlan)>, MemoryPlanError> {
    let (first, last) = node_points(node)?;
    plan.allocations
        .iter()
        // Input and output port storage is owned by artifact slots and was
        // already placed by the program planner. A value-level transaction
        // stage may also already own the call's publication backing. Only
        // genuinely call-local material is remapped here; otherwise every
        // call double-counts its live ports or its publication stage.
        .filter(|allocation| {
            !existing_objects.contains_key(&allocation.id)
                && matches!(
                    allocation.role,
                    mech_core::AllocationRole::OrderedIndex
                        | mech_core::AllocationRole::SelectorPlan
                        | mech_core::AllocationRole::Scratch
                        | mech_core::AllocationRole::TransactionStage
                        | mech_core::AllocationRole::TransferStage
                )
        })
        .map(|allocation| {
            let id = MemoryObjectId::new(*next_id);
            *next_id = next_id
                .checked_add(1)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "program memory-object id",
                })?;
            let owner = match allocation.owner {
                MemoryObjectOwner::DirectCallPort {
                    direction: PortDirection::Input,
                    port,
                    ..
                } => MemoryObjectOwner::NodeInput { node, port },
                MemoryObjectOwner::DirectCallPort {
                    direction: PortDirection::Output,
                    port,
                    ..
                } => MemoryObjectOwner::NodeOutput { node, port },
                MemoryObjectOwner::NodeScratch { ordinal, .. } => {
                    MemoryObjectOwner::NodeScratch { node, ordinal }
                }
                MemoryObjectOwner::TransactionStage { output, .. } => {
                    MemoryObjectOwner::TransactionStage { node, output }
                }
                ref owner => owner.clone(),
            };
            Ok((
                allocation.id,
                AllocationPlan {
                    id,
                    owner,
                    role: allocation.role,
                    space: allocation.space,
                    current_bytes: allocation.current_bytes,
                    capacity_bytes: allocation.capacity_bytes,
                    alignment: allocation.alignment,
                    lifetime: match allocation.lifetime {
                        MemoryLifetime::Transaction { .. } => {
                            MemoryLifetime::Transaction { first, last }
                        }
                        _ => MemoryLifetime::Turn { first, last },
                    },
                    placement: mech_core::ArenaPlacement {
                        arena: mech_core::MemoryArenaId::new(0),
                        offset: 0,
                    },
                    reuse_group: None,
                },
            ))
        })
        .collect()
}
