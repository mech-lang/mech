use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    AliasDecision, AliasGroupId, AllocationPlan, AllocationRole, ArenaPlacement, ArenaPlan,
    BoundCall, CallMemoryPlan, CellSlotId, CurrentMemoryFootprint, MemoryArenaId,
    MemoryBudgetViolation, MemoryFootprintWitness, MemoryLifetime, MemoryObjectId,
    MemoryObjectOwner, MemoryPlanError, MemoryPlanPoint, MemorySpace, PhysicalStorageDescriptor,
    ResourceDemand, ReuseGroupId, TargetMemoryProfile, TransactionRequirement, TransferPlan,
    ValueLayoutPlan, ValueLayoutPlanningRequest, plan_value_layout,
};

use crate::{ArtifactSource, BindingDeclaration, ProducerReference, ProgramArtifact, SlotRole};

use super::{node_points, remap_call_allocations};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlannedValueClass {
    Constant,
    Input,
    State,
    Scratch,
}

fn program_alias_sources(
    artifact: &ProgramArtifact,
    calls: &[CallMemoryPlan],
) -> Result<BTreeMap<CellSlotId, CellSlotId>, MemoryPlanError> {
    let mut aliases = BTreeMap::new();
    for (node, call) in artifact.nodes().iter().zip(calls) {
        let inputs = &artifact.bindings()
            [node.input_bindings.start as usize..node.input_bindings.end as usize];
        let outputs = &artifact.bindings()
            [node.output_bindings.start as usize..node.output_bindings.end as usize];
        for (output_ordinal, decision) in call.aliases.iter().copied().enumerate() {
            let input_ordinal = match decision {
                AliasDecision::BorrowInput { input }
                | AliasDecision::ReuseInput { input }
                | AliasDecision::InPlaceRequired { input } => input,
                AliasDecision::Disjoint | AliasDecision::StageThenPublish { .. } => continue,
            };
            let source = inputs.iter().find_map(|binding| match binding {
                BindingDeclaration::Input {
                    port_ordinal,
                    source: ArtifactSource::Slot(slot),
                    ..
                } if *port_ordinal == input_ordinal => Some(*slot),
                _ => None,
            });
            let output_ordinal =
                u16::try_from(output_ordinal).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                    field: "alias output ordinal",
                })?;
            let target = outputs.iter().find_map(|binding| match binding {
                BindingDeclaration::Output {
                    port_ordinal,
                    target,
                    ..
                } if *port_ordinal == output_ordinal => Some(*target),
                _ => None,
            });
            if let (Some(source), Some(target)) = (source, target) {
                if aliases.insert(target, source).is_some() {
                    return Err(MemoryPlanError::DescriptorMismatch);
                }
            }
        }
    }
    Ok(aliases)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueMemoryPlanTemplate {
    pub slot: CellSlotId,
    pub descriptor: Option<mech_core::ResolvedValueDescriptor>,
    pub class: PlannedValueClass,
    pub producer: Option<mech_core::NodeId>,
    pub last_consumer: Option<mech_core::NodeId>,
    /// Another logical slot selected by the call plan as the same physical
    /// allocation. The slot identity remains distinct from this alias edge.
    pub alias_source: Option<CellSlotId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueMemoryPlan {
    pub slot: CellSlotId,
    pub object: MemoryObjectId,
    pub descriptor: mech_core::ResolvedValueDescriptor,
    pub layout: ValueLayoutPlan,
    pub class: PlannedValueClass,
    pub producer: Option<mech_core::NodeId>,
    pub lifetime: MemoryLifetime,
    pub alias_group: Option<AliasGroupId>,
    pub reuse_group: Option<ReuseGroupId>,
    pub transaction: TransactionRequirement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationValueFact {
    pub descriptor: mech_core::ResolvedValueDescriptor,
    pub storage: PhysicalStorageDescriptor,
    pub witness: MemoryFootprintWitness,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActivationMemoryFacts {
    pub values: BTreeMap<CellSlotId, ActivationValueFact>,
    pub classes: BTreeMap<CellSlotId, PlannedValueClass>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMemoryPlanTemplate {
    pub values: Box<[ValueMemoryPlanTemplate]>,
    pub calls: Box<[CallMemoryPlan]>,
    pub allocations: Box<[AllocationPlan]>,
    pub transfers: Box<[TransferPlan]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramMemoryPlan {
    pub values: Box<[ValueMemoryPlan]>,
    pub calls: Box<[CallMemoryPlan]>,
    pub allocations: Box<[AllocationPlan]>,
    pub arenas: Box<[ArenaPlan]>,
    pub transfers: Box<[TransferPlan]>,
    pub peak: ResourceDemand,
    pub budget_violations: Box<[MemoryBudgetViolation]>,
}

pub fn plan_program_memory_template(
    artifact: &ProgramArtifact,
    instruction_bindings: &[Option<BoundCall>],
    instruction_memory_plans: &[Option<CallMemoryPlan>],
) -> Result<ProgramMemoryPlanTemplate, MemoryPlanError> {
    if instruction_bindings.len() != instruction_memory_plans.len() {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    let mut calls = Vec::new();
    for (binding, plan) in instruction_bindings.iter().zip(instruction_memory_plans) {
        match (binding, plan) {
            (Some(binding), Some(plan)) if binding == &plan.bound_call => calls.push(plan.clone()),
            (None, None) => {}
            _ => return Err(MemoryPlanError::DescriptorMismatch),
        }
    }
    if calls.len() != artifact.nodes().len() {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    for (node, call) in artifact.nodes().iter().zip(&calls) {
        if call
            .bound_call
            .operation_descriptor()
            .canonical_name
            .as_ref()
            != node.operation.canonical_name()
        {
            return Err(MemoryPlanError::DescriptorMismatch);
        }
    }

    let mut consumers = BTreeMap::<CellSlotId, mech_core::NodeId>::new();
    for binding in artifact.bindings() {
        if let BindingDeclaration::Input {
            node,
            source: ArtifactSource::Slot(slot),
            ..
        } = binding
        {
            consumers
                .entry(*slot)
                .and_modify(|last| *last = (*last).max(*node))
                .or_insert(*node);
        }
    }
    let alias_sources = program_alias_sources(artifact, &calls)?;
    let values = artifact
        .slots()
        .iter()
        .map(|slot| {
            let descriptor = slot_descriptor(artifact, slot.slot, slot.schema);
            let producer = match slot.producer {
                ProducerReference::NodeOutput { node, .. } => Some(node),
                _ => None,
            };
            ValueMemoryPlanTemplate {
                slot: slot.slot,
                descriptor,
                class: default_value_class(slot.role, slot.producer),
                producer,
                last_consumer: consumers.get(&slot.slot).copied(),
                alias_source: alias_sources.get(&slot.slot).copied(),
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    Ok(ProgramMemoryPlanTemplate {
        values,
        calls: calls.into_boxed_slice(),
        allocations: Box::new([]),
        transfers: Box::new([]),
    })
}

pub fn instantiate_program_memory_plan(
    template: &ProgramMemoryPlanTemplate,
    target: &TargetMemoryProfile,
    facts: &ActivationMemoryFacts,
) -> Result<ProgramMemoryPlan, MemoryPlanError> {
    let mut next_id = 0_u32;
    let mut values = Vec::with_capacity(template.values.len());
    let mut allocations = Vec::new();
    for value in &template.values {
        let fact =
            facts
                .values
                .get(&value.slot)
                .ok_or(MemoryPlanError::MissingFootprintWitness {
                    stage: mech_core::MemoryWitnessStage::Activation,
                })?;
        if value
            .descriptor
            .as_ref()
            .is_some_and(|descriptor| descriptor != &fact.descriptor)
        {
            return Err(MemoryPlanError::DescriptorMismatch);
        }
        let layout = plan_value_layout(ValueLayoutPlanningRequest {
            descriptor: &fact.descriptor,
            storage: &fact.storage,
            witness: fact.witness,
            target,
        })?;
        let class = facts
            .classes
            .get(&value.slot)
            .copied()
            .unwrap_or(value.class);
        let lifetime = value_lifetime(value, class)?;
        let object = MemoryObjectId::new(next_id);
        next_id = checked_next_id(next_id)?;
        allocations.push(AllocationPlan {
            id: object,
            owner: MemoryObjectOwner::Slot(value.slot),
            role: AllocationRole::FixedStorage,
            space: fact.storage.space,
            current_bytes: layout.current_address_span_bytes,
            capacity_bytes: layout.capacity_bytes,
            alignment: layout.slot.alignment,
            lifetime,
            placement: ArenaPlacement {
                arena: MemoryArenaId::new(0),
                offset: 0,
            },
            reuse_group: None,
        });
        if layout.payload.required_bytes != 0 {
            let payload = MemoryObjectId::new(next_id);
            next_id = checked_next_id(next_id)?;
            allocations.push(AllocationPlan {
                id: payload,
                owner: MemoryObjectOwner::Slot(value.slot),
                role: AllocationRole::VariablePayload,
                space: fact.storage.space,
                current_bytes: layout.payload.current_bytes,
                capacity_bytes: layout.payload.required_bytes,
                alignment: 1,
                lifetime,
                placement: ArenaPlacement {
                    arena: MemoryArenaId::new(0),
                    offset: 0,
                },
                reuse_group: None,
            });
        }
        let transaction = if class == PlannedValueClass::State {
            let next = MemoryObjectId::new(next_id);
            next_id = checked_next_id(next_id)?;
            allocations.push(AllocationPlan {
                id: next,
                owner: MemoryObjectOwner::Slot(value.slot),
                role: AllocationRole::TransactionStage,
                space: fact.storage.space,
                current_bytes: layout.current_address_span_bytes,
                capacity_bytes: layout.capacity_bytes,
                alignment: layout.slot.alignment,
                lifetime: MemoryLifetime::Activation,
                placement: ArenaPlacement {
                    arena: MemoryArenaId::new(0),
                    offset: 0,
                },
                reuse_group: None,
            });
            TransactionRequirement::DoubleBuffer {
                current: object,
                next,
            }
        } else {
            TransactionRequirement::None
        };
        values.push(ValueMemoryPlan {
            slot: value.slot,
            object,
            descriptor: fact.descriptor.clone(),
            layout,
            class,
            producer: value.producer,
            lifetime,
            alias_group: None,
            reuse_group: None,
            transaction,
        });
    }

    for (node, call) in template.calls.iter().enumerate() {
        let node = mech_core::NodeId::new(u32::try_from(node).map_err(|_| {
            MemoryPlanError::ArithmeticOverflow {
                field: "program node id",
            }
        })?);
        allocations.extend(remap_call_allocations(node, call, &mut next_id)?);
    }
    for mut allocation in template.allocations.iter().cloned() {
        allocation.id = MemoryObjectId::new(next_id);
        next_id = checked_next_id(next_id)?;
        allocations.push(allocation);
    }

    assign_alias_groups(&mut values, &template.values)?;
    assign_reuse_groups(&mut allocations, &values)?;
    let arenas = place_allocations(&mut allocations)?;
    for value in &mut values {
        value.reuse_group = allocations
            .iter()
            .find(|allocation| allocation.id == value.object)
            .and_then(|allocation| allocation.reuse_group);
    }
    let peak = program_peak(&allocations, &template.calls, &template.transfers)?;
    let mut budget_violations = Vec::new();
    for allocation in &allocations {
        let demand = demand_for_allocation(allocation)?;
        budget_violations.extend(mech_core::evaluate_memory_budget(
            allocation.owner.clone(),
            demand,
            allocation.capacity_bytes,
            matches!(allocation.space, MemorySpace::Device { .. })
                .then_some(allocation.capacity_bytes)
                .unwrap_or(0),
            target.limits,
        ));
    }
    budget_violations.sort();
    budget_violations.dedup();
    Ok(ProgramMemoryPlan {
        values: values.into_boxed_slice(),
        calls: template.calls.clone(),
        allocations: allocations.into_boxed_slice(),
        arenas,
        transfers: template.transfers.clone(),
        peak,
        budget_violations: budget_violations.into_boxed_slice(),
    })
}

impl ProgramMemoryPlan {
    /// Stable, pointer-free text used by audits and cross-process determinism
    /// checks. The plan model contains only ordered collections and semantic
    /// identifiers, so its structured debug form is a canonical diagnostic
    /// projection rather than a wire format.
    pub fn diagnostic_text(&self) -> String {
        format!("{self:#?}")
    }
}
fn slot_descriptor(
    artifact: &ProgramArtifact,
    slot: CellSlotId,
    schema: mech_core::SchemaId,
) -> Option<mech_core::ResolvedValueDescriptor> {
    let declaration = artifact.slots().get(slot.get() as usize)?;
    let shape = declaration
        .initializer
        .and_then(|initializer| match initializer {
            crate::InitializerReference::Constant(constant) => artifact
                .constants()
                .get(constant)
                .map(|value| value.shape().clone()),
        })
        .or_else(|| artifact.slot_shape_hint(slot).cloned())?;
    let schema = artifact.schemas().entry(schema)?.schema().clone();
    mech_core::ResolvedValueDescriptor::from_schema(schema, shape).ok()
}

fn default_value_class(role: SlotRole, producer: ProducerReference) -> PlannedValueClass {
    match role {
        SlotRole::Input => PlannedValueClass::Input,
        SlotRole::State | SlotRole::Output => PlannedValueClass::State,
        SlotRole::Derived => match producer {
            ProducerReference::Input(_) => PlannedValueClass::Input,
            ProducerReference::Output { .. } => PlannedValueClass::State,
            ProducerReference::NodeOutput { .. } => PlannedValueClass::Scratch,
        },
    }
}

fn value_lifetime(
    value: &ValueMemoryPlanTemplate,
    class: PlannedValueClass,
) -> Result<MemoryLifetime, MemoryPlanError> {
    if class != PlannedValueClass::Scratch {
        return Ok(match class {
            PlannedValueClass::Constant => MemoryLifetime::Program,
            PlannedValueClass::Input | PlannedValueClass::State => MemoryLifetime::Activation,
            PlannedValueClass::Scratch => unreachable!(),
        });
    }
    let producer = value
        .producer
        .ok_or(MemoryPlanError::LifetimeOrderInvalid)?;
    let (first, producer_after) = node_points(producer)?;
    let last = match value.last_consumer {
        Some(consumer) => node_points(consumer)?.1,
        None => producer_after,
    };
    if last < first {
        return Err(MemoryPlanError::LifetimeOrderInvalid);
    }
    Ok(MemoryLifetime::Turn { first, last })
}

fn assign_alias_groups(
    values: &mut [ValueMemoryPlan],
    templates: &[ValueMemoryPlanTemplate],
) -> Result<(), MemoryPlanError> {
    let object_by_slot = values
        .iter()
        .map(|value| (value.slot, value.object))
        .collect::<BTreeMap<_, _>>();
    let mut parent = object_by_slot
        .values()
        .copied()
        .map(|object| (object, object))
        .collect::<BTreeMap<_, _>>();
    for value in values.iter() {
        let Some(source_slot) = templates
            .get(value.slot.get() as usize)
            .and_then(|template| template.alias_source)
        else {
            continue;
        };
        let source = object_by_slot
            .get(&source_slot)
            .copied()
            .ok_or(MemoryPlanError::DescriptorMismatch)?;
        union_objects(&mut parent, value.object, source)?;
    }
    let mut members = BTreeMap::<MemoryObjectId, Vec<MemoryObjectId>>::new();
    for object in object_by_slot.values().copied() {
        let root = find_object(&parent, object)?;
        members.entry(root).or_default().push(object);
    }
    for value in values {
        let root = find_object(&parent, value.object)?;
        if members.get(&root).is_some_and(|group| group.len() > 1) {
            value.alias_group = Some(AliasGroupId::new(root.get()));
        }
    }
    Ok(())
}

fn find_object(
    parent: &BTreeMap<MemoryObjectId, MemoryObjectId>,
    mut object: MemoryObjectId,
) -> Result<MemoryObjectId, MemoryPlanError> {
    loop {
        let next = parent
            .get(&object)
            .copied()
            .ok_or(MemoryPlanError::DescriptorMismatch)?;
        if next == object {
            return Ok(object);
        }
        object = next;
    }
}

fn union_objects(
    parent: &mut BTreeMap<MemoryObjectId, MemoryObjectId>,
    left: MemoryObjectId,
    right: MemoryObjectId,
) -> Result<(), MemoryPlanError> {
    let left = find_object(parent, left)?;
    let right = find_object(parent, right)?;
    let root = left.min(right);
    let child = left.max(right);
    parent.insert(child, root);
    Ok(())
}

fn assign_reuse_groups(
    allocations: &mut [AllocationPlan],
    values: &[ValueMemoryPlan],
) -> Result<(), MemoryPlanError> {
    #[derive(Clone, Copy)]
    struct Group {
        id: ReuseGroupId,
        space: MemorySpace,
        alignment: u32,
        capacity: u64,
        last: MemoryPlanPoint,
    }
    let mut candidates = allocations
        .iter()
        .enumerate()
        .filter_map(|(index, allocation)| {
            let MemoryLifetime::Turn { first, last } = allocation.lifetime else {
                return None;
            };
            let eligible = allocation.role == AllocationRole::FixedStorage
                && values.iter().any(|value| {
                    value.object == allocation.id
                        && value.class == PlannedValueClass::Scratch
                        && value.alias_group.is_none()
                        && value.layout.payload.required_bytes == 0
                        && matches!(
                            value.layout.storage,
                            mech_core::StorageLayoutClass::Scalar {
                                slot: mech_core::PlannedSlotKind::FixedScalar(_)
                            } | mech_core::StorageLayoutClass::DenseColumnMajor {
                                slot: mech_core::PlannedSlotKind::FixedScalar(_)
                            }
                        )
                        && value.layout.capacity_elements.maximum.is_some()
                });
            eligible.then_some((first, allocation.id, last, index))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(first, id, _, _)| (*first, *id));
    let mut groups = Vec::<Group>::new();
    for (_, _, last, index) in candidates {
        let allocation = &allocations[index];
        let selected = groups.iter().position(|group| {
            group.last
                < match allocation.lifetime {
                    MemoryLifetime::Turn { first, .. } => first,
                    _ => unreachable!(),
                }
                && group.space == allocation.space
                && group.alignment >= allocation.alignment
                && group.capacity >= allocation.capacity_bytes
        });
        let id = if let Some(selected) = selected {
            groups[selected].last = last;
            groups[selected].id
        } else {
            let id = ReuseGroupId::new(u32::try_from(groups.len()).map_err(|_| {
                MemoryPlanError::ArithmeticOverflow {
                    field: "reuse group id",
                }
            })?);
            groups.push(Group {
                id,
                space: allocation.space,
                alignment: allocation.alignment,
                capacity: allocation.capacity_bytes,
                last,
            });
            id
        };
        allocations[index].reuse_group = Some(id);
    }
    Ok(())
}

fn place_allocations(
    allocations: &mut [AllocationPlan],
) -> Result<Box<[ArenaPlan]>, MemoryPlanError> {
    let mut spaces = allocations
        .iter()
        .map(|allocation| allocation.space)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    spaces.sort();
    let mut arenas = Vec::new();
    for (raw_arena, space) in spaces.into_iter().enumerate() {
        let arena = MemoryArenaId::new(
            u32::try_from(raw_arena)
                .map_err(|_| MemoryPlanError::ArithmeticOverflow { field: "arena id" })?,
        );
        let mut cursor = 0_u64;
        let mut alignment = 1_u32;
        let mut members = Vec::new();
        let mut reused = BTreeMap::<ReuseGroupId, (u64, u64)>::new();
        for allocation in allocations
            .iter_mut()
            .filter(|allocation| allocation.space == space)
        {
            alignment = alignment.max(allocation.alignment);
            let offset = if let Some(group) = allocation.reuse_group {
                if let Some((offset, capacity)) = reused.get_mut(&group) {
                    *capacity = (*capacity).max(allocation.capacity_bytes);
                    *offset
                } else {
                    cursor = align_up(cursor, allocation.alignment)?;
                    let offset = cursor;
                    cursor = cursor.checked_add(allocation.capacity_bytes).ok_or(
                        MemoryPlanError::ArithmeticOverflow {
                            field: "arena capacity",
                        },
                    )?;
                    reused.insert(group, (offset, allocation.capacity_bytes));
                    offset
                }
            } else {
                cursor = align_up(cursor, allocation.alignment)?;
                let offset = cursor;
                cursor = cursor.checked_add(allocation.capacity_bytes).ok_or(
                    MemoryPlanError::ArithmeticOverflow {
                        field: "arena capacity",
                    },
                )?;
                offset
            };
            allocation.placement = ArenaPlacement { arena, offset };
            members.push(allocation.id);
        }
        arenas.push(ArenaPlan {
            id: arena,
            space,
            alignment,
            capacity_bytes: cursor,
            members: members.into_boxed_slice(),
        });
    }
    Ok(arenas.into_boxed_slice())
}

fn program_peak(
    allocations: &[AllocationPlan],
    calls: &[CallMemoryPlan],
    transfers: &[TransferPlan],
) -> Result<ResourceDemand, MemoryPlanError> {
    let mut demand = ResourceDemand::default();
    let mut turn_intervals = Vec::new();
    let mut transaction_intervals = Vec::new();
    for allocation in allocations {
        match allocation.lifetime {
            MemoryLifetime::Program => {
                demand.persistent_bytes = checked_add(
                    demand.persistent_bytes,
                    allocation.capacity_bytes,
                    "program persistent bytes",
                )?;
            }
            MemoryLifetime::Activation => {
                demand.activation_bytes = checked_add(
                    demand.activation_bytes,
                    allocation.capacity_bytes,
                    "program activation bytes",
                )?;
            }
            MemoryLifetime::Turn { first, last } => {
                turn_intervals.push((first, last, allocation.capacity_bytes));
            }
            MemoryLifetime::Transaction { first, last } => {
                transaction_intervals.push((first, last, allocation.capacity_bytes));
            }
            MemoryLifetime::Transfer { .. } => {
                demand.transfer_bytes = checked_add(
                    demand.transfer_bytes,
                    allocation.capacity_bytes,
                    "program transfer bytes",
                )?;
            }
        }
    }
    demand.turn_peak_bytes = closed_interval_peak(&turn_intervals)?;
    demand.transaction_peak_bytes = closed_interval_peak(&transaction_intervals)?;
    for call in calls {
        demand.turn_peak_bytes = demand.turn_peak_bytes.max(call.demand.turn_peak_bytes);
        demand.transaction_peak_bytes = demand
            .transaction_peak_bytes
            .max(call.demand.transaction_peak_bytes);
        demand.cloned_bytes = demand.cloned_bytes.max(call.demand.cloned_bytes);
        demand.retained_nodes = demand.retained_nodes.max(call.demand.retained_nodes);
        demand.output_elements = demand.output_elements.max(call.demand.output_elements);
        demand.storage_bindings = demand.storage_bindings.max(call.demand.storage_bindings);
        demand.work.comparison = checked_add(
            demand.work.comparison,
            call.demand.work.comparison,
            "program comparison work",
        )?;
        demand.work.compute = checked_add(
            demand.work.compute,
            call.demand.work.compute,
            "program compute work",
        )?;
        demand.work.canonicalization = checked_add(
            demand.work.canonicalization,
            call.demand.work.canonicalization,
            "program canonicalization work",
        )?;
        demand.work.scalar_instructions = checked_add(
            demand.work.scalar_instructions,
            call.demand.work.scalar_instructions,
            "program scalar instructions",
        )?;
    }
    for transfer in transfers {
        demand.transfer_bytes = checked_add(
            demand.transfer_bytes,
            transfer.capacity_bytes,
            "program transfer bytes",
        )?;
    }
    Ok(demand)
}

fn closed_interval_peak(
    intervals: &[(MemoryPlanPoint, MemoryPlanPoint, u64)],
) -> Result<u64, MemoryPlanError> {
    let mut starts = BTreeMap::<MemoryPlanPoint, u64>::new();
    let mut ends = BTreeMap::<MemoryPlanPoint, u64>::new();
    let mut points = BTreeSet::new();
    for (first, last, bytes) in intervals {
        if last < first {
            return Err(MemoryPlanError::LifetimeOrderInvalid);
        }
        *starts.entry(*first).or_default() = checked_add(
            starts.get(first).copied().unwrap_or(0),
            *bytes,
            "lifetime start bytes",
        )?;
        *ends.entry(*last).or_default() = checked_add(
            ends.get(last).copied().unwrap_or(0),
            *bytes,
            "lifetime end bytes",
        )?;
        points.insert(*first);
        points.insert(*last);
    }
    let mut live = 0_u64;
    let mut peak = 0_u64;
    for point in points {
        live = checked_add(
            live,
            starts.get(&point).copied().unwrap_or(0),
            "live lifetime bytes",
        )?;
        peak = peak.max(live);
        live = live
            .checked_sub(ends.get(&point).copied().unwrap_or(0))
            .ok_or(MemoryPlanError::LifetimeOrderInvalid)?;
    }
    Ok(peak)
}

pub(crate) fn checked_demand_add(
    left: ResourceDemand,
    right: ResourceDemand,
) -> Result<ResourceDemand, MemoryPlanError> {
    Ok(ResourceDemand {
        persistent_bytes: checked_add(
            left.persistent_bytes,
            right.persistent_bytes,
            "persistent bytes",
        )?,
        activation_bytes: checked_add(
            left.activation_bytes,
            right.activation_bytes,
            "activation bytes",
        )?,
        turn_peak_bytes: checked_add(
            left.turn_peak_bytes,
            right.turn_peak_bytes,
            "turn peak bytes",
        )?,
        transaction_peak_bytes: checked_add(
            left.transaction_peak_bytes,
            right.transaction_peak_bytes,
            "transaction peak bytes",
        )?,
        cloned_bytes: checked_add(left.cloned_bytes, right.cloned_bytes, "cloned bytes")?,
        transfer_bytes: checked_add(left.transfer_bytes, right.transfer_bytes, "transfer bytes")?,
        retained_nodes: checked_add(left.retained_nodes, right.retained_nodes, "retained nodes")?,
        output_elements: checked_add(
            left.output_elements,
            right.output_elements,
            "output elements",
        )?,
        storage_bindings: left
            .storage_bindings
            .checked_add(right.storage_bindings)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "storage bindings",
            })?,
        work: mech_core::WorkDemand {
            comparison: checked_add(
                left.work.comparison,
                right.work.comparison,
                "comparison work",
            )?,
            compute: checked_add(left.work.compute, right.work.compute, "compute work")?,
            canonicalization: checked_add(
                left.work.canonicalization,
                right.work.canonicalization,
                "canonicalization work",
            )?,
            scalar_instructions: checked_add(
                left.work.scalar_instructions,
                right.work.scalar_instructions,
                "scalar instructions",
            )?,
        },
    })
}

fn demand_for_allocation(allocation: &AllocationPlan) -> Result<ResourceDemand, MemoryPlanError> {
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
    demand.storage_bindings = u32::from(allocation.capacity_bytes != 0);
    Ok(demand)
}

fn checked_next_id(id: u32) -> Result<u32, MemoryPlanError> {
    id.checked_add(1)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "program memory-object id",
        })
}

fn checked_add(left: u64, right: u64, field: &'static str) -> Result<u64, MemoryPlanError> {
    left.checked_add(right)
        .ok_or(MemoryPlanError::ArithmeticOverflow { field })
}

fn align_up(value: u64, alignment: u32) -> Result<u64, MemoryPlanError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(MemoryPlanError::InvalidAlignment { alignment });
    }
    let alignment = u64::from(alignment);
    let mask = alignment - 1;
    value
        .checked_add(mask)
        .map(|value| value & !mask)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "arena alignment",
        })
}

pub fn known_fixed_witness(
    layout: &ValueLayoutPlan,
) -> Result<MemoryFootprintWitness, MemoryPlanError> {
    Ok(MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements: layout.current_elements,
        fixed_bytes: layout.current_address_span_bytes,
        payload_bytes: layout.payload.current_bytes,
        encoded_bytes: layout
            .current_address_span_bytes
            .checked_add(layout.payload.current_bytes)
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "known fixed witness bytes",
            })?,
        retained_nodes: layout.payload.current_nodes,
        schema_bytes: 0,
        shape_parameter_count: u64::try_from(layout.axes.len()).map_err(|_| {
            MemoryPlanError::ArithmeticOverflow {
                field: "known fixed witness shape parameters",
            }
        })?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allocation(
        raw: u32,
        bytes: u64,
        lifetime: MemoryLifetime,
        reuse_group: Option<ReuseGroupId>,
    ) -> AllocationPlan {
        AllocationPlan {
            id: MemoryObjectId::new(raw),
            owner: MemoryObjectOwner::NodeScratch {
                node: mech_core::NodeId::new(raw),
                ordinal: 0,
            },
            role: AllocationRole::Scratch,
            space: MemorySpace::ResidentCpu,
            current_bytes: bytes,
            capacity_bytes: bytes,
            alignment: 8,
            lifetime,
            placement: ArenaPlacement {
                arena: MemoryArenaId::new(0),
                offset: 0,
            },
            reuse_group,
        }
    }

    #[test]
    fn closed_intervals_overlap_at_shared_endpoint() {
        let intervals = [
            (MemoryPlanPoint::new(0), MemoryPlanPoint::new(2), 8),
            (MemoryPlanPoint::new(2), MemoryPlanPoint::new(3), 16),
        ];
        assert_eq!(closed_interval_peak(&intervals).unwrap(), 24);
    }

    #[test]
    fn arena_placement_is_stable_and_reuse_members_share_an_offset() {
        let group = ReuseGroupId::new(0);
        let mut first = vec![
            allocation(
                0,
                16,
                MemoryLifetime::Turn {
                    first: MemoryPlanPoint::new(0),
                    last: MemoryPlanPoint::new(1),
                },
                Some(group),
            ),
            allocation(
                1,
                8,
                MemoryLifetime::Turn {
                    first: MemoryPlanPoint::new(2),
                    last: MemoryPlanPoint::new(3),
                },
                Some(group),
            ),
            allocation(2, 4, MemoryLifetime::Activation, None),
        ];
        let mut second = first.clone();
        let first_arenas = place_allocations(&mut first).unwrap();
        let second_arenas = place_allocations(&mut second).unwrap();
        assert_eq!(first, second);
        assert_eq!(first_arenas, second_arenas);
        assert_eq!(first[0].placement.offset, first[1].placement.offset);
        assert_ne!(first[0].placement.offset, first[2].placement.offset);
    }

    #[test]
    fn plan_points_are_checked_and_exact() {
        assert_eq!(
            node_points(mech_core::NodeId::new(7)).unwrap(),
            (MemoryPlanPoint::new(14), MemoryPlanPoint::new(15))
        );
        assert_eq!(
            node_points(mech_core::NodeId::new(u32::MAX)),
            Err(MemoryPlanError::ArithmeticOverflow {
                field: "node memory-plan point",
            })
        );
    }
}
