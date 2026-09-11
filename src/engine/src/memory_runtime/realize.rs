use mech_core::{
    MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, RealizedMemoryPlan, RuntimePlanView,
    TransactionRequirement,
};

use crate::memory_planner::{ProgramMemoryPlan, TurnMemoryPlan};

/// Owner-thread runtime state for one realized deterministic program plan.
pub struct ManagedProgramMemory {
    domain: MemoryDomain,
    realized: RealizedMemoryPlan,
}

impl core::fmt::Debug for ManagedProgramMemory {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ManagedProgramMemory")
            .field("domain", &self.domain.id())
            .field("revision", &self.realized.revision())
            .finish_non_exhaustive()
    }
}

impl ManagedProgramMemory {
    pub fn realize(plan: &ProgramMemoryPlan) -> MemoryRuntimeResult<Self> {
        Self::realize_with_memory_budget(plan, None)
    }

    /// Retains caller-supplied aggregate admission authority across physical
    /// program realizations, including concurrently retained replacements.
    pub fn realize_with_memory_budget(
        plan: &ProgramMemoryPlan,
        budget: Option<&mech_core::ManagedMemoryBudget>,
    ) -> MemoryRuntimeResult<Self> {
        let domain = match budget {
            Some(budget) => MemoryDomain::with_memory_budget(budget.clone())?,
            None => MemoryDomain::new()?,
        };
        let revision = domain.issue_plan_revision()?;
        let transactions = program_transactions(plan);
        let view = RuntimePlanView::for_aggregate(
            revision,
            &plan.allocations,
            &plan.arenas,
            plan.peak,
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
    let view = RuntimePlanView::for_call(
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
