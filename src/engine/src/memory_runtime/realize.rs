use mech_core::{MemoryDomain, MemoryRuntimeResult, RealizedMemoryPlan, RuntimePlanView};

use crate::memory_planner::{ProgramMemoryPlan, TurnMemoryPlan};

/// Owner-thread runtime state for one realized deterministic program plan.
pub struct ManagedProgramMemory {
    domain: MemoryDomain,
    realized: RealizedMemoryPlan,
}

impl ManagedProgramMemory {
    pub fn realize(plan: &ProgramMemoryPlan) -> MemoryRuntimeResult<Self> {
        let domain = MemoryDomain::new()?;
        let revision = domain.issue_plan_revision()?;
        let view = RuntimePlanView::new(
            revision,
            &plan.allocations,
            &plan.arenas,
            plan.peak,
            plan.budget_limits,
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
        plan.budget_limits,
        &plan.budget_violations,
    );
    let reservation = domain.prepare_realization(view)?;
    let realized = domain.materialize(reservation)?;
    Ok((domain, realized))
}
