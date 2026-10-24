//! Bounded lock reuse for allocation-free, callback-free leaf work.
use super::*;
use std::cell::RefMut;
use std::sync::MutexGuard;

const MAX_CHARGES: u32 = 32;
const MAX_WORK: u64 = 32 * 1024;

/// A synchronous leaf meter over the original account. Each charge is still
/// checked and committed individually; this neither borrows nor prepays fuel.
/// It cannot be cloned, transferred, or retained beyond its callback.
pub struct NativeWorkDebitBatch<'a> {
    mode: Mode<'a>,
}
enum Mode<'a> {
    Ordinary(&'a NativeLibraryWorkBudget),
    Worker(Worker<'a>),
}
struct Worker<'a> {
    account: RefMut<'a, ControlWorkAccount>,
    shared: &'a Mutex<Escrow>,
    guard: Option<MutexGuard<'a, Escrow>>,
    charges: u32,
    work: u64,
}
impl Worker<'_> {
    fn release(&mut self) {
        self.guard.take();
        self.charges = 0;
        self.work = 0;
    }

    fn charge(&mut self, compute: u64, comparison: u64) -> Result<(), ResidentKernelError> {
        let result = (|| {
            // Local invocation/call caps are checked before changing either
            // ledger, even if this worker already owns the shared mutex.
            let next = checked_local_control_account_work(&self.account, compute, comparison)?;
            let shared = self.shared;
            let state = self.guard.get_or_insert_with(||
                lock_escrow(shared));
            let shared_next = state.next(compute, comparison)?;
            state.used = shared_next;
            self.account.used = next;
            self.charges += 1;
            self.work = self.work.saturating_add(compute).saturating_add(comparison);
            Ok(())
        })();
        // Release BEFORE the leaf performs the newly paid work. A very large
        // key comparison therefore runs without holding another worker up.
        // Zero-cost calls also yield after a fixed number of original debits.
        if result.is_err() || self.charges >= MAX_CHARGES || self.work >= MAX_WORK {
            self.release();
        }
        result
    }
}
impl Drop for Worker<'_> {
    fn drop(&mut self) { self.release(); }
}
impl NativeWorkDebitBatch<'_> {
    pub fn charge(&mut self, compute: u64, comparison: u64) -> Result<(), ResidentKernelError> {
        match &mut self.mode {
            Mode::Ordinary(work) => work.charge(compute, comparison),
            Mode::Worker(work) => work.charge(compute, comparison),
        }
    }
}

impl NativeLibraryWorkBudget {
    /// Reuse the shared worker lock for bounded groups of original debits.
    ///
    /// The callback must contain only allocation-free, callback-free leaf
    /// work paid through this meter. It must not access another budget/account,
    /// allocate, invoke host code, block, or enter another call scope. Ordinary
    /// calls retain their existing path; only a worker uses a held lock. The
    /// enclosing account and call limits remain installed and unchanged.
    ///
    /// No allowance is reserved: unused fuel is always available to peers.
    /// Each accepted debit reaches both ledgers immediately, and early return,
    /// denial or unwind drops the lock without refunding successful work.
    pub fn with_debit_batch<T>(
        &self,
        execute: impl FnOnce(&mut NativeWorkDebitBatch<'_>) -> T,
    ) -> Result<T, ResidentKernelError> {
        self.with_scope(|| {
            let shared = {
                let account = self.account.try_borrow().map_err(|_| ResidentKernelError::InvalidInput)?;
                account.worker.as_ref().map(|worker| worker.shared.clone())
            };
            let Some(shared) = shared else {
                let mut batch = NativeWorkDebitBatch { mode: Mode::Ordinary(self) };
                return Ok(execute(&mut batch));
            };
            let account = self.account.try_borrow_mut().map_err(|_| ResidentKernelError::InvalidInput)?;
            let mut batch = NativeWorkDebitBatch { mode: Mode::Worker(Worker {
                account, shared: &shared, guard: None, charges: 0, work: 0,
            }) };
            Ok(execute(&mut batch))
        })?
    }
}

#[cfg(test)]
#[path = "native_work_debit_batch_tests.rs"]
mod tests;
