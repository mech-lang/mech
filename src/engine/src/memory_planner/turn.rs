use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    AllocationPlan, ArenaPlan, CurrentMemoryFootprint, MemoryBudgetLimits, MemoryBudgetViolation,
    MemoryObjectOwner, MemoryPlanError, NodeId, PortDirection, RegionAccessPlan, ResourceDemand,
    TransactionRequirement,
};

use super::{ProgramMemoryPlan, checked_demand_add, place_allocations};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TurnMemoryFacts {
    pub resolved_regions: BTreeMap<(NodeId, u16), RegionAccessPlan>,
    pub resolved_footprints: BTreeMap<(NodeId, PortDirection, u16), CurrentMemoryFootprint>,
    /// The value retained until publication, distinct from the candidate.
    pub published_footprints: BTreeMap<(NodeId, u16), CurrentMemoryFootprint>,
    pub additional_demand: ResourceDemand,
    /// A complete concrete executor estimate for this turn. Each component
    /// replaces a weaker declaration by maximum rather than being added to
    /// the same operation twice.
    pub observed_demand: Option<ResourceDemand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnMemoryPlan {
    pub node: NodeId,
    /// Concrete facts are retained so admission can refine the candidate
    /// without falling back to activation-time input footprints.
    pub facts: TurnMemoryFacts,
    pub call: Option<mech_core::CallMemoryPlan>,
    pub allocations: Box<[AllocationPlan]>,
    pub arenas: Box<[ArenaPlan]>,
    pub transactions: Box<[TransactionRequirement]>,
    pub demand: ResourceDemand,
    pub output_bytes: u64,
    pub storage_buffer_bytes: u64,
    pub budget_limits: MemoryBudgetLimits,
    pub budget_violations: Box<[MemoryBudgetViolation]>,
}

pub fn plan_turn_memory(
    plan: &ProgramMemoryPlan,
    node: NodeId,
    facts: &TurnMemoryFacts,
) -> Result<TurnMemoryPlan, MemoryPlanError> {
    let value_transactions = plan
        .values
        .iter()
        .filter(|value| {
            value.transaction != TransactionRequirement::None
                && matches!(
                    value.class,
                    super::PlannedValueClass::State | super::PlannedValueClass::PublishedOutput
                )
                && value.producer == Some(node)
        })
        .map(|value| value.transaction)
        .collect::<Vec<_>>();
    let call_transactions = plan
        .call_for_node(node)
        .map(|call| call.transactions.to_vec())
        .unwrap_or_default();
    let transaction_objects = value_transactions
        .iter()
        .chain(&call_transactions)
        .filter_map(|transaction| transaction_stage(*transaction))
        .collect::<BTreeSet<_>>();
    let transaction_owners = plan
        .allocations
        .iter()
        .filter(|allocation| transaction_objects.contains(&allocation.id))
        .map(|allocation| allocation.owner.clone())
        .collect::<BTreeSet<_>>();
    let port_objects = plan
        .call_for_node(node)
        .into_iter()
        .flat_map(|call| call.inputs.iter().chain(call.outputs.iter()))
        .map(|port| port.object)
        .collect::<BTreeSet<_>>();
    let port_owners = plan
        .allocations
        .iter()
        .filter(|a| port_objects.contains(&a.id))
        .map(|a| a.owner.clone())
        .collect::<BTreeSet<_>>();
    let mut allocations = plan
        .allocations
        .iter()
        .filter(|allocation| {
            owner_node(&allocation.owner) == Some(node)
                || port_objects.contains(&allocation.id)
                || (allocation.role == mech_core::AllocationRole::VariablePayload
                    && port_owners.contains(&allocation.owner))
                || transaction_objects.contains(&allocation.id)
                || (allocation.role == mech_core::AllocationRole::TransactionStage
                    && transaction_owners.contains(&allocation.owner))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut transactions = Vec::new();
    let mut demand = facts.additional_demand;
    let mut output_bytes = 0_u64;
    let mut turn_call = None;
    if let Some(call) = plan.call_for_node(node) {
        let resolved = facts
            .resolved_footprints
            .iter()
            .filter_map(|(&(fact_node, direction, port), &footprint)| {
                (fact_node == node).then_some(((direction, port), footprint))
            })
            .collect::<BTreeMap<_, _>>();
        let mut template = call.clone();
        for (ordinal, region) in template.output_regions.iter_mut().enumerate() {
            if let Some(actual) = facts.resolved_regions.get(&(node, ordinal as u16)) {
                *region = actual.clone();
            }
        }
        let mut resolved_call = derive_turn_call(&template, &resolved)?;
        adjust_publication_comparison(&mut resolved_call, node, facts)?;
        refresh_turn_allocations(&mut allocations, call, &resolved_call, node, facts)?;
        demand = checked_demand_add(demand, resolved_call.demand)?;
        transactions.extend(call_transactions.iter().copied());
        for deferred in &call.deferred_witnesses {
            if deferred.stage != mech_core::MemoryWitnessStage::Turn {
                continue;
            }
            let Some(footprint) =
                facts
                    .resolved_footprints
                    .get(&(node, deferred.direction, deferred.port))
            else {
                return Err(MemoryPlanError::MissingFootprintWitness {
                    stage: mech_core::MemoryWitnessStage::Turn,
                });
            };
            if deferred.direction == PortDirection::Input {
                let input = call
                    .inputs
                    .get(usize::from(deferred.port))
                    .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
                if let Some(maximum) = input.value.capacity_elements.maximum
                    && footprint.logical_elements > maximum
                {
                    return Err(MemoryPlanError::DynamicCardinalityExceedsBound {
                        current: footprint.logical_elements,
                        maximum,
                    });
                }
            }
        }
        for (ordinal, output) in call.outputs.iter().enumerate() {
            let ordinal_index = ordinal;
            let ordinal =
                u16::try_from(ordinal).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                    field: "turn output ordinal",
                })?;
            if matches!(output.region, RegionAccessPlan::Deferred(_))
                && !facts.resolved_regions.contains_key(&(node, ordinal))
            {
                return Err(MemoryPlanError::MissingFootprintWitness {
                    stage: mech_core::MemoryWitnessStage::Turn,
                });
            }
            let footprint = facts
                .resolved_footprints
                .get(&(node, PortDirection::Output, ordinal));
            if let Some(footprint) = footprint {
                if output
                    .value
                    .capacity_elements
                    .maximum
                    .is_some_and(|maximum| footprint.logical_elements > maximum)
                {
                    return Err(MemoryPlanError::DynamicCardinalityExceedsBound {
                        current: footprint.logical_elements,
                        maximum: output.value.capacity_elements.maximum.unwrap(),
                    });
                }
                if let Some(stage) = call
                    .transactions
                    .get(usize::from(ordinal))
                    .and_then(|transaction| transaction_stage(*transaction))
                {
                    let resolved_stage = resolved_call
                        .transactions
                        .get(usize::from(ordinal))
                        .and_then(|transaction| transaction_stage(*transaction))
                        .and_then(|stage| {
                            resolved_call
                                .allocations
                                .iter()
                                .find(|allocation| allocation.id == stage)
                        })
                        .ok_or(MemoryPlanError::DescriptorMismatch)?;
                    grow_transaction_family(
                        &mut allocations,
                        stage,
                        resolved_stage.current_bytes,
                        resolved_stage.capacity_bytes,
                    )?;
                }
            }
            let resolved_output = resolved_call
                .outputs
                .get(ordinal_index)
                .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            output_bytes = output_bytes
                .checked_add(resolved_output.value.capacity_bytes)
                .and_then(|value| value.checked_add(resolved_output.value.payload.required_bytes))
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "complete turn output bytes",
                })?;
        }
        turn_call = Some(scope_turn_call(resolved_call, call, &allocations));
    }
    transactions.extend(value_transactions);
    transactions.sort_by_key(transaction_key);
    transactions.dedup();
    if let Some(observed) = facts.observed_demand {
        demand = demand_max(demand, observed);
        output_bytes = output_bytes.max(observed.persistent_bytes);
        grow_transaction_stage_total(
            &mut allocations,
            observed.persistent_bytes,
            observed.persistent_bytes,
        )?;
    }
    let arenas = place_allocations(&mut allocations)?;
    demand.turn_peak_bytes = demand.turn_peak_bytes.max(turn_storage_peak(&allocations)?);
    let mut budget_violations = plan
        .budget_violations
        .iter()
        .filter(|violation| owner_node(&violation.owner) == Some(node))
        .cloned()
        .collect::<Vec<_>>();
    let storage_buffer_bytes = allocations
        .iter()
        .filter(|allocation| matches!(allocation.space, mech_core::MemorySpace::Device { .. }))
        .map(|allocation| allocation.capacity_bytes)
        .max()
        .unwrap_or(0);
    budget_violations.extend(mech_core::evaluate_memory_budget(
        MemoryObjectOwner::NodeOutput { node, port: 0 },
        demand,
        output_bytes,
        storage_buffer_bytes,
        plan.budget_limits,
    ));
    budget_violations.sort();
    budget_violations.dedup();
    Ok(TurnMemoryPlan {
        node,
        facts: facts.clone(),
        call: turn_call,
        allocations: allocations.into_boxed_slice(),
        arenas,
        transactions: transactions.into_boxed_slice(),
        demand,
        output_bytes,
        storage_buffer_bytes,
        budget_limits: plan.budget_limits,
        budget_violations: budget_violations.into_boxed_slice(),
    })
}

/// Reconciles an executor's complete concrete estimate with the declaration-
/// and footprint-derived turn plan. This preserves real node identity,
/// allocations, transactions, and placement while replacing weaker demand
/// components and growing/re-placing publication stages before admission.
#[cfg(feature = "resident-artifact")]
pub(crate) fn apply_observed_turn_demand(
    mut plan: TurnMemoryPlan,
    observed: ResourceDemand,
    final_output: Option<CurrentMemoryFootprint>,
) -> Result<TurnMemoryPlan, MemoryPlanError> {
    if let Some(call) = plan.call.clone() {
        if call.outputs.len() == 1
            && (observed.persistent_bytes != 0
                || observed.output_elements != 0
                || final_output.is_some())
        {
            let old = plan
                .facts
                .resolved_footprints
                .get(&(plan.node, PortDirection::Output, 0))
                .copied()
                .unwrap_or_default();
            let fixed = call.outputs[0].value.current_address_span_bytes;
            let retained = final_output
                .map(|f| {
                    f.fixed_bytes
                        .checked_add(f.payload_bytes)
                        .ok_or(MemoryPlanError::TargetAddressOverflow)
                })
                .transpose()?
                .unwrap_or(observed.persistent_bytes);
            let candidate = CurrentMemoryFootprint {
                logical_elements: final_output
                    .map_or(observed.output_elements, |f| f.logical_elements),
                fixed_bytes: fixed,
                payload_bytes: retained.saturating_sub(fixed),
                // Resident preflight reports complete retained bytes. This is
                // a conservative encoded-size bound until a finalized value
                // exists; unlike the published side, it is not a live value.
                encoded_bytes: retained,
                retained_nodes: final_output.map_or(observed.retained_nodes, |f| f.retained_nodes),
                schema_bytes: old.schema_bytes,
                shape_parameter_count: old.shape_parameter_count,
            };
            plan.facts
                .resolved_footprints
                .insert((plan.node, PortDirection::Output, 0), candidate);
            let resolved = plan
                .facts
                .resolved_footprints
                .iter()
                .filter_map(|(&(node, direction, port), &f)| {
                    (node == plan.node).then_some(((direction, port), f))
                })
                .collect();
            let mut complete = derive_turn_call(&call, &resolved)?;
            adjust_publication_comparison(&mut complete, plan.node, &plan.facts)?;
            refresh_turn_allocations(
                &mut plan.allocations,
                &call,
                &complete,
                plan.node,
                &plan.facts,
            )?;
            plan.demand = checked_demand_add(complete.demand, plan.facts.additional_demand)?;
            plan.call = Some(scope_turn_call(complete, &call, &plan.allocations));
        }
    }
    let observed = checked_demand_add(observed, plan.facts.additional_demand)?;
    plan.demand = demand_max(plan.demand, observed);
    plan.output_bytes = plan.output_bytes.max(observed.persistent_bytes);
    grow_transaction_stage_total(
        &mut plan.allocations,
        observed.persistent_bytes,
        observed.persistent_bytes,
    )?;
    plan.arenas = place_allocations(&mut plan.allocations)?;
    plan.demand.turn_peak_bytes = plan
        .demand
        .turn_peak_bytes
        .max(turn_storage_peak(&plan.allocations)?);
    plan.storage_buffer_bytes = plan
        .allocations
        .iter()
        .filter(|allocation| matches!(allocation.space, mech_core::MemorySpace::Device { .. }))
        .map(|allocation| allocation.capacity_bytes)
        .max()
        .unwrap_or(0);
    let mut violations = mech_core::evaluate_memory_budget(
        MemoryObjectOwner::NodeOutput {
            node: plan.node,
            port: 0,
        },
        plan.demand,
        plan.output_bytes,
        plan.storage_buffer_bytes,
        plan.budget_limits,
    )
    .into_vec();
    // The scoped plan can still contain a provisional candidate. Admission
    // evaluates the complete replacement, not obsolete candidate violations.
    violations.sort();
    violations.dedup();
    plan.budget_violations = violations.into_boxed_slice();
    Ok(plan)
}

/// The runtime must supply live borrowed facts. No footprint can be recovered
/// from an activation allocation plan after a value has changed.
#[cfg(feature = "resident-artifact")]
pub(crate) fn plan_current_resident_turn(
    plan: &ProgramMemoryPlan,
    node: NodeId,
    facts: &TurnMemoryFacts,
) -> Result<TurnMemoryPlan, MemoryPlanError> {
    plan_turn_memory(plan, node, facts)
}

/// Call derivation uses a same-candidate publication estimate. A turn must
/// first replace that estimate with the distinct old/candidate evidence before
/// evaluating policy budgets. Semantic bounds, addressability, and checked
/// arithmetic remain enforced by the core planner throughout.
fn derive_turn_call(
    template: &mech_core::CallMemoryPlan,
    resolved: &BTreeMap<(PortDirection, u16), CurrentMemoryFootprint>,
) -> Result<mech_core::CallMemoryPlan, MemoryPlanError> {
    let mut derivation = template.clone();
    derivation.target.limits = MemoryBudgetLimits::default();
    let mut call = mech_core::resolve_deferred_call_memory(&derivation, resolved)?;
    call.target = template.target.clone();
    Ok(call)
}

fn scope_turn_call(
    mut resolved: mech_core::CallMemoryPlan,
    original: &mech_core::CallMemoryPlan,
    allocations: &[AllocationPlan],
) -> mech_core::CallMemoryPlan {
    for (port, global) in resolved
        .inputs
        .iter_mut()
        .zip(&original.inputs)
        .chain(resolved.outputs.iter_mut().zip(&original.outputs))
    {
        port.object = global.object;
    }
    resolved.transactions = original.transactions.clone();
    resolved.allocations = allocations.into();
    resolved
}

fn adjust_publication_comparison(
    call: &mut mech_core::CallMemoryPlan,
    node: NodeId,
    facts: &TurnMemoryFacts,
) -> Result<(), MemoryPlanError> {
    let requirements = call
        .bound_call
        .operation_descriptor()
        .contract
        .memory_requirements(call.inputs.len())
        .map_err(|_| MemoryPlanError::DescriptorMismatch)?;
    for (ordinal, output) in requirements.outputs.iter().enumerate() {
        if output.change_detection != Some(mech_core::ChangeDetectionPolicy::SemanticHash) {
            continue;
        }
        let Some(current) = facts.published_footprints.get(&(node, ordinal as u16)) else {
            continue;
        };
        let mech_core::MemoryFootprintWitness::Known(candidate) = call.output_witnesses[ordinal]
        else {
            return Err(MemoryPlanError::MissingFootprintWitness {
                stage: mech_core::MemoryWitnessStage::Turn,
            });
        };
        let previous = mech_core::publication_comparison_work(candidate, candidate)?;
        let replacement = mech_core::publication_comparison_work(*current, candidate)?;
        call.demand.work.comparison = call
            .demand
            .work
            .comparison
            .checked_sub(previous)
            .and_then(|work| work.checked_add(replacement))
            .ok_or(MemoryPlanError::ArithmeticOverflow {
                field: "live publication comparison",
            })?;
    }
    Ok(())
}

/// Refresh stable global objects from a freshly derived call-local plan.
/// Scratch IDs are matched by their declared ordinal, never by allocation order.
fn refresh_turn_allocations(
    allocations: &mut [AllocationPlan],
    original: &mech_core::CallMemoryPlan,
    resolved: &mech_core::CallMemoryPlan,
    node: NodeId,
    facts: &TurnMemoryFacts,
) -> Result<(), MemoryPlanError> {
    for (direction, old_ports, new_ports) in [
        (PortDirection::Input, &original.inputs, &resolved.inputs),
        (PortDirection::Output, &original.outputs, &resolved.outputs),
    ] {
        for (ordinal, (old, new)) in old_ports.iter().zip(new_ports.iter()).enumerate() {
            let current = if direction == PortDirection::Output {
                facts.published_footprints.get(&(node, ordinal as u16))
            } else {
                facts
                    .resolved_footprints
                    .get(&(node, direction, ordinal as u16))
            };
            let Some(current) = current else {
                continue;
            };
            let fixed = allocations
                .iter_mut()
                .find(|a| a.id == old.object)
                .ok_or(MemoryPlanError::DescriptorMismatch)?;
            let owner = fixed.owner.clone();
            fixed.current_bytes = new.value.current_address_span_bytes;
            fixed.capacity_bytes = fixed.capacity_bytes.max(fixed.current_bytes);
            if let Some(payload) = allocations
                .iter_mut()
                .find(|a| a.owner == owner && a.role == mech_core::AllocationRole::VariablePayload)
            {
                payload.current_bytes = current.payload_bytes;
                payload.capacity_bytes = payload.capacity_bytes.max(current.payload_bytes);
            }
        }
    }
    for local in resolved
        .allocations
        .iter()
        .filter(|a| matches!(a.owner, MemoryObjectOwner::NodeScratch { .. }))
    {
        let MemoryObjectOwner::NodeScratch { ordinal, .. } = local.owner else {
            unreachable!()
        };
        let global = allocations
            .iter_mut()
            .find(|a| a.owner == MemoryObjectOwner::NodeScratch { node, ordinal })
            .ok_or(MemoryPlanError::DescriptorMismatch)?;
        global.current_bytes = local.current_bytes;
        global.capacity_bytes = local.capacity_bytes;
        global.alignment = local.alignment;
    }
    Ok(())
}

fn turn_storage_peak(allocations: &[AllocationPlan]) -> Result<u64, MemoryPlanError> {
    let mut events = BTreeMap::<mech_core::MemoryPlanPoint, (u64, u64)>::new();
    for allocation in allocations {
        let (first, last) = match allocation.lifetime {
            mech_core::MemoryLifetime::Turn { first, last }
            | mech_core::MemoryLifetime::Transaction { first, last }
            | mech_core::MemoryLifetime::Transfer { first, last } => (first, last),
            _ => continue,
        };
        let start = events.entry(first).or_default();
        start.0 = start
            .0
            .checked_add(allocation.capacity_bytes)
            .ok_or(MemoryPlanError::TargetAddressOverflow)?;
        let end = events.entry(last).or_default();
        end.1 = end
            .1
            .checked_add(allocation.capacity_bytes)
            .ok_or(MemoryPlanError::TargetAddressOverflow)?;
    }
    let (mut live, mut peak) = (0_u64, 0_u64);
    for (starts, ends) in events.into_values() {
        live = live
            .checked_add(starts)
            .ok_or(MemoryPlanError::TargetAddressOverflow)?;
        peak = peak.max(live);
        live = live
            .checked_sub(ends)
            .ok_or(MemoryPlanError::LifetimeOrderInvalid)?;
    }
    Ok(peak)
}

fn demand_max(left: ResourceDemand, right: ResourceDemand) -> ResourceDemand {
    ResourceDemand {
        persistent_bytes: left.persistent_bytes.max(right.persistent_bytes),
        activation_bytes: left.activation_bytes.max(right.activation_bytes),
        turn_peak_bytes: left.turn_peak_bytes.max(right.turn_peak_bytes),
        transaction_peak_bytes: left
            .transaction_peak_bytes
            .max(right.transaction_peak_bytes),
        cloned_bytes: left.cloned_bytes.max(right.cloned_bytes),
        transfer_bytes: left.transfer_bytes.max(right.transfer_bytes),
        retained_nodes: left.retained_nodes.max(right.retained_nodes),
        output_elements: left.output_elements.max(right.output_elements),
        storage_bindings: left.storage_bindings.max(right.storage_bindings),
        work: mech_core::WorkDemand {
            comparison: left.work.comparison.max(right.work.comparison),
            compute: left.work.compute.max(right.work.compute),
            canonicalization: left.work.canonicalization.max(right.work.canonicalization),
            scalar_instructions: left
                .work
                .scalar_instructions
                .max(right.work.scalar_instructions),
        },
    }
}

fn grow_transaction_family(
    allocations: &mut [AllocationPlan],
    primary: mech_core::MemoryObjectId,
    current_bytes: u64,
    capacity_bytes: u64,
) -> Result<(), MemoryPlanError> {
    let owner = allocations
        .iter()
        .find(|allocation| allocation.id == primary)
        .map(|allocation| allocation.owner.clone())
        .ok_or(MemoryPlanError::DescriptorMismatch)?;
    let indices = allocations
        .iter()
        .enumerate()
        .filter(|(_, allocation)| {
            allocation.role == mech_core::AllocationRole::TransactionStage
                && allocation.owner == owner
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    grow_transaction_stage_indices(allocations, &indices, current_bytes, capacity_bytes)
}

fn grow_transaction_stage_total(
    allocations: &mut [AllocationPlan],
    current_bytes: u64,
    capacity_bytes: u64,
) -> Result<(), MemoryPlanError> {
    let indices = allocations
        .iter()
        .enumerate()
        .filter(|(_, allocation)| allocation.role == mech_core::AllocationRole::TransactionStage)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if indices.is_empty() && (current_bytes != 0 || capacity_bytes != 0) {
        return Err(MemoryPlanError::DescriptorMismatch);
    }
    grow_transaction_stage_indices(allocations, &indices, current_bytes, capacity_bytes)
}

fn grow_transaction_stage_indices(
    allocations: &mut [AllocationPlan],
    indices: &[usize],
    current_bytes: u64,
    capacity_bytes: u64,
) -> Result<(), MemoryPlanError> {
    if indices.is_empty() {
        return Ok(());
    }
    let current_total = indices.iter().try_fold(0_u64, |total, &index| {
        total.checked_add(allocations[index].current_bytes).ok_or(
            MemoryPlanError::ArithmeticOverflow {
                field: "transaction family current bytes",
            },
        )
    })?;
    let capacity_total = indices.iter().try_fold(0_u64, |total, &index| {
        total.checked_add(allocations[index].capacity_bytes).ok_or(
            MemoryPlanError::ArithmeticOverflow {
                field: "transaction family capacity bytes",
            },
        )
    })?;
    let target = indices
        .iter()
        .copied()
        .find(|&index| allocations[index].alignment == 1)
        .unwrap_or(indices[0]);
    let capacity_delta = capacity_bytes.saturating_sub(capacity_total);
    allocations[target].capacity_bytes = allocations[target]
        .capacity_bytes
        .checked_add(capacity_delta)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "grown transaction family capacity",
        })?;
    let current_delta = current_bytes.saturating_sub(current_total);
    allocations[target].current_bytes = allocations[target]
        .current_bytes
        .checked_add(current_delta)
        .ok_or(MemoryPlanError::ArithmeticOverflow {
            field: "grown transaction family current bytes",
        })?;
    if allocations[target].current_bytes > allocations[target].capacity_bytes {
        return Err(MemoryPlanError::CapacityBelowCurrent {
            current: allocations[target].current_bytes,
            maximum: allocations[target].capacity_bytes,
        });
    }
    Ok(())
}

fn transaction_stage(transaction: TransactionRequirement) -> Option<mech_core::MemoryObjectId> {
    match transaction {
        TransactionRequirement::StageAndSwap { staged, .. } => Some(staged),
        TransactionRequirement::DoubleBuffer { next, .. } => Some(next),
        TransactionRequirement::UndoSnapshot { undo, .. } => Some(undo),
        _ => None,
    }
}

fn owner_node(owner: &MemoryObjectOwner) -> Option<NodeId> {
    match *owner {
        MemoryObjectOwner::NodeInput { node, .. }
        | MemoryObjectOwner::NodeOutput { node, .. }
        | MemoryObjectOwner::NodeScratch { node, .. }
        | MemoryObjectOwner::TransactionStage { node, .. } => Some(node),
        _ => None,
    }
}

fn transaction_key(transaction: &TransactionRequirement) -> (u8, u32, u32) {
    match *transaction {
        TransactionRequirement::None => (0, 0, 0),
        TransactionRequirement::StageAndSwap { current, staged } => {
            (1, current.get(), staged.get())
        }
        TransactionRequirement::UndoSnapshot { target, undo } => (2, target.get(), undo.get()),
        TransactionRequirement::DoubleBuffer { current, next } => (3, current.get(), next.get()),
    }
}
