//! Disjoint finite fuel for callback-free native code on a worker thread.
use super::*;

/// One thread-local native reservation. A normal exit validates its generated
/// fuel delta and commits that spent work; only unspent reservation is released.
/// Abandonment charges the full grant, including if the parent closes early.
/// This handle retains the worker's existing account and admitted metadata.
#[derive(Debug)]
pub struct NativeWorkerNativeFuel {
    account: Rc<RefCell<ControlWorkAccount>>,
    shared: Arc<Mutex<Escrow>>,
    allowance: ControlWork,
    settled: bool,
}

impl NativeLibraryWorkBudget {
    /// Reserve a share of currently free parent fuel. This grants no source
    /// execution or publication authority. Tight/active scopes decline so the
    /// original individually admitted path remains available.
    pub fn reserve_native_worker_fuel(
        &self,
    ) -> Result<Option<NativeWorkerNativeFuel>, ResidentKernelError> {
        if with_control_work(|v| v.borrow().is_some())
            || with_control_work_scope(|v| v.borrow().is_some())
        {
            return Ok(None);
        }
        let mut account = self
            .account
            .try_borrow_mut()
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        let Some(local) = unscoped_local_remaining(&account) else {
            return Ok(None);
        };
        let Some(worker) = &account.worker else {
            return Ok(None);
        };
        let shared = worker.shared.clone();
        let mut state = lock_escrow(&shared);
        if state.sealed || state.live == 0 {
            return Ok(None);
        }
        state.settle_credits();
        let free = state.free();
        let allowance = ControlWork {
            compute: local.0.min(free.0 / state.live),
            comparison: local.1.min(free.1 / state.live),
        };
        // A zero grant is valid: generated code takes its original unfunded
        // header without executing a body, then releases this empty loan.
        // Each product/sum is bounded by this exact account's free remainder.
        account.reserved.compute += allowance.compute;
        account.reserved.comparison += allowance.comparison;
        state.native_reserved.compute += allowance.compute;
        state.native_reserved.comparison += allowance.comparison;
        drop(state);
        Ok(Some(NativeWorkerNativeFuel {
            account: self.account.clone(),
            shared,
            allowance,
            settled: false,
        }))
    }
}

impl NativeWorkerNativeFuel {
    pub fn allowance(&self) -> (u64, u64) {
        (self.allowance.compute, self.allowance.comparison)
    }
    pub fn shares_account(&self, budget: &NativeLibraryWorkBudget) -> bool {
        Rc::ptr_eq(&self.account, &budget.account)
    }
    pub fn finish(mut self, compute: u64, comparison: u64) -> Result<(), ResidentKernelError> {
        if compute > self.allowance.compute || comparison > self.allowance.comparison {
            return Err(ResidentKernelError::InvalidInput);
        }
        self.settle(ControlWork {
            compute,
            comparison,
        });
        Ok(())
    }
    fn settle(&mut self, mut used: ControlWork) {
        let mut account = self.account.borrow_mut();
        let mut shared = lock_escrow(&self.shared);
        account.reserved.compute -= self.allowance.compute;
        account.reserved.comparison -= self.allowance.comparison;
        if shared.sealed {
            // Group close already charged every outstanding grant in full.
            used = self.allowance;
        } else {
            shared.native_reserved.compute -= self.allowance.compute;
            shared.native_reserved.comparison -= self.allowance.comparison;
            shared.used.compute += used.compute;
            shared.used.comparison += used.comparison;
        }
        account.used.compute += used.compute;
        account.used.comparison += used.comparison;
        shared.native_changed.notify_all();
        self.settled = true;
    }
}
impl Drop for NativeWorkerNativeFuel {
    fn drop(&mut self) {
        if !self.settled {
            self.settle(self.allowance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn native_worker_fuel_is_disjoint_finite_and_reconciles_only_actual_spending() {
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
        let root = NativeLibraryWorkBudget::new(600, 100, &memory).unwrap();
        assert!(root.reserve_native_worker_fuel().unwrap().is_none());
        let group = root.reserve_worker_group(500, 50).unwrap();
        let loans = [group.worker_loan().unwrap(), group.worker_loan().unwrap()];
        let barrier = Arc::new(Barrier::new(2));
        let handles = loans.map(|loan| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
                let fuel = worker.reserve_native_worker_fuel().unwrap().unwrap();
                let allowance = fuel.allowance();
                assert!(allowance.0 >= 30 && allowance.1 >= 3);
                assert!(fuel.shares_account(&worker.clone()));
                barrier.wait();
                assert!(worker.charge(501, 0).is_err());
                assert_eq!(worker.used(), (0, 0));
                fuel.finish(30, 3).unwrap();
                assert_eq!(worker.used(), (30, 3));
                allowance
            })
        });
        let [a, b] = handles.map(|handle| handle.join().unwrap());
        assert!(a.0 + b.0 <= 500 && a.1 + b.1 <= 50);
        assert_eq!(group.finish().unwrap(), (60, 6));
        assert_eq!(root.used(), (60, 6));
        drop(root);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }

    #[test]
    fn native_worker_fuel_abandonment_and_invalid_deltas_never_refund_spent_fuel() {
        for invalid in [false, true] {
            let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
            let root = NativeLibraryWorkBudget::new(500, 50, &memory).unwrap();
            let group = root.reserve_worker_group(500, 50).unwrap();
            let worker =
                NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
            let fuel = worker.reserve_native_worker_fuel().unwrap().unwrap();
            assert_eq!(fuel.allowance(), (500, 50));
            if invalid {
                assert!(fuel.finish(501, 0).is_err());
            } else {
                drop(fuel);
            }
            assert_eq!(worker.used(), (500, 50));
            assert!(worker.charge(1, 0).is_err());
            drop(worker);
            assert_eq!(group.finish().unwrap(), (500, 50));
            assert_eq!(root.used(), (500, 50));
            drop(root);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }

    #[test]
    fn native_worker_fuel_early_group_close_conservatively_keeps_outstanding_grant() {
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
        let root = NativeLibraryWorkBudget::new(500, 50, &memory).unwrap();
        let group = root.reserve_worker_group(500, 50).unwrap();
        let worker =
            NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
        let fuel = worker.reserve_native_worker_fuel().unwrap().unwrap();
        assert!(group.finish().is_err());
        assert_eq!(root.used(), (500, 50));
        fuel.finish(0, 0).unwrap();
        assert_eq!(worker.used(), (500, 50));
        assert!(worker.charge(1, 0).is_err());
        drop((worker, root));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }

    #[test]
    fn native_worker_fuel_settles_existing_credits_and_preserves_both_dimensions() {
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
        let root = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
        let group = root.reserve_worker_group(1000, 1000).unwrap();
        let worker =
            NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
        for _ in 0..10 {
            assert!(worker.try_charge_funded_batch(1, 1).unwrap());
        }
        let fuel = worker.reserve_native_worker_fuel().unwrap().unwrap();
        assert_eq!(fuel.allowance(), (990, 990));
        fuel.finish(5, 7).unwrap();
        assert_eq!(worker.used(), (15, 17));
        drop(worker);
        assert_eq!(group.finish().unwrap(), (15, 17));
        drop(root);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }

    #[test]
    fn native_worker_fuel_zero_allowance_can_only_settle_zero() {
        let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
        let root = NativeLibraryWorkBudget::new(0, 0, &memory).unwrap();
        let group = root.reserve_worker_group(0, 0).unwrap();
        let worker =
            NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
        let fuel = worker.reserve_native_worker_fuel().unwrap().unwrap();
        assert_eq!(fuel.allowance(), (0, 0));
        fuel.finish(0, 0).unwrap();
        drop(worker);
        assert_eq!(group.finish().unwrap(), (0, 0));
        drop(root);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }

    #[test]
    fn native_worker_fuel_pressure_waits_for_unspent_fuel_before_denial() {
        for mode in 0..3 {
            let memory = mech_core::ManagedMemoryBudget::new(1 << 20);
            let root = NativeLibraryWorkBudget::new(100, 100, &memory).unwrap();
            let group = root.reserve_worker_group(100, 100).unwrap();
            let first = group.worker_loan().unwrap();
            let second = group.worker_loan().unwrap();
            let (holding, held) = std::sync::mpsc::channel();
            let (release, released) = std::sync::mpsc::channel();
            let a = std::thread::spawn(move || {
                let worker = NativeLibraryWorkBudget::from_worker_loan(first).unwrap();
                let fuel = worker.reserve_native_worker_fuel().unwrap().unwrap();
                assert_eq!(fuel.allowance(), (50, 50));
                holding.send(()).unwrap();
                released.recv().unwrap();
                fuel.finish(10, 0).unwrap();
            });
            held.recv().unwrap();
            let (waiting, wait) = std::sync::mpsc::channel();
            let b = std::thread::spawn(move || {
                let worker = NativeLibraryWorkBudget::from_worker_loan(second).unwrap();
                NATIVE_FUEL_WAIT_NOTIFY.with(|slot| *slot.borrow_mut() = Some(waiting));
                match mode {
                    0 => worker.charge(80, 0).unwrap(),
                    1 => worker
                        .with_debit_batch(|meter| meter.charge(80, 0))
                        .unwrap()
                        .unwrap(),
                    _ => {
                        let result: Result<Option<()>, ResidentKernelError> = worker
                            .with_unscoped_debit(|remaining| {
                                assert!(remaining.0 >= 80);
                                Ok((80, 0, ()))
                            });
                        assert!(result.unwrap().is_some());
                    }
                }
            });
            wait.recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            release.send(()).unwrap();
            a.join().unwrap();
            b.join().unwrap();
            assert_eq!(group.finish().unwrap(), (90, 0));
            assert_eq!(root.used(), (90, 0));
            drop(root);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
