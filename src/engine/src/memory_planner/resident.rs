use std::collections::BTreeMap;

use mech_core::{
    AllocationPlan, AllocationRole, ArenaPlacement, ArenaPlan, AxisCapacityPlan, CapacityAuthority,
    CapacityRequirement, GrowthPolicy, MemoryArenaId, MemoryLifetime, MemoryObjectId,
    MemoryObjectOwner, MemoryPlanError, MemorySpace, PayloadCapacityPlan,
    PhysicalStorageDescriptor, PlannedSlotKind, ResidentValueKind, ResourceDemand,
    ScalarMemoryKind, SlotLayout, StorageAccessCapabilities, StorageAccountingCapability,
    StorageAddressingCapabilities, StorageCanonicalizationCapabilities,
    StorageCapabilityDescriptor, StorageElementKind, StorageExtentCapability, StorageLayoutClass,
    StorageOwnershipCapabilities, StoragePublicationCapabilities, StorageTopology,
    TargetMemoryProfile, TransactionRequirement, ValueLayoutPlan,
};

use super::{PlannedValueClass, ProgramMemoryPlan, ValueMemoryPlan, checked_demand_add};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentValuePlanInput {
    pub owner: MemoryObjectOwner,
    pub slot: Option<mech_core::CellSlotId>,
    pub descriptor: mech_core::ResolvedValueDescriptor,
    pub class: PlannedValueClass,
    pub kind: ResidentValueKind,
    pub elements: u64,
    pub lifetime: MemoryLifetime,
    pub producer: Option<mech_core::NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentArenaProjection {
    pub plan: ProgramMemoryPlan,
    pub element_offsets: BTreeMap<MemoryObjectOwner, usize>,
}

/// Projects the current typed Resident arenas into the shared R5 plan.
///
/// Arena identities are stable by `(class, lane, state-buffer-index)`. The
/// second state buffer is explicit in the plan; returned offsets address the
/// published buffer because the runtime uses the same offset in both buffers.
pub fn plan_resident_arenas(
    inputs: &[ResidentValuePlanInput],
) -> Result<ResidentArenaProjection, MemoryPlanError> {
    let target = TargetMemoryProfile::current_resident_cpu()?;
    let mut ordered = inputs.to_vec();
    ordered.sort_by(|left, right| left.owner.cmp(&right.owner));
    let mut cursors = BTreeMap::<(PlannedValueClass, ResidentValueKind, u8), u64>::new();
    let mut members =
        BTreeMap::<(PlannedValueClass, ResidentValueKind, u8), Vec<MemoryObjectId>>::new();
    let mut allocations = Vec::new();
    let mut values = Vec::new();
    let mut offsets = BTreeMap::new();
    let mut demand = ResourceDemand::default();
    let mut next_id = 0_u32;
    for input in ordered {
        let slot = resident_slot_layout(&target, input.kind);
        let bytes =
            input
                .elements
                .checked_mul(slot.bytes)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "resident arena bytes",
                })?;
        let key = (input.class, input.kind, 0);
        let offset_bytes = *cursors.get(&key).unwrap_or(&0);
        let offset_elements = if slot.bytes == 0 {
            0
        } else {
            offset_bytes / slot.bytes
        };
        offsets.insert(
            input.owner.clone(),
            usize::try_from(offset_elements).map_err(|_| MemoryPlanError::TargetAddressOverflow)?,
        );
        let id = MemoryObjectId::new(next_id);
        next_id = checked_next(next_id)?;
        let lifetime = input.lifetime;
        let arena = resident_arena_id(key)?;
        let allocation = AllocationPlan {
            id,
            owner: input.owner.clone(),
            role: AllocationRole::FixedStorage,
            space: MemorySpace::ResidentCpu,
            current_bytes: bytes,
            capacity_bytes: bytes,
            alignment: slot.alignment,
            lifetime,
            placement: ArenaPlacement {
                arena,
                offset: offset_bytes,
            },
            reuse_group: None,
        };
        cursors.insert(
            key,
            offset_bytes
                .checked_add(bytes)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "resident arena capacity",
                })?,
        );
        members.entry(key).or_default().push(id);
        demand = checked_demand_add(demand, allocation_demand(&allocation))?;

        let mut transaction = TransactionRequirement::None;
        if input.class == PlannedValueClass::State {
            let next = MemoryObjectId::new(next_id);
            next_id = checked_next(next_id)?;
            let next_key = (input.class, input.kind, 1);
            let next_offset = *cursors.get(&next_key).unwrap_or(&0);
            let next_arena = resident_arena_id(next_key)?;
            let staged = AllocationPlan {
                id: next,
                owner: input.owner.clone(),
                role: AllocationRole::TransactionStage,
                space: MemorySpace::ResidentCpu,
                current_bytes: bytes,
                capacity_bytes: bytes,
                alignment: slot.alignment,
                lifetime: MemoryLifetime::Activation,
                placement: ArenaPlacement {
                    arena: next_arena,
                    offset: next_offset,
                },
                reuse_group: None,
            };
            cursors.insert(
                next_key,
                next_offset
                    .checked_add(bytes)
                    .ok_or(MemoryPlanError::ArithmeticOverflow {
                        field: "resident state buffer capacity",
                    })?,
            );
            members.entry(next_key).or_default().push(next);
            demand = checked_demand_add(demand, allocation_demand(&staged))?;
            allocations.push(staged);
            transaction = TransactionRequirement::DoubleBuffer { current: id, next };
        }
        let layout = resident_value_layout(&input, slot, bytes)?;
        if let Some(value_slot) = input.slot {
            values.push(ValueMemoryPlan {
                slot: value_slot,
                object: id,
                descriptor: input.descriptor,
                layout,
                class: input.class,
                producer: input.producer,
                lifetime,
                alias_group: None,
                reuse_group: None,
                transaction,
            });
        }
        allocations.push(allocation);
    }
    allocations.sort_by_key(|allocation| allocation.id);
    values.sort_by_key(|value| value.slot);
    let mut arenas = cursors
        .into_iter()
        .map(|(key, capacity_bytes)| {
            let id = resident_arena_id(key)?;
            let alignment = resident_slot_layout(&target, key.1).alignment;
            Ok(ArenaPlan {
                id,
                space: MemorySpace::ResidentCpu,
                alignment,
                capacity_bytes,
                members: members.remove(&key).unwrap_or_default().into_boxed_slice(),
            })
        })
        .collect::<Result<Vec<_>, MemoryPlanError>>()?;
    arenas.sort_by_key(|arena| arena.id);
    let mut budget_violations = Vec::new();
    for allocation in &allocations {
        budget_violations.extend(mech_core::evaluate_memory_budget(
            allocation.owner.clone(),
            allocation_demand(allocation),
            allocation.capacity_bytes,
            0,
            target.limits,
        ));
    }
    for arena in &arenas {
        let owner = arena
            .members
            .first()
            .and_then(|id| allocations.iter().find(|allocation| allocation.id == *id))
            .map(|allocation| allocation.owner.clone())
            .unwrap_or(MemoryObjectOwner::NodeScratch {
                node: mech_core::NodeId::new(0),
                ordinal: 0,
            });
        budget_violations.extend(mech_core::evaluate_memory_budget(
            owner,
            ResourceDemand::default(),
            arena.capacity_bytes,
            0,
            target.limits,
        ));
    }
    budget_violations.sort();
    budget_violations.dedup();
    Ok(ResidentArenaProjection {
        plan: ProgramMemoryPlan {
            values: values.into_boxed_slice(),
            call_nodes: Box::new([]),
            calls: Box::new([]),
            allocations: allocations.into_boxed_slice(),
            arenas: arenas.into_boxed_slice(),
            transfers: Box::new([]),
            peak: demand,
            budget_violations: budget_violations.into_boxed_slice(),
        },
        element_offsets: offsets,
    })
}

pub fn resident_storage_descriptor(
    descriptor: &mech_core::ResolvedValueDescriptor,
    kind: ResidentValueKind,
    lifetime: MemoryLifetime,
) -> Result<PhysicalStorageDescriptor, MemoryPlanError> {
    let scalar = match kind {
        ResidentValueKind::Bool => Some(ScalarMemoryKind::Bool),
        ResidentValueKind::Index => Some(ScalarMemoryKind::Index),
        ResidentValueKind::F64 => Some(ScalarMemoryKind::Floating(mech_core::FloatWidth::W64)),
        ResidentValueKind::String => Some(ScalarMemoryKind::String),
        ResidentValueKind::Snapshot => None,
    };
    let semantic_topology = descriptor
        .schema()
        .type_memory_contract()
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?
        .topology;
    let topology = match semantic_topology {
        mech_core::MemoryTopology::Scalar(_) => scalar
            .map(StorageTopology::Scalar)
            .unwrap_or(StorageTopology::CanonicalValue),
        mech_core::MemoryTopology::DenseSequence { .. } => {
            scalar.map_or(StorageTopology::CanonicalValue, |element| {
                StorageTopology::DenseSequence {
                    element: StorageElementKind::Scalar(element),
                }
            })
        }
        _ => StorageTopology::CanonicalValue,
    };
    let extent = match topology {
        StorageTopology::Scalar(_) => StorageExtentCapability::Single,
        StorageTopology::DenseSequence { .. } => {
            StorageExtentCapability::ResizableDimensions(vec![None, None].into_boxed_slice())
        }
        StorageTopology::CanonicalValue => StorageExtentCapability::Any,
        _ => return Err(MemoryPlanError::UnsupportedStorageLayout),
    };
    Ok(PhysicalStorageDescriptor {
        capabilities: StorageCapabilityDescriptor {
            topology,
            extent,
            addressing: StorageAddressingCapabilities {
                whole_value: true,
                positional: mech_core::PositionalAddressingCapability::AnyRank,
                named_members: kind == ResidentValueKind::Snapshot,
                keyed_members: kind == ResidentValueKind::Snapshot,
                arbitrary_regions: true,
            },
            canonicalization: StorageCanonicalizationCapabilities {
                self_describing: kind == ResidentValueKind::Snapshot,
                recursive: kind == ResidentValueKind::Snapshot
                    || matches!(
                        semantic_topology,
                        mech_core::MemoryTopology::DenseSequence { .. }
                    ),
                tagged: kind == ResidentValueKind::Snapshot,
                ordered_keys: kind == ResidentValueKind::Snapshot,
                unique_keys: kind == ResidentValueKind::Snapshot,
            },
            access: StorageAccessCapabilities {
                readable: true,
                writable: true,
                replaceable: true,
                region_mutable: true,
                canonical_snapshot: true,
            },
            ownership: StorageOwnershipCapabilities {
                shared_read: true,
                exclusive_write: true,
                owned_value: true,
                detachable: true,
            },
            publication: StoragePublicationCapabilities {
                atomic_replace: true,
                preserves_previous_on_failure: true,
            },
            accounting: StorageAccountingCapability::CanonicalSnapshot,
        },
        slot: match kind {
            ResidentValueKind::Bool => PlannedSlotKind::FixedScalar(ScalarMemoryKind::Bool),
            ResidentValueKind::Index => PlannedSlotKind::FixedScalar(ScalarMemoryKind::Index),
            ResidentValueKind::F64 => {
                PlannedSlotKind::FixedScalar(ScalarMemoryKind::Floating(mech_core::FloatWidth::W64))
            }
            ResidentValueKind::String => PlannedSlotKind::StringHeader,
            ResidentValueKind::Snapshot => PlannedSlotKind::CanonicalValueHandle,
        },
        space: MemorySpace::ResidentCpu,
        lifetime,
        reusable_turn_temporary: matches!(lifetime, MemoryLifetime::Turn { .. }),
    })
}

pub fn resident_arena_id(
    key: (PlannedValueClass, ResidentValueKind, u8),
) -> Result<MemoryArenaId, MemoryPlanError> {
    let class = match key.0 {
        PlannedValueClass::Constant => 0_u32,
        PlannedValueClass::Input => 1,
        PlannedValueClass::State => 2,
        PlannedValueClass::Scratch => 3,
    };
    let kind = match key.1 {
        ResidentValueKind::Bool => 0_u32,
        ResidentValueKind::Index => 1,
        ResidentValueKind::F64 => 2,
        ResidentValueKind::String => 3,
        ResidentValueKind::Snapshot => 4,
    };
    let raw = u32::from(key.2)
        .checked_mul(20)
        .and_then(|base| {
            class
                .checked_mul(5)
                .and_then(|class| base.checked_add(class))
        })
        .and_then(|base| base.checked_add(kind))
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "resident arena id",
        })?;
    Ok(MemoryArenaId::new(raw))
}

/// Adds the turn-scoped capture backing for an external effect to the shared
/// resident plan and returns its element offset in the dedicated effect arena.
pub fn plan_resident_effect_payload(
    plan: &mut ProgramMemoryPlan,
    node: mech_core::NodeId,
    kind: ResidentValueKind,
    elements: u64,
) -> Result<usize, MemoryPlanError> {
    let target = TargetMemoryProfile::current_resident_cpu()?;
    let slot = resident_slot_layout(&target, kind);
    let bytes = elements
        .checked_mul(slot.bytes)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "resident effect payload bytes",
        })?;
    let key = (PlannedValueClass::Scratch, kind, 1);
    let arena_id = resident_arena_id(key)?;
    let mut arenas = plan.arenas.to_vec();
    let arena_index = match arenas.iter().position(|arena| arena.id == arena_id) {
        Some(index) => index,
        None => {
            arenas.push(ArenaPlan {
                id: arena_id,
                space: MemorySpace::ResidentCpu,
                alignment: slot.alignment,
                capacity_bytes: 0,
                members: Box::new([]),
            });
            arenas.len() - 1
        }
    };
    let offset_bytes = arenas[arena_index].capacity_bytes;
    let end = offset_bytes
        .checked_add(bytes)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "resident effect arena capacity",
        })?;
    let id = MemoryObjectId::new(u32::try_from(plan.allocations.len()).map_err(|_| {
        MemoryPlanError::ArithmeticOverflow {
            field: "resident effect memory-object id",
        }
    })?);
    let before = node
        .get()
        .checked_mul(2)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "resident effect start point",
        })?;
    let after = before
        .checked_add(1)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "resident effect end point",
        })?;
    let allocation = AllocationPlan {
        id,
        owner: MemoryObjectOwner::NodeInput { node, port: 0 },
        role: AllocationRole::Scratch,
        space: MemorySpace::ResidentCpu,
        current_bytes: bytes,
        capacity_bytes: bytes,
        alignment: slot.alignment,
        lifetime: MemoryLifetime::Turn {
            first: mech_core::MemoryPlanPoint::new(before),
            last: mech_core::MemoryPlanPoint::new(after),
        },
        placement: ArenaPlacement {
            arena: arena_id,
            offset: offset_bytes,
        },
        reuse_group: None,
    };
    let mut allocations = plan.allocations.to_vec();
    allocations.push(allocation);
    arenas[arena_index].capacity_bytes = end;
    let mut members = arenas[arena_index].members.to_vec();
    members.push(id);
    arenas[arena_index].members = members.into_boxed_slice();
    arenas.sort_by_key(|arena| arena.id);
    plan.allocations = allocations.into_boxed_slice();
    plan.arenas = arenas.into_boxed_slice();
    plan.peak.turn_peak_bytes = plan.peak.turn_peak_bytes.checked_add(bytes).ok_or(
        MemoryPlanError::ArithmeticOverflow {
            field: "resident effect turn peak",
        },
    )?;
    let mut violations = plan.budget_violations.to_vec();
    violations.extend(mech_core::evaluate_memory_budget(
        MemoryObjectOwner::NodeInput { node, port: 0 },
        allocation_demand(
            plan.allocations
                .last()
                .ok_or(MemoryPlanError::DescriptorMismatch)?,
        ),
        end,
        0,
        target.limits,
    ));
    violations.sort();
    violations.dedup();
    plan.budget_violations = violations.into_boxed_slice();
    Ok(usize::try_from(offset_bytes / slot.bytes)
        .map_err(|_| MemoryPlanError::TargetAddressOverflow)?)
}

fn resident_slot_layout(target: &TargetMemoryProfile, kind: ResidentValueKind) -> SlotLayout {
    match kind {
        ResidentValueKind::Bool => target.primitives.bool_slot,
        ResidentValueKind::Index => target.primitives.index_slot,
        ResidentValueKind::F64 => target.primitives.f64_slot,
        ResidentValueKind::String => target.primitives.string_header,
        ResidentValueKind::Snapshot => target.primitives.canonical_value_handle,
    }
}

fn resident_value_layout(
    input: &ResidentValuePlanInput,
    slot: SlotLayout,
    bytes: u64,
) -> Result<ValueLayoutPlan, MemoryPlanError> {
    let dimensions = input.descriptor.current_extents().unwrap_or_default();
    let axes = dimensions
        .iter()
        .copied()
        .map(|current| AxisCapacityPlan {
            current,
            capacity: CapacityRequirement {
                current,
                required: current,
                maximum: Some(current),
                authority: CapacityAuthority::ActivationSemantic,
                growth: GrowthPolicy::Fixed,
            },
            evolution: mech_core::ExtentEvolution::ActivationFixed,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let strides_bytes = if dimensions.len() == 2 {
        vec![
            slot.bytes,
            dimensions[0]
                .checked_mul(slot.bytes)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "resident column stride",
                })?,
        ]
        .into_boxed_slice()
    } else {
        Box::new([])
    };
    Ok(ValueLayoutPlan {
        storage: match input.kind {
            ResidentValueKind::Snapshot => StorageLayoutClass::CanonicalSnapshot {
                topology: input
                    .descriptor
                    .schema()
                    .type_memory_contract()
                    .map_err(|_| MemoryPlanError::DescriptorMismatch)?
                    .topology,
            },
            ResidentValueKind::String if dimensions.is_empty() => StorageLayoutClass::Scalar {
                slot: PlannedSlotKind::StringHeader,
            },
            kind => StorageLayoutClass::DenseColumnMajor {
                slot: match kind {
                    ResidentValueKind::Bool => PlannedSlotKind::FixedScalar(ScalarMemoryKind::Bool),
                    ResidentValueKind::Index => {
                        PlannedSlotKind::FixedScalar(ScalarMemoryKind::Index)
                    }
                    ResidentValueKind::F64 => PlannedSlotKind::FixedScalar(
                        ScalarMemoryKind::Floating(mech_core::FloatWidth::W64),
                    ),
                    ResidentValueKind::String => PlannedSlotKind::StringHeader,
                    ResidentValueKind::Snapshot => unreachable!(),
                },
            },
        },
        axes,
        current_elements: input.elements,
        capacity_elements: CapacityRequirement {
            current: input.elements,
            required: input.elements,
            maximum: Some(input.elements),
            authority: CapacityAuthority::ActivationSemantic,
            growth: GrowthPolicy::Fixed,
        },
        slot,
        strides_bytes,
        current_address_span_bytes: bytes,
        capacity_bytes: bytes,
        payload: PayloadCapacityPlan {
            current_bytes: 0,
            required_bytes: 0,
            maximum_bytes: None,
            current_nodes: input.elements,
            required_nodes: input.elements,
            maximum_nodes: None,
            authority: CapacityAuthority::CurrentValueWitness,
            growth: GrowthPolicy::ReplanBeforeGrowth,
        },
    })
}

fn allocation_demand(allocation: &AllocationPlan) -> ResourceDemand {
    let mut demand = ResourceDemand::default();
    match allocation.lifetime {
        MemoryLifetime::Program => demand.persistent_bytes = allocation.capacity_bytes,
        MemoryLifetime::Activation => demand.activation_bytes = allocation.capacity_bytes,
        MemoryLifetime::Turn { .. } => demand.turn_peak_bytes = allocation.capacity_bytes,
        MemoryLifetime::Transaction { .. } => {
            demand.transaction_peak_bytes = allocation.capacity_bytes
        }
        MemoryLifetime::Transfer { .. } => demand.transfer_bytes = allocation.capacity_bytes,
    }
    demand
}

fn checked_next(value: u32) -> Result<u32, MemoryPlanError> {
    value
        .checked_add(1)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "resident memory-object id",
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_arena_ids_are_stable_and_state_buffers_are_distinct() {
        let current =
            resident_arena_id((PlannedValueClass::State, ResidentValueKind::F64, 0)).unwrap();
        let next =
            resident_arena_id((PlannedValueClass::State, ResidentValueKind::F64, 1)).unwrap();
        assert_eq!(current, MemoryArenaId::new(12));
        assert_eq!(next, MemoryArenaId::new(32));
    }
}
