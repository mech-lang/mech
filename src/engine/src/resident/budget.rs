use std::sync::{Arc, Mutex};

use mech_core::snapshot::{
    SequenceView, SnapshotCanonicalizationBudget, SnapshotValueError, ValueFootprint,
};
use mech_core::{
    MemoryBudgetDimension, ResidentKernelError, ResidentValueKind, ResourceDemand, SchemaBody,
    SchemaId, SchemaTable, Value, ValueData,
};

use crate::memory_planner::{
    TurnMemoryPlan, apply_observed_turn_demand, check_turn_planning_progress,
    try_admit_fixed_turn_memory,
};

#[cfg(test)]
#[path = "borrowed_metadata_scan_tests.rs"]
mod borrowed_metadata_scan_tests;
#[cfg(all(
    feature = "native-artifact",
    any(feature = "pure_text", feature = "map_ops")
))]
#[path = "native_turn_plan_cache.rs"]
pub(crate) mod native_turn_plan_cache;
#[path = "payload_budget.rs"]
pub(crate) mod payload;
#[cfg(feature = "native-artifact")]
#[path = "native_worker_budget.rs"]
mod workers;
#[cfg(feature = "native-artifact")]
pub use workers::{NativeWorkDebitBatch, NativeWorkGroup, NativeWorkerBudgetError, NativeWorkerWorkLoan};

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

pub(super) fn projected_dense_string_payload(
    value: &Value,
    kind: ResidentValueKind,
) -> Result<usize, ResidentKernelError> {
    match (kind, value.data()) {
        (ResidentValueKind::Bool, ValueData::Bool(value)) => Ok(if *value { 4 } else { 5 }),
        (ResidentValueKind::Bool, ValueData::Matrix(matrix)) => {
            let SequenceView::Bool(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(if *value { 4 } else { 5 })
                    .ok_or(ResidentKernelError::InvalidShape)
            })
        }
        (ResidentValueKind::Index, ValueData::Index(value)) => displayed_bytes(value),
        (ResidentValueKind::Index, ValueData::Matrix(matrix)) => {
            let SequenceView::Index(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            projected_display_sequence(values)
        }
        (ResidentValueKind::F64, ValueData::F64(value)) => displayed_bytes(value.to_f64()),
        (ResidentValueKind::F64, ValueData::Matrix(matrix)) => {
            let SequenceView::F64(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(displayed_bytes(value.to_f64())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            })
        }
        (ResidentValueKind::String, ValueData::String(value)) => Ok(value.len()),
        (ResidentValueKind::String, ValueData::Matrix(matrix)) => {
            let SequenceView::String(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(value.len())
                    .ok_or(ResidentKernelError::InvalidShape)
            })
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

thread_local! {
    static ACTIVE_TURN_PLAN: Mutex<Option<Arc<TurnMemoryPlan>>> = const { Mutex::new(None) };
    static ACTIVE_PAYLOAD_ADMISSION: std::cell::RefCell<Option<std::rc::Rc<payload::ResidentPayloadAdmission>>> = const { std::cell::RefCell::new(None) };
}

// Repeated control execution accumulates the costs admitted by the existing
// kernel budget authority. Each invocation contributes its maximum observed
// demand once; incremental estimates within that invocation are not summed.
#[derive(Clone, Copy, Debug, Default)]
struct ControlWork {
    compute: u64,
    comparison: u64,
}

#[derive(Debug)]
struct ControlWorkAccount {
    identity: u64,
    used: ControlWork,
    reserved: ControlWork,
    compute_limit: u64,
    comparison_limit: u64,
    _reservation: Option<mech_core::ManagedMemoryReservation>,
    #[cfg(feature = "native-artifact")]
    worker: Option<NativeWorkerWorkLoan>,
}
static NEXT_CONTROL_WORK_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn next_control_work_identity() -> u64 {
    NEXT_CONTROL_WORK_ID
        .fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |current| current.checked_add(1),
        )
        .expect("control work identity exhausted")
}
impl Default for ControlWorkAccount {
    fn default() -> Self {
        Self {
            identity: next_control_work_identity(),
            used: ControlWork::default(),
            reserved: ControlWork::default(),
            compute_limit: mech_core::RESIDENT_MAX_COMPUTE_WORK,
            comparison_limit: mech_core::RESIDENT_MAX_COMPARISON_WORK,
            _reservation: None,
            #[cfg(feature = "native-artifact")]
            worker: None,
        }
    }
}

/// A shared library-work account for one native invocation, not permission to
/// execute a call. Actual NativeHost call plans and payload admission remain
/// mandatory. Clones retain the same spent work and can never renew limits.
#[cfg(feature = "native-artifact")]
#[derive(Clone, Debug)]
pub struct NativeLibraryWorkBudget {
    account: std::rc::Rc<std::cell::RefCell<ControlWorkAccount>>,
}
#[cfg(feature = "native-artifact")]
impl NativeLibraryWorkBudget {
    pub fn new(
        compute_limit: u64,
        comparison_limit: u64,
        memory: &mech_core::ManagedMemoryBudget,
    ) -> mech_core::MemoryRuntimeResult<Self> {
        let bytes = (core::mem::size_of::<std::cell::RefCell<ControlWorkAccount>>()
            + 2 * core::mem::size_of::<usize>()) as u64;
        let reservation = memory.reserve_capacity(bytes)?;
        Ok(Self {
            account: std::rc::Rc::new(std::cell::RefCell::new(ControlWorkAccount {
                identity: next_control_work_identity(),
                used: ControlWork::default(),
                reserved: ControlWork::default(),
                compute_limit,
                comparison_limit,
                _reservation: Some(reservation),
                worker: None,
            })),
        })
    }

    /// Identity check for invocation-owned prepared metadata. Equal
    /// limits or an equal memory account cannot renew a different work ledger.
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub fn shares_account(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.account, &other.account)
    }

    /// These are observations of this exact account, not caller-supplied cost
    /// certificates. The native owner reconciles its other work against them.
    pub fn used(&self) -> (u64, u64) {
        let state = self.account.borrow();
        (state.used.compute, state.used.comparison)
    }

    /// Observe finite fuel for synchronous, callback-free generated code.
    /// Nested maintained-call scopes must use their ordinary accounting path.
    /// This neither reserves nor renews work: the owner must reconcile actual
    /// debits to this SAME account before any helper or publication can occur.
    pub fn unscoped_remaining(&self) -> Option<(u64, u64)> {
        if CONTROL_WORK.with(|v| v.borrow().is_some())
            || CONTROL_WORK_SCOPE.with(|v| v.borrow().is_some())
        {
            return None;
        }
        let root = CONTROL_ROOT_LIMITS
            .with(|v| v.get())
            .unwrap_or((u64::MAX, u64::MAX));
        let state = self.account.borrow();
        let local = (
            state
                .compute_limit
                .min(root.0)
                .checked_sub(state.used.compute)?.checked_sub(state.reserved.compute)?,
            state
                .comparison_limit
                .min(root.1)
                .checked_sub(state.used.comparison)?.checked_sub(state.reserved.comparison)?,
        );
        if let Some(worker) = &state.worker {
            let shared = worker.remaining()?;
            Some((local.0.min(shared.0), local.1.min(shared.1)))
        } else {
            Some(local)
        }
    }

    /// Identity of the account that admitted this ledger's actual backing.
    /// A mixed invocation must keep its values and work metadata together.
    pub fn memory_budget(&self) -> mech_core::ManagedMemoryBudget {
        self.account
            .borrow()
            ._reservation
            .as_ref()
            .expect("native work metadata has an admitted owner")
            .budget()
    }

    /// An independent native meter must not bypass an active invocation.
    /// Callers must reuse that invocation's account or reject nested entry.
    pub fn has_active_scope() -> bool {
        CONTROL_WORK.with(|active| active.borrow().is_some())
    }

    /// A reusable measured summary requires a real paid native scope, not
    /// merely an account installed beside an absent or foreign observer.
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    pub(in crate::resident) fn has_matching_active_scope(&self) -> bool {
        CONTROL_WORK.with(|active| {
            active
                .borrow()
                .as_ref()
                .is_some_and(|current| std::rc::Rc::ptr_eq(current, &self.account))
        }) && CONTROL_WORK_SCOPE.with(|active| {
            active
                .borrow()
                .as_ref()
                .is_some_and(|scope| std::rc::Rc::ptr_eq(&scope.total, &self.account))
        })
    }

    /// Generated scalar/control work and maintained leaf work debit this same
    /// account. This method never replaces the current invocation's account.
    pub fn charge(&self, compute: u64, comparison: u64) -> Result<(), ResidentKernelError> {
        CONTROL_WORK.with(|active| {
            if active
                .borrow()
                .as_ref()
                .is_some_and(|current| !std::rc::Rc::ptr_eq(current, &self.account))
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            Ok(())
        })?;
        charge_control_account(&mut self.account.borrow_mut(), compute, comparison)
    }

    /// Preview the same account's debit before optional backing is selected.
    /// A declined storage preference must neither spend work nor renew fuel.
    #[cfg(all(
        unix,
        feature = "native-artifact",
        feature = "native-capture-pages",
        feature = "pure_text"
    ))]
    pub(in crate::resident) fn preview_allocation_charge(
        &self,
        compute: u64,
    ) -> Result<(), ResidentKernelError> {
        CONTROL_WORK.with(|active| {
            if active
                .borrow()
                .as_ref()
                .is_some_and(|current| !std::rc::Rc::ptr_eq(current, &self.account))
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            Ok(())
        })?;
        checked_control_account_work(&self.account.borrow(), compute, 0).map(drop)
    }

    /// Other native work may only reduce the allowance left to library calls.
    /// A failed restriction leaves both limits and spent work unchanged.
    pub fn restrict(
        &self,
        compute_limit: u64,
        comparison_limit: u64,
    ) -> Result<(), ResidentKernelError> {
        self.restrict_and_charge(compute_limit, comparison_limit, 0, 0)
    }

    /// Commit a restriction and its setup debit together. A denial must not
    /// narrow the allowance of handles the caller already holds.
    pub fn restrict_and_charge(
        &self,
        compute_limit: u64,
        comparison_limit: u64,
        compute: u64,
        comparison: u64,
    ) -> Result<(), ResidentKernelError> {
        CONTROL_WORK.with(|active| {
            if active
                .borrow()
                .as_ref()
                .is_some_and(|current| !std::rc::Rc::ptr_eq(current, &self.account))
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            Ok(())
        })?;
        let mut state = self.account.borrow_mut();
        if compute_limit > state.compute_limit || comparison_limit > state.comparison_limit {
            return Err(ResidentKernelError::InvalidInput);
        }
        for (required, limit, dimension) in [
            (
                state
                    .used
                    .compute
                    .checked_add(compute)
                    .and_then(|used| used.checked_add(state.reserved.compute))
                    .ok_or(ResidentKernelError::InvalidShape)?,
                compute_limit,
                MemoryBudgetDimension::ComputeWork,
            ),
            (
                state
                    .used
                    .comparison
                    .checked_add(comparison)
                    .and_then(|used| used.checked_add(state.reserved.comparison))
                    .ok_or(ResidentKernelError::InvalidShape)?,
                comparison_limit,
                MemoryBudgetDimension::ComparisonWork,
            ),
        ] {
            if required > limit {
                return Err(ResidentKernelError::ResourceLimit {
                    phase: mech_core::ResidentResourcePhase::Admission,
                    dimension,
                    required,
                    limit,
                });
            }
        }
        // This also applies the enclosing call/Resident caps before either
        // the used counters or the account's limits can change.
        charge_control_account(&mut state, compute, comparison)?;
        state.compute_limit = compute_limit;
        state.comparison_limit = comparison_limit;
        Ok(())
    }

    /// Nested native/resident leaf execution must inherit the same account.
    /// Replacing a live caller's account, even with equal limits, is rejected.
    pub fn with_scope<T>(&self, execute: impl FnOnce() -> T) -> Result<T, ResidentKernelError> {
        struct Guard(Option<std::rc::Rc<std::cell::RefCell<ControlWorkAccount>>>);
        impl Drop for Guard {
            fn drop(&mut self) {
                CONTROL_WORK.with(|active| *active.borrow_mut() = self.0.take());
            }
        }
        let previous = CONTROL_WORK.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .as_ref()
                .is_some_and(|current| !std::rc::Rc::ptr_eq(current, &self.account))
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            Ok(active.replace(self.account.clone()))
        })?;
        let _guard = Guard(previous);
        let _scope = ControlWorkScopeGuard::enter();
        Ok(execute())
    }

    /// A call may impose a tighter relative allowance on this same account.
    /// Leaving a call restores its enclosing cap, never spent work. Nested
    /// scopes cannot exceed either the enclosing call or invocation allowance.
    pub fn with_call_limits<T>(
        &self,
        compute: u64,
        comparison: u64,
        execute: impl FnOnce() -> T,
    ) -> Result<T, ResidentKernelError> {
        use std::cell::{Cell, RefCell};
        use std::rc::Rc;
        // Retain each thread-local address across this synchronous call. This
        // is the same account, fresh observer and relative cap as with_scope
        // followed by a call cap; teardown need not look up those addresses
        // again. Guards still restore all three in reverse order on unwind.
        struct AccountGuard<'a>(&'a RefCell<Option<Rc<RefCell<ControlWorkAccount>>>>,
            Option<Rc<RefCell<ControlWorkAccount>>>);
        impl Drop for AccountGuard<'_> {
            fn drop(&mut self) { *self.0.borrow_mut() = self.1.take(); }
        }
        struct ScopeGuard<'a>(&'a RefCell<Option<ControlWorkScope>>, Option<ControlWorkScope>);
        impl Drop for ScopeGuard<'_> {
            fn drop(&mut self) { *self.0.borrow_mut() = self.1.take(); }
        }
        struct LimitGuard<'a>(&'a Cell<Option<(u64, u64)>>, Option<(u64, u64)>);
        impl Drop for LimitGuard<'_> {
            fn drop(&mut self) { self.0.set(self.1); }
        }
        CONTROL_WORK.with(|active| {
            let previous = {
                let mut current = active.borrow_mut();
                if current.as_ref().is_some_and(|account| !Rc::ptr_eq(account, &self.account)) {
                    return Err(ResidentKernelError::InvalidInput);
                }
                current.replace(self.account.clone())
            };
            let _account = AccountGuard(active, previous);
            CONTROL_WORK_SCOPE.with(|scope| {
                let previous = scope.replace(Some(ControlWorkScope {
                    total: self.account.clone(), observed: ControlWork::default(),
                }));
                let _scope = ScopeGuard(scope, previous);
                let used = self.used();
                CONTROL_ROOT_LIMITS.with(|limits| {
                    let previous = limits.get();
                    let enclosing = previous.unwrap_or((u64::MAX, u64::MAX));
                    limits.set(Some((
                        used.0.saturating_add(compute).min(enclosing.0),
                        used.1.saturating_add(comparison).min(enclosing.1),
                    )));
                    let _limits = LimitGuard(limits, previous);
                    Ok(execute())
                })
            })
        })
    }
}

struct ControlWorkScope {
    total: std::rc::Rc<std::cell::RefCell<ControlWorkAccount>>,
    observed: ControlWork,
}

thread_local! {
    static CONTROL_WORK: std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<ControlWorkAccount>>>> = const { std::cell::RefCell::new(None) };
    static CONTROL_WORK_SCOPE: std::cell::RefCell<Option<ControlWorkScope>> = const { std::cell::RefCell::new(None) };
    static CONTROL_ROOT_LIMITS: std::cell::Cell<Option<(u64, u64)>> = const { std::cell::Cell::new(None) };
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
    struct Guard {
        previous: Option<std::rc::Rc<std::cell::RefCell<ControlWorkAccount>>>,
        limits: Option<(u64, u64)>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            CONTROL_WORK.with(|active| *active.borrow_mut() = self.previous.take());
            CONTROL_ROOT_LIMITS.with(|active| active.set(self.limits));
        }
    }
    let previous = CONTROL_WORK.with(|active| {
        let mut active = active.borrow_mut();
        let total = active.clone().unwrap_or_default();
        active.replace(total)
    });
    // Native callers cannot lift a resident program's original whole-turn
    // caps. Nested work inherits the tighter limits without a fresh account.
    let limits = CONTROL_ROOT_LIMITS.with(|active| {
        let previous = active.get();
        let (compute, comparison) = previous.unwrap_or((u64::MAX, u64::MAX));
        active.set(Some((
            compute.min(mech_core::RESIDENT_MAX_COMPUTE_WORK),
            comparison.min(mech_core::RESIDENT_MAX_COMPARISON_WORK),
        )));
        previous
    });
    let _guard = Guard { previous, limits };
    execute()
}

pub(super) struct ControlWorkScopeGuard(Option<ControlWorkScope>);

impl ControlWorkScopeGuard {
    pub(super) fn enter() -> Self {
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

fn checked_local_control_account_work(
    account: &ControlWorkAccount,
    compute: u64,
    comparison: u64,
) -> Result<ControlWork, ResidentKernelError> {
    let (root_compute, root_comparison) = CONTROL_ROOT_LIMITS
        .with(|active| active.get())
        .unwrap_or((u64::MAX, u64::MAX));
    let compute_limit = account.compute_limit.min(root_compute);
    let comparison_limit = account.comparison_limit.min(root_comparison);
    let compute = account
        .used
        .compute
        .checked_add(compute)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let comparison = account
        .used
        .comparison
        .checked_add(comparison)
        .ok_or(ResidentKernelError::InvalidShape)?;
    for (required, limit, dimension) in [
        (compute.checked_add(account.reserved.compute).ok_or(ResidentKernelError::InvalidShape)?,
            compute_limit, MemoryBudgetDimension::ComputeWork),
        (
            comparison.checked_add(account.reserved.comparison).ok_or(ResidentKernelError::InvalidShape)?,
            comparison_limit,
            MemoryBudgetDimension::ComparisonWork,
        ),
    ] {
        if required > limit {
            return Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension,
                required,
                limit,
            });
        }
    }
    Ok(ControlWork {
        compute,
        comparison,
    })
}

fn checked_control_account_work(
    account: &ControlWorkAccount,
    compute: u64,
    comparison: u64,
) -> Result<ControlWork, ResidentKernelError> {
    let next = checked_local_control_account_work(account, compute, comparison)?;
    #[cfg(feature = "native-artifact")]
    if let Some(worker) = &account.worker {
        worker.preview(compute, comparison)?;
    }
    Ok(next)
}

fn charge_control_account(
    account: &mut ControlWorkAccount,
    compute: u64,
    comparison: u64,
) -> Result<(), ResidentKernelError> {
    // A real debit checks and commits both shared dimensions under one lock.
    // Eligibility-only callers retain the separate, non-spending preview.
    let next = checked_local_control_account_work(account, compute, comparison)?;
    #[cfg(feature = "native-artifact")]
    if let Some(worker) = &account.worker {
        // Local call caps were checked first. Both shared dimensions commit
        // together before the infallible local-counter assignment.
        worker.charge(compute, comparison)?;
    }
    account.used = next;
    Ok(())
}

// Eligibility only: a failed whole-scan preview must not spend work or change
// the observer high-water mark before the original incremental path runs.
pub(in crate::resident) fn preview_control_work(
    demand: ResourceDemand,
) -> Result<(), ResidentKernelError> {
    CONTROL_WORK_SCOPE.with(|active| {
        let active = active.borrow();
        let Some(scope) = active.as_ref() else {
            return Ok(());
        };
        checked_control_account_work(
            &scope.total.borrow(),
            demand.work.compute.saturating_sub(scope.observed.compute),
            demand
                .work
                .comparison
                .saturating_sub(scope.observed.comparison),
        )
        .map(drop)
    })
}

pub(crate) fn observe_control_work(demand: ResourceDemand) -> Result<(), ResidentKernelError> {
    CONTROL_WORK_SCOPE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(scope) = active.as_mut() else {
            return Ok(());
        };
        let mut account = scope.total.borrow_mut();
        charge_control_account(
            &mut account,
            demand.work.compute.saturating_sub(scope.observed.compute),
            demand
                .work
                .comparison
                .saturating_sub(scope.observed.comparison),
        )?;
        scope.observed.compute = scope.observed.compute.max(demand.work.compute);
        scope.observed.comparison = scope.observed.comparison.max(demand.work.comparison);
        Ok(())
    })
}

#[cfg(all(test, feature = "native-artifact"))]
mod native_library_work_tests {
    use super::*;

    fn demand(compute: u64, comparison: u64) -> ResourceDemand {
        let mut demand = ResourceDemand::default();
        demand.work.compute = compute;
        demand.work.comparison = comparison;
        demand
    }

    #[test]
    fn native_scalar_debits_and_library_observations_use_one_counter() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        budget.charge(2, 1).unwrap();
        budget
            .with_scope(|| observe_control_work(demand(5, 2)))
            .unwrap()
            .unwrap();
        assert_eq!(budget.used(), (7, 3));
        assert!(budget.charge(2, 0).is_err());
        assert_eq!(budget.used(), (7, 3));
        budget.with_scope(|| budget.charge(1, 1)).unwrap().unwrap();
        assert_eq!(budget.used(), (8, 4));
        let fresh = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        assert_eq!(
            budget.with_scope(|| fresh.charge(1, 0)).unwrap(),
            Err(ResidentKernelError::InvalidInput)
        );
        assert_eq!(fresh.used(), (0, 0));
    }

    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    #[test]
    fn reusable_measurement_requires_matching_account_and_paid_observer_scope() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        let other = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        let cloned = budget.clone();
        assert!(!budget.has_matching_active_scope());
        budget
            .with_scope(|| {
                assert!(budget.has_matching_active_scope());
                assert!(cloned.has_matching_active_scope());
                assert!(!other.has_matching_active_scope());
                {
                    let _restore = ControlWorkScopeGuard(
                        CONTROL_WORK_SCOPE.with(|active| active.replace(None)),
                    );
                    assert!(!budget.has_matching_active_scope());
                    // Observing no scope is deliberately a no-op elsewhere;
                    // that success must never authenticate a cached summary.
                    observe_control_work(demand(100, 0)).unwrap();
                    assert_eq!(budget.used(), (0, 0));
                }
                {
                    let _restore = ControlWorkScopeGuard(CONTROL_WORK_SCOPE.with(|active| {
                        active.replace(Some(ControlWorkScope {
                            total: other.account.clone(),
                            observed: ControlWork::default(),
                        }))
                    }));
                    assert!(!budget.has_matching_active_scope());
                    assert!(!other.has_matching_active_scope());
                }
                assert!(budget.has_matching_active_scope());
            })
            .unwrap();
        assert!(!budget.has_matching_active_scope());
        assert_eq!(budget.used(), (0, 0));
        assert_eq!(other.used(), (0, 0));
    }

    #[test]
    fn native_per_call_caps_share_root_work_and_never_refill_the_invocation() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(10, 4, &memory).unwrap();
        budget.charge(2, 0).unwrap();
        budget
            .with_call_limits(3, 1, || {
                budget.charge(2, 1).unwrap();
                assert!(budget.charge(2, 0).is_err());
                budget
                    .with_call_limits(9, 9, || {
                        assert!(budget.charge(2, 0).is_err());
                        budget.charge(1, 0).unwrap();
                    })
                    .unwrap();
            })
            .unwrap();
        assert_eq!(budget.used(), (5, 1));
        budget
            .with_call_limits(3, 1, || budget.charge(3, 1))
            .unwrap()
            .unwrap();
        assert_eq!(budget.used(), (8, 2));
        budget
            .with_call_limits(3, 1, || {
                assert!(budget.charge(3, 0).is_err());
                budget.charge(2, 1).unwrap();
            })
            .unwrap();
        assert_eq!(budget.used(), (10, 3));
        assert!(budget.charge(1, 0).is_err());
    }

    #[test]
    #[cfg(panic = "unwind")]
    fn native_call_unwind_restores_parent_observations_caps_and_account() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(100, 100, &memory).unwrap();
        let foreign = NativeLibraryWorkBudget::new(100, 100, &memory).unwrap();
        budget.with_call_limits(30, 10, || {
            observe_control_work(demand(3, 1)).unwrap();
            let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                budget.with_call_limits(5, 2, || {
                    observe_control_work(demand(4, 1)).unwrap();
                    panic!("owned test call unwind");
                }).unwrap();
            }));
            assert!(failed.is_err());
            assert_eq!(budget.used(), (7, 2));
            // The parent's existing high-water mark survives the inner call.
            observe_control_work(demand(3, 1)).unwrap();
            assert_eq!(budget.used(), (7, 2));
            observe_control_work(demand(4, 2)).unwrap();
            assert_eq!(budget.used(), (8, 3));
            assert_eq!(foreign.with_call_limits::<()>(100, 100, || unreachable!()),
                Err(ResidentKernelError::InvalidInput));
            budget.charge(22, 7).unwrap();
            assert!(matches!(budget.charge(1, 0), Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComputeWork, required: 31, limit: 30, ..
            })));
        }).unwrap();
        assert!(!NativeLibraryWorkBudget::has_active_scope());
        // Parent caps are gone, but all successful inner work stays spent.
        budget.charge(1, 0).unwrap();
        foreign.charge(1, 0).unwrap();
        assert_eq!(budget.used(), (31, 10));
        assert_eq!(foreign.used(), (1, 0));
        drop(foreign);
        drop(budget);
        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn native_library_clones_and_repeated_scopes_share_spent_work_and_reject_replacement() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        let cloned = budget.clone();
        budget
            .with_scope(|| observe_control_work(demand(5, 2)))
            .unwrap()
            .unwrap();
        cloned
            .with_scope(|| observe_control_work(demand(3, 1)))
            .unwrap()
            .unwrap();
        assert_eq!(budget.used(), (8, 3));
        assert!(matches!(
            budget
                .with_scope(|| observe_control_work(demand(1, 0)))
                .unwrap(),
            Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComputeWork,
                required: 9,
                limit: 8,
                ..
            })
        ));
        assert_eq!(cloned.used(), (8, 3));
        let fresh = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        assert_eq!(
            budget.with_scope(|| fresh.with_scope(|| ())).unwrap(),
            Err(ResidentKernelError::InvalidInput)
        );
        assert_eq!(fresh.used(), (0, 0));
        budget
            .with_scope(|| cloned.with_scope(|| observe_control_work(demand(0, 1))))
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(budget.used(), (8, 4));
    }

    #[test]
    fn native_library_limits_only_decrease_and_failed_restrictions_are_atomic() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(8, 4, &memory).unwrap();
        budget
            .with_scope(|| observe_control_work(demand(5, 2)))
            .unwrap()
            .unwrap();
        assert_eq!(
            budget.restrict(9, 4),
            Err(ResidentKernelError::InvalidInput)
        );
        assert!(matches!(
            budget.restrict(4, 4),
            Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComputeWork,
                required: 5,
                limit: 4,
                ..
            })
        ));
        assert!(matches!(
            budget.restrict(8, 1),
            Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComparisonWork,
                required: 2,
                limit: 1,
                ..
            })
        ));
        assert_eq!(budget.used(), (5, 2));
        budget.restrict(6, 3).unwrap();
        assert!(
            budget
                .with_scope(|| observe_control_work(demand(2, 0)))
                .unwrap()
                .is_err()
        );
        assert_eq!(budget.used(), (5, 2));
        budget
            .with_scope(|| observe_control_work(demand(1, 1)))
            .unwrap()
            .unwrap();
        assert_eq!(budget.used(), (6, 3));
    }

    #[test]
    fn native_restriction_and_setup_debit_are_atomic_under_root_and_call_caps() {
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(10, 4, &memory).unwrap();
        budget.charge(9, 0).unwrap();
        assert!(budget.restrict_and_charge(10, 2, 2, 0).is_err());
        assert_eq!(budget.used(), (9, 0));
        budget
            .with_call_limits(0, 4, || {
                assert!(budget.restrict_and_charge(10, 1, 1, 0).is_err());
            })
            .unwrap();
        assert_eq!(budget.used(), (9, 0));
        budget.charge(0, 4).unwrap();
        assert!(budget.restrict_and_charge(10, 3, 1, 0).is_err());
        assert_eq!(budget.used(), (9, 4));
        budget.restrict_and_charge(10, 4, 1, 0).unwrap();
        assert_eq!(budget.used(), (10, 4));
    }

    #[test]
    fn native_library_metadata_has_one_account_owner_and_resident_defaults_stay_unchanged() {
        let empty = mech_core::ManagedMemoryBudget::new(0);
        assert!(NativeLibraryWorkBudget::new(1, 1, &empty).is_err());
        assert_eq!(empty.used_bytes(), 0);
        let memory = mech_core::ManagedMemoryBudget::new(4096);
        let budget = NativeLibraryWorkBudget::new(
            2 * mech_core::RESIDENT_MAX_COMPUTE_WORK,
            2 * mech_core::RESIDENT_MAX_COMPARISON_WORK,
            &memory,
        )
        .unwrap();
        let before = memory.used_bytes();
        assert!(before > 0);
        let cloned = budget.clone();
        assert_eq!(memory.used_bytes(), before);
        budget
            .with_scope(|| {
                observe_control_work(demand(mech_core::RESIDENT_MAX_COMPUTE_WORK + 1, 0))
            })
            .unwrap()
            .unwrap();
        with_control_work_budget(|| {
            let _scope = ControlWorkScopeGuard::enter();
            assert!(matches!(
                observe_control_work(demand(mech_core::RESIDENT_MAX_COMPUTE_WORK + 1, 0)),
                Err(ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    limit: mech_core::RESIDENT_MAX_COMPUTE_WORK,
                    ..
                })
            ));
            assert_eq!(
                budget.with_scope(|| ()),
                Err(ResidentKernelError::InvalidInput)
            );
        });
        assert_eq!(cloned.used().0, mech_core::RESIDENT_MAX_COMPUTE_WORK + 1);
        budget
            .with_scope(|| {
                with_control_work_budget(|| {
                    let _scope = ControlWorkScopeGuard::enter();
                    assert!(matches!(
                        observe_control_work(demand(1, 0)),
                        Err(ResidentKernelError::ResourceLimit {
                            dimension: MemoryBudgetDimension::ComputeWork,
                            limit: mech_core::RESIDENT_MAX_COMPUTE_WORK,
                            ..
                        })
                    ));
                })
            })
            .unwrap();
        drop(budget);
        assert_eq!(memory.used_bytes(), before);
        drop(cloned);
        assert_eq!(memory.used_bytes(), 0);
    }
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

#[cfg(all(
    feature = "native-artifact",
    any(feature = "pure_text", feature = "map_ops")
))]
pub(in crate::resident) fn record_native_index_memory_error(
    error: mech_core::MemoryRuntimeError,
) -> ResidentKernelError {
    ACTIVE_PAYLOAD_ADMISSION.with(|active| {
        if let Some(admission) = active.borrow().as_ref() {
            admission.record_error(error);
        }
    });
    ResidentKernelError::InvalidShape
}

fn admit_payload_plan(plan: &TurnMemoryPlan) -> Result<(), ResidentKernelError> {
    ACTIVE_PAYLOAD_ADMISSION.with(|active| {
        if let Some(admission) = active.borrow().as_ref() {
            if plan
                .call
                .as_ref()
                .is_some_and(|call| call.target.kind == mech_core::MemoryTargetKind::NativeHost)
            {
                let mut demand = plan.demand;
                demand.turn_peak_bytes = demand
                    .turn_peak_bytes
                    .checked_add(admission.auxiliary_bytes())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                #[cfg(all(
                    feature = "native-artifact",
                    any(feature = "pure_text", feature = "map_ops")
                ))]
                {
                    // Cache capacity is independently reserved on this exact
                    // work/memory account. Count it in the finite call cap,
                    // but do not reserve the same bytes a second time.
                    demand.turn_peak_bytes = demand
                        .turn_peak_bytes
                        .checked_add(native_turn_plan_cache::retained_bytes())
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    demand.retained_nodes = demand
                        .retained_nodes
                        .checked_add(native_turn_plan_cache::retained_index_nodes())
                        .ok_or(ResidentKernelError::InvalidShape)?;
                }
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: plan.node,
                        port: 0,
                    },
                    demand,
                    plan.output_bytes,
                    plan.storage_buffer_bytes,
                    plan.budget_limits,
                )
                .first()
                {
                    return Err(resource_budget_error(violation));
                }
            }
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
        return measure_borrowed_canonical_value_footprint(meter, current, schemas);
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
    #[cfg(all(
        feature = "native-artifact",
        any(feature = "pure_text", feature = "map_ops")
    ))]
    let _cache_guard = native_turn_plan_cache::suspend();
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
/// exactly one shared `ResourceDemand`; it has no parallel cost authority.
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
    analysis_allowance: Option<(u64, u64)>,
    // A borrowed native projection has real invocation work authority but no
    // resident TurnMemoryPlan. This admits measurement only, never a kernel
    // execution or allocation/publication permit.
    native_projection_limits: Option<(mech_core::MemoryBudgetLimits, u64)>,
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
    pub(crate) fn check_analysis_limits(self) -> Result<(), ResidentKernelError> {
        if self.demand.output_elements > mech_core::RESIDENT_MAX_OUTPUT_ELEMENTS as u64
            || self.demand.persistent_bytes > mech_core::RESIDENT_MAX_BYTES
            || self.temporary_bytes() > mech_core::RESIDENT_MAX_BYTES
            || self.cloned_bytes() > mech_core::RESIDENT_MAX_BYTES
            || self.retained_nodes() > mech_core::RESIDENT_MAX_RETAINED_NODES
            || self.compute_work() > MAX_RESIDENT_COMPUTE_WORK
            || self.comparison_work() > MAX_RESIDENT_COMPARISON_WORK
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(())
    }

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
            if !super::text_index::has_index_scope()
                && index_growth(&active)? == (0, 0)
                && let Some(demand) =
                    try_admit_fixed_turn_memory(&active, self.demand, final_output)
                        .map_err(memory_plan_kernel_error)?
            {
                admit_payload_plan(&active)?;
                return Ok(ResidentBudgetPermit {
                    _plan: active,
                    _demand: demand,
                });
            }
            #[cfg(all(
                feature = "native-artifact",
                any(feature = "pure_text", feature = "map_ops")
            ))]
            if !super::text_index::has_index_scope()
                && index_growth(&active)? == (0, 0)
                && let Some(result) =
                    native_turn_plan_cache::refine(&active, self.demand, final_output)
            {
                return result;
            }
            return apply_observed_turn_demand((*active).clone(), self.demand, final_output)
                .map_err(memory_plan_kernel_error)
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
        self.check_planning_limits()
    }

    fn check_planning_limits(self) -> Result<(), ResidentKernelError> {
        let result = with_active_turn_plan(|active| {
            active.as_ref().map(|plan| {
                check_turn_planning_progress(plan, self.demand)
                    .map_err(memory_plan_kernel_error)?;
                let (bytes, nodes) = index_growth(plan)?;
                let temporary = self
                    .demand
                    .turn_peak_bytes
                    .checked_add(plan.facts.additional_demand.turn_peak_bytes)
                    .map(|value| value.max(plan.demand.turn_peak_bytes))
                    .and_then(|value| value.checked_add(bytes))
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let retained_nodes = self
                    .demand
                    .retained_nodes
                    .checked_add(plan.facts.additional_demand.retained_nodes)
                    .map(|value| value.max(plan.demand.retained_nodes))
                    .and_then(|value| value.checked_add(nodes))
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let demand = ResourceDemand {
                    turn_peak_bytes: temporary,
                    retained_nodes,
                    ..ResourceDemand::default()
                };
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: plan.node,
                        port: 0,
                    },
                    demand,
                    plan.output_bytes,
                    plan.storage_buffer_bytes,
                    plan.budget_limits,
                )
                .first()
                {
                    return Err(resource_budget_error(violation));
                }
                Ok(())
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
        // A native invocation may carry a different finite policy from the
        // resident default. Its already-admitted plan remains the authority;
        // subtracting a large successful prefix from the default profile can
        // otherwise reject an unrelated scalar finalization as InvalidShape.
        let (compute, comparison) = with_active_turn_plan(|active| {
            active.as_ref().map_or((MAX_RESIDENT_COMPUTE_WORK, MAX_RESIDENT_COMPARISON_WORK),
                |plan| (plan.budget_limits.max_compute_work.unwrap_or(MAX_RESIDENT_COMPUTE_WORK),
                    plan.budget_limits.max_comparison_work.unwrap_or(MAX_RESIDENT_COMPARISON_WORK)))
        });
        let remaining = comparison
            .checked_sub(self.comparison_work())
            .ok_or(ResidentKernelError::InvalidShape)?
            .min(
                compute
                    .checked_sub(self.compute_work())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            );
        // The call policy cannot replenish its enclosing cumulative account or
        // a tighter nested call. The planning check above charged this prefix;
        // only the currently remaining finite allowance may reach a finalizer.
        CONTROL_WORK_SCOPE.with(|scope| {
            let scope = scope.borrow();
            let Some(scope) = scope.as_ref() else { return Ok(remaining); };
            let account = scope.total.borrow();
            let (root_compute, root_comparison) = CONTROL_ROOT_LIMITS.with(|limits| limits.get())
                .unwrap_or((account.compute_limit, account.comparison_limit));
            let compute = account.compute_limit.min(root_compute)
                .checked_sub(account.used.compute).and_then(|n| n.checked_sub(account.reserved.compute))
                .ok_or(ResidentKernelError::InvalidShape)?;
            let comparison = account.comparison_limit.min(root_comparison)
                .checked_sub(account.used.comparison).and_then(|n| n.checked_sub(account.reserved.comparison))
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(remaining.min(compute).min(comparison))
        })
    }
}

#[cfg(test)]
pub(crate) fn detached_turn_plan(
    demand: ResourceDemand,
) -> Result<TurnMemoryPlan, ResidentKernelError> {
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

fn index_growth(plan: &TurnMemoryPlan) -> Result<(u64, u64), ResidentKernelError> {
    let (bytes, nodes) = super::text_index::live_demand();
    Ok((
        bytes
            .checked_sub(plan.resident_text_index_baseline.0)
            .ok_or(ResidentKernelError::InvalidShape)?,
        nodes
            .checked_sub(plan.resident_text_index_baseline.1)
            .ok_or(ResidentKernelError::InvalidShape)?,
    ))
}

impl ResidentBudgetPermit {
    fn from_turn_plan(mut plan: TurnMemoryPlan) -> Result<Self, ResidentKernelError> {
        if let Some(violation) = plan.budget_violations.first() {
            return Err(resource_budget_error(violation));
        }
        let (bytes, nodes) = index_growth(&plan)?;
        plan.demand.turn_peak_bytes = plan
            .demand
            .turn_peak_bytes
            .checked_add(bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        plan.demand.retained_nodes = plan
            .demand
            .retained_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        if let Some(violation) = mech_core::evaluate_call_memory_budget(
            mech_core::MemoryObjectOwner::NodeOutput {
                node: plan.node,
                port: 0,
            },
            plan.demand,
            plan.output_bytes,
            plan.storage_buffer_bytes,
            plan.budget_limits,
        )
        .first()
        {
            return Err(resource_budget_error(violation));
        }
        admit_payload_plan(&plan)?;
        Ok(Self {
            _demand: plan.demand,
            _plan: Arc::new(plan),
        })
    }
}

pub(crate) fn resource_budget_error(
    violation: &mech_core::MemoryBudgetViolation,
) -> ResidentKernelError {
    ResidentKernelError::ResourceLimit {
        phase: mech_core::ResidentResourcePhase::Admission,
        dimension: violation.dimension,
        required: violation.required,
        limit: violation.limit,
    }
}

#[cfg(test)]
thread_local! {
    // Fixed scalar evidence only; no allocation or assertion in a callback.
    static OUTPUT_ELEMENT_PREFLIGHT: std::cell::Cell<(u64, u64, u64)> =
        const { std::cell::Cell::new((0, 0, 0)) };
}

/// Check prospective element count before a selected-cell walk or draft.
/// This is only a current-policy guard, never a materialization permit.
pub(crate) fn check_output_element_count(required: u64) -> Result<(), ResidentKernelError> {
    let current = with_active_turn_plan(|active| {
        active.as_ref().map(|plan| {
            let limit = plan
                .budget_limits
                .max_output_elements
                .ok_or(ResidentKernelError::InvalidInput)?;
            #[cfg(test)]
            OUTPUT_ELEMENT_PREFLIGHT.with(|observed| {
                observed.set((observed.get().0.saturating_add(1), required, limit));
            });
            if required <= limit {
                return Ok(());
            }
            let demand = ResourceDemand {
                output_elements: required,
                ..ResourceDemand::default()
            };
            if let Some(violation) = mech_core::evaluate_call_memory_budget(
                mech_core::MemoryObjectOwner::NodeOutput {
                    node: plan.node,
                    port: 0,
                },
                demand,
                0,
                0,
                plan.budget_limits,
            )
            .first()
            {
                return Err(resource_budget_error(violation));
            }
            Err(ResidentKernelError::InvalidInput)
        })
    });
    if let Some(result) = current {
        return result;
    }
    // Closed/ordinary analysis has no selected current invocation policy.
    // Preserve its historical default and error exactly.
    if required > MAX_RESIDENT_OUTPUT_ELEMENTS {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "native-artifact",
    any(feature = "pure_text", feature = "map_ops"),
    any(feature = "full_source", feature = "full_compiler")
))]
#[path = "snapshot_output_element_tests.rs"]
mod snapshot_output_element_tests;

pub(crate) fn memory_plan_kernel_error(error: mech_core::MemoryPlanError) -> ResidentKernelError {
    match error {
        mech_core::MemoryPlanError::TargetLimitExceeded { violation } => {
            resource_budget_error(&violation)
        }
        _ => ResidentKernelError::InvalidShape,
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

    /// Reserve a closed construction under analysis authority, without
    /// manufacturing a turn permit or invoking a resident executor.
    pub(crate) fn prepare_for_analysis(
        self,
        admit: impl FnOnce(KernelCostEstimate) -> Result<(), ResidentKernelError>,
    ) -> Result<P, ResidentKernelError> {
        self.cost.check_analysis_limits()?;
        admit(self.cost)?;
        Ok(self.plan)
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

    /// Analysis owns a separate cumulative allowance. This authorizes only
    /// closed-value construction, never a resident turn or publication.
    #[cfg(feature = "table")]
    pub(crate) fn prepare_for_analysis(
        self,
        admit: impl FnOnce(KernelCostEstimate) -> Result<(), ResidentKernelError>,
    ) -> Result<P, ResidentKernelError> {
        self.cost.check_analysis_limits()?;
        admit(self.cost)?;
        Ok(self.operation)
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
    if borrowed_footprint_schema_supported(meter, schema, 0)? {
        let footprint =
            measure_proven_borrowed_canonical_data_footprint(meter, schema, data, false)?;
        // Metadata traversal is not a payload comparison. Reserve the second
        // finalization walk and later canonical encoding/copy bytes as compute;
        // actual ordered-container key work below remains comparison work.
        meter.charge_compute_work(checked_cost_product(&[
            footprint.node_count.max(1),
            BORROWED_FOOTPRINT_METADATA_WORK,
        ])?)?;
        meter.charge_compute_work(
            footprint
                .encoded_bytes
                .checked_add(footprint.node_count)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
    } else {
        measure_canonical_data_comparison_work(meter, schema, data)?;
    }
    let remaining = meter.remaining_incremental_work()?;
    let budget = SnapshotCanonicalizationBudget::new(remaining);
    let result = mech_core::snapshot::canonical_data_draft_finalization_work_with_budget(
        schema, data, &budget,
    );
    if result.is_err() && meter.native_projection_limits.is_some() {
        // A failed preflight already performed its admitted prefix of key
        // comparisons. Keep that spent work on the invocation across retries.
        meter.charge_comparison_work(budget.consumed())?;
    }
    let work = result.map_err(map_snapshot_work_error)?;
    meter.charge_comparison_work(work)?;
    Ok(work)
}

/// Shared preparation for a selected immutable snapshot input. Closed
/// evaluation and the runtime constructor use the same retention inspection
/// and finalization reservation; only actual equality consumes scan work.
pub(crate) fn prepare_snapshot_copy_input(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
    finalizes: bool,
) -> Result<(ValueFootprint, u64), ResidentKernelError> {
    let footprint = measure_borrowed_canonical_value_footprint(meter, value, schemas)?;
    let finalization = if finalizes {
        let schema = value
            .validate_against(schemas)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        preflight_canonical_data_finalization(meter, schema.body(), value.data())?
    } else {
        0
    };
    Ok((footprint, finalization))
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

// These are logical metadata-inspection units, not a serialized-byte scan or
// machine instruction count. Each schema node and data chunk needs a bounded
// number of variant, slice-length, checked-size and child-selection operations.
const BORROWED_FOOTPRINT_METADATA_WORK: u64 = 64;
// This is only a fast-path eligibility cutoff. Deeper schemas keep the existing
// full measurement path; the cutoff does not make a value ineligible to execute.
const BORROWED_FOOTPRINT_SCHEMA_DEPTH: usize = 32;

/// Execution-local proof that a semantic schema supports metadata-only
/// borrowed footprint measurement. The proof is keyed by semantic identity,
/// so an equal schema in another immutable owner can reuse it safely. Callers
/// must still measure each concrete value; this only avoids walking the same
/// immutable schema definition for every tail carrier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BorrowedFootprintSchemaProof {
    key: mech_core::SchemaKey,
    supported: bool,
}

/// A bounded binding-time proof for the common scalar/packed-matrix layouts.
/// It inspects at most two schema variants, allocates nothing and grants no
/// concrete value footprint. Other schemas retain the metered recursive proof.
pub(crate) fn shallow_borrowed_footprint_schema_proof(
    schema: &mech_core::Schema,
) -> Option<BorrowedFootprintSchemaProof> {
    let scalar = |body: &SchemaBody| {
        matches!(
            body,
            SchemaBody::Bool
                | SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Complex(_)
                | SchemaBody::Rational64
                | SchemaBody::String
                | SchemaBody::Id
                | SchemaBody::Index
                | SchemaBody::Atom(_)
        )
    };
    let supported = scalar(schema.body())
        || matches!(schema.body(), SchemaBody::Matrix { element, .. } if scalar(element));
    supported.then_some(BorrowedFootprintSchemaProof {
        key: schema.key(),
        supported,
    })
}

fn borrowed_footprint_schema_supported(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    depth: usize,
) -> Result<bool, ResidentKernelError> {
    meter.charge_compute_work(BORROWED_FOOTPRINT_METADATA_WORK)?;
    if depth > BORROWED_FOOTPRINT_SCHEMA_DEPTH {
        return Ok(false);
    }
    let child = |meter: &mut ResidentBudgetMeter, schema: &SchemaBody| {
        borrowed_footprint_schema_supported(meter, schema, depth + 1)
    };
    match schema {
        // These branches are deliberately outside the initial metadata-only
        // proof. Their old traversal and comparison allowance remain intact.
        SchemaBody::Dynamic | SchemaBody::ReifiedType | SchemaBody::IntegerInterval(_) => Ok(false),
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_) => Ok(true),
        SchemaBody::Option(element)
        | SchemaBody::Matrix { element, .. }
        | SchemaBody::Set { element, .. } => child(meter, element),
        SchemaBody::Tuple(elements) => {
            for element in elements {
                if !child(meter, element)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        SchemaBody::Record(fields)
        | SchemaBody::Table {
            columns: fields, ..
        } => {
            for field in fields {
                if !child(meter, &field.schema)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        SchemaBody::Enum { variants, .. } => {
            for variant in variants {
                // Payload-free variants still require a header inspection.
                meter.charge_compute_work(BORROWED_FOOTPRINT_METADATA_WORK)?;
                if let Some(payload) = &variant.payload
                    && !child(meter, payload)?
                {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        SchemaBody::Map { key, value, .. } => Ok(child(meter, key)? && child(meter, value)?),
    }
}

fn measure_proven_borrowed_canonical_data_footprint(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
    retain_nodes: bool,
) -> Result<ValueFootprint, ResidentKernelError> {
    // A packed String matrix has exactly one matrix chunk, one sequence chunk
    // and one chunk per cell. Its immutable, actual slice length therefore
    // proves the entire metadata scan's work before any cell is visited.
    // Unsupported layouts and tight budgets retain the incremental visitor,
    // including its first-denied requirement and already-spent work prefix.
    let whole_scan = if let (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) =
        (schema, data)
        && matches!(element.as_ref(), SchemaBody::String)
        && let SequenceView::String(strings) = matrix.elements()
        && let Some(nodes) = u64::try_from(strings.len())
            .ok()
            .and_then(|count| count.checked_add(2))
    {
        meter.try_charge_borrowed_metadata_scan(nodes, retain_nodes)?
    } else {
        false
    };
    let mut footprint = ValueFootprint::zero();
    mech_core::snapshot::visit_canonical_data_work(schema, data, |chunk| {
        // The core visitor reports retained/encoded sizes before descent. For
        // the proven schema branches it reads lengths and container metadata;
        // packed numeric bytes and String UTF-8 payloads are not traversed.
        if !whole_scan {
            meter.charge_compute_work(checked_cost_product(&[
                BORROWED_FOOTPRINT_METADATA_WORK,
                chunk.node_count.max(1),
            ])?)?;
            if retain_nodes {
                meter.charge_retained_nodes(chunk.node_count)?;
            }
        }
        footprint = footprint
            .checked_add(ValueFootprint {
                encoded_bytes: chunk.encoded_bytes,
                retained_bytes: chunk.retained_bytes,
                node_count: chunk.node_count,
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

/// Measures already-finalized borrowed data for retention bookkeeping. This
/// does not authorize payload comparison, encoding, cloning or reconstruction.
/// Unsupported/deep schemas retain the older conservative full-work path.
pub(crate) fn measure_borrowed_canonical_data_footprint(
    meter: &mut ResidentBudgetMeter,
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<ValueFootprint, ResidentKernelError> {
    if !borrowed_footprint_schema_supported(meter, schema, 0)? {
        return measure_canonical_data_footprint(meter, schema, data);
    }
    measure_proven_borrowed_canonical_data_footprint(meter, schema, data, true)
}

/// Measures an immutable root and its actual owned shape using metadata-only
/// work for the proven data layouts. The caller supplies its semantic schema
/// owner. Equal schema id/key owners are valid even when separately allocated;
/// an unproved root identity receives no metadata-only fast path.
pub(crate) fn measure_borrowed_canonical_value_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
) -> Result<ValueFootprint, ResidentKernelError> {
    let mut proof = None;
    measure_borrowed_canonical_value_footprint_with_proof(meter, value, schemas, &mut proof)
}

/// The snapshot-access source alone may reuse a complete DATA-only summary
/// owned by the current native invocation. Current schema support, wrapper,
/// shape and retained nodes are still measured/admitted here on every use.
/// Ordinary execution and unsupported layouts retain the original visitor.
pub(crate) fn measure_snapshot_access_source_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
) -> Result<ValueFootprint, ResidentKernelError> {
    let mut proof = None;
    measure_borrowed_value_footprint(meter, value, schemas, &mut proof, true)
}

/// Measures a borrowed value while reusing only the immutable schema-support
/// proof. Concrete shape, retained nodes, and selected data are charged on
/// every call.
pub(crate) fn measure_borrowed_canonical_value_footprint_with_proof(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
    proof: &mut Option<BorrowedFootprintSchemaProof>,
) -> Result<ValueFootprint, ResidentKernelError> {
    measure_borrowed_value_footprint(meter, value, schemas, proof, false)
}

fn measure_borrowed_value_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schemas: &SchemaTable,
    proof: &mut Option<BorrowedFootprintSchemaProof>,
    source_cache: bool,
) -> Result<ValueFootprint, ResidentKernelError> {
    meter.charge_compute_work(BORROWED_FOOTPRINT_METADATA_WORK)?;
    let entry = schemas
        .entry(value.schema())
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schema = entry.schema();
    if entry.key() != value.schema_key() {
        return measure_canonical_value_footprint(meter, value, schemas);
    }
    let supported = match proof {
        Some(proof) if proof.key == entry.key() => proof.supported,
        proof => {
            let supported = borrowed_footprint_schema_supported(meter, schema.body(), 0)?;
            *proof = Some(BorrowedFootprintSchemaProof {
                key: entry.key(),
                supported,
            });
            supported
        }
    };
    if !supported {
        return measure_canonical_value_footprint(meter, value, schemas);
    }
    let shape_parameters = value.shape().parameter_values().len();
    meter.charge_retained_nodes(1)?;
    #[cfg(all(
        feature = "native-artifact",
        any(feature = "pure_text", feature = "map_ops")
    ))]
    if source_cache
        && let Some(data) =
            native_turn_plan_cache::borrowed_data_footprint(value, entry.key(), |work| {
                meter.charge_compute_work(work)
            })?
    {
        // Reusing a successful immutable DATA measurement eliminates the
        // visitor, not the source's full current retained-node obligation.
        meter.charge_retained_nodes(data.node_count)?;
        return projected_canonical_value_footprint(data, shape_parameters);
    }
    #[cfg(not(all(
        feature = "native-artifact",
        any(feature = "pure_text", feature = "map_ops")
    )))]
    let _ = source_cache;
    let data =
        measure_proven_borrowed_canonical_data_footprint(meter, schema.body(), value.data(), true)?;
    projected_canonical_value_footprint(data, shape_parameters)
}

/// Measures a borrowed value whose exact schema identity and supported body
/// were already proven by an internal activation certificate and rechecked at
/// this execution boundary. Concrete shape, retained nodes, and selected data
/// are still charged on every call.
pub(crate) fn measure_certified_borrowed_canonical_value_footprint(
    meter: &mut ResidentBudgetMeter,
    value: &Value,
    schema: &SchemaBody,
) -> Result<ValueFootprint, ResidentKernelError> {
    let shape_parameters = value.shape().parameter_values().len();
    meter.charge_retained_nodes(1)?;
    let data = measure_proven_borrowed_canonical_data_footprint(meter, schema, value.data(), true)?;
    projected_canonical_value_footprint(data, shape_parameters)
}

impl<P> AdmittedMutationPlan<P> {
    /// Consumes the complete post-state admission immediately before staging.
    pub(crate) fn into_plan(self) -> P {
        self.operation
    }
}

impl ResidentBudgetMeter {
    /// Prepay a fixed-count metadata scan, never payload work, allocation or
    /// publication. The preview uses the same authorities as ordinary charges
    /// and is side-effect free; a rejected preview is only a fast-path miss.
    pub(super) fn try_charge_borrowed_metadata_scan(
        &mut self,
        nodes: u64,
        retain_nodes: bool,
    ) -> Result<bool, ResidentKernelError> {
        let Some(work) = nodes.checked_mul(BORROWED_FOOTPRINT_METADATA_WORK) else {
            return Ok(false);
        };
        let Some(next) = self.preview_compute_and_nodes(work, if retain_nodes { nodes } else { 0 })
        else {
            return Ok(false);
        };
        self.charge(|cost| {
            *cost = next;
            Ok(())
        })?;
        Ok(true)
    }

    /// Eligibility, not a permit or debit. A caller may use a conservative
    /// bound to prove a prepaid metadata scan can finish, then charge only
    /// the exact measured copy work before ordinary candidate admission.
    pub(super) fn can_charge_compute_and_nodes(&self, compute: u64, nodes: u64) -> bool {
        self.preview_compute_and_nodes(compute, nodes).is_some()
    }

    fn preview_compute_and_nodes(&self, compute: u64, nodes: u64) -> Option<KernelCostEstimate> {
        let mut next = self.accumulated;
        next.set_compute_work(next.compute_work().checked_add(compute)?)
            .ok()?;
        next.set_retained_nodes(next.retained_nodes().checked_add(nodes)?);
        let fits = if let Some((compute, comparison)) = self.analysis_allowance {
            next.check_analysis_limits().is_ok()
                && next.compute_work() <= compute
                && next.comparison_work() <= comparison
        } else {
            // A preview must not enter the detached test-only admission path,
            // which can observe work. Production scans require a real turn.
            with_active_turn_plan(|active| active.is_some())
                && preview_control_work(next.demand).is_ok()
                && next.check_planning_limits().is_ok()
        };
        if !fits {
            return None;
        }
        Some(next)
    }

    /// Explicit borrowed measurement outside a turn. Default meters retain
    /// their turn-only authority in both production and test builds.
    pub(crate) fn for_closed_analysis(compute: u64, comparison: u64) -> Self {
        Self {
            accumulated: KernelCostEstimate::default(),
            analysis_allowance: Some((compute, comparison)),
            native_projection_limits: None,
        }
    }

    #[cfg(feature = "native-artifact")]
    pub(crate) fn for_native_projection(
        work: &NativeLibraryWorkBudget,
        limits: mech_core::MemoryBudgetLimits,
    ) -> Result<Self, ResidentKernelError> {
        let same_account = CONTROL_WORK.with(|active| {
            active
                .borrow()
                .as_ref()
                .is_some_and(|account| std::rc::Rc::ptr_eq(account, &work.account))
        });
        let same_scope = CONTROL_WORK_SCOPE.with(|scope| {
            scope
                .borrow()
                .as_ref()
                .is_some_and(|scope| std::rc::Rc::ptr_eq(&scope.total, &work.account))
        });
        if !same_account
            || !same_scope
            || limits.max_compute_work.is_none()
            || limits.max_comparison_work.is_none()
            || limits.max_temporary_bytes.is_none()
            || limits.max_retained_nodes.is_none()
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        Ok(Self {
            native_projection_limits: Some((limits, work.account.borrow().identity)),
            ..Self::default()
        })
    }

    fn check_native_projection_scope(&self, identity: u64) -> Result<(), ResidentKernelError> {
        let live = CONTROL_WORK_SCOPE.with(|scope| {
            scope
                .borrow()
                .as_ref()
                .is_some_and(|scope| scope.total.borrow().identity == identity)
        });
        if !live {
            return Err(ResidentKernelError::InvalidInput);
        }
        Ok(())
    }

    pub(crate) fn preflight_meter(&self) -> Result<Self, ResidentKernelError> {
        Ok(match self.analysis_allowance {
            Some((compute, comparison)) => Self::for_closed_analysis(
                compute
                    .checked_sub(self.accumulated.compute_work())
                    .ok_or(ResidentKernelError::InvalidShape)?,
                comparison
                    .checked_sub(self.accumulated.comparison_work())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            ),
            None => Self {
                native_projection_limits: self.native_projection_limits,
                ..Self::default()
            },
        })
    }

    pub(crate) fn remaining_incremental_work(&self) -> Result<u64, ResidentKernelError> {
        if let Some((limits, identity)) = self.native_projection_limits {
            self.check_native_projection_scope(identity)?;
            return CONTROL_WORK_SCOPE.with(|scope| {
                let scope = scope.borrow();
                let account = scope
                    .as_ref()
                    .ok_or(ResidentKernelError::InvalidInput)?
                    .total
                    .borrow();
                let (root_compute, root_comparison) = CONTROL_ROOT_LIMITS
                    .with(|root| root.get())
                    .ok_or(ResidentKernelError::InvalidInput)?;
                let compute = account
                    .compute_limit
                    .min(root_compute)
                    .checked_sub(account.used.compute)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let comparison = account
                    .comparison_limit
                    .min(root_comparison)
                    .checked_sub(account.used.comparison)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                Ok(compute
                    .min(comparison)
                    .min(
                        limits
                            .max_compute_work
                            .ok_or(ResidentKernelError::InvalidInput)?
                            .checked_sub(self.accumulated.compute_work())
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    )
                    .min(
                        limits
                            .max_comparison_work
                            .ok_or(ResidentKernelError::InvalidInput)?
                            .checked_sub(self.accumulated.comparison_work())
                            .ok_or(ResidentKernelError::InvalidShape)?,
                    ))
            });
        }
        if let Some((compute, comparison)) = self.analysis_allowance {
            self.accumulated.check_analysis_limits()?;
            Ok(compute
                .checked_sub(self.accumulated.compute_work())
                .ok_or(ResidentKernelError::InvalidShape)?
                .min(
                    comparison
                        .checked_sub(self.accumulated.comparison_work())
                        .ok_or(ResidentKernelError::InvalidShape)?,
                ))
        } else {
            self.accumulated.remaining_incremental_work()
        }
    }

    fn charge(
        &mut self,
        update: impl FnOnce(&mut KernelCostEstimate) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        let mut next = self.accumulated;
        update(&mut next)?;
        if let Some((compute, comparison)) = self.analysis_allowance {
            next.check_analysis_limits()?;
            if next.compute_work() > compute || next.comparison_work() > comparison {
                return Err(ResidentKernelError::InvalidShape);
            }
        } else if let Some((limits, identity)) = self.native_projection_limits {
            self.check_native_projection_scope(identity)?;
            // Charge the same live invocation before continuing the borrowed
            // traversal. A tighter call cannot replenish its enclosing ledger.
            observe_control_work(next.demand)?;
            if let Some(violation) = mech_core::evaluate_call_memory_budget(
                mech_core::MemoryObjectOwner::DirectCallPort {
                    call: 0,
                    direction: mech_core::PortDirection::Input,
                    port: 0,
                },
                next.demand,
                0,
                0,
                limits,
            )
            .first()
            {
                return Err(resource_budget_error(violation));
            }
        } else {
            next.check_planning_progress()?;
        }
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

    /// Releases a lexical live payload after its owner is dropped. Work and
    /// clone traffic remain cumulative; only simultaneous retained nodes fall.
    pub(crate) fn release_retained_nodes(
        &mut self,
        amount: u64,
    ) -> Result<(), ResidentKernelError> {
        self.accumulated.demand.retained_nodes = self
            .accumulated
            .demand
            .retained_nodes
            .checked_sub(amount)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
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
    fn retention_finalization_preserves_encoding_compute_without_false_comparisons() {
        let (value, schemas) = borrowed_retention_value(
            SchemaBody::String,
            mech_core::ValueDataDraft::String("x".repeat(8_192)),
        );
        let body = schemas.get(value.schema()).unwrap().body();
        let token = value.resident_token();
        let mut denied = ResidentBudgetMeter::for_closed_analysis(4_096, 0);
        assert!(
            preflight_canonical_data_finalization(&mut denied, body, value.data()).is_err(),
            "real encoding/copy compute must be finite even without any key comparisons"
        );
        let mut retry = ResidentBudgetMeter::for_closed_analysis(65_536, 0);
        assert_eq!(
            preflight_canonical_data_finalization(&mut retry, body, value.data()),
            Ok(0)
        );
        assert_eq!(retry.estimate().comparison_work(), 0);
        assert!(retry.estimate().compute_work() >= 8_192);
        assert_eq!(value.resident_token(), token);
    }

    #[test]
    fn retention_finalization_keeps_actual_ordered_key_work_and_denial() {
        let body = SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
        };
        let (value, schemas) = borrowed_retention_value(
            body,
            mech_core::ValueDataDraft::Set(
                vec![
                    mech_core::ValueDataDraft::String(format!("{}a", "x".repeat(2_048))),
                    mech_core::ValueDataDraft::String(format!("{}b", "x".repeat(2_048))),
                ]
                .into_boxed_slice(),
            ),
        );
        let body = schemas.get(value.schema()).unwrap().body();
        let before = value.resident_token();
        assert!(
            preflight_canonical_data_finalization(
                &mut ResidentBudgetMeter::for_closed_analysis(1_000_000, 0),
                body,
                value.data()
            )
            .is_err()
        );
        let mut retry = ResidentBudgetMeter::for_closed_analysis(1_000_000, 65_536);
        let work = preflight_canonical_data_finalization(&mut retry, body, value.data()).unwrap();
        assert!(
            work >= 2_048,
            "long common-prefix keys really compare bytes"
        );
        assert_eq!(retry.estimate().comparison_work(), work);
        assert_eq!(value.resident_token(), before);
    }

    fn borrowed_retention_value(
        body: SchemaBody,
        data: mech_core::ValueDataDraft,
    ) -> (Value, Arc<SchemaTable>) {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let handle = builder
            .insert(
                mech_core::SchemaDraft {
                    body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let schemas = Arc::new(build.table);
        let value = mech_core::ValueDraft {
            schema,
            shape_values: Box::new([]),
            data,
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        (value, schemas)
    }

    fn borrowed_retention_matrix(element: SchemaBody, count: usize) -> SchemaBody {
        SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: Box::new([
                mech_core::DimensionExpr::Constant(1),
                mech_core::DimensionExpr::Constant(count as u64),
            ]),
        }
    }

    #[test]
    fn snapshot_access_source_without_active_native_cache_keeps_original_visitor() {
        use mech_core::{ValueDataDraft, snapshot::OptionDraft};

        let empty =
            borrowed_retention_value(SchemaBody::String, ValueDataDraft::String(String::new()));
        let matrix = borrowed_retention_value(
            borrowed_retention_matrix(SchemaBody::String, 3),
            ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::String("e\u{301}👩‍👩‍👧‍👦".to_owned()),
                    ValueDataDraft::String("é".repeat(40_000)),
                    ValueDataDraft::String(String::new()),
                ]
                .into_boxed_slice(),
            ),
        );
        let mut body = SchemaBody::Bool;
        let mut data = ValueDataDraft::Bool(true);
        for _ in 0..40 {
            body = SchemaBody::Option(Box::new(body));
            data = ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(data)),
            });
        }
        let deep = borrowed_retention_value(body, data);
        for (value, schemas) in [empty, matrix, deep] {
            for compute in [128, 1_000_000] {
                let mut original = ResidentBudgetMeter::for_closed_analysis(compute, 1_000_000);
                let expected =
                    measure_borrowed_canonical_value_footprint(&mut original, &value, &schemas);
                let mut source = ResidentBudgetMeter::for_closed_analysis(compute, 1_000_000);
                assert_eq!(
                    measure_snapshot_access_source_footprint(&mut source, &value, &schemas),
                    expected,
                );
                assert_eq!(source.estimate(), original.estimate());
            }
        }
        let (number, number_owner) = borrowed_retention_value(
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
            ValueDataDraft::U64(7),
        );
        let (text, text_owner) =
            borrowed_retention_value(SchemaBody::String, ValueDataDraft::String("x".to_owned()));
        assert_eq!(number.schema(), text.schema());
        assert_ne!(number.schema_key(), text.schema_key());
        assert_ne!(
            number_owner.entry(number.schema()).unwrap().key(),
            text.schema_key()
        );
        let mut original = ResidentBudgetMeter::for_closed_analysis(1_024, 1_024);
        let expected =
            measure_borrowed_canonical_value_footprint(&mut original, &text, &number_owner);
        let mut source = ResidentBudgetMeter::for_closed_analysis(1_024, 1_024);
        assert_eq!(
            measure_snapshot_access_source_footprint(&mut source, &text, &number_owner),
            expected,
        );
        assert_eq!(source.estimate(), original.estimate());
        assert!(text.validate_against(&text_owner).is_ok());
    }

    #[test]
    fn shallow_borrowed_proofs_do_not_cache_concrete_shape_or_payload() {
        use mech_core::{
            DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin, IntegerWidth, SchemaDraft, SchemaTableBuilder,
            ValueDataDraft, ValueDraft, snapshot::SnapshotValidationContext,
        };
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    dimension_parameters: vec![DimensionParameterDeclaration {
                        id: DimensionParameterId::new(0),
                        origin: DimensionParameterOrigin::Inferred,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: None,
                    }]
                    .into_boxed_slice(),
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                        dimensions: Box::new([
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]),
                    },
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let schema = built.resolve(handle).unwrap();
        let mut proof = shallow_borrowed_footprint_schema_proof(built.table.get(schema).unwrap());
        assert!(proof.is_some());
        let mut compute = None;
        for count in [1usize, 10_000, 0, 3] {
            let value = ValueDraft {
                schema,
                shape_values: vec![count as u64].into_boxed_slice(),
                data: ValueDataDraft::Matrix(
                    vec![ValueDataDraft::U64(7); count].into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(&built.table))
            .unwrap();
            let mut meter = ResidentBudgetMeter::for_closed_analysis(1_024, 0);
            let footprint = measure_borrowed_canonical_value_footprint_with_proof(
                &mut meter,
                &value,
                &built.table,
                &mut proof,
            )
            .unwrap();
            assert_eq!(footprint, value.retained_footprint(&built.table).unwrap());
            assert_eq!(footprint.encoded_bytes, count as u64 * 8);
            assert_eq!(meter.estimate().comparison_work(), 0);
            let observed = meter.estimate().compute_work();
            assert_eq!(*compute.get_or_insert(observed), observed);
            assert!(observed > 0);
        }
    }

    #[test]
    fn shallow_borrowed_proofs_recheck_current_owner_semantic_identity() {
        let (number, number_owner) = borrowed_retention_value(
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
            mech_core::ValueDataDraft::U64(7),
        );
        let (text, text_owner) = borrowed_retention_value(
            SchemaBody::String,
            mech_core::ValueDataDraft::String("x".repeat(40_000)),
        );
        assert_eq!(number.schema(), text.schema());
        assert_ne!(number.schema_key(), text.schema_key());
        let original =
            shallow_borrowed_footprint_schema_proof(number_owner.get(number.schema()).unwrap());
        let mut proof = original;
        let mut meter = ResidentBudgetMeter::for_closed_analysis(1_024, 0);
        let footprint = measure_borrowed_canonical_value_footprint_with_proof(
            &mut meter,
            &text,
            &text_owner,
            &mut proof,
        )
        .unwrap();
        assert_eq!(footprint, text.retained_footprint(&text_owner).unwrap());
        assert_ne!(proof, original);
        assert_eq!(
            meter.estimate().compute_work(),
            3 * BORROWED_FOOTPRINT_METADATA_WORK
        );
    }

    #[test]
    fn shallow_borrowed_proofs_decline_recursive_and_dynamic_layouts() {
        for body in [
            SchemaBody::Dynamic,
            SchemaBody::Tuple(vec![SchemaBody::Bool; 1_000].into_boxed_slice()),
            borrowed_retention_matrix(SchemaBody::Dynamic, 1),
        ] {
            let schema = mech_core::SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap();
            assert!(shallow_borrowed_footprint_schema_proof(&schema).is_none());
        }
    }

    #[test]
    fn borrowed_retention_preserves_exact_sizes_and_semantic_schema_owner_identity() {
        use mech_core::{ValueDataDraft, snapshot::OptionDraft};

        let (value, owner) = borrowed_retention_value(
            SchemaBody::Tuple(
                vec![
                    SchemaBody::String,
                    SchemaBody::Option(Box::new(borrowed_retention_matrix(
                        SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
                        6,
                    ))),
                ]
                .into_boxed_slice(),
            ),
            ValueDataDraft::Tuple(
                vec![
                    ValueDataDraft::String("e\u{301}👩‍👩‍👧‍👦".to_owned()),
                    ValueDataDraft::Option(OptionDraft {
                        present: true,
                        value: Some(Box::new(ValueDataDraft::Matrix(
                            (0..6).map(ValueDataDraft::U64).collect(),
                        ))),
                    }),
                ]
                .into_boxed_slice(),
            ),
        );
        let mut old = ResidentBudgetMeter::for_closed_analysis(
            MAX_RESIDENT_COMPUTE_WORK,
            MAX_RESIDENT_COMPARISON_WORK,
        );
        let expected = measure_canonical_value_footprint(&mut old, &value, &owner).unwrap();
        let separate_owner = Arc::new((*owner).clone());
        assert!(!Arc::ptr_eq(&owner, &separate_owner));
        let mut borrowed = ResidentBudgetMeter::for_closed_analysis(4_096, 0);
        assert_eq!(
            measure_borrowed_canonical_value_footprint(&mut borrowed, &value, &separate_owner)
                .unwrap(),
            expected,
        );
        assert_eq!(expected, value.retained_footprint(&owner).unwrap());
        assert_eq!(borrowed.estimate().comparison_work(), 0);
        assert_eq!(borrowed.estimate().retained_nodes(), expected.node_count);
        assert!(borrowed.estimate().compute_work() > 0);
        assert!(old.estimate().comparison_work() > 0);
    }

    #[test]
    fn borrowed_retention_packed_numeric_inspection_is_independent_of_payload_length() {
        use mech_core::ValueDataDraft;

        let mut compute = None;
        for count in [1usize, 100_000] {
            let (value, schemas) = borrowed_retention_value(
                borrowed_retention_matrix(
                    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
                    count,
                ),
                ValueDataDraft::Matrix(vec![ValueDataDraft::U64(7); count].into_boxed_slice()),
            );
            let mut meter = ResidentBudgetMeter::for_closed_analysis(512, 0);
            let footprint =
                measure_borrowed_canonical_value_footprint(&mut meter, &value, &schemas).unwrap();
            assert_eq!(footprint, value.retained_footprint(&schemas).unwrap());
            assert_eq!(footprint.encoded_bytes, count as u64 * 8);
            assert_eq!(footprint.node_count, 3);
            assert_eq!(meter.estimate().comparison_work(), 0);
            let observed = meter.estimate().compute_work();
            assert_eq!(observed, BORROWED_FOOTPRINT_METADATA_WORK * 5);
            assert_eq!(*compute.get_or_insert(observed), observed);
        }
    }

    #[test]
    fn borrowed_retention_string_inspection_counts_headers_not_utf8_bytes() {
        use mech_core::ValueDataDraft;

        let mut scalar_work = None;
        for text in [String::new(), "e\u{301}é".to_owned(), "é".repeat(40_000)] {
            let bytes = text.len() as u64;
            let (value, schemas) =
                borrowed_retention_value(SchemaBody::String, ValueDataDraft::String(text));
            let mut meter = ResidentBudgetMeter::for_closed_analysis(512, 0);
            let footprint =
                measure_borrowed_canonical_value_footprint(&mut meter, &value, &schemas).unwrap();
            assert_eq!(footprint, value.retained_footprint(&schemas).unwrap());
            assert_eq!(footprint.encoded_bytes, bytes + 8);
            let observed = meter.estimate().compute_work();
            assert_eq!(observed, BORROWED_FOOTPRINT_METADATA_WORK * 3);
            assert_eq!(*scalar_work.get_or_insert(observed), observed);
        }
        let mut matrix_work = None;
        for strings in [
            vec![String::new(), String::new(), String::new()],
            vec!["e\u{301}".to_owned(), "é".to_owned(), String::new()],
            vec!["é".repeat(40_000), "👩‍👩‍👧‍👦".to_owned(), String::new()],
        ] {
            let bytes = strings.iter().map(|text| text.len() as u64).sum::<u64>();
            let (value, schemas) = borrowed_retention_value(
                borrowed_retention_matrix(SchemaBody::String, strings.len()),
                ValueDataDraft::Matrix(strings.into_iter().map(ValueDataDraft::String).collect()),
            );
            let mut meter = ResidentBudgetMeter::for_closed_analysis(1_024, 0);
            let footprint =
                measure_borrowed_canonical_value_footprint(&mut meter, &value, &schemas).unwrap();
            assert_eq!(footprint, value.retained_footprint(&schemas).unwrap());
            assert_eq!(footprint.encoded_bytes, bytes + 3 * 8);
            assert_eq!(footprint.node_count, 6);
            let observed = meter.estimate().compute_work();
            assert_eq!(observed, BORROWED_FOOTPRINT_METADATA_WORK * 8);
            assert_eq!(*matrix_work.get_or_insert(observed), observed);
        }
    }

    #[test]
    fn borrowed_retention_schema_inspection_denies_before_unfunded_tree_descent() {
        use mech_core::ValueDataDraft;

        let (value, schemas) = borrowed_retention_value(
            SchemaBody::Tuple(
                vec![SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64); 32]
                    .into_boxed_slice(),
            ),
            ValueDataDraft::Tuple(vec![ValueDataDraft::U64(1); 32].into_boxed_slice()),
        );
        let limit = BORROWED_FOOTPRINT_METADATA_WORK * 4;
        let mut denied = ResidentBudgetMeter::for_closed_analysis(limit, 0);
        assert_eq!(
            measure_borrowed_canonical_value_footprint(&mut denied, &value, &schemas),
            Err(ResidentKernelError::InvalidShape),
        );
        // No data chunk or retained node was visited after schema inspection
        // exhausted the allowance, and the failed charge is not accumulated.
        assert_eq!(denied.estimate().compute_work(), limit);
        assert_eq!(denied.estimate().comparison_work(), 0);
        assert_eq!(denied.estimate().retained_nodes(), 0);
        let mut recovered = ResidentBudgetMeter::for_closed_analysis(8_192, 0);
        let footprint =
            measure_borrowed_canonical_value_footprint(&mut recovered, &value, &schemas).unwrap();
        assert_eq!(footprint, value.retained_footprint(&schemas).unwrap());
        assert_eq!(footprint.node_count, 34);
        assert_eq!(recovered.estimate().comparison_work(), 0);
    }

    #[test]
    fn borrowed_retention_unproved_and_deep_schemas_keep_full_measurement_charges() {
        use mech_core::{ValueDataDraft, snapshot::OptionDraft};

        let reified = borrowed_retention_value(
            SchemaBody::ReifiedType,
            ValueDataDraft::Type(mech_core::snapshot::ReifiedTypeDraft::Kind {
                kind: mech_core::KindExpr::Index,
                dimension_parameters: Box::new([]),
            }),
        );
        let mut body = SchemaBody::Bool;
        let mut data = ValueDataDraft::Bool(true);
        for _ in 0..40 {
            body = SchemaBody::Option(Box::new(body));
            data = ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(data)),
            });
        }
        let deep = borrowed_retention_value(body, data);
        for (value, schemas) in [reified, deep] {
            let mut old = ResidentBudgetMeter::for_closed_analysis(16_384, 16_384);
            let expected = measure_canonical_value_footprint(&mut old, &value, &schemas).unwrap();
            let mut borrowed = ResidentBudgetMeter::for_closed_analysis(16_384, 16_384);
            assert_eq!(
                measure_borrowed_canonical_value_footprint(&mut borrowed, &value, &schemas)
                    .unwrap(),
                expected,
            );
            assert_eq!(
                borrowed.estimate().comparison_work(),
                old.estimate().comparison_work(),
            );
            assert!(borrowed.estimate().comparison_work() > 0);
            assert!(borrowed.estimate().compute_work() > old.estimate().compute_work());
            assert_eq!(borrowed.estimate().retained_nodes(), expected.node_count);
        }
        let mut meter = ResidentBudgetMeter::for_closed_analysis(128, 0);
        assert!(!borrowed_footprint_schema_supported(&mut meter, &SchemaBody::Dynamic, 0).unwrap());
    }

    #[test]
    fn borrowed_retention_never_authorizes_real_comparison_or_finalization_scans() {
        use mech_core::ValueDataDraft;

        let (value, schemas) = borrowed_retention_value(
            SchemaBody::String,
            ValueDataDraft::String("x".repeat(MAX_RESIDENT_COMPARISON_WORK as usize + 1)),
        );
        let body = schemas.get(value.schema()).unwrap().body();
        let mut metadata = ResidentBudgetMeter::for_closed_analysis(512, 0);
        let measured =
            measure_borrowed_canonical_data_footprint(&mut metadata, body, value.data()).unwrap();
        assert_eq!(measured.encoded_bytes, MAX_RESIDENT_COMPARISON_WORK + 9);
        assert_eq!(metadata.estimate().comparison_work(), 0);
        let mut compare = ResidentBudgetMeter::for_closed_analysis(
            MAX_RESIDENT_COMPUTE_WORK,
            MAX_RESIDENT_COMPARISON_WORK,
        );
        assert_eq!(
            measure_canonical_data_comparison_work(&mut compare, body, value.data()),
            Err(ResidentKernelError::InvalidShape),
        );
        // String encoding consumes compute, while ordered-key comparisons
        // keep their independent allowance. Metadata alone grants neither.
        let mut finalization = ResidentBudgetMeter::for_closed_analysis(
            MAX_RESIDENT_COMPARISON_WORK,
            MAX_RESIDENT_COMPARISON_WORK,
        );
        assert_eq!(
            preflight_canonical_data_finalization(&mut finalization, body, value.data()),
            Err(ResidentKernelError::InvalidShape),
        );
        let mut funded_finalization =
            ResidentBudgetMeter::for_closed_analysis(MAX_RESIDENT_COMPUTE_WORK, 0);
        assert_eq!(
            preflight_canonical_data_finalization(&mut funded_finalization, body, value.data()),
            Ok(0),
        );
        assert_eq!(funded_finalization.estimate().comparison_work(), 0);
        assert!(funded_finalization.estimate().compute_work() >= measured.encoded_bytes);
        let mut clone = ResidentBudgetMeter::for_closed_analysis(
            MAX_RESIDENT_COMPUTE_WORK,
            MAX_RESIDENT_COMPARISON_WORK,
        );
        assert_eq!(
            charge_canonical_value_footprint(&mut clone, &value, &schemas),
            Err(ResidentKernelError::InvalidShape),
        );
    }

    #[test]
    fn ordinary_admission_retains_existing_and_recomputed_resource_violations() {
        for (index, dimension) in [
            mech_core::MemoryBudgetDimension::TemporaryBytes,
            mech_core::MemoryBudgetDimension::ClonedBytes,
            mech_core::MemoryBudgetDimension::RetainedNodes,
            mech_core::MemoryBudgetDimension::ComparisonWork,
            mech_core::MemoryBudgetDimension::ComputeWork,
        ]
        .into_iter()
        .enumerate()
        {
            let mut plan = detached_turn_plan(ResourceDemand::default()).unwrap();
            plan.node = mech_core::NodeId::new(17);
            match index {
                0 => {
                    plan.demand.turn_peak_bytes = 42;
                    plan.budget_limits.max_temporary_bytes = Some(41);
                }
                1 => {
                    plan.demand.cloned_bytes = 42;
                    plan.budget_limits.max_cloned_bytes = Some(41);
                }
                2 => {
                    plan.demand.retained_nodes = 42;
                    plan.budget_limits.max_retained_nodes = Some(41);
                }
                3 => {
                    plan.demand.work.comparison = 42;
                    plan.budget_limits.max_comparison_work = Some(41);
                }
                4 => {
                    plan.demand.work.compute = 42;
                    plan.budget_limits.max_compute_work = Some(41);
                }
                _ => unreachable!(),
            }
            let expected = ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension,
                required: 42,
                limit: 41,
            };
            let mut precomputed = plan.clone();
            precomputed.budget_violations = vec![mech_core::MemoryBudgetViolation {
                owner: mech_core::MemoryObjectOwner::NodeOutput {
                    node: plan.node,
                    port: 0,
                },
                dimension,
                required: 42,
                limit: 41,
            }]
            .into_boxed_slice();
            assert_eq!(
                ResidentBudgetPermit::from_turn_plan(precomputed).map(drop),
                Err(expected.clone())
            );
            assert_eq!(
                ResidentBudgetPermit::from_turn_plan(plan).map(drop),
                Err(expected)
            );
        }
    }

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
                    Err(ResidentKernelError::ResourceLimit {
                        phase: mech_core::ResidentResourcePhase::Admission,
                        dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                        required,
                        limit,
                    }) if required == MAX_RESIDENT_COMPUTE_WORK + 2
                        && limit == MAX_RESIDENT_COMPUTE_WORK
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
                    Err(ResidentKernelError::ResourceLimit {
                        phase: mech_core::ResidentResourcePhase::Admission,
                        dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                        required,
                        limit,
                    }) if required == MAX_RESIDENT_COMPARISON_WORK + 2
                        && limit == MAX_RESIDENT_COMPARISON_WORK
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
            Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::OutputBytes,
                required: 24,
                limit: 23,
            })
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
            ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::ClonedBytes,
                required: 16777217,
                limit: 16777216
            }
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
            ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::TemporaryBytes,
                required: 16777217,
                limit: 16777216
            }
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
            ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::TemporaryBytes,
                required: 18446744073709551615,
                limit: 16777216
            }
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
            Err(ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
                required: 65537,
                limit: 65536
            }),
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
    fn incremental_finalization_uses_large_finite_plan_and_rejects_actual_cap() {
        let mut plan = detached_turn_plan(ResourceDemand::default()).unwrap();
        plan.budget_limits.max_compute_work = Some(MAX_RESIDENT_COMPUTE_WORK + 4096);
        plan.budget_limits.max_comparison_work = Some(4096);
        let prefix = MAX_RESIDENT_COMPUTE_WORK + 1;
        let schema = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64);
        let value = ValueData::U64(37);
        with_resident_turn_plan(plan.clone(), || {
            let mut meter = ResidentBudgetMeter::default();
            meter.charge_compute_work(prefix).unwrap();
            preflight_canonical_data_finalization(&mut meter, &schema, &value).unwrap();
            assert!(meter.estimate().compute_work() > prefix);
        });
        plan.budget_limits.max_compute_work = Some(prefix);
        with_resident_turn_plan(plan, || {
            let mut meter = ResidentBudgetMeter::default();
            meter.charge_compute_work(prefix).unwrap();
            assert!(matches!(preflight_canonical_data_finalization(&mut meter, &schema, &value),
                Err(ResidentKernelError::ResourceLimit {
                    dimension: mech_core::MemoryBudgetDimension::ComputeWork, ..
                })));
            assert_eq!(meter.estimate().compute_work(), prefix);
        });
    }

    #[test]
    fn incremental_allowance_keeps_finite_defaults_for_unspecified_plan_bounds() {
        let mut plan = detached_turn_plan(ResourceDemand::default()).unwrap();
        plan.budget_limits.max_compute_work = None;
        plan.budget_limits.max_comparison_work = None;
        with_resident_turn_plan(plan, || {
            assert_eq!(KernelCostEstimate::default().remaining_incremental_work().unwrap(),
                MAX_RESIDENT_COMPUTE_WORK.min(MAX_RESIDENT_COMPARISON_WORK));
        });
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
            ResidentKernelError::ResourceLimit {
                phase: mech_core::ResidentResourcePhase::Admission,
                dimension: mech_core::MemoryBudgetDimension::RetainedNodes,
                required: 65537,
                limit: 65536
            },
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

#[cfg(all(test, feature = "native-artifact"))]
#[path = "native_projection_meter_tests.rs"]
mod native_projection_meter_tests;
