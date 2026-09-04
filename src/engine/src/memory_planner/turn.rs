use std::collections::BTreeMap;

use mech_core::{
    AllocationPlan, MemoryBudgetViolation, MemoryObjectOwner, MemoryPlanError, NodeId,
    RegionAccessPlan, ResourceDemand, TransactionRequirement,
};

use super::{ProgramMemoryPlan, checked_demand_add};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TurnMemoryFacts {
    pub resolved_regions: BTreeMap<(NodeId, u16), RegionAccessPlan>,
    pub additional_demand: ResourceDemand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnMemoryPlan {
    pub node: NodeId,
    pub allocations: Box<[AllocationPlan]>,
    pub transactions: Box<[TransactionRequirement]>,
    pub demand: ResourceDemand,
    pub budget_violations: Box<[MemoryBudgetViolation]>,
}

pub fn plan_turn_memory(
    plan: &ProgramMemoryPlan,
    node: NodeId,
    facts: &TurnMemoryFacts,
) -> Result<TurnMemoryPlan, MemoryPlanError> {
    let allocations = plan
        .allocations
        .iter()
        .filter(|allocation| owner_node(&allocation.owner) == Some(node))
        .cloned()
        .collect::<Vec<_>>();
    let mut transactions = Vec::new();
    let mut demand = facts.additional_demand;
    if let Some(call) = plan.call_for_node(node) {
        demand = checked_demand_add(demand, call.demand)?;
        transactions.extend(call.transactions.iter().copied());
        for (ordinal, output) in call.outputs.iter().enumerate() {
            if matches!(output.region, RegionAccessPlan::Deferred(_))
                && !facts.resolved_regions.contains_key(&(
                    node,
                    u16::try_from(ordinal).map_err(|_| MemoryPlanError::ArithmeticOverflow {
                        field: "turn output ordinal",
                    })?,
                ))
            {
                return Err(MemoryPlanError::MissingFootprintWitness {
                    stage: mech_core::MemoryWitnessStage::Turn,
                });
            }
        }
    }
    for value in plan.values.iter().filter(|value| {
        value.transaction != TransactionRequirement::None
            && value.class == super::PlannedValueClass::State
            && value.producer == Some(node)
    }) {
        transactions.push(value.transaction);
    }
    transactions.sort_by_key(transaction_key);
    transactions.dedup();
    let mut budget_violations = plan
        .budget_violations
        .iter()
        .filter(|violation| owner_node(&violation.owner) == Some(node))
        .cloned()
        .collect::<Vec<_>>();
    budget_violations.sort();
    Ok(TurnMemoryPlan {
        node,
        allocations: allocations.into_boxed_slice(),
        transactions: transactions.into_boxed_slice(),
        demand,
        budget_violations: budget_violations.into_boxed_slice(),
    })
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
