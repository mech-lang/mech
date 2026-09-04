use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "resident-artifact")]
use mech_core::PortMemoryPlan;
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
    pub additional_demand: ResourceDemand,
    /// A complete concrete executor estimate for this turn. Each component
    /// replaces a weaker declaration by maximum rather than being added to
    /// the same operation twice.
    pub observed_demand: Option<ResourceDemand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnMemoryPlan {
    pub node: NodeId,
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
    let mut allocations = plan
        .allocations
        .iter()
        .filter(|allocation| {
            owner_node(&allocation.owner) == Some(node)
                || transaction_objects.contains(&allocation.id)
                || (allocation.role == mech_core::AllocationRole::TransactionStage
                    && transaction_owners.contains(&allocation.owner))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut transactions = Vec::new();
    let mut demand = facts.additional_demand;
    let mut output_bytes = 0_u64;
    if let Some(call) = plan.call_for_node(node) {
        let resolved = facts
            .resolved_footprints
            .iter()
            .filter_map(|(&(fact_node, direction, port), &footprint)| {
                (fact_node == node).then_some(((direction, port), footprint))
            })
            .collect::<BTreeMap<_, _>>();
        let resolved_call = mech_core::resolve_deferred_call_memory(call, &resolved)?;
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
) -> Result<TurnMemoryPlan, MemoryPlanError> {
    plan.demand = demand_max(plan.demand, observed);
    plan.output_bytes = plan.output_bytes.max(observed.persistent_bytes);
    grow_transaction_stage_total(
        &mut plan.allocations,
        observed.persistent_bytes,
        observed.persistent_bytes,
    )?;
    plan.arenas = place_allocations(&mut plan.allocations)?;
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
    violations.extend(plan.budget_violations);
    violations.sort();
    violations.dedup();
    plan.budget_violations = violations.into_boxed_slice();
    Ok(plan)
}

/// Produces the real node-scoped plan used by Resident execution. Current
/// program allocations provide already-admitted retained footprints; the
/// operation's concrete candidate estimate is reconciled immediately before
/// its first materialization.
#[cfg(feature = "resident-artifact")]
pub(crate) fn plan_current_resident_turn(
    plan: &ProgramMemoryPlan,
    node: NodeId,
) -> Result<TurnMemoryPlan, MemoryPlanError> {
    let mut facts = TurnMemoryFacts::default();
    if let Some(call) = plan.call_for_node(node) {
        for deferred in &call.deferred_witnesses {
            let port = match deferred.direction {
                PortDirection::Input => call.inputs.get(usize::from(deferred.port)),
                PortDirection::Output => call.outputs.get(usize::from(deferred.port)),
            }
            .ok_or(MemoryPlanError::DescriptorArityMismatch)?;
            facts.resolved_footprints.insert(
                (node, deferred.direction, deferred.port),
                current_port_footprint(plan, port)?,
            );
        }
        for (ordinal, output) in call.outputs.iter().enumerate() {
            if matches!(output.region, RegionAccessPlan::Deferred(_)) {
                facts.resolved_regions.insert(
                    (
                        node,
                        u16::try_from(ordinal).map_err(|_| {
                            MemoryPlanError::ArithmeticOverflow {
                                field: "resident turn output ordinal",
                            }
                        })?,
                    ),
                    RegionAccessPlan::WholeValue,
                );
            }
        }
    }
    plan_turn_memory(plan, node, &facts)
}

#[cfg(feature = "resident-artifact")]
fn current_port_footprint(
    plan: &ProgramMemoryPlan,
    port: &PortMemoryPlan,
) -> Result<CurrentMemoryFootprint, MemoryPlanError> {
    let fixed = plan
        .allocations
        .iter()
        .find(|allocation| allocation.id == port.object)
        .ok_or(MemoryPlanError::DescriptorMismatch)?;
    let payload_bytes = plan
        .allocations
        .iter()
        .filter(|allocation| {
            allocation.owner == fixed.owner
                && allocation.role == mech_core::AllocationRole::VariablePayload
        })
        .try_fold(0_u64, |total, allocation| {
            total
                .checked_add(allocation.current_bytes)
                .ok_or(MemoryPlanError::ArithmeticOverflow {
                    field: "resident current payload bytes",
                })
        })?;
    let retained_nodes = plan
        .values
        .iter()
        .find(|value| value.object == port.object)
        .map_or(0, |value| value.layout.payload.current_nodes);
    Ok(CurrentMemoryFootprint {
        logical_elements: port.value.current_elements,
        fixed_bytes: fixed.current_bytes,
        payload_bytes,
        encoded_bytes: fixed.current_bytes.checked_add(payload_bytes).ok_or(
            MemoryPlanError::ArithmeticOverflow {
                field: "resident current encoded bytes",
            },
        )?,
        retained_nodes,
        ..CurrentMemoryFootprint::default()
    })
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
