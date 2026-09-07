use mech_core::{
    MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, RealizedMemoryPlan, RuntimePlanView,
    TransactionRequirement,
};

use crate::memory_planner::{PlannedValueClass, ProgramMemoryPlan, TurnMemoryPlan};

/// Owner-thread runtime state for one realized deterministic program plan.
pub struct ManagedProgramMemory {
    domain: MemoryDomain,
    realized: RealizedMemoryPlan,
}

impl ManagedProgramMemory {
    pub fn realize(plan: &ProgramMemoryPlan) -> MemoryRuntimeResult<Self> {
        let domain = MemoryDomain::new()?;
        let revision = domain.issue_plan_revision()?;
        let transactions = program_transactions(plan);
        let output_bytes = program_output_bytes(plan)?;
        let view = RuntimePlanView::new(
            revision,
            &plan.allocations,
            &plan.arenas,
            plan.peak,
            output_bytes,
            plan.budget_limits,
            &transactions,
            program_max_concurrent_leases(plan)?,
            &plan.budget_violations,
        );
        let reservation = domain.prepare_realization(view)?;
        let realized = domain.materialize(reservation)?;
        Ok(Self { domain, realized })
    }

    pub const fn domain(&self) -> &MemoryDomain {
        &self.domain
    }

    pub const fn realized(&self) -> &RealizedMemoryPlan {
        &self.realized
    }

    pub fn close(self) -> MemoryRuntimeResult<()> {
        self.domain.close()
    }
}

/// Realizes one already planned turn in a fresh owner-thread domain.
///
/// Program instances use [`ManagedProgramMemory`] for retained state. This
/// helper exists for isolated direct-call and qualification paths whose turn
/// plan is itself the complete live ownership graph.
pub fn realize_turn_memory(
    plan: &TurnMemoryPlan,
) -> MemoryRuntimeResult<(MemoryDomain, RealizedMemoryPlan)> {
    let domain = MemoryDomain::new()?;
    let revision = domain.issue_plan_revision()?;
    let view = RuntimePlanView::new(
        revision,
        &plan.allocations,
        &plan.arenas,
        plan.demand,
        plan.output_bytes,
        plan.budget_limits,
        &plan.transactions,
        plan.call
            .as_ref()
            .map(|call| call.inputs.len().saturating_add(call.outputs.len()))
            .map(u32::try_from)
            .transpose()
            .map_err(|_| MemoryRuntimeError::IdentityExhausted {
                identity: "turn call access count",
            })?
            .unwrap_or(0),
        &plan.budget_violations,
    );
    let reservation = domain.prepare_realization(view)?;
    let realized = domain.materialize(reservation)?;
    Ok((domain, realized))
}

fn program_transactions(plan: &ProgramMemoryPlan) -> Box<[TransactionRequirement]> {
    let mut transactions = plan
        .values
        .iter()
        .map(|value| value.transaction)
        .chain(
            plan.calls
                .iter()
                .flat_map(|call| call.transactions.iter().copied()),
        )
        .filter(|transaction| *transaction != TransactionRequirement::None)
        .collect::<Vec<_>>();
    transactions.sort();
    transactions.dedup();
    transactions.into_boxed_slice()
}

fn program_output_bytes(plan: &ProgramMemoryPlan) -> MemoryRuntimeResult<u64> {
    plan.values
        .iter()
        .filter(|value| value.class == PlannedValueClass::PublishedOutput)
        .try_fold(0_u64, |total, value| {
            let bytes = value
                .layout
                .current_address_span_bytes
                .checked_add(value.layout.payload.current_bytes)
                .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "published output bytes",
                    current: value.layout.current_address_span_bytes,
                    change: value.layout.payload.current_bytes,
                })?;
            total
                .checked_add(bytes)
                .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "published output bytes",
                    current: total,
                    change: bytes,
                })
        })
}

fn program_max_concurrent_leases(plan: &ProgramMemoryPlan) -> MemoryRuntimeResult<u32> {
    plan.calls
        .iter()
        .map(|call| call.inputs.len().saturating_add(call.outputs.len()))
        .max()
        .map(u32::try_from)
        .transpose()
        .map_err(|_| MemoryRuntimeError::IdentityExhausted {
            identity: "program call access count",
        })
        .map(|count| count.unwrap_or(0))
}
