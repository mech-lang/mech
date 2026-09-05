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

/// Resident value storage owns the stable arena-id range `0..64`. Call-local
/// arenas live above that range so a sparse value plan cannot accidentally
/// assign scratch storage an id reserved for an absent typed lane.
#[cfg(feature = "resident-artifact")]
const RESIDENT_CALL_ARENA_BASE: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlannedValueClass {
    Constant,
    Input,
    State,
    PublishedOutput,
    Scratch,
}

fn program_alias_sources(
    call_nodes: &[mech_core::NodeId],
    call_sites: &[CallSiteMemoryTemplate],
    calls: &mut [CallMemoryPlan],
    values: &[ValueMemoryPlanTemplate],
) -> Result<BTreeMap<CellSlotId, CellSlotId>, MemoryPlanError> {
    let mut aliases = BTreeMap::new();
    for ((&node, site), call) in call_nodes.iter().zip(call_sites).zip(calls) {
        for output_ordinal in 0..call.aliases.len() {
            let decision = call.aliases[output_ordinal];
            let input_ordinal = match decision {
                AliasDecision::BorrowInput { input }
                | AliasDecision::ReuseInput { input }
                | AliasDecision::InPlaceRequired { input } => input,
                AliasDecision::Disjoint | AliasDecision::StageThenPublish { .. } => continue,
            };
            let source = site
                .input_sources
                .get(usize::from(input_ordinal))
                .and_then(|source| match source {
                    ArtifactSource::Slot(slot) => Some(*slot),
                    ArtifactSource::Constant(_) => None,
                });
            let target = site.output_slots.get(output_ordinal).copied();
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };
            let source_value = values
                .iter()
                .find(|value| value.slot == source)
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            let safe_destructive_alias = source == target
                || (source_value.class == PlannedValueClass::Scratch
                    && source_value.last_consumer == Some(node));
            match decision {
                AliasDecision::ReuseInput { .. } if !safe_destructive_alias => {
                    call.aliases[output_ordinal] = AliasDecision::StageThenPublish {
                        input: Some(input_ordinal),
                    };
                    continue;
                }
                AliasDecision::InPlaceRequired { .. } if !safe_destructive_alias => {
                    return Err(MemoryPlanError::IncompatibleAlias {
                        input: input_ordinal,
                        reason: "program lifetime outlives the destructive alias".to_owned(),
                    });
                }
                _ => {}
            }
            if aliases
                .get(&target)
                .is_some_and(|existing| *existing != source)
            {
                return Err(MemoryPlanError::DescriptorMismatch);
            }
            aliases.insert(target, source);
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramMemoryPlanTemplate {
    pub values: Box<[ValueMemoryPlanTemplate]>,
    /// Artifact node identity for each entry in `calls`. This remains
    /// explicit because marker, observation, and external-effect nodes may
    /// not have an executable call plan in every target projection.
    pub call_nodes: Box<[mech_core::NodeId]>,
    pub call_sites: Box<[CallSiteMemoryTemplate]>,
    pub calls: Box<[CallMemoryPlan]>,
    pub allocations: Box<[AllocationPlan]>,
    pub transfers: Box<[TransferPlan]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallSiteMemoryTemplate {
    pub node: mech_core::NodeId,
    pub input_sources: Box<[ArtifactSource]>,
    pub output_slots: Box<[CellSlotId]>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramMemoryPlan {
    pub values: Box<[ValueMemoryPlan]>,
    pub call_nodes: Box<[mech_core::NodeId]>,
    pub calls: Box<[CallMemoryPlan]>,
    pub allocations: Box<[AllocationPlan]>,
    pub arenas: Box<[ArenaPlan]>,
    pub transfers: Box<[TransferPlan]>,
    pub budget_limits: mech_core::MemoryBudgetLimits,
    pub peak: ResourceDemand,
    pub budget_violations: Box<[MemoryBudgetViolation]>,
}

pub fn plan_program_memory_template(
    artifact: &ProgramArtifact,
    instruction_nodes: &[mech_core::NodeId],
    instruction_bindings: &[Option<BoundCall>],
    instruction_memory_plans: &[Option<CallMemoryPlan>],
) -> Result<ProgramMemoryPlanTemplate, MemoryPlanError> {
    if instruction_nodes.len() != instruction_bindings.len()
        || instruction_bindings.len() != instruction_memory_plans.len()
    {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    let mut calls = Vec::new();
    let mut call_nodes = Vec::new();
    let mut call_sites = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    for ((&node_id, binding), plan) in instruction_nodes
        .iter()
        .zip(instruction_bindings)
        .zip(instruction_memory_plans)
    {
        if !seen_nodes.insert(node_id) {
            return Err(MemoryPlanError::DescriptorMismatch);
        }
        let node = artifact
            .nodes()
            .iter()
            .find(|node| node.node == node_id)
            .ok_or(MemoryPlanError::DescriptorMismatch)?;
        match (binding, plan) {
            (Some(binding), Some(plan)) if binding == &plan.bound_call => {
                if plan
                    .bound_call
                    .operation_descriptor()
                    .canonical_name
                    .as_ref()
                    != node.operation.canonical_name()
                {
                    return Err(MemoryPlanError::DescriptorMismatch);
                }
                let inputs = &artifact.bindings()
                    [node.input_bindings.start as usize..node.input_bindings.end as usize];
                let outputs = &artifact.bindings()
                    [node.output_bindings.start as usize..node.output_bindings.end as usize];
                let mut input_sources = vec![None; plan.inputs.len()];
                for declaration in inputs {
                    let BindingDeclaration::Input {
                        port_ordinal,
                        source,
                        ..
                    } = declaration
                    else {
                        return Err(MemoryPlanError::DescriptorMismatch);
                    };
                    let entry = input_sources
                        .get_mut(usize::from(*port_ordinal))
                        .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
                    if entry.replace(*source).is_some() {
                        return Err(MemoryPlanError::DescriptorMismatch);
                    }
                }
                let mut output_slots = vec![None; plan.outputs.len()];
                for declaration in outputs {
                    let BindingDeclaration::Output {
                        port_ordinal,
                        target,
                        ..
                    } = declaration
                    else {
                        return Err(MemoryPlanError::DescriptorMismatch);
                    };
                    let entry = output_slots
                        .get_mut(usize::from(*port_ordinal))
                        .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
                    if entry.replace(*target).is_some() {
                        return Err(MemoryPlanError::DescriptorMismatch);
                    }
                }
                call_nodes.push(node.node);
                call_sites.push(CallSiteMemoryTemplate {
                    node: node.node,
                    input_sources: input_sources
                        .into_iter()
                        .collect::<Option<Vec<_>>>()
                        .ok_or(MemoryPlanError::DescriptorArityMismatch)?
                        .into_boxed_slice(),
                    output_slots: output_slots
                        .into_iter()
                        .collect::<Option<Vec<_>>>()
                        .ok_or(MemoryPlanError::DescriptorArityMismatch)?
                        .into_boxed_slice(),
                });
                calls.push(plan.clone());
            }
            (None, None) => {}
            _ => return Err(MemoryPlanError::DescriptorMismatch),
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
    let mut values = artifact
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
                alias_source: None,
            }
        })
        .collect::<Vec<_>>();
    let alias_sources = program_alias_sources(&call_nodes, &call_sites, &mut calls, &values)?;
    for value in &mut values {
        value.alias_source = alias_sources.get(&value.slot).copied();
    }

    Ok(ProgramMemoryPlanTemplate {
        values: values.into_boxed_slice(),
        call_nodes: call_nodes.into_boxed_slice(),
        call_sites: call_sites.into_boxed_slice(),
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
    instantiate_program_memory_plan_with_target_overrides(template, target, &BTreeMap::new(), facts)
}

/// Instantiates one program with the physical authority for each memory
/// space. Mixed Host/Device artifacts use a host profile for Host values and
/// the adapter-derived profile for Device values instead of projecting every
/// allocation through whichever target happened to be passed by the caller.
pub fn instantiate_program_memory_plan_with_target_overrides(
    template: &ProgramMemoryPlanTemplate,
    default_target: &TargetMemoryProfile,
    target_overrides: &BTreeMap<MemorySpace, TargetMemoryProfile>,
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
        let target = target_overrides
            .get(&fact.storage.space)
            .unwrap_or(default_target);
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
        let transaction = if matches!(
            class,
            PlannedValueClass::State | PlannedValueClass::PublishedOutput
        ) {
            let state_double_buffer = matches!(
                (target.kind, class),
                (
                    mech_core::MemoryTargetKind::ResidentCpu | mech_core::MemoryTargetKind::Gpu,
                    PlannedValueClass::State,
                )
            );
            let stage_lifetime = if state_double_buffer {
                MemoryLifetime::Activation
            } else if let Some(producer) = value.producer {
                let (first, last) = node_points(producer)?;
                MemoryLifetime::Transaction { first, last }
            } else {
                MemoryLifetime::Activation
            };
            let staged_current_bytes = layout
                .current_address_span_bytes
                .checked_add(layout.payload.current_bytes)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "program transaction current bytes",
                })?;
            let staged_capacity_bytes = layout
                .capacity_bytes
                .checked_add(layout.payload.required_bytes)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "program transaction capacity bytes",
                })?;
            let next = MemoryObjectId::new(next_id);
            next_id = checked_next_id(next_id)?;
            allocations.push(AllocationPlan {
                id: next,
                owner: MemoryObjectOwner::Slot(value.slot),
                role: AllocationRole::TransactionStage,
                space: fact.storage.space,
                current_bytes: staged_current_bytes,
                capacity_bytes: staged_capacity_bytes,
                alignment: layout.slot.alignment,
                lifetime: stage_lifetime,
                placement: ArenaPlacement {
                    arena: MemoryArenaId::new(0),
                    offset: 0,
                },
                reuse_group: None,
            });
            if state_double_buffer {
                TransactionRequirement::DoubleBuffer {
                    current: object,
                    next,
                }
            } else {
                TransactionRequirement::StageAndSwap {
                    current: object,
                    staged: next,
                }
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

    if template.call_nodes.len() != template.calls.len()
        || template.call_sites.len() != template.calls.len()
    {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    let object_by_slot = values
        .iter()
        .map(|value| (value.slot, value.object))
        .collect::<BTreeMap<_, _>>();
    let mut constant_objects = BTreeMap::new();
    let mut calls = Vec::with_capacity(template.calls.len());
    for ((&node, site), call) in template
        .call_nodes
        .iter()
        .zip(&template.call_sites)
        .zip(&template.calls)
    {
        if site.node != node
            || site.input_sources.len() != call.inputs.len()
            || site.output_slots.len() != call.outputs.len()
        {
            return Err(MemoryPlanError::DescriptorArityMismatch);
        }
        let mut object_map = BTreeMap::new();
        for (source, port) in site.input_sources.iter().zip(&call.inputs) {
            let object = match *source {
                ArtifactSource::Slot(slot) => object_by_slot
                    .get(&slot)
                    .copied()
                    .ok_or(MemoryPlanError::DescriptorMismatch)?,
                ArtifactSource::Constant(constant) => {
                    let source_allocation = call
                        .allocations
                        .iter()
                        .find(|allocation| allocation.id == port.object)
                        .ok_or(MemoryPlanError::DescriptorMismatch)?;
                    let key = (constant, source_allocation.space);
                    if let Some(object) = constant_objects.get(&key).copied() {
                        let existing = allocations
                            .iter()
                            .find(|allocation| allocation.id == object)
                            .ok_or(MemoryPlanError::DescriptorMismatch)?;
                        if existing.capacity_bytes != port.value.capacity_bytes
                            || existing.alignment != port.value.slot.alignment
                        {
                            return Err(MemoryPlanError::DescriptorMismatch);
                        }
                        object
                    } else {
                        let object = MemoryObjectId::new(next_id);
                        next_id = checked_next_id(next_id)?;
                        allocations.push(AllocationPlan {
                            id: object,
                            owner: MemoryObjectOwner::Constant(constant),
                            role: AllocationRole::FixedStorage,
                            space: source_allocation.space,
                            current_bytes: port.value.current_address_span_bytes,
                            capacity_bytes: port.value.capacity_bytes,
                            alignment: port.value.slot.alignment,
                            lifetime: MemoryLifetime::Program,
                            placement: ArenaPlacement {
                                arena: MemoryArenaId::new(0),
                                offset: 0,
                            },
                            reuse_group: None,
                        });
                        if port.value.payload.required_bytes != 0 {
                            let payload = MemoryObjectId::new(next_id);
                            next_id = checked_next_id(next_id)?;
                            allocations.push(AllocationPlan {
                                id: payload,
                                owner: MemoryObjectOwner::Constant(constant),
                                role: AllocationRole::VariablePayload,
                                space: source_allocation.space,
                                current_bytes: port.value.payload.current_bytes,
                                capacity_bytes: port.value.payload.required_bytes,
                                alignment: 1,
                                lifetime: MemoryLifetime::Program,
                                placement: ArenaPlacement {
                                    arena: MemoryArenaId::new(0),
                                    offset: 0,
                                },
                                reuse_group: None,
                            });
                        }
                        constant_objects.insert(key, object);
                        object
                    }
                }
            };
            insert_object_mapping(&mut object_map, port.object, object)?;
        }
        for (ordinal, (slot, port)) in site.output_slots.iter().zip(&call.outputs).enumerate() {
            let object = object_by_slot
                .get(slot)
                .copied()
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            insert_object_mapping(&mut object_map, port.object, object)?;
            let value = values
                .iter()
                .find(|value| value.slot == *slot)
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            if let (Some(local_stage), Some(global_stage)) = (
                call.transactions
                    .get(ordinal)
                    .and_then(|transaction| transaction_stage_object(*transaction)),
                transaction_stage_object(value.transaction),
            ) {
                insert_object_mapping(&mut object_map, local_stage, global_stage)?;
            }
        }
        let remapped_allocations = remap_call_allocations(node, call, &object_map, &mut next_id)?;
        for (local, allocation) in &remapped_allocations {
            insert_object_mapping(&mut object_map, *local, allocation.id)?;
        }
        let mut remapped = call.clone();
        for port in remapped
            .inputs
            .iter_mut()
            .chain(remapped.outputs.iter_mut())
        {
            port.object = remap_object(&object_map, port.object)?;
        }
        remapped.transactions = remapped
            .transactions
            .iter()
            .copied()
            .map(|transaction| remap_transaction(transaction, &object_map))
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        for (ordinal, slot) in site.output_slots.iter().enumerate() {
            let transaction = remapped
                .transactions
                .get(ordinal)
                .copied()
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            if transaction != TransactionRequirement::None
                && let Some(value) = values.iter_mut().find(|value| value.slot == *slot)
                && matches!(
                    value.class,
                    PlannedValueClass::State | PlannedValueClass::PublishedOutput
                )
            {
                value.transaction = transaction;
            }
        }
        remapped.allocations = remapped_allocations
            .iter()
            .map(|(_, allocation)| allocation.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        allocations.extend(
            remapped_allocations
                .into_iter()
                .map(|(_, allocation)| allocation),
        );
        calls.push(remapped);
    }
    for mut allocation in template.allocations.iter().cloned() {
        allocation.id = MemoryObjectId::new(next_id);
        next_id = checked_next_id(next_id)?;
        allocations.push(allocation);
    }
    validate_global_object_namespace(&allocations, &calls)?;

    assign_alias_groups(&mut values, &template.values)?;
    assign_reuse_groups(&mut allocations, &values)?;
    let arenas = place_allocations(&mut allocations)?;
    for value in &mut values {
        value.reuse_group = allocations
            .iter()
            .find(|allocation| allocation.id == value.object)
            .and_then(|allocation| allocation.reuse_group);
    }
    let peak = program_peak(&allocations, &calls, &template.transfers)?;
    let mut budget_violations = Vec::new();
    for allocation in &allocations {
        let allocation_target = target_overrides
            .get(&allocation.space)
            .unwrap_or(default_target);
        let demand = demand_for_allocation(allocation)?;
        budget_violations.extend(mech_core::evaluate_memory_budget(
            allocation.owner.clone(),
            demand,
            allocation.capacity_bytes,
            matches!(allocation.space, MemorySpace::Device { .. })
                .then_some(allocation.capacity_bytes)
                .unwrap_or(0),
            allocation_target.limits,
        ));
    }
    budget_violations.extend(aggregate_budget_violations(
        &allocations,
        &calls,
        &template.transfers,
        default_target.limits,
        &target_overrides
            .iter()
            .map(|(space, target)| (*space, target.limits))
            .collect(),
    )?);
    budget_violations.sort();
    budget_violations.dedup();
    Ok(ProgramMemoryPlan {
        values: values.into_boxed_slice(),
        call_nodes: template.call_nodes.clone(),
        calls: calls.into_boxed_slice(),
        allocations: allocations.into_boxed_slice(),
        arenas,
        transfers: template.transfers.clone(),
        budget_limits: target_overrides
            .values()
            .fold(default_target.limits, |limits, target| {
                restrictive_budget_limits(limits, target.limits)
            }),
        peak,
        budget_violations: budget_violations.into_boxed_slice(),
    })
}

fn restrictive_budget_limits(
    left: mech_core::MemoryBudgetLimits,
    right: mech_core::MemoryBudgetLimits,
) -> mech_core::MemoryBudgetLimits {
    fn limit(left: Option<u64>, right: Option<u64>) -> Option<u64> {
        match (left, right) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        }
    }
    mech_core::MemoryBudgetLimits {
        max_output_elements: limit(left.max_output_elements, right.max_output_elements),
        max_output_bytes: limit(left.max_output_bytes, right.max_output_bytes),
        max_temporary_bytes: limit(left.max_temporary_bytes, right.max_temporary_bytes),
        max_cloned_bytes: limit(left.max_cloned_bytes, right.max_cloned_bytes),
        max_retained_nodes: limit(left.max_retained_nodes, right.max_retained_nodes),
        max_comparison_work: limit(left.max_comparison_work, right.max_comparison_work),
        max_compute_work: limit(left.max_compute_work, right.max_compute_work),
        max_scalar_instructions: limit(left.max_scalar_instructions, right.max_scalar_instructions),
        max_transfer_bytes: limit(left.max_transfer_bytes, right.max_transfer_bytes),
        max_storage_buffer_bytes: limit(
            left.max_storage_buffer_bytes,
            right.max_storage_buffer_bytes,
        ),
        max_storage_bindings: match (left.max_storage_bindings, right.max_storage_bindings) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        },
    }
}

/// Attaches the authoritative program-level call identity to an existing
/// target-specific value/arena projection. Resident storage has dedicated
/// typed arenas, so it cannot be rebuilt by the generic placement pass, but
/// call-local scratch and transaction objects still use the same global
/// remapping and namespace validation as every other target.
#[cfg(feature = "resident-artifact")]
pub(crate) fn attach_resident_call_memory_template(
    plan: &mut ProgramMemoryPlan,
    template: &ProgramMemoryPlanTemplate,
) -> Result<(), MemoryPlanError> {
    if template.call_nodes.len() != template.calls.len()
        || template.call_sites.len() != template.calls.len()
        || !plan.calls.is_empty()
        || !plan.call_nodes.is_empty()
    {
        return Err(MemoryPlanError::DescriptorArityMismatch);
    }
    let object_by_slot = plan
        .values
        .iter()
        .map(|value| (value.slot, value.object))
        .collect::<BTreeMap<_, _>>();
    let mut next_id = plan
        .allocations
        .iter()
        .map(|allocation| allocation.id.get())
        .max()
        .map_or(Ok(0), checked_next_id)?;
    let mut call_allocations = Vec::new();
    let mut calls = Vec::with_capacity(template.calls.len());

    for ((&node, site), call) in template
        .call_nodes
        .iter()
        .zip(&template.call_sites)
        .zip(&template.calls)
    {
        if site.node != node
            || site.input_sources.len() != call.inputs.len()
            || site.output_slots.len() != call.outputs.len()
        {
            return Err(MemoryPlanError::DescriptorArityMismatch);
        }
        let mut object_map = BTreeMap::new();
        for (source, port) in site.input_sources.iter().zip(&call.inputs) {
            let object = match *source {
                ArtifactSource::Slot(slot) => object_by_slot
                    .get(&slot)
                    .copied()
                    .ok_or(MemoryPlanError::DescriptorMismatch)?,
                ArtifactSource::Constant(constant) => {
                    let source_allocation = call
                        .allocations
                        .iter()
                        .find(|allocation| allocation.id == port.object)
                        .ok_or(MemoryPlanError::DescriptorMismatch)?;
                    let existing = plan
                        .allocations
                        .iter()
                        .find(|allocation| {
                            allocation.owner == MemoryObjectOwner::Constant(constant)
                                && allocation.role == AllocationRole::FixedStorage
                                && allocation.space == source_allocation.space
                        })
                        .ok_or(MemoryPlanError::DescriptorMismatch)?;
                    if existing.capacity_bytes != port.value.capacity_bytes
                        || existing.alignment != port.value.slot.alignment
                    {
                        return Err(MemoryPlanError::DescriptorMismatch);
                    }
                    existing.id
                }
            };
            insert_object_mapping(&mut object_map, port.object, object)?;
        }
        for (ordinal, (slot, port)) in site.output_slots.iter().zip(&call.outputs).enumerate() {
            let object = object_by_slot
                .get(slot)
                .copied()
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            insert_object_mapping(&mut object_map, port.object, object)?;
            let value = plan
                .values
                .iter()
                .find(|value| value.slot == *slot)
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            if let (Some(local_stage), Some(global_stage)) = (
                call.transactions
                    .get(ordinal)
                    .and_then(|transaction| transaction_stage_object(*transaction)),
                transaction_stage_object(value.transaction),
            ) {
                insert_object_mapping(&mut object_map, local_stage, global_stage)?;
            }
        }
        let remapped_allocations = remap_call_allocations(node, call, &object_map, &mut next_id)?;
        for (local, allocation) in &remapped_allocations {
            insert_object_mapping(&mut object_map, *local, allocation.id)?;
        }
        let mut remapped = call.clone();
        for port in remapped
            .inputs
            .iter_mut()
            .chain(remapped.outputs.iter_mut())
        {
            port.object = remap_object(&object_map, port.object)?;
        }
        remapped.transactions = remapped
            .transactions
            .iter()
            .copied()
            .map(|transaction| remap_transaction(transaction, &object_map))
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice();
        for (ordinal, slot) in site.output_slots.iter().enumerate() {
            let transaction = remapped
                .transactions
                .get(ordinal)
                .copied()
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            if transaction != TransactionRequirement::None
                && let Some(value) = plan.values.iter_mut().find(|value| value.slot == *slot)
                && matches!(
                    value.class,
                    PlannedValueClass::State | PlannedValueClass::PublishedOutput
                )
            {
                value.transaction = transaction;
            }
        }
        remapped.allocations = remapped_allocations
            .iter()
            .map(|(_, allocation)| allocation.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        call_allocations.extend(
            remapped_allocations
                .into_iter()
                .map(|(_, allocation)| allocation),
        );
        calls.push(remapped);
    }

    let mut call_arenas = place_allocations(&mut call_allocations)?;
    if plan
        .arenas
        .iter()
        .any(|arena| arena.id.get() >= RESIDENT_CALL_ARENA_BASE)
    {
        return Err(MemoryPlanError::DescriptorMismatch);
    }
    for arena in &mut call_arenas {
        let remapped = arena.id.get().checked_add(RESIDENT_CALL_ARENA_BASE).ok_or(
            MemoryPlanError::ArithmeticOverflow {
                field: "resident call arena id",
            },
        )?;
        let old = arena.id;
        arena.id = MemoryArenaId::new(remapped);
        for allocation in &mut call_allocations {
            if allocation.placement.arena == old {
                allocation.placement.arena = arena.id;
            }
        }
    }
    let mut allocations = plan.allocations.to_vec();
    allocations.extend(call_allocations);
    allocations.sort_by_key(|allocation| allocation.id);
    let mut arenas = plan.arenas.to_vec();
    arenas.extend(call_arenas);
    arenas.sort_by_key(|arena| arena.id);
    validate_global_object_namespace(&allocations, &calls)?;

    plan.call_nodes = template.call_nodes.clone();
    plan.calls = calls.into_boxed_slice();
    plan.allocations = allocations.into_boxed_slice();
    plan.arenas = arenas.into_boxed_slice();
    plan.peak = program_peak(&plan.allocations, &plan.calls, &plan.transfers)?;
    let mut violations = plan.budget_violations.to_vec();
    for allocation in plan.allocations.iter().filter(|allocation| {
        matches!(
            allocation.owner,
            MemoryObjectOwner::NodeInput { .. }
                | MemoryObjectOwner::NodeOutput { .. }
                | MemoryObjectOwner::NodeScratch { .. }
                | MemoryObjectOwner::TransactionStage { .. }
        )
    }) {
        violations.extend(mech_core::evaluate_memory_budget(
            allocation.owner.clone(),
            demand_for_allocation(allocation)?,
            allocation.capacity_bytes,
            matches!(allocation.space, MemorySpace::Device { .. })
                .then_some(allocation.capacity_bytes)
                .unwrap_or(0),
            plan.budget_limits,
        ));
    }
    violations.extend(aggregate_budget_violations(
        &plan.allocations,
        &plan.calls,
        &plan.transfers,
        plan.budget_limits,
        &BTreeMap::new(),
    )?);
    violations.sort();
    violations.dedup();
    plan.budget_violations = violations.into_boxed_slice();
    Ok(())
}

#[cfg(feature = "resident-artifact")]
pub(crate) fn recompute_program_peak(
    plan: &ProgramMemoryPlan,
) -> Result<ResourceDemand, MemoryPlanError> {
    program_peak(&plan.allocations, &plan.calls, &plan.transfers)
}

fn insert_object_mapping(
    objects: &mut BTreeMap<MemoryObjectId, MemoryObjectId>,
    local: MemoryObjectId,
    global: MemoryObjectId,
) -> Result<(), MemoryPlanError> {
    if let Some(previous) = objects.insert(local, global)
        && previous != global
    {
        return Err(MemoryPlanError::DescriptorMismatch);
    }
    Ok(())
}

fn remap_object(
    objects: &BTreeMap<MemoryObjectId, MemoryObjectId>,
    local: MemoryObjectId,
) -> Result<MemoryObjectId, MemoryPlanError> {
    objects
        .get(&local)
        .copied()
        .ok_or(MemoryPlanError::DescriptorMismatch)
}

fn remap_transaction(
    transaction: TransactionRequirement,
    objects: &BTreeMap<MemoryObjectId, MemoryObjectId>,
) -> Result<TransactionRequirement, MemoryPlanError> {
    Ok(match transaction {
        TransactionRequirement::None => TransactionRequirement::None,
        TransactionRequirement::StageAndSwap { current, staged } => {
            TransactionRequirement::StageAndSwap {
                current: remap_object(objects, current)?,
                staged: remap_object(objects, staged)?,
            }
        }
        TransactionRequirement::UndoSnapshot { target, undo } => {
            TransactionRequirement::UndoSnapshot {
                target: remap_object(objects, target)?,
                undo: remap_object(objects, undo)?,
            }
        }
        TransactionRequirement::DoubleBuffer { current, next } => {
            TransactionRequirement::DoubleBuffer {
                current: remap_object(objects, current)?,
                next: remap_object(objects, next)?,
            }
        }
    })
}

fn transaction_stage_object(transaction: TransactionRequirement) -> Option<MemoryObjectId> {
    match transaction {
        TransactionRequirement::StageAndSwap { staged, .. } => Some(staged),
        TransactionRequirement::UndoSnapshot { undo, .. } => Some(undo),
        TransactionRequirement::DoubleBuffer { next, .. } => Some(next),
        TransactionRequirement::None => None,
    }
}

fn validate_global_object_namespace(
    allocations: &[AllocationPlan],
    calls: &[CallMemoryPlan],
) -> Result<(), MemoryPlanError> {
    let mut objects = BTreeSet::new();
    for allocation in allocations {
        if !objects.insert(allocation.id) {
            return Err(MemoryPlanError::DescriptorMismatch);
        }
    }
    for call in calls {
        for port in call.inputs.iter().chain(&call.outputs) {
            if !objects.contains(&port.object) {
                return Err(MemoryPlanError::DescriptorMismatch);
            }
        }
        for transaction in &call.transactions {
            let referenced = match *transaction {
                TransactionRequirement::None => continue,
                TransactionRequirement::StageAndSwap { current, staged } => [current, staged],
                TransactionRequirement::UndoSnapshot { target, undo } => [target, undo],
                TransactionRequirement::DoubleBuffer { current, next } => [current, next],
            };
            if referenced.iter().any(|object| !objects.contains(object)) {
                return Err(MemoryPlanError::DescriptorMismatch);
            }
        }
    }
    Ok(())
}

impl ProgramMemoryPlan {
    /// Stable, pointer-free text used by audits and cross-process determinism
    /// checks. The plan model contains only ordered collections and semantic
    /// identifiers, so its structured debug form is a canonical diagnostic
    /// projection rather than a wire format.
    pub fn diagnostic_text(&self) -> String {
        format!("{self:#?}")
    }

    /// Returns the call plan attached to an artifact node without assuming
    /// that call plans are dense over every artifact node kind.
    pub fn call_for_node(&self, node: mech_core::NodeId) -> Option<&CallMemoryPlan> {
        self.call_nodes
            .binary_search(&node)
            .ok()
            .and_then(|index| self.calls.get(index))
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
        SlotRole::State => PlannedValueClass::State,
        SlotRole::Output => PlannedValueClass::PublishedOutput,
        SlotRole::Derived => match producer {
            ProducerReference::Input(_) => PlannedValueClass::Input,
            ProducerReference::Output { .. } => PlannedValueClass::PublishedOutput,
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
            PlannedValueClass::Input
            | PlannedValueClass::State
            | PlannedValueClass::PublishedOutput => MemoryLifetime::Activation,
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

pub(crate) fn place_allocations(
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
                turn_intervals.push((first, last, allocation.capacity_bytes));
            }
            // Transfer descriptors are the single authority for transfer
            // demand. Their backing allocations are placed but not charged
            // a second time here.
            MemoryLifetime::Transfer { first, last } => {
                turn_intervals.push((first, last, allocation.capacity_bytes));
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

/// Memory caps constrain simultaneous allocations in a memory space, not each
/// allocation independently. Comparison/compute/output limits are per call;
/// retain that existing scope rather than inventing a program execution quota.
fn aggregate_budget_violations(
    allocations: &[AllocationPlan],
    calls: &[CallMemoryPlan],
    transfers: &[TransferPlan],
    default_limits: mech_core::MemoryBudgetLimits,
    overrides: &BTreeMap<MemorySpace, mech_core::MemoryBudgetLimits>,
) -> Result<Vec<mech_core::MemoryBudgetViolation>, MemoryPlanError> {
    let spaces = allocations
        .iter()
        .map(|a| a.space)
        .chain(transfers.iter().flat_map(|t| [t.source, t.destination]))
        .collect::<BTreeSet<_>>();
    let mut violations = Vec::new();
    for space in spaces {
        let objects = allocations
            .iter()
            .filter(|a| a.space == space)
            .cloned()
            .collect::<Vec<_>>();
        let scoped_transfers = transfers
            .iter()
            .filter(|t| t.source == space || t.destination == space)
            .cloned()
            .collect::<Vec<_>>();
        let mut demand = program_peak(&objects, &[], &scoped_transfers)?;
        // Per-call peaks may include a conservative pre-materialization bound.
        // Such a bound is useful only for calls entirely within this space.
        for call in calls.iter().filter(|call| {
            call.input_storage
                .iter()
                .chain(call.output_storage.iter())
                .all(|storage| storage.space == space)
        }) {
            demand.turn_peak_bytes = demand.turn_peak_bytes.max(call.demand.turn_peak_bytes);
        }
        if matches!(space, MemorySpace::Device { .. }) {
            demand.storage_bindings = u32::try_from(
                objects
                    .iter()
                    .filter(|a| a.role == AllocationRole::FixedStorage)
                    .count(),
            )
            .map_err(|_| MemoryPlanError::ArithmeticOverflow {
                field: "program binding count",
            })?;
        }
        let limits = overrides.get(&space).copied().unwrap_or(default_limits);
        let owner = objects
            .first()
            .map(|a| a.owner.clone())
            .unwrap_or(MemoryObjectOwner::Transfer { ordinal: 0 });
        violations.extend(mech_core::evaluate_memory_budget(
            owner, demand, 0, 0, limits,
        ));
    }
    Ok(violations)
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

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn every_transaction_object_is_remapped_into_the_program_namespace() {
        let objects = BTreeMap::from([
            (MemoryObjectId::new(0), MemoryObjectId::new(40)),
            (MemoryObjectId::new(1), MemoryObjectId::new(41)),
        ]);
        let cases = [
            TransactionRequirement::StageAndSwap {
                current: MemoryObjectId::new(0),
                staged: MemoryObjectId::new(1),
            },
            TransactionRequirement::UndoSnapshot {
                target: MemoryObjectId::new(0),
                undo: MemoryObjectId::new(1),
            },
            TransactionRequirement::DoubleBuffer {
                current: MemoryObjectId::new(0),
                next: MemoryObjectId::new(1),
            },
        ];
        for transaction in cases {
            let remapped = remap_transaction(transaction, &objects).unwrap();
            assert_eq!(transaction_key(remapped).1, 40);
            assert_eq!(transaction_key(remapped).2, 41);
        }
        assert_eq!(
            remap_transaction(
                TransactionRequirement::StageAndSwap {
                    current: MemoryObjectId::new(0),
                    staged: MemoryObjectId::new(2),
                },
                &objects,
            ),
            Err(MemoryPlanError::DescriptorMismatch)
        );
    }

    fn transaction_key(transaction: TransactionRequirement) -> (u8, u32, u32) {
        match transaction {
            TransactionRequirement::StageAndSwap { current, staged } => {
                (1, current.get(), staged.get())
            }
            TransactionRequirement::UndoSnapshot { target, undo } => (2, target.get(), undo.get()),
            TransactionRequirement::DoubleBuffer { current, next } => {
                (3, current.get(), next.get())
            }
            TransactionRequirement::None => (0, 0, 0),
        }
    }
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

    #[test]
    fn published_outputs_are_not_reclassified_as_state() {
        assert_eq!(
            default_value_class(
                SlotRole::Output,
                ProducerReference::NodeOutput {
                    node: mech_core::NodeId::new(0),
                    output_ordinal: 0,
                },
            ),
            PlannedValueClass::PublishedOutput,
        );
        assert_eq!(
            default_value_class(
                SlotRole::State,
                ProducerReference::NodeOutput {
                    node: mech_core::NodeId::new(0),
                    output_ordinal: 0,
                },
            ),
            PlannedValueClass::State,
        );
    }
}
