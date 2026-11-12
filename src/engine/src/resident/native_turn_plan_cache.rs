//! Bounded native planning metadata, never Values, results or visit authority.
use super::*;
use crate::memory_planner::{TurnMemoryFacts, TurnMemoryPlan};
use mech_core::{
    CallMemoryPlan, CurrentMemoryFootprint, ManagedMemoryReservation, ManagedNodeReservation,
    MemoryBudgetLimits,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[path = "native_string_projection_context.rs"]
mod string_projection;
pub(in crate::resident) use string_projection::NativeProjectionIndexContext;

#[path = "native_snapshot_footprint.rs"]
mod snapshot_footprint;
pub(in crate::resident) use snapshot_footprint::{
    DataFootprintTransaction, borrowed_data_footprint,
};

#[derive(Debug)]
struct Refined {
    base: Arc<TurnMemoryPlan>,
    observed: ResourceDemand,
    final_output: Option<CurrentMemoryFootprint>,
    plan: Arc<TurnMemoryPlan>,
}
// Index construction, selector scratch and candidate output are three
// separately admitted maintained String-selection phases. This fixed LRU
// avoids their deterministic alternation; other families safely miss/evict.
const REFINEMENTS: usize = 3;

#[derive(Debug)]
pub(crate) struct NativeTurnPlanCache {
    facts: RefCell<TurnMemoryFacts>,
    base: RefCell<Option<Arc<TurnMemoryPlan>>>,
    refined: RefCell<[Option<Refined>; REFINEMENTS]>,
    snapshot_index: crate::resident::text_index::snapshot::Slot,
    data_footprints: snapshot_footprint::DataFootprints,
    data_nodes: u64,
    limits: MemoryBudgetLimits,
    work: NativeLibraryWorkBudget,
    probe_work: u64,
    miss_work: u64,
    // Every owned map/plan and its retirement precede this reservation's drop.
    reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}

impl NativeTurnPlanCache {
    pub(crate) fn owner_bytes() -> u64 {
        (core::mem::size_of::<Self>() + 2 * core::mem::size_of::<usize>()) as u64
    }

    pub(crate) fn retained_bytes(call: &CallMemoryPlan, metadata: u64) -> Option<u64> {
        // Separate bounded populations: reusable measured facts, one base
        // plan and the fixed refined lanes. Each uses the unchanged
        // full construction/coexistence envelope, not observed allocator peaks.
        metadata
            .checked_mul(2 + REFINEMENTS as u64)?
            .checked_add(Self::owner_bytes())
            .and_then(|n| {
                n.checked_add(snapshot_footprint::DataFootprints::bytes(
                    call.inputs.len(),
                )?)
            })
    }

    pub(crate) fn retained_data_nodes(call: &CallMemoryPlan) -> Option<u64> {
        snapshot_footprint::DataFootprints::nodes(call.inputs.len())
    }

    pub(crate) fn construction_owner_bytes(call: &CallMemoryPlan) -> Option<u64> {
        Self::construction_owner_bytes_for_inputs(call.inputs.len())
    }

    pub(crate) fn construction_owner_bytes_for_inputs(inputs: usize) -> Option<u64> {
        Self::owner_bytes().checked_add(snapshot_footprint::DataFootprints::bytes(inputs)?)
    }

    // The exact maintained warm lookup price, without constructing a call,
    // cache or plan. Selected replay previews use only immutable port counts
    // and a separately proved descriptor envelope.
    pub(crate) fn probe_work_for_ports(
        inputs: usize,
        outputs: usize,
        descriptor_work: u64,
    ) -> Option<u64> {
        let ports = u64::try_from(inputs.checked_add(outputs)?).ok()?;
        let keys = ports.checked_add(2)?;
        let entry = (core::mem::size_of::<(mech_core::NodeId, mech_core::PortDirection, u16)>()
            + 2 * core::mem::size_of::<CurrentMemoryFootprint>()
            + 2 * core::mem::size_of::<mech_core::RegionAccessPlan>()) as u64;
        let bytes = keys
            .checked_mul(keys)?
            .checked_mul(entry)?
            .checked_add(ports.checked_mul(
                (2 * core::mem::size_of::<mech_core::ResidentValueRef<'static>>()) as u64,
            )?)?
            .checked_add(payload::native_scope_metadata_bytes())?
            .checked_add(Self::owner_bytes())?;
        bytes
            .checked_mul(8)?
            .checked_add(256)?
            .checked_add(descriptor_work)
    }

    fn refinement_probe_work() -> Option<u64> {
        ((2 * core::mem::size_of::<Refined>() * REFINEMENTS) as u64)
            .checked_mul(8)?
            .checked_add(256)
    }

    pub(crate) fn refinement_work_bound(metadata: u64) -> Option<u64> {
        Self::refinement_probe_work()?.checked_add(metadata)
    }

    pub(crate) fn selected_access_data_work_bound(inputs: usize) -> Option<u64> {
        snapshot_footprint::DataFootprints::selected_access_metadata_work_bound(inputs)
    }

    pub(crate) fn new(
        call: &CallMemoryPlan,
        metadata: u64,
        descriptor_work: u64,
        reservation: ManagedMemoryReservation,
        nodes: ManagedNodeReservation,
        work: &NativeLibraryWorkBudget,
    ) -> Result<Rc<Self>, ResidentKernelError> {
        if reservation.capacity_bytes()
            != Self::retained_bytes(call, metadata).ok_or(ResidentKernelError::InvalidShape)?
            || reservation.budget() != work.memory_budget()
            || nodes.budget() != work.memory_budget()
            || nodes.nodes()
                != 1u64
                    .checked_add(
                        Self::retained_data_nodes(call).ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .ok_or(ResidentKernelError::InvalidShape)?
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        // Keep descriptor/schema work on every hit. Only the actually
        // eliminated planner-entry construction component moves to misses.
        let probe_work =
            Self::probe_work_for_ports(call.inputs.len(), call.outputs.len(), descriptor_work)
                .ok_or(ResidentKernelError::InvalidShape)?;
        let data_nodes =
            Self::retained_data_nodes(call).ok_or(ResidentKernelError::InvalidShape)?;
        let data_footprints = snapshot_footprint::DataFootprints::new(call)?;
        Ok(Rc::new(Self {
            facts: RefCell::new(TurnMemoryFacts::default()),
            base: RefCell::new(None),
            refined: RefCell::new(std::array::from_fn(|_| None)),
            snapshot_index: Default::default(),
            data_footprints,
            data_nodes,
            limits: call.target.limits,
            work: work.clone(),
            probe_work,
            miss_work: metadata,
            reservation,
            _nodes: nodes,
        }))
    }

    pub(crate) fn validate(
        &self,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> bool {
        self.limits == limits
            && self.work.shares_account(work)
            && self.reservation.budget() == work.memory_budget()
    }
    pub(crate) fn bytes(&self) -> u64 {
        // Both independent reservations belong to this SAME finite account;
        // their sum cannot exceed its u64 capacity. Overflow still fails closed.
        self.reservation
            .capacity_bytes()
            .checked_add(self.snapshot_index.bytes())
            .unwrap_or(u64::MAX)
    }

    pub(crate) fn retained_metadata_demand(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<ResourceDemand, ResidentKernelError> {
        if !self.validate(self.limits, work) {
            return Err(ResidentKernelError::InvalidInput);
        }
        // Planning/fact/schema metadata remains fully byte-admitted, including
        // unused coexistence capacity. Count its one cache owner, then the
        // existing index/weak-data witnesses, as maintained planner admission
        // does. Reserved bytes are not live semantic/data nodes.
        let metadata = self.reservation.capacity_bytes();
        Ok(ResourceDemand {
            turn_peak_bytes: metadata
                .checked_add(self.snapshot_index.bytes())
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes: 1u64
                .checked_add(self.snapshot_index.nodes())
                .and_then(|n| n.checked_add(self.data_nodes))
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..ResourceDemand::default()
        })
    }
    pub(crate) fn index_nodes(&self) -> u64 {
        self.snapshot_index
            .nodes()
            .checked_add(self.data_nodes)
            .unwrap_or(u64::MAX)
    }
    fn check_index_capacity(
        &self,
        plan: &TurnMemoryPlan,
        bytes: u64,
        nodes: u64,
    ) -> Result<(), ResidentKernelError> {
        let mut demand = plan.demand;
        let auxiliary = ACTIVE_PAYLOAD_ADMISSION.with(|active| {
            active
                .borrow()
                .as_ref()
                .map_or(0, |admission| admission.auxiliary_bytes())
        });
        demand.turn_peak_bytes = demand
            .turn_peak_bytes
            .checked_add(auxiliary)
            .and_then(|n| n.checked_add(self.reservation.capacity_bytes()))
            .and_then(|n| n.checked_add(bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        demand.retained_nodes = demand
            .retained_nodes
            .checked_add(nodes)
            .and_then(|n| n.checked_add(self.data_nodes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        if let Some(violation) = mech_core::evaluate_call_memory_budget(
            mech_core::MemoryObjectOwner::NodeOutput {
                node: plan.node,
                port: 0,
            },
            demand,
            plan.output_bytes,
            plan.storage_buffer_bytes,
            self.limits,
        )
        .first()
        {
            return Err(resource_budget_error(violation));
        }
        Ok(())
    }

    pub(crate) fn has_base(&self) -> bool {
        self.base.borrow().is_some()
    }
    pub(crate) fn charge_probe(&self) -> Result<(), ResidentKernelError> {
        self.work.charge(self.probe_work, 0)
    }
    pub(crate) fn with_facts<T>(&self, f: impl FnOnce(&mut TurnMemoryFacts) -> T) -> T {
        f(&mut self.facts.borrow_mut())
    }
    pub(crate) fn matching_base(&self, facts: &TurnMemoryFacts) -> Option<Arc<TurnMemoryPlan>> {
        self.base
            .borrow()
            .as_ref()
            .filter(|p| &p.facts == facts)
            .cloned()
    }
    pub(crate) fn retire(&self) {
        // The refined key owns its base; drop it first, before replacement.
        for entry in self.refined.borrow_mut().iter_mut() {
            entry.take();
        }
        self.base.borrow_mut().take();
    }
    pub(crate) fn remember_base(&self, plan: Arc<TurnMemoryPlan>) {
        *self.base.borrow_mut() = Some(plan);
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<Rc<NativeTurnPlanCache>>> = const { RefCell::new(None) };
}
pub(super) struct Guard(Option<Rc<NativeTurnPlanCache>>);
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = self.0.take());
    }
}
pub(super) fn suspend() -> Guard {
    Guard(ACTIVE.with(|active| active.borrow_mut().take()))
}
pub(crate) fn with_cached_turn_plan<T>(
    plan: Arc<TurnMemoryPlan>,
    cache: Rc<NativeTurnPlanCache>,
    execute: impl FnOnce() -> T,
) -> T {
    with_resident_turn_plan(plan, || {
        let _guard = Guard(ACTIVE.with(|active| active.replace(Some(cache))));
        execute()
    })
}
pub(super) fn retained_bytes() -> u64 {
    ACTIVE.with(|active| active.borrow().as_ref().map_or(0, |cache| cache.bytes()))
}
pub(super) fn retained_index_nodes() -> u64 {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .map_or(0, |cache| cache.index_nodes())
    })
}
pub(in crate::resident) fn prepared_snapshot_index(
    value: &Value,
) -> Option<Result<crate::resident::text_index::IndexUse, ResidentKernelError>> {
    // Dense lexical memo baselines retain their unchanged isolation behavior.
    if crate::resident::text_index::has_index_scope()
        || crate::resident::text_index::live_demand() != (0, 0)
    {
        return None;
    }
    let cache = ACTIVE.with(|active| active.borrow().clone())?;
    let plan = with_active_turn_plan(|active| active.clone())?;
    if !cache
        .base
        .borrow()
        .as_ref()
        .is_some_and(|base| Arc::ptr_eq(base, &plan))
    {
        return None;
    }
    Some(
        cache
            .snapshot_index
            .prepare(value, &cache.work, |bytes, nodes| {
                cache.check_index_capacity(&plan, bytes, nodes)
            }),
    )
}

pub(super) fn refine(
    base: &Arc<TurnMemoryPlan>,
    observed: ResourceDemand,
    final_output: Option<CurrentMemoryFootprint>,
) -> Option<Result<ResidentBudgetPermit, ResidentKernelError>> {
    let cache = ACTIVE.with(|active| active.borrow().clone())?;
    if !cache
        .base
        .borrow()
        .as_ref()
        .is_some_and(|p| Arc::ptr_eq(p, base))
    {
        return None;
    }
    Some((|| {
        // Complete fixed-size key comparison is prepaid before reading it.
        // Two complete passes over the concrete fixed lanes cover all key
        // comparisons and bounded in-place LRU moves; no key is hashed.
        cache.work.charge(
            NativeTurnPlanCache::refinement_probe_work()
                .ok_or(ResidentKernelError::InvalidShape)?,
            0,
        )?;
        let hit = cache.refined.borrow().iter().position(|entry| {
            entry.as_ref().is_some_and(|p| {
                Arc::ptr_eq(&p.base, base)
                    && p.observed == observed
                    && p.final_output == final_output
            })
        });
        if let Some(index) = hit {
            let plan = {
                let mut entries = cache.refined.borrow_mut();
                entries[..=index].rotate_right(1);
                entries[0].as_ref().unwrap().plan.clone()
            };
            // A cached plan is not a cached permit. Re-admit current payload
            // ownership and finite coexistence under this invocation's scope.
            admit_payload_plan(&plan)?;
            return Ok(ResidentBudgetPermit {
                _demand: plan.demand,
                _plan: plan,
            });
        }
        cache.work.charge(cache.miss_work, 0)?;
        {
            let mut entries = cache.refined.borrow_mut();
            entries[REFINEMENTS - 1].take();
            entries.rotate_right(1);
        }
        let plan = apply_observed_turn_demand((**base).clone(), observed, final_output)
            .map_err(memory_plan_kernel_error)?;
        let permit = ResidentBudgetPermit::from_turn_plan(plan)?;
        cache.refined.borrow_mut()[0] = Some(Refined {
            base: base.clone(),
            observed,
            final_output,
            plan: permit._plan.clone(),
        });
        Ok(permit)
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_replay_metadata_arithmetic_declines_overflow_without_authority() {
        assert!(NativeTurnPlanCache::probe_work_for_ports(2, 1, 1024).is_some());
        assert!(NativeTurnPlanCache::probe_work_for_ports(usize::MAX, 1, 0).is_none());
        assert!(NativeTurnPlanCache::probe_work_for_ports(2, 1, u64::MAX).is_none());
        assert!(NativeTurnPlanCache::construction_owner_bytes_for_inputs(usize::MAX).is_none());
        assert!(NativeTurnPlanCache::refinement_work_bound(u64::MAX).is_none());
        assert!(NativeTurnPlanCache::selected_access_data_work_bound(usize::MAX).is_none());
    }

    #[test]
    fn cached_context_is_suspended_by_nested_ordinary_scope_and_unwind() {
        // Context isolation only: this detached test plan never executes a
        // native body or grants a materialization permit.
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
        let work = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
        let cache = Rc::new(NativeTurnPlanCache {
            facts: RefCell::new(TurnMemoryFacts::default()),
            base: RefCell::new(None),
            refined: RefCell::new(std::array::from_fn(|_| None)),
            snapshot_index: Default::default(),
            data_footprints: Default::default(),
            data_nodes: 0,
            limits: MemoryBudgetLimits::default(),
            work: work.clone(),
            probe_work: 1,
            miss_work: 1,
            reservation: memory.reserve_capacity(4096).unwrap(),
            _nodes: memory.reserve_nodes(1).unwrap(),
        });
        let plan = Arc::new(detached_turn_plan(ResourceDemand::default()).unwrap());
        assert_eq!(retained_bytes(), 0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_cached_turn_plan(plan.clone(), cache.clone(), || {
                assert_eq!(retained_bytes(), 4096);
                with_resident_turn_plan(plan.clone(), || assert_eq!(retained_bytes(), 0));
                assert_eq!(retained_bytes(), 4096);
                panic!("controlled native metadata scope unwind");
            });
        }));
        assert!(result.is_err());
        assert_eq!(retained_bytes(), 0);
        drop((cache, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
