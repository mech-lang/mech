use std::sync::{Arc, Mutex};

use mech_core::snapshot::{
    SequenceView, SnapshotCanonicalizationBudget, SnapshotValueError, ValueFootprint,
};
use mech_core::{
    ResidentKernelError, ResourceDemand, SchemaBody, SchemaId, SchemaTable, Value, ValueData,
};

use crate::memory_planner::{
    TurnMemoryPlan, apply_observed_turn_demand, check_turn_planning_progress,
    try_admit_fixed_turn_memory,
};

#[path = "payload_budget.rs"]
pub(crate) mod payload;

#[derive(Default)]
struct DisplayByteCounter(usize);

impl core::fmt::Write for DisplayByteCounter {
    fn write_str(&mut self, value: &str) -> core::fmt::Result {
        self.0 = self.0.checked_add(value.len()).ok_or(core::fmt::Error)?;
        Ok(())
    }
}

fn displayed_bytes(value: impl core::fmt::Display) -> Result<usize, ResidentKernelError> {
    use core::fmt::Write;

    let mut counter = DisplayByteCounter::default();
    write!(&mut counter, "{value}").map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok(counter.0)
}

fn projected_string_value_bytes(value: &ValueData) -> Result<usize, ResidentKernelError> {
    match value {
        ValueData::U8(value) => displayed_bytes(value),
        ValueData::U16(value) => displayed_bytes(value),
        ValueData::U32(value) => displayed_bytes(value),
        ValueData::U64(value) => displayed_bytes(value),
        ValueData::U128(value) => displayed_bytes(value),
        ValueData::I8(value) => displayed_bytes(value),
        ValueData::I16(value) => displayed_bytes(value),
        ValueData::I32(value) => displayed_bytes(value),
        ValueData::I64(value) => displayed_bytes(value),
        ValueData::I128(value) => displayed_bytes(value),
        ValueData::F32(value) => displayed_bytes(value.to_f32()),
        ValueData::F64(value) => displayed_bytes(value.to_f64()),
        ValueData::Complex32(value) => displayed_bytes(value.real().to_f32())?
            .checked_add(displayed_bytes(value.imaginary().to_f32())?)
            .and_then(|bytes| bytes.checked_add(2))
            .ok_or(ResidentKernelError::InvalidShape),
        ValueData::Complex64(value) => displayed_bytes(value.real().to_f64())?
            .checked_add(displayed_bytes(value.imaginary().to_f64())?)
            .and_then(|bytes| bytes.checked_add(2))
            .ok_or(ResidentKernelError::InvalidShape),
        ValueData::Rational64(value) => displayed_bytes(value.numerator())?
            .checked_add(displayed_bytes(value.denominator())?)
            .and_then(|bytes| bytes.checked_add(1))
            .ok_or(ResidentKernelError::InvalidShape),
        ValueData::Bool(value) => Ok(if *value { 4 } else { 5 }),
        ValueData::String(value) => Ok(value.len()),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn projected_display_sequence<T: core::fmt::Display>(
    values: &[T],
) -> Result<usize, ResidentKernelError> {
    values.iter().try_fold(0usize, |bytes, value| {
        bytes
            .checked_add(displayed_bytes(value)?)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

pub(super) fn projected_snapshot_string_payload(
    value: &Value,
) -> Result<usize, ResidentKernelError> {
    match value.data() {
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::U8(values) => projected_display_sequence(values),
            SequenceView::U16(values) => projected_display_sequence(values),
            SequenceView::U32(values) => projected_display_sequence(values),
            SequenceView::U64(values) => projected_display_sequence(values),
            SequenceView::U128(values) => projected_display_sequence(values),
            SequenceView::I8(values) => projected_display_sequence(values),
            SequenceView::I16(values) => projected_display_sequence(values),
            SequenceView::I32(values) => projected_display_sequence(values),
            SequenceView::I64(values) => projected_display_sequence(values),
            SequenceView::I128(values) => projected_display_sequence(values),
            SequenceView::F32(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(displayed_bytes(value.to_f32())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::F64(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(displayed_bytes(value.to_f64())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::Complex32(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(projected_string_value_bytes(&ValueData::Complex32(*value))?)
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::Complex64(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(projected_string_value_bytes(&ValueData::Complex64(*value))?)
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::Rational64(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(projected_string_value_bytes(&ValueData::Rational64(
                        value.clone(),
                    ))?)
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::Bool(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(if *value { 4 } else { 5 })
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::String(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(value.len())
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::Values(values) => values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(projected_string_value_bytes(value)?)
                    .ok_or(ResidentKernelError::InvalidShape)
            }),
            SequenceView::Id(_) | SequenceView::Index(_) | SequenceView::Unit(_) => {
                Err(ResidentKernelError::InvalidInput)
            }
        },
        value => projected_string_value_bytes(value),
    }
}

thread_local! {
    static ACTIVE_TURN_PLAN: Mutex<Option<Arc<TurnMemoryPlan>>> = const { Mutex::new(None) };
    static ACTIVE_PAYLOAD_ADMISSION: std::cell::RefCell<Option<std::rc::Rc<payload::ResidentPayloadAdmission>>> = const { std::cell::RefCell::new(None) };
}

// Repeated control execution accumulates the costs admitted by the existing
// kernel budget authority. Each invocation contributes its maximum observed
// demand once; incremental estimates within that invocation are not summed.
#[derive(Default)]
struct ControlWork {
    compute: u64,
    comparison: u64,
}

struct ControlWorkScope {
    total: std::rc::Rc<std::cell::RefCell<ControlWork>>,
    observed: ControlWork,
}

thread_local! {
    static CONTROL_WORK: std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<ControlWork>>>> = const { std::cell::RefCell::new(None) };
    static CONTROL_WORK_SCOPE: std::cell::RefCell<Option<ControlWorkScope>> = const { std::cell::RefCell::new(None) };
    static PLANNING_STEPS: std::cell::RefCell<Option<PlanningSteps>> = const { std::cell::RefCell::new(None) };
}

struct PlanningSteps {
    limit: usize,
    remaining: usize,
    exhausted: bool,
}

/// Bounds synchronous compiler evaluation across activation and initial-turn
/// execution. Nested control operations share the caller's step authority.
#[doc(hidden)]
pub fn with_planning_step_limit<T>(limit: usize, execute: impl FnOnce() -> T) -> Result<T, usize> {
    struct Guard(Option<PlanningSteps>);
    impl Drop for Guard {
        fn drop(&mut self) {
            PLANNING_STEPS.with(|active| *active.borrow_mut() = self.0.take());
        }
    }
    let _guard = Guard(PLANNING_STEPS.with(|active| {
        active.replace(Some(PlanningSteps {
            limit,
            remaining: limit,
            exhausted: false,
        }))
    }));
    let result = execute();
    PLANNING_STEPS.with(|active| {
        let active = active.borrow();
        let steps = active.as_ref().expect("planning step scope is installed");
        if steps.exhausted {
            Err(steps.limit)
        } else {
            Ok(result)
        }
    })
}

pub(crate) fn charge_planning_step() -> Result<(), super::general::ResidentExecutionError> {
    PLANNING_STEPS.with(|active| {
        let mut active = active.borrow_mut();
        let Some(steps) = active.as_mut() else {
            return Ok(());
        };
        if steps.remaining == 0 {
            steps.exhausted = true;
            return Err(super::general::ResidentExecutionError::PlanningStepLimit {
                limit: steps.limit,
            });
        }
        steps.remaining -= 1;
        Ok(())
    })
}

pub(crate) fn with_control_work_budget<T>(execute: impl FnOnce() -> T) -> T {
    struct Guard(Option<std::rc::Rc<std::cell::RefCell<ControlWork>>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            CONTROL_WORK.with(|active| *active.borrow_mut() = self.0.take());
        }
    }
    let previous = CONTROL_WORK.with(|active| {
        let mut active = active.borrow_mut();
        let total = active.clone().unwrap_or_default();
        active.replace(total)
    });
    let _guard = Guard(previous);
    execute()
}

struct ControlWorkScopeGuard(Option<ControlWorkScope>);

impl ControlWorkScopeGuard {
    fn enter() -> Self {
        let next = CONTROL_WORK
            .with(|active| active.borrow().clone())
            .map(|total| ControlWorkScope {
                total,
                observed: ControlWork::default(),
            });
        Self(CONTROL_WORK_SCOPE.with(|active| active.replace(next)))
    }
}

impl Drop for ControlWorkScopeGuard {
    fn drop(&mut self) {
        CONTROL_WORK_SCOPE.with(|active| *active.borrow_mut() = self.0.take());
    }
}

fn observe_control_work(demand: ResourceDemand) -> Result<(), ResidentKernelError> {
    CONTROL_WORK_SCOPE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(scope) = active.as_mut() else {
            return Ok(());
        };
        let mut total = scope.total.borrow_mut();
        let compute = total
            .compute
            .checked_add(demand.work.compute.saturating_sub(scope.observed.compute))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let comparison = total
            .comparison
            .checked_add(
                demand
                    .work
                    .comparison
                    .saturating_sub(scope.observed.comparison),
            )
            .ok_or(ResidentKernelError::InvalidShape)?;
        if compute > mech_core::RESIDENT_MAX_COMPUTE_WORK
            || comparison > mech_core::RESIDENT_MAX_COMPARISON_WORK
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        total.compute = compute;
        total.comparison = comparison;
        scope.observed.compute = scope.observed.compute.max(demand.work.compute);
        scope.observed.comparison = scope.observed.comparison.max(demand.work.comparison);
        Ok(())
    })
}

pub(crate) fn with_payload_admission<T>(
    admission: Option<std::rc::Rc<payload::ResidentPayloadAdmission>>,
    execute: impl FnOnce() -> T,
) -> T {
    struct Guard(Option<std::rc::Rc<payload::ResidentPayloadAdmission>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            ACTIVE_PAYLOAD_ADMISSION.with(|active| *active.borrow_mut() = self.0.take());
        }
    }
    let previous = ACTIVE_PAYLOAD_ADMISSION.with(|active| active.replace(admission));
    let _guard = Guard(previous);
    execute()
}

fn admit_payload_plan(plan: &TurnMemoryPlan) -> Result<(), ResidentKernelError> {
    ACTIVE_PAYLOAD_ADMISSION.with(|active| {
        if let Some(admission) = active.borrow().as_ref() {
            admission
                .admit_plan(plan)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
        }
        Ok(())
    })
}

/// Reuse the old output witness measured at this call's execution boundary.
/// The turn plan is scoped to the kernel invocation; no snapshot or permit is
/// cached across turns, and candidate admission still charges coexistence.
pub(crate) fn published_canonical_footprint(
    meter: &mut ResidentBudgetMeter,
    current: &Value,
    schemas: &SchemaTable,
) -> Result<ValueFootprint, ResidentKernelError> {
    let witnessed = with_active_turn_plan(|active| {
        active.as_ref().map(|plan| {
            let output = plan
                .call
                .as_ref()
                .and_then(|call| call.outputs.first())
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let schema = schemas
                .get(current.schema())
                .ok_or(ResidentKernelError::InvalidOutput)?;
            if schema != output.descriptor.schema()
                || !mech_core::shape_change_allowed(
                    schema,
                    output.descriptor.shape(),
                    current.shape(),
                )
            {
                return Err(ResidentKernelError::InvalidOutput);
            }
            let footprint = plan
                .facts
                .published_footprints
                .get(&(plan.node, 0))
                .ok_or(ResidentKernelError::InvalidOutput)?;
            Ok(ValueFootprint {
                encoded_bytes: footprint.encoded_bytes,
                retained_bytes: footprint.payload_bytes,
                node_count: footprint.retained_nodes,
            })
        })
    });
    if let Some(witnessed) = witnessed {
        let footprint = witnessed?;
        meter.charge_retained_nodes(footprint.node_count)?;
        meter.charge_compute_work(1)?;
        return Ok(footprint);
    }
    #[cfg(test)]
    {
        // Direct kernel unit tests have no executor boundary or scoped plan.
        return measure_canonical_value_footprint(meter, current, schemas);
    }
    #[cfg(not(test))]
    Err(ResidentKernelError::InvalidInput)
}

fn with_active_turn_plan<T>(use_plan: impl FnOnce(&mut Option<Arc<TurnMemoryPlan>>) -> T) -> T {
    ACTIVE_TURN_PLAN.with(|plans| {
        let mut plans = plans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        use_plan(&mut plans)
    })
}

struct ActiveTurnPlanGuard(Option<Arc<TurnMemoryPlan>>);

impl Drop for ActiveTurnPlanGuard {
    fn drop(&mut self) {
        with_active_turn_plan(|plan| *plan = self.0.take());
    }
}

/// Scopes every Resident materialization permit to the real node plan built
/// from the activated program. Nested execution remains deterministic and a
/// panic cannot leak authority to the next kernel on the worker thread.
pub(crate) fn with_resident_turn_plan<T>(
    plan: impl Into<Arc<TurnMemoryPlan>>,
    execute: impl FnOnce() -> T,
) -> T {
    let previous = with_active_turn_plan(|active| active.replace(plan.into()));
    let _guard = ActiveTurnPlanGuard(previous);
    let _work_scope = ControlWorkScopeGuard::enter();
    execute()
}

macro_rules! resident_cost {
    (@value $field:ident, $value:expr) => {
        $value
    };
    (@value $field:ident) => {
        $field
    };
    (@set $cost:ident, comparison_work, $value:expr) => {
        $cost.set_comparison_work($crate::resident::budget::checked_u64($value)?)?;
    };
    (@set $cost:ident, compute_work, $value:expr) => {
        $cost.set_compute_work($crate::resident::budget::checked_u64($value)?)?;
    };
    (@set $cost:ident, output_elements, $value:expr) => {
        $cost.set_output_elements($crate::resident::budget::checked_u64($value)?);
    };
    (@set $cost:ident, output_bytes, $value:expr) => {
        $cost.set_output_bytes($crate::resident::budget::checked_u64($value)?);
    };
    (@set $cost:ident, temporary_bytes, $value:expr) => {
        $cost.add_temporary_bytes($crate::resident::budget::checked_u64($value)?)?;
    };
    (@set $cost:ident, container_bytes, $value:expr) => {
        $cost.add_temporary_bytes($crate::resident::budget::checked_u64($value)?)?;
    };
    (@set $cost:ident, selector_bytes, $value:expr) => {
        $cost.add_temporary_bytes($crate::resident::budget::checked_u64($value)?)?;
    };
    (@set $cost:ident, index_bytes, $value:expr) => {
        $cost.add_temporary_bytes($crate::resident::budget::checked_u64($value)?)?;
    };
    (@set $cost:ident, cloned_bytes, $value:expr) => {
        $cost.set_cloned_bytes($crate::resident::budget::checked_u64($value)?);
    };
    (@set $cost:ident, retained_nodes, $value:expr) => {
        $cost.set_retained_nodes($crate::resident::budget::checked_u64($value)?);
    };
    (
        $( $field:ident $( : $value:expr )? ,)*
        .. $base:expr $(,)?
    ) => {{
        let mut cost = $base;
        $(
            $crate::resident::budget::resident_cost!(
                @set cost,
                $field,
                $crate::resident::budget::resident_cost!(@value $field $(, $value)?)
            );
        )*
        cost
    }};
}

pub(crate) use resident_cost;

#[cfg(test)]
pub(crate) use mech_core::RESIDENT_MAX_BYTES as MAX_RESIDENT_CLONED_BYTES;
#[cfg(test)]
pub(crate) use mech_core::RESIDENT_MAX_BYTES as MAX_RESIDENT_OUTPUT_BYTES;
#[cfg(test)]
pub(crate) use mech_core::RESIDENT_MAX_BYTES as MAX_RESIDENT_TEMPORARY_BYTES;
#[cfg(test)]
pub(crate) use mech_core::RESIDENT_MAX_RETAINED_NODES as MAX_RESIDENT_RETAINED_NODES;
pub(crate) use mech_core::{
    RESIDENT_MAX_COMPARISON_WORK as MAX_RESIDENT_COMPARISON_WORK,
    RESIDENT_MAX_COMPUTE_WORK as MAX_RESIDENT_COMPUTE_WORK,
    RESIDENT_MAX_OUTPUT_ELEMENTS as MAX_RESIDENT_OUTPUT_ELEMENTS,
};

/// Private compatibility shell for existing Resident call sites. It contains
/// exactly one shared R5 `ResourceDemand`; it has no parallel cost authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct KernelCostEstimate {
    demand: ResourceDemand,
}

/// Complete retained state after a mutation publishes, never merely the bytes
/// changed by the current execution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PublishedOutputFootprint {
    pub elements: u64,
    pub retained_bytes: u64,
    pub retained_nodes: u64,
}

/// Node populations that remain simultaneously live while a mutation stages
/// its final value. The published output is supplied separately so callers
/// cannot accidentally replace peak liveness with a per-object maximum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct MutationRetainedNodeFootprint {
    pub current_persistent: u64,
    pub normalized_plan: u64,
    pub temporary_draft: u64,
}

/// A mutation plan whose final published footprint is part of the admission
/// authority rather than an optional call-site convention.
#[derive(Clone, Debug)]
pub(crate) struct PreparedMutationPlan<P> {
    operation: P,
    final_output: PublishedOutputFootprint,
    cost: KernelCostEstimate,
}

#[derive(Clone, Debug)]
pub(crate) struct AdmittedMutationPlan<P> {
    operation: P,
    _permit: ResidentBudgetPermit,
}

/// Incremental fail-closed accounting for data-dependent borrowed traversals.
/// Every charge is checked immediately, so measuring a late or missing key
/// cannot perform more resident work than the shared limits permit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ResidentBudgetMeter {
    accumulated: KernelCostEstimate,
}

/// Authority proving one complete checked estimate passed central resident
/// admission. Its fields and constructor are private so materializers cannot
/// manufacture a permit locally.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResidentBudgetPermit {
    _plan: Arc<TurnMemoryPlan>,
    // Complete demand checked for this admission, not a cached permit.
    _demand: ResourceDemand,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedKernel<P> {
    plan: P,
    cost: KernelCostEstimate,
}

#[derive(Clone, Debug)]
pub(crate) struct AdmittedKernel<P> {
    plan: P,
    _permit: ResidentBudgetPermit,
}

impl KernelCostEstimate {
    pub(crate) const fn comparison_work(self) -> u64 {
        self.demand.work.comparison
    }

    pub(crate) const fn compute_work(self) -> u64 {
        self.demand.work.compute
    }

    pub(crate) const fn temporary_bytes(self) -> u64 {
        self.demand.turn_peak_bytes
    }

    pub(crate) const fn cloned_bytes(self) -> u64 {
        self.demand.cloned_bytes
    }

    pub(crate) const fn retained_nodes(self) -> u64 {
        self.demand.retained_nodes
    }

    pub(crate) fn set_comparison_work(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.demand.work.comparison = amount;
        Ok(())
    }

    pub(crate) fn set_compute_work(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.demand.work.compute = amount;
        Ok(())
    }

    pub(crate) fn set_output_elements(&mut self, amount: u64) {
        self.demand.output_elements = amount;
    }

    pub(crate) fn set_output_bytes(&mut self, amount: u64) {
        self.demand.persistent_bytes = amount;
    }

    pub(crate) fn add_temporary_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.demand.turn_peak_bytes = self
            .demand
            .turn_peak_bytes
            .checked_add(amount)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    }

    pub(crate) fn set_cloned_bytes(&mut self, amount: u64) {
        self.demand.cloned_bytes = amount;
    }

    pub(crate) fn set_retained_nodes(&mut self, amount: u64) {
        self.demand.retained_nodes = amount;
    }

    fn turn_plan(
        self,
        final_output: Option<PublishedOutputFootprint>,
    ) -> Result<ResidentBudgetPermit, ResidentKernelError> {
        observe_control_work(self.demand)?;
        let active = with_active_turn_plan(|plan| plan.clone());
        if let Some(active) = active {
            let fixed = active
                .call
                .as_ref()
                .and_then(|call| call.outputs.first())
                .map_or(0, |output| output.value.current_address_span_bytes);
            let final_output = final_output
                .map(|output| {
                    Ok(mech_core::CurrentMemoryFootprint {
                        logical_elements: output.elements,
                        fixed_bytes: fixed,
                        payload_bytes: output
                            .retained_bytes
                            .checked_sub(fixed)
                            .ok_or(ResidentKernelError::InvalidShape)?,
                        encoded_bytes: output.retained_bytes,
                        retained_nodes: output.retained_nodes,
                        ..mech_core::CurrentMemoryFootprint::default()
                    })
                })
                .transpose()?;
            if let Some(demand) = try_admit_fixed_turn_memory(&active, self.demand, final_output)
                .map_err(|_| ResidentKernelError::InvalidShape)?
            {
                admit_payload_plan(&active)?;
                return Ok(ResidentBudgetPermit {
                    _plan: active,
                    _demand: demand,
                });
            }
            return apply_observed_turn_demand((*active).clone(), self.demand, final_output)
                .map_err(|_| ResidentKernelError::InvalidShape)
                .and_then(ResidentBudgetPermit::from_turn_plan);
        }
        #[cfg(test)]
        {
            return detached_turn_plan(self.demand).and_then(ResidentBudgetPermit::from_turn_plan);
        }
        #[cfg(not(test))]
        {
            Err(ResidentKernelError::InvalidInput)
        }
    }

    // This admits another borrowed measurement step, never allocation or
    // publication. Complete candidate facts still require `turn_plan`.
    fn check_planning_progress(self) -> Result<(), ResidentKernelError> {
        observe_control_work(self.demand)?;
        let result = with_active_turn_plan(|active| {
            active.as_ref().map(|plan| {
                check_turn_planning_progress(plan, self.demand)
                    .map_err(|_| ResidentKernelError::InvalidShape)
            })
        });
        if let Some(result) = result {
            return result;
        }
        #[cfg(test)]
        {
            self.checked().map(drop)
        }
        #[cfg(not(test))]
        {
            Err(ResidentKernelError::InvalidInput)
        }
    }

    fn checked(self) -> Result<ResidentBudgetPermit, ResidentKernelError> {
        self.turn_plan(None)
    }

    #[cfg(test)]
    pub(crate) fn admit(self) -> Result<(), ResidentKernelError> {
        self.checked().map(drop)
    }

    /// Returns the single allowance left for data-dependent work that must be
    /// metered while an already-admitted plan is materialized. Comparison
    /// work also consumes compute work, so the smaller remaining limit is the
    /// only sound authority to pass into recursive canonicalization.
    pub(crate) fn remaining_incremental_work(&self) -> Result<u64, ResidentKernelError> {
        if self.demand.persistent_bytes == 0 && self.demand.output_elements == 0 {
            self.check_planning_progress()?;
        } else {
            self.checked()?;
        }
        Ok(MAX_RESIDENT_COMPARISON_WORK
            .checked_sub(self.comparison_work())
            .ok_or(ResidentKernelError::InvalidShape)?
            .min(
                MAX_RESIDENT_COMPUTE_WORK
                    .checked_sub(self.compute_work())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            ))
    }
}

#[cfg(test)]
fn detached_turn_plan(demand: ResourceDemand) -> Result<TurnMemoryPlan, ResidentKernelError> {
    let target = mech_core::TargetMemoryProfile::current_resident_cpu()
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let node = mech_core::NodeId::new(0);
    let program = crate::memory_planner::ProgramMemoryPlan {
        allocations: vec![mech_core::AllocationPlan {
            id: mech_core::MemoryObjectId::new(0),
            owner: mech_core::MemoryObjectOwner::TransactionStage { node, output: 0 },
            role: mech_core::AllocationRole::TransactionStage,
            slot: None,
            space: mech_core::MemorySpace::ResidentCpu,
            current_bytes: 0,
            capacity_bytes: 0,
            payload_block_capacity: 0,
            alignment: 1,
            lifetime: mech_core::MemoryLifetime::Transaction {
                first: mech_core::MemoryPlanPoint::new(0),
                last: mech_core::MemoryPlanPoint::new(1),
            },
            placement: mech_core::ArenaPlacement {
                arena: mech_core::MemoryArenaId::new(0),
                offset: 0,
            },
            reuse_group: None,
        }]
        .into_boxed_slice(),
        budget_limits: target.limits,
        ..crate::memory_planner::ProgramMemoryPlan::default()
    };
    crate::memory_planner::plan_turn_memory(
        &program,
        node,
        &crate::memory_planner::TurnMemoryFacts {
            observed_demand: Some(demand),
            ..crate::memory_planner::TurnMemoryFacts::default()
        },
    )
    .map_err(|_| ResidentKernelError::InvalidShape)
}

impl ResidentBudgetPermit {
    fn from_turn_plan(plan: TurnMemoryPlan) -> Result<Self, ResidentKernelError> {
        if !plan.budget_violations.is_empty() {
            return Err(ResidentKernelError::InvalidShape);
        }
        admit_payload_plan(&plan)?;
        Ok(Self {
            _demand: plan.demand,
            _plan: Arc::new(plan),
        })
    }
}

impl<P> PreparedKernel<P> {
    pub(crate) const fn new(plan: P, cost: KernelCostEstimate) -> Self {
        Self { plan, cost }
    }

    pub(crate) fn admit(self) -> Result<AdmittedKernel<P>, ResidentKernelError> {
        Ok(AdmittedKernel {
            plan: self.plan,
            _permit: self.cost.checked()?,
        })
    }

    /// Admit work performed by a control node outside an ordinary kernel
    /// call. Match result conversion owns its payload through the control
    /// write scope, while this permit validates and accumulates the complete
    /// conversion demand against the same resident target limits.
    pub(crate) fn admit_control(self) -> Result<AdmittedKernel<P>, ResidentKernelError> {
        let _scope = ControlWorkScopeGuard::enter();
        Ok(AdmittedKernel {
            plan: self.plan,
            _permit: self.cost.checked()?,
        })
    }
}

impl<P> AdmittedKernel<P> {
    /// Consumes the admission authority immediately before materialization.
    pub(crate) fn into_plan(self) -> P {
        self.plan
    }
}

impl<P> PreparedMutationPlan<P> {
    pub(crate) fn new(
        operation: P,
        final_output: PublishedOutputFootprint,
        retained_nodes: MutationRetainedNodeFootprint,
        mut cost: KernelCostEstimate,
    ) -> Result<Self, ResidentKernelError> {
        cost.set_output_elements(final_output.elements);
        cost.set_output_bytes(final_output.retained_bytes);
        // The borrowed current value, normalized plan, transient staging
        // trees, and published output can coexist. Every phase is explicit;
        // an ad hoc retained-node estimate in `cost` would either omit or
        // double-count one of those populations.
        if cost.retained_nodes() != 0 {
            return Err(ResidentKernelError::InvalidShape);
        }
        cost.set_retained_nodes(
            retained_nodes
                .current_persistent
                .checked_add(retained_nodes.normalized_plan)
                .and_then(|nodes| nodes.checked_add(retained_nodes.temporary_draft))
                .and_then(|nodes| nodes.checked_add(final_output.retained_nodes))
                .ok_or(ResidentKernelError::InvalidShape)?,
        );
        Ok(Self {
            operation,
            final_output,
            cost,
        })
    }

    pub(crate) fn admit(self) -> Result<AdmittedMutationPlan<P>, ResidentKernelError> {
        Ok(AdmittedMutationPlan {
            operation: self.operation,
            _permit: self.cost.turn_plan(Some(self.final_output))?,
        })
    }
}

/// Charges recursive canonical traversal one bounded chunk at a time before
/// any caller performs a complete footprint pass or key comparison.
fn charge_canonical_data_footprint_with(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
    charge_retained_bytes: Option<
        fn(&mut ResidentBudgetMeter, u64) -> Result<(), ResidentKernelError>,
    >,
) -> Result<mech_core::snapshot::ValueFootprint, ResidentKernelError> {
    let mut footprint = mech_core::snapshot::ValueFootprint::zero();
    mech_core::snapshot::visit_canonical_data_work(schema, data, |work| {
        meter.charge_comparison_work(work.encoded_bytes.max(work.node_count).max(1))?;
        if let Some(charge_retained_bytes) = charge_retained_bytes {
            charge_retained_bytes(meter, work.retained_bytes)?;
        }
        meter.charge_retained_nodes(work.node_count)?;
        footprint = footprint
            .checked_add(mech_core::snapshot::ValueFootprint {
                encoded_bytes: work.encoded_bytes,
                retained_bytes: work.retained_bytes,
                node_count: work.node_count,
            })
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        Ok(())
    })
    .map_err(|error| match error {
        mech_core::snapshot::CanonicalDataWorkError::Visitor(error) => error,
        mech_core::snapshot::CanonicalDataWorkError::ArithmeticOverflow
        | mech_core::snapshot::CanonicalDataWorkError::UnknownDynamicSchema
        | mech_core::snapshot::CanonicalDataWorkError::InvalidValue => {
            ResidentKernelError::InvalidInput
        }
    })?;
    Ok(footprint)
}

pub(crate) fn charge_canonical_key_footprint(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<mech_core::snapshot::ValueFootprint, ResidentKernelError> {
    charge_canonical_data_footprint_with(
        meter,
        schema,
        data,
        Some(ResidentBudgetMeter::charge_selector_bytes),
    )
}

/// Measures borrowed canonical data with immediate recursive work/node
/// checks, but without classifying its already-retained bytes as allocation.
pub(crate) fn measure_canonical_data_footprint(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<mech_core::snapshot::ValueFootprint, ResidentKernelError> {
    charge_canonical_data_footprint_with(meter, schema, data, None)
}

fn map_snapshot_work_error(error: SnapshotValueError) -> ResidentKernelError {
    match error {
        SnapshotValueError::CanonicalizationWorkLimitExceededV1 { .. } => {
            ResidentKernelError::InvalidShape
        }
        _ => ResidentKernelError::InvalidInput,
    }
}

/// Measures recursive comparison material without treating the borrowed tree
/// as another live allocation. Each visited chunk is charged before descent
/// continues so planning cannot hide an oversized second traversal.
pub(crate) fn measure_canonical_data_comparison_work(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<u64, ResidentKernelError> {
    let mut total = 0_u64;
    mech_core::snapshot::visit_canonical_data_work(schema, data, |chunk| {
        let work = chunk.encoded_bytes.max(chunk.node_count).max(1);
        meter.charge_comparison_work(work)?;
        total = total
            .checked_add(work)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })
    .map_err(|error| match error {
        mech_core::snapshot::CanonicalDataWorkError::Visitor(error) => error,
        mech_core::snapshot::CanonicalDataWorkError::ArithmeticOverflow => {
            ResidentKernelError::InvalidShape
        }
        mech_core::snapshot::CanonicalDataWorkError::UnknownDynamicSchema
        | mech_core::snapshot::CanonicalDataWorkError::InvalidValue => {
            ResidentKernelError::InvalidInput
        }
    })?;
    Ok(total)
}

/// Proves that recursively rebuilding already-canonical data fits before a
/// caller creates its first owned draft. The planning walk is itself charged
/// incrementally; the returned allowance is the exact work reserved for the
/// later finalizer, not whatever budget happened to remain.
pub(crate) fn preflight_canonical_data_finalization(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<u64, ResidentKernelError> {
    measure_canonical_data_comparison_work(meter, schema, data)?;
    let remaining = meter.estimate().remaining_incremental_work()?;
    let budget = SnapshotCanonicalizationBudget::new(remaining);
    let work = mech_core::snapshot::canonical_data_draft_finalization_work_with_budget(
        schema, data, &budget,
    )
    .map_err(map_snapshot_work_error)?;
    meter.charge_comparison_work(work)?;
    Ok(work)
}

/// Conservative work for the recursive `Value::language_eq` performed before
/// publication. Both payloads have already been measured under the same
/// meter, so this helper adds the cached schema, shape, and complete payload
/// scans without another unbounded planning traversal.
pub(crate) fn projected_language_equality_work(
    schemas: &SchemaTable,
    current: &Value,
    current_footprint: ValueFootprint,
    next_schema: SchemaId,
    next_shape_parameters: usize,
    next_footprint: ValueFootprint,
) -> Result<u64, ResidentKernelError> {
    let current_entry = schemas
        .entry(current.schema())
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next_entry = schemas
        .entry(next_schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema_work = if current_entry.key() == next_entry.key() {
        checked_u64(
            current_entry
                .canonical_bytes()
                .len()
                .max(next_entry.canonical_bytes().len()),
        )?
    } else {
        0
    };
    checked_cost_sum(&[
        schema_work,
        checked_u64(
            current
                .shape()
                .parameter_values()
                .len()
                .max(next_shape_parameters),
        )?,
        current_footprint
            .encoded_bytes
            .max(current_footprint.node_count),
        next_footprint.encoded_bytes.max(next_footprint.node_count),
    ])
}

fn measure_canonical_value_footprint_with(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
    charge_retained_bytes: Option<
        fn(&mut ResidentBudgetMeter, u64) -> Result<(), ResidentKernelError>,
    >,
) -> Result<mech_core::snapshot::ValueFootprint, ResidentKernelError> {
    let schema = schemas
        .get(value.schema())
        .ok_or(ResidentKernelError::InvalidInput)?;
    let shape_parameters = value.shape().parameter_values().len();
    let shape_bytes = checked_cost_product(&[
        checked_u64(shape_parameters)?,
        checked_u64(core::mem::size_of::<u64>())?,
    ])?;
    let wrapper_bytes =
        projected_canonical_value_footprint(ValueFootprint::zero(), shape_parameters)?
            .retained_bytes;
    meter.charge_comparison_work(shape_bytes.max(1))?;
    if let Some(charge_retained_bytes) = charge_retained_bytes {
        charge_retained_bytes(meter, wrapper_bytes)?;
    }
    meter.charge_retained_nodes(1)?;
    let data = charge_canonical_data_footprint_with(
        meter,
        schema.body(),
        value.data(),
        charge_retained_bytes,
    )?;
    projected_canonical_value_footprint(data, shape_parameters)
}

/// Extends a canonical data footprint with the immutable `Value` wrapper and
/// owned shape-parameter storage that the published snapshot retains.
pub(crate) fn projected_canonical_value_footprint(
    data: ValueFootprint,
    shape_parameters: usize,
) -> Result<ValueFootprint, ResidentKernelError> {
    let shape_bytes = checked_cost_product(&[
        checked_u64(shape_parameters)?,
        checked_u64(core::mem::size_of::<u64>())?,
    ])?;
    let wrapper_bytes = checked_u64(core::mem::size_of::<Value>())?
        .checked_add(shape_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    ValueFootprint {
        encoded_bytes: 0,
        retained_bytes: wrapper_bytes,
        node_count: 1,
    }
    .checked_add(data)
    .map_err(|_| ResidentKernelError::InvalidShape)
}

/// Includes the immutable `Value` wrapper and shape storage in the same
/// fail-closed traversal used for its canonical data. Retained bytes are
/// charged as temporary materialization because this entry point authorizes
/// a later clone or canonical draft.
pub(crate) fn charge_canonical_value_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
) -> Result<mech_core::snapshot::ValueFootprint, ResidentKernelError> {
    measure_canonical_value_footprint_with(
        meter,
        value,
        schemas,
        Some(ResidentBudgetMeter::charge_temporary_bytes),
    )
}

/// Measures a borrowed value while bounding each recursive step, without
/// classifying already-retained bytes as newly allocated storage.
pub(crate) fn measure_canonical_value_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
) -> Result<mech_core::snapshot::ValueFootprint, ResidentKernelError> {
    measure_canonical_value_footprint_with(meter, value, schemas, None)
}

impl<P> AdmittedMutationPlan<P> {
    /// Consumes the complete post-state admission immediately before staging.
    pub(crate) fn into_plan(self) -> P {
        self.operation
    }
}

impl ResidentBudgetMeter {
    fn charge(
        &mut self,
        update: impl FnOnce(&mut KernelCostEstimate) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        let mut next = self.accumulated;
        update(&mut next)?;
        next.check_planning_progress()?;
        self.accumulated = next;
        Ok(())
    }

    pub(crate) fn charge_comparison_work(
        &mut self,
        amount: u64,
    ) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.set_comparison_work(
                cost.comparison_work()
                    .checked_add(amount)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            cost.set_compute_work(
                cost.compute_work()
                    .checked_add(amount)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?;
            Ok(())
        })
    }

    pub(crate) fn charge_compute_work(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.set_compute_work(
                cost.compute_work()
                    .checked_add(amount)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
        })
    }

    pub(crate) fn charge_temporary_bytes(
        &mut self,
        amount: u64,
    ) -> Result<(), ResidentKernelError> {
        self.charge(|cost| cost.add_temporary_bytes(amount))
    }

    pub(crate) fn charge_cloned_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.set_cloned_bytes(
                cost.cloned_bytes()
                    .checked_add(amount)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            );
            Ok(())
        })
    }

    pub(crate) fn charge_retained_nodes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.set_retained_nodes(
                cost.retained_nodes()
                    .checked_add(amount)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            );
            Ok(())
        })
    }

    pub(crate) fn charge_selector_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| cost.add_temporary_bytes(amount))
    }

    pub(crate) fn estimate(self) -> KernelCostEstimate {
        self.accumulated
    }
}

pub(crate) fn checked_u64<T>(value: T) -> Result<u64, ResidentKernelError>
where
    T: TryInto<u64>,
{
    value
        .try_into()
        .map_err(|_| ResidentKernelError::InvalidShape)
}

pub(crate) fn checked_product(values: &[usize]) -> Result<usize, ResidentKernelError> {
    values.iter().try_fold(1usize, |product, value| {
        product
            .checked_mul(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

pub(crate) fn checked_sum(values: &[usize]) -> Result<usize, ResidentKernelError> {
    values.iter().try_fold(0usize, |sum, value| {
        sum.checked_add(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

pub(crate) fn checked_cost_product(values: &[u64]) -> Result<u64, ResidentKernelError> {
    values.iter().try_fold(1u64, |product, value| {
        product
            .checked_mul(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

pub(crate) fn checked_cost_sum(values: &[u64]) -> Result<u64, ResidentKernelError> {
    values.iter().try_fold(0u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_turn_authority_restores_previous_plan_after_panic() {
        let outer = Arc::new(detached_turn_plan(ResourceDemand::default()).unwrap());
        let inner = Arc::new(detached_turn_plan(ResourceDemand::default()).unwrap());
        assert!(with_active_turn_plan(|plan| plan.is_none()));
        with_resident_turn_plan(outer.clone(), || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                with_resident_turn_plan(inner.clone(), || {
                    assert!(with_active_turn_plan(|plan| Arc::ptr_eq(
                        plan.as_ref().unwrap(),
                        &inner
                    )));
                    panic!("injected nested kernel failure");
                });
            }));
            assert!(result.is_err());
            assert!(with_active_turn_plan(|plan| Arc::ptr_eq(
                plan.as_ref().unwrap(),
                &outer
            )));
        });
        assert!(with_active_turn_plan(|plan| plan.is_none()));
        assert_eq!(Arc::strong_count(&outer), 1);
        assert_eq!(Arc::strong_count(&inner), 1);
    }

    #[test]
    fn standalone_control_admission_accumulates_repeated_work() {
        let cost = KernelCostEstimate {
            demand: ResourceDemand {
                work: mech_core::WorkDemand {
                    compute: MAX_RESIDENT_COMPUTE_WORK / 2 + 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        };
        let plan = Arc::new(detached_turn_plan(ResourceDemand::default()).unwrap());
        with_resident_turn_plan(plan, || {
            with_control_work_budget(|| {
                assert!(PreparedKernel::new((), cost).admit_control().is_ok());
                assert!(matches!(
                    PreparedKernel::new((), cost).admit_control(),
                    Err(ResidentKernelError::InvalidShape),
                ));
            });
        });
    }

    #[test]
    fn standalone_control_admission_accumulates_repeated_comparisons() {
        let cost = KernelCostEstimate {
            demand: ResourceDemand {
                work: mech_core::WorkDemand {
                    comparison: MAX_RESIDENT_COMPARISON_WORK / 2 + 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        };
        let plan = Arc::new(detached_turn_plan(ResourceDemand::default()).unwrap());
        with_resident_turn_plan(plan, || {
            with_control_work_budget(|| {
                assert!(PreparedKernel::new((), cost).admit_control().is_ok());
                assert!(matches!(
                    PreparedKernel::new((), cost).admit_control(),
                    Err(ResidentKernelError::InvalidShape),
                ));
            });
        });
    }

    #[test]
    fn control_admission_reconciles_with_the_real_node_turn_plan() {
        let node = mech_core::NodeId::new(7);
        let program = crate::memory_planner::ProgramMemoryPlan {
            allocations: vec![mech_core::AllocationPlan {
                id: mech_core::MemoryObjectId::new(3),
                owner: mech_core::MemoryObjectOwner::NodeScratch { node, ordinal: 0 },
                role: mech_core::AllocationRole::TransactionStage,
                slot: None,
                space: mech_core::MemorySpace::ResidentCpu,
                current_bytes: 8,
                capacity_bytes: 8,
                payload_block_capacity: 0,
                alignment: 8,
                lifetime: mech_core::MemoryLifetime::Transaction {
                    first: mech_core::MemoryPlanPoint::new(14),
                    last: mech_core::MemoryPlanPoint::new(15),
                },
                placement: mech_core::ArenaPlacement {
                    arena: mech_core::MemoryArenaId::new(0),
                    offset: 0,
                },
                reuse_group: None,
            }]
            .into_boxed_slice(),
            budget_limits: mech_core::TargetMemoryProfile::current_resident_cpu()
                .unwrap()
                .limits,
            ..crate::memory_planner::ProgramMemoryPlan::default()
        };
        let base = crate::memory_planner::plan_turn_memory(
            &program,
            node,
            &crate::memory_planner::TurnMemoryFacts::default(),
        )
        .unwrap();
        let checked = with_resident_turn_plan(base.clone(), || {
            with_control_work_budget(|| {
                PreparedKernel::new(
                    (),
                    KernelCostEstimate {
                        demand: ResourceDemand {
                            persistent_bytes: 24,
                            ..ResourceDemand::default()
                        },
                    },
                )
                .admit_control()
            })
        })
        .unwrap();
        assert_eq!(checked._permit._plan.node, node);
        assert_eq!(
            checked._permit._plan.allocations[0].id,
            mech_core::MemoryObjectId::new(3)
        );
        assert_eq!(
            checked._permit._plan.allocations[0].owner,
            mech_core::MemoryObjectOwner::NodeScratch { node, ordinal: 0 }
        );
        assert_eq!(checked._permit._plan.allocations[0].capacity_bytes, 24);
        assert_eq!(checked._permit._plan.arenas[0].capacity_bytes, 24);

        let mut bounded = base;
        bounded.budget_limits.max_output_bytes = Some(23);
        assert!(matches!(
            with_resident_turn_plan(bounded, || {
                with_control_work_budget(|| {
                    PreparedKernel::new(
                        (),
                        KernelCostEstimate {
                            demand: ResourceDemand {
                                persistent_bytes: 24,
                                ..ResourceDemand::default()
                            },
                        },
                    )
                    .admit_control()
                })
            }),
            Err(ResidentKernelError::InvalidShape)
        ));
    }

    #[test]
    fn incremental_progress_checks_match_full_admission_without_rebuilding_the_plan() {
        let mut base = detached_turn_plan(ResourceDemand::default()).unwrap();
        base.facts.additional_demand.work.compute = 17;
        let base = Arc::new(base);
        let snapshot = (*base).clone();
        for amount in [
            0,
            1,
            MAX_RESIDENT_COMPARISON_WORK,
            MAX_RESIDENT_COMPARISON_WORK + 1,
        ] {
            for demand in [
                ResourceDemand {
                    work: mech_core::WorkDemand {
                        comparison: amount,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ResourceDemand {
                    retained_nodes: amount,
                    ..Default::default()
                },
            ] {
                let full = apply_observed_turn_demand((*base).clone(), demand, None).unwrap();
                let progress = with_resident_turn_plan(base.clone(), || {
                    KernelCostEstimate { demand }.check_planning_progress()
                });
                assert_eq!(progress.is_ok(), full.budget_violations.is_empty());
            }
        }
        for amount in [
            MAX_RESIDENT_TEMPORARY_BYTES,
            MAX_RESIDENT_TEMPORARY_BYTES + 1,
        ] {
            let demand = ResourceDemand {
                turn_peak_bytes: amount,
                ..Default::default()
            };
            let full = apply_observed_turn_demand((*base).clone(), demand, None).unwrap();
            let progress = with_resident_turn_plan(base.clone(), || {
                KernelCostEstimate { demand }.check_planning_progress()
            });
            assert_eq!(progress.is_ok(), full.budget_violations.is_empty());
        }
        assert_eq!(
            *base, snapshot,
            "progress must not mutate cached layouts or facts"
        );
        assert_eq!(
            Arc::strong_count(&base),
            1,
            "a completed scope must release its plan"
        );
    }

    #[test]
    fn incremental_progress_cannot_admit_candidate_publication_facts() {
        let base = detached_turn_plan(ResourceDemand::default()).unwrap();
        with_resident_turn_plan(base, || {
            assert_eq!(
                KernelCostEstimate {
                    demand: ResourceDemand {
                        persistent_bytes: 8,
                        output_elements: 1,
                        ..Default::default()
                    }
                }
                .check_planning_progress(),
                Err(ResidentKernelError::InvalidShape)
            );
        });
    }

    #[test]
    fn resident_budget_fails_closed_on_limits_and_arithmetic() {
        assert!(KernelCostEstimate::default().admit().is_ok());
        assert_eq!(
            KernelCostEstimate {
                demand: ResourceDemand {
                    cloned_bytes: MAX_RESIDENT_CLONED_BYTES + 1,
                    ..ResourceDemand::default()
                },
            }
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape
        );
        assert_eq!(
            checked_product(&[usize::MAX, 2]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_sum(&[usize::MAX, 1]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_cost_product(&[u64::MAX, 2]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_cost_sum(&[u64::MAX, 1]),
            Err(ResidentKernelError::InvalidShape)
        );
    }

    #[test]
    fn permit_requires_one_complete_peak_estimate() {
        let prepared = PreparedKernel::new(
            41_u64,
            KernelCostEstimate {
                demand: ResourceDemand {
                    turn_peak_bytes: MAX_RESIDENT_TEMPORARY_BYTES,
                    ..ResourceDemand::default()
                },
            },
        );
        assert_eq!(prepared.admit().unwrap().into_plan(), 41);
        assert_eq!(
            PreparedKernel::new(
                (),
                KernelCostEstimate {
                    demand: ResourceDemand {
                        turn_peak_bytes: MAX_RESIDENT_TEMPORARY_BYTES + 1,
                        ..ResourceDemand::default()
                    },
                },
            )
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape
        );
        assert_eq!(
            PreparedKernel::new(
                (),
                KernelCostEstimate {
                    demand: ResourceDemand {
                        turn_peak_bytes: u64::MAX,
                        ..ResourceDemand::default()
                    },
                },
            )
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape
        );
    }

    #[test]
    fn incremental_meter_rejects_the_first_over_limit_charge() {
        let mut meter = ResidentBudgetMeter::default();
        meter
            .charge_comparison_work(MAX_RESIDENT_COMPARISON_WORK)
            .unwrap();
        assert_eq!(
            meter.charge_comparison_work(1),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(
            meter.estimate().comparison_work(),
            MAX_RESIDENT_COMPARISON_WORK,
        );
    }

    #[test]
    fn incremental_allowance_respects_comparison_and_compute_remainders() {
        assert_eq!(
            KernelCostEstimate {
                demand: ResourceDemand {
                    work: mech_core::WorkDemand {
                        comparison: MAX_RESIDENT_COMPARISON_WORK - 7,
                        compute: MAX_RESIDENT_COMPUTE_WORK - 9,
                        ..mech_core::WorkDemand::default()
                    },
                    ..ResourceDemand::default()
                },
            }
            .remaining_incremental_work()
            .unwrap(),
            7,
        );
        assert_eq!(
            KernelCostEstimate {
                demand: ResourceDemand {
                    work: mech_core::WorkDemand {
                        compute: MAX_RESIDENT_COMPUTE_WORK - 3,
                        ..mech_core::WorkDemand::default()
                    },
                    ..ResourceDemand::default()
                },
            }
            .remaining_incremental_work()
            .unwrap(),
            3,
        );
    }

    #[test]
    fn mutation_node_admission_sums_simultaneously_live_phases() {
        let phases = MutationRetainedNodeFootprint {
            current_persistent: 20_000,
            normalized_plan: 5_000,
            temporary_draft: 20_000,
        };
        let exact = PreparedMutationPlan::new(
            7_u64,
            PublishedOutputFootprint {
                elements: 1,
                retained_bytes: 1,
                retained_nodes: 20_536,
            },
            phases,
            KernelCostEstimate::default(),
        )
        .unwrap()
        .admit()
        .unwrap()
        .into_plan();
        assert_eq!(exact, 7);

        assert_eq!(
            PreparedMutationPlan::new(
                0_u8,
                PublishedOutputFootprint {
                    elements: 1,
                    retained_bytes: 1,
                    retained_nodes: 20_537,
                },
                phases,
                KernelCostEstimate::default(),
            )
            .unwrap()
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape,
        );
        assert_eq!(
            PreparedMutationPlan::new(
                0_u8,
                PublishedOutputFootprint {
                    elements: 1,
                    retained_bytes: 1,
                    retained_nodes: u64::MAX,
                },
                phases,
                KernelCostEstimate::default(),
            )
            .unwrap_err(),
            ResidentKernelError::InvalidShape,
        );
        assert_eq!(
            PreparedMutationPlan::new(
                0_u8,
                PublishedOutputFootprint {
                    elements: 1,
                    retained_bytes: 1,
                    retained_nodes: 1,
                },
                MutationRetainedNodeFootprint::default(),
                KernelCostEstimate {
                    demand: ResourceDemand {
                        retained_nodes: 1,
                        ..ResourceDemand::default()
                    },
                },
            )
            .unwrap_err(),
            ResidentKernelError::InvalidShape,
        );
    }
}
