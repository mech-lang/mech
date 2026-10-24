//! Transferable finite fuel loans; execution state and call observers stay local.
use super::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{MutexGuard, TryLockError};

// Retry only short ownership transfers. A bounded exponential pause avoids
// repeatedly contending on one cache line before the original blocking lock.
const ESCROW_SPIN_ROUNDS: u32 = 12;
#[cfg(test)]
thread_local! {
    // Attempts, pause hints, blocking fallbacks, poison recoveries.
    static ESCROW_LOCK_STATS: std::cell::Cell<(u32, u32, u32, u32)> = const { std::cell::Cell::new((0, 0, 0, 0)) };
    static ESCROW_PARK_NOTIFY: RefCell<Option<std::sync::mpsc::Sender<()>>> = const { RefCell::new(None) };
}
fn lock_escrow(shared: &Mutex<Escrow>) -> MutexGuard<'_, Escrow> {
    for round in 0..ESCROW_SPIN_ROUNDS {
        #[cfg(test)]
        ESCROW_LOCK_STATS.with(|v| { let (a,b,c,d) = v.get(); v.set((a+1,b,c,d)); });
        match shared.try_lock() {
            Ok(guard) => return guard,
            Err(TryLockError::Poisoned(error)) => {
                // Retain the existing recovery policy and successful debits.
                #[cfg(test)]
                ESCROW_LOCK_STATS.with(|v| { let (a,b,c,d) = v.get(); v.set((a,b,c,d+1)); });
                return error.into_inner();
            }
            Err(TryLockError::WouldBlock) => {
                for _ in 0..(1u32 << round) { std::hint::spin_loop(); }
                #[cfg(test)]
                ESCROW_LOCK_STATS.with(|v| { let (a,b,c,d) = v.get(); v.set((a,b+(1<<round),c,d)); });
            }
        }
    }
    #[cfg(test)]
    {
        ESCROW_LOCK_STATS.with(|v| { let (a,b,c,d) = v.get(); v.set((a,b,c+1,d)); });
        ESCROW_PARK_NOTIFY.with(|v| {
            if let Some(notify) = v.borrow_mut().take() { let _ = notify.send(()); }
        });
    }
    shared.lock().unwrap_or_else(|error| error.into_inner())
}


#[derive(Debug)]
pub enum NativeWorkerBudgetError {
    Work(ResidentKernelError),
    Memory(mech_core::MemoryRuntimeError),
}
impl From<ResidentKernelError> for NativeWorkerBudgetError {
    fn from(error: ResidentKernelError) -> Self { Self::Work(error) }
}
impl From<mech_core::MemoryRuntimeError> for NativeWorkerBudgetError {
    fn from(error: mech_core::MemoryRuntimeError) -> Self { Self::Memory(error) }
}

#[derive(Debug)]
struct Escrow {
    limit: ControlWork,
    used: ControlWork,
    live: u64,
    sealed: bool,
    // The shared allocation remains admitted until the final worker retires,
    // including when its parent closes early after an error.
    _reservation: mech_core::ManagedMemoryReservation,
}
impl Escrow {
    fn next(&self, compute: u64, comparison: u64) -> Result<ControlWork, ResidentKernelError> {
        if self.sealed { return Err(ResidentKernelError::InvalidInput); }
        let next = ControlWork {
            compute: self.used.compute.checked_add(compute).ok_or(ResidentKernelError::InvalidShape)?,
            comparison: self.used.comparison.checked_add(comparison).ok_or(ResidentKernelError::InvalidShape)?,
        };
        for (required, limit, dimension) in [
            (next.compute, self.limit.compute, MemoryBudgetDimension::ComputeWork),
            (next.comparison, self.limit.comparison, MemoryBudgetDimension::ComparisonWork),
        ] {
            if required > limit {
                return Err(ResidentKernelError::ResourceLimit {
                    phase: mech_core::ResidentResourcePhase::Admission, dimension, required, limit,
                });
            }
        }
        Ok(next)
    }
}

/// Parent-thread reservation of existing fuel. This handle is deliberately not
/// Send: the parent Rc ledger is reconciled on its owning thread. Worker tokens
/// carry only admitted backing and a synchronized escrow, never that Rc.
#[derive(Debug)]
pub struct NativeWorkGroup {
    parent: Rc<RefCell<ControlWorkAccount>>,
    shared: Arc<Mutex<Escrow>>,
    memory: mech_core::ManagedMemoryBudget,
    allowance: ControlWork,
    closed: bool,
}

/// One non-cloneable, Send worker authorization. Consuming it constructs a
/// fresh *local* observer/account backed by the parent's finite reservation.
/// Clones of the resulting local budget share its Rc, not another allowance.
#[derive(Debug)]
pub struct NativeWorkerWorkLoan {
    shared: Arc<Mutex<Escrow>>,
    reservation: Option<mech_core::ManagedMemoryReservation>,
}
impl NativeWorkerWorkLoan {
    pub(super) fn remaining(&self) -> Option<(u64, u64)> {
        let state = lock_escrow(&self.shared);
        if state.sealed { return None; }
        Some((state.limit.compute - state.used.compute, state.limit.comparison - state.used.comparison))
    }
    pub(super) fn preview(&self, compute: u64, comparison: u64) -> Result<(), ResidentKernelError> {
        lock_escrow(&self.shared).next(compute, comparison).map(drop)
    }
    pub(super) fn charge(&self, compute: u64, comparison: u64) -> Result<(), ResidentKernelError> {
        let mut state = lock_escrow(&self.shared);
        state.used = state.next(compute, comparison)?;
        Ok(())
    }
}
impl Drop for NativeWorkerWorkLoan {
    fn drop(&mut self) {
        let mut state = lock_escrow(&self.shared);
        state.live = state.live.checked_sub(1).expect("worker loan retired once");
    }
}

impl NativeLibraryWorkBudget {
    /// Decide a synchronous debit from this account's remaining allowance and
    /// commit both dimensions before returning the result. A worker holds its
    /// existing escrow mutex once across the decision and commit. No fuel is
    /// reserved, renewed or refunded, and all local/root limits still apply.
    ///
    /// Like `unscoped_remaining`, active call scopes and sealed groups return
    /// None without invoking `prepare`. The callback must be bounded, pure
    /// scalar work: no allocation, blocking, host calls or account access. Its
    /// error, a denied debit or an unwind leaves both ledgers unchanged. A
    /// successful callback returns (compute debit, comparison debit, result).
    pub fn with_unscoped_debit<T, E>(
        &self, prepare: impl FnOnce((u64, u64)) -> Result<(u64, u64, T), E>,
    ) -> Result<Option<T>, E>
    where E: From<ResidentKernelError> {
        if CONTROL_WORK.with(|v| v.borrow().is_some())
            || CONTROL_WORK_SCOPE.with(|v| v.borrow().is_some())
        { return Ok(None); }
        let mut account = self.account.try_borrow_mut()
            .map_err(|_| E::from(ResidentKernelError::InvalidInput))?;
        let account = &mut *account;
        let Some(local) = unscoped_local_remaining(account) else { return Ok(None) };
        if let Some(worker) = &account.worker {
            let mut state = lock_escrow(&worker.shared);
            if state.sealed { return Ok(None); }
            let remaining = (local.0.min(state.limit.compute - state.used.compute),
                local.1.min(state.limit.comparison - state.used.comparison));
            let (compute, comparison, result) = prepare(remaining)?;
            let next = checked_local_control_account_work(account, compute, comparison)?;
            let shared_next = state.next(compute, comparison)?;
            state.used = shared_next;
            account.used = next;
            Ok(Some(result))
        } else {
            let (compute, comparison, result) = prepare(local)?;
            let next = checked_local_control_account_work(account, compute, comparison)?;
            account.used = next;
            Ok(Some(result))
        }
    }

    /// Reserve existing invocation fuel, also bounded by an enclosing call's
    /// remaining allowance. The parent must join workers before publication.
    /// Outstanding reservations count against parent charges/restrictions.
    /// Nested parallel groups currently decline; child code may run serially.
    /// As with `new`, this admits metadata but grants no call permission; the
    /// native entry must account for its ordinary setup/execution work.
    pub fn reserve_worker_group(
        &self, compute: u64, comparison: u64,
    ) -> Result<NativeWorkGroup, NativeWorkerBudgetError> {
        self.reserve_worker_group_with_memory(compute, comparison, &self.memory_budget())
    }

    /// Also impose the caller's same-account memory ceiling on group metadata,
    /// before allocation. Workers inherit this view even if given a wider one.
    pub fn reserve_worker_group_with_memory(
        &self, compute: u64, comparison: u64, memory: &mech_core::ManagedMemoryBudget,
    ) -> Result<NativeWorkGroup, NativeWorkerBudgetError> {
        self.charge(0, 0)?; // Reject a foreign active account before allocation.
        let mut account = self.account.borrow_mut();
        if account.worker.is_some() { return Err(ResidentKernelError::InvalidInput.into()); }
        checked_control_account_work(&account, compute, comparison)?;
        let reserved = ControlWork {
            compute: account.reserved.compute.checked_add(compute).ok_or(ResidentKernelError::InvalidShape)?,
            comparison: account.reserved.comparison.checked_add(comparison).ok_or(ResidentKernelError::InvalidShape)?,
        };
        let admitted = account._reservation.as_ref().ok_or(ResidentKernelError::InvalidInput)?.budget();
        if admitted != *memory { return Err(ResidentKernelError::InvalidInput.into()); }
        let memory = admitted.with_limits(memory.limit_bytes(), memory.limit_nodes());
        let bytes = (core::mem::size_of::<Mutex<Escrow>>() + 2 * core::mem::size_of::<usize>()) as u64;
        let reservation = memory.reserve_capacity(bytes)?;
        let allowance = ControlWork { compute, comparison };
        let shared = Arc::new(Mutex::new(Escrow {
            limit: allowance, used: ControlWork::default(), live: 0, sealed: false,
            _reservation: reservation,
        }));
        account.reserved = reserved;
        Ok(NativeWorkGroup { parent: self.account.clone(), shared, memory, allowance, closed: false })
    }

    /// Consumes one transferable token on its destination thread. No root fuel
    /// is created and no parent TLS observer is transferred. Local call limits
    /// therefore observe only this worker, while every debit reaches escrow.
    pub fn from_worker_loan(mut loan: NativeWorkerWorkLoan) -> Result<Self, ResidentKernelError> {
        if Self::has_active_scope() { return Err(ResidentKernelError::InvalidInput); }
        let limits = {
            let state = lock_escrow(&loan.shared);
            if state.sealed { return Err(ResidentKernelError::InvalidInput); }
            state.limit
        };
        let reservation = loan.reservation.take().ok_or(ResidentKernelError::InvalidInput)?;
        Ok(Self { account: Rc::new(RefCell::new(ControlWorkAccount {
            identity: next_control_work_identity(), used: ControlWork::default(),
            reserved: ControlWork::default(), compute_limit: limits.compute,
            comparison_limit: limits.comparison, _reservation: Some(reservation), worker: Some(loan),
        })) })
    }

    /// Origin check for a worker meter: attaching it must not mint a new root.
    pub fn is_worker_budget(&self) -> bool { self.account.borrow().worker.is_some() }
    pub fn limits(&self) -> (u64, u64) {
        let account = self.account.borrow();
        (account.compute_limit, account.comparison_limit)
    }
}

impl NativeWorkGroup {
    /// Admit each worker's local ledger before dispatch. The token is single
    /// use; failed dispatch can simply drop it without spending or renewing fuel.
    pub fn worker_loan(&self) -> Result<NativeWorkerWorkLoan, NativeWorkerBudgetError> {
        self.worker_loan_with_memory(&self.memory)
    }

    /// The same-account capped view must follow the worker into all native and
    /// maintained allocations. A different account cannot rehome its backing.
    pub fn worker_loan_with_memory(
        &self, memory: &mech_core::ManagedMemoryBudget,
    ) -> Result<NativeWorkerWorkLoan, NativeWorkerBudgetError> {
        if *memory != self.memory { return Err(ResidentKernelError::InvalidInput.into()); }
        let memory = self.memory.with_limits(memory.limit_bytes(), memory.limit_nodes());
        let bytes = (core::mem::size_of::<RefCell<ControlWorkAccount>>()
            + 2 * core::mem::size_of::<usize>()) as u64;
        let reservation = memory.reserve_capacity(bytes)?;
        let mut state = lock_escrow(&self.shared);
        if state.sealed { return Err(ResidentKernelError::InvalidInput.into()); }
        let live = state.live.checked_add(1).ok_or(ResidentKernelError::InvalidShape)?;
        state.live = live;
        Ok(NativeWorkerWorkLoan { shared: self.shared.clone(), reservation: Some(reservation) })
    }

    /// Reconcile after all worker budgets/clones/tokens have retired. On error,
    /// Drop seals the group and still retains every successful debit. Unused
    /// reservation is released; spent work is never refunded.
    pub fn finish(mut self) -> Result<(u64, u64), ResidentKernelError> {
        if lock_escrow(&self.shared).live != 0 {
            return Err(ResidentKernelError::InvalidInput);
        }
        Ok(self.close())
    }

    fn close(&mut self) -> (u64, u64) {
        let mut state = lock_escrow(&self.shared);
        if !self.closed {
            state.sealed = true;
            let mut parent = self.parent.borrow_mut();
            parent.reserved.compute = parent.reserved.compute.checked_sub(self.allowance.compute)
                .expect("group owns its parent reservation");
            parent.reserved.comparison = parent.reserved.comparison.checked_sub(self.allowance.comparison)
                .expect("group owns its parent reservation");
            parent.used.compute = parent.used.compute.checked_add(state.used.compute)
                .expect("worker debit fits previously reserved fuel");
            parent.used.comparison = parent.used.comparison.checked_add(state.used.comparison)
                .expect("worker debit fits previously reserved fuel");
            self.closed = true;
        }
        (state.used.compute, state.used.comparison)
    }
}
impl Drop for NativeWorkGroup {
    fn drop(&mut self) { self.close(); }
}

#[path = "native_work_debit_batch.rs"]
mod debit_batch;
pub use debit_batch::NativeWorkDebitBatch;

#[cfg(test)]
#[path = "native_worker_budget_tests.rs"]
mod tests;
