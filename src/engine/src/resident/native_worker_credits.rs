//! Revocable, paired finite credits for repeated, unchanged funded batches.
//! A single atomic count represents both original work dimensions together.
use super::*;
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

const SLOTS: usize = 32;
const SEALED: u64 = 1 << 63;
const BATCHES: u64 = 64;
pub(super) const HEADROOM: u64 = 4;

// Independent workers must not write the same cache line. Apple silicon uses
// 128-byte lines; this also separates conventional 64-byte cache lines.
#[derive(Debug)]
#[repr(align(128))]
struct Slot(AtomicU64);
#[derive(Debug)]
pub(super) struct Pool {
    slots: [Slot; SLOTS],
}
#[derive(Clone, Copy, Debug)]
struct Quote {
    compute: u64,
    comparison: u64,
    capacity: u64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeWorkerCreditStatistics {
    pub claims: u64,
    pub batches: u64,
    pub reclaims: u64,
}
#[derive(Debug)]
pub(super) struct Book {
    pub pool: Arc<Pool>,
    registered: u32,
    quotes: [Option<Quote>; SLOTS],
    pub(super) reserved: ControlWork,
    pub statistics: NativeWorkerCreditStatistics,
}
impl Book {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Pool {
                slots: std::array::from_fn(|_| Slot(AtomicU64::new(SEALED))),
            }),
            registered: 0,
            quotes: [None; SLOTS],
            reserved: ControlWork::default(),
            statistics: NativeWorkerCreditStatistics::default(),
        }
    }
    pub fn backing_bytes() -> u64 {
        // ArcInner's two counters precede an aligned payload. Include its
        // padding rather than undercharging this cache-line-aligned pool.
        std::alloc::Layout::new::<[usize; 2]>()
            .extend(std::alloc::Layout::new::<Pool>())
            .unwrap()
            .0
            .pad_to_align()
            .size() as u64
    }
}
#[derive(Debug)]
pub(super) struct Loan {
    pool: Arc<Pool>,
    slot: usize,
    // Only the unique worker thread resets a sealed slot, under the escrow
    // mutex and after its previous atomic attempt has completed. Reclaimers
    // only seal: no old attempt can see an unsealed replacement (ABA).
    quote: Cell<Option<Quote>>,
}
impl Loan {
    pub fn charge(&self, compute: u64, comparison: u64) -> bool {
        let Some(quote) = self.quote.get() else {
            return false;
        };
        if (quote.compute, quote.comparison) != (compute, comparison) {
            return false;
        }
        let word = &self.pool.slots[self.slot].0;
        let used = word.load(Ordering::Acquire);
        if used & SEALED != 0 || used > quote.capacity - HEADROOM {
            return false;
        }
        // One linearization point commits BOTH dimensions before funded code.
        word.compare_exchange(used, used + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}
impl Escrow {
    pub(super) fn register_credit(&mut self) -> Option<Loan> {
        let slot = (0..SLOTS).find(|i| self.credits.registered & (1u32 << *i) == 0)?;
        self.credits.registered |= 1u32 << slot;
        self.credits.pool.slots[slot]
            .0
            .store(SEALED, Ordering::Release);
        Some(Loan {
            pool: self.credits.pool.clone(),
            slot,
            quote: Cell::new(None),
        })
    }
    fn settle_credit(&mut self, slot: usize) {
        let word = &self.credits.pool.slots[slot].0;
        let observed = word.fetch_or(SEALED, Ordering::AcqRel) & !SEALED;
        let Some(quote) = self.credits.quotes[slot].take() else {
            debug_assert_eq!(observed, 0);
            return;
        };
        assert!(
            observed <= quote.capacity,
            "credit cannot spend beyond its reservation"
        );
        for (used, reserved, unit) in [
            (
                &mut self.used.compute,
                &mut self.credits.reserved.compute,
                quote.compute,
            ),
            (
                &mut self.used.comparison,
                &mut self.credits.reserved.comparison,
                quote.comparison,
            ),
        ] {
            // Products and sums were checked when this exact reservation was
            // made. No release refunds a successful atomic debit.
            *reserved = reserved
                .checked_sub(unit.checked_mul(quote.capacity).unwrap())
                .unwrap();
            *used = used
                .checked_add(unit.checked_mul(observed).unwrap())
                .unwrap();
        }
        self.credits.statistics.batches = self.credits.statistics.batches.saturating_add(observed);
        if observed < quote.capacity {
            self.credits.statistics.reclaims = self.credits.statistics.reclaims.saturating_add(1);
        }
        // Leave a retired slot SEALED with zero count; it remains registered
        // until its unique loan retires, or that owner obtains a new quote.
        word.store(SEALED, Ordering::Release);
    }
    pub(super) fn settle_credits(&mut self) {
        for slot in 0..SLOTS {
            if self.credits.registered & (1u32 << slot) != 0 {
                self.settle_credit(slot);
            }
        }
    }
    pub(super) fn retire_credit(&mut self, loan: &Loan) {
        self.settle_credit(loan.slot);
        self.credits.registered &= !(1u32 << loan.slot);
    }
    pub(super) fn direct_next(
        &mut self,
        compute: u64,
        comparison: u64,
    ) -> Result<ControlWork, ResidentKernelError> {
        if self.next(compute, comparison).is_err() && !self.sealed {
            // Idle reservations may never introduce a false source failure.
            // Seal and fold both dimensions before the authoritative denial.
            self.settle_credits();
        }
        self.next(compute, comparison)
    }
    fn free(&self) -> (u64, u64) {
        (
            self.limit.compute - self.used.compute - self.credits.reserved.compute,
            self.limit.comparison - self.used.comparison - self.credits.reserved.comparison,
        )
    }
    pub(super) fn claim_credit(
        &mut self,
        loan: &Loan,
        compute: u64,
        comparison: u64,
        local: (u64, u64),
    ) -> bool {
        if self.sealed {
            return false;
        }
        self.settle_credit(loan.slot);
        let fit = |free: (u64, u64)| (free.0 / compute).min(free.1 / comparison);
        if fit(self.free()) < HEADROOM {
            self.settle_credits();
        }
        let capacity = fit(self.free()).min(fit(local)).min(BATCHES);
        if capacity < HEADROOM {
            return false;
        }
        // Division bounds every multiplication and both cumulative sums.
        self.credits.reserved.compute += compute * capacity;
        self.credits.reserved.comparison += comparison * capacity;
        let quote = Quote {
            compute,
            comparison,
            capacity,
        };
        self.credits.quotes[loan.slot] = Some(quote);
        loan.quote.set(Some(quote));
        self.credits.statistics.claims = self.credits.statistics.claims.saturating_add(1);
        self.credits.pool.slots[loan.slot]
            .0
            .store(0, Ordering::Release);
        true
    }
}

#[cfg(test)]
#[path = "native_worker_credit_tests.rs"]
mod tests;

impl NativeLibraryWorkBudget {
    /// Try one unchanged full batch, backed by revocable finite parent credit.
    /// This never reduces a batch to fit local credit. A miss retains the
    /// original synchronized selector, including all tight-limit/error rules.
    /// Fourfold headroom is preserved at the atomic debit: the worker's own
    /// unspent reservation alone proves that much group fuel remains.
    pub fn try_charge_funded_batch(
        &self,
        compute: u64,
        comparison: u64,
    ) -> Result<bool, ResidentKernelError> {
        if compute == 0
            || comparison == 0
            || with_control_work(|v| v.borrow().is_some())
            || with_control_work_scope(|v| v.borrow().is_some())
        {
            return Ok(false);
        }
        let Some(required) = compute
            .checked_mul(HEADROOM)
            .zip(comparison.checked_mul(HEADROOM))
        else {
            return Ok(false);
        };
        let mut account = self
            .account
            .try_borrow_mut()
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        let Some(local) = unscoped_local_remaining(&account) else {
            return Ok(false);
        };
        if local.0 < required.0 || local.1 < required.1 {
            return Ok(false);
        }
        let Some(worker) = account.worker.as_ref() else {
            return Ok(false);
        };
        let Some(loan) = worker.credit.as_ref() else {
            return Ok(false);
        };
        let next = checked_local_control_account_work(&account, compute, comparison)?;
        if !loan.charge(compute, comparison) {
            let mut state = lock_escrow(&worker.shared);
            if !state.claim_credit(loan, compute, comparison, local) {
                return Ok(false);
            }
            drop(state);
            if !loan.charge(compute, comparison) {
                return Ok(false);
            }
        }
        account.used = next;
        Ok(true)
    }
}
