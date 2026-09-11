//! Optional caller-owned admission account shared by managed domains.

#[cfg(feature = "no_std")]
use alloc::sync::Arc;
#[cfg(not(feature = "no_std"))]
use std::sync::Arc;

use core::sync::atomic::{AtomicU64, Ordering};

use super::{MemoryRuntimeError, MemoryRuntimeResult};

#[derive(Debug)]
struct ManagedMemoryBudgetAccount {
    limit: u64,
    used: AtomicU64,
    snapshot_import_failure_countdown: AtomicU64,
}

/// A configured limit on admitted Mech-owned backing, including unconsumed
/// reservations and retained owners. Clones share one account across domains,
/// so replacement activation includes the old and candidate storage together.
/// This is not a process-RSS limit and installs no default host quota.
#[derive(Clone, Debug)]
pub struct ManagedMemoryBudget {
    account: Arc<ManagedMemoryBudgetAccount>,
}

impl PartialEq for ManagedMemoryBudget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.account, &other.account)
    }
}

impl Eq for ManagedMemoryBudget {}

impl ManagedMemoryBudget {
    pub fn new(limit_bytes: u64) -> Self {
        Self {
            account: Arc::new(ManagedMemoryBudgetAccount {
                limit: limit_bytes,
                used: AtomicU64::new(0),
                snapshot_import_failure_countdown: AtomicU64::new(0),
            }),
        }
    }

    pub fn limit_bytes(&self) -> u64 {
        self.account.limit
    }

    pub fn used_bytes(&self) -> u64 {
        self.account.used.load(Ordering::Acquire)
    }

    /// One-shot account-scoped fixture for immutable import allocation tests.
    /// Zero fails the next allocation; one lets the registration allocation
    /// succeed and rejects the subsequent owning-wrapper allocation.
    #[doc(hidden)]
    pub fn inject_snapshot_import_failure_after(&self, successful_steps: u32) {
        self.account
            .snapshot_import_failure_countdown
            .store(u64::from(successful_steps) + 1, Ordering::Release);
    }

    pub(crate) fn check_snapshot_import_allocation(
        &self,
        requested: u64,
        alignment: u32,
    ) -> MemoryRuntimeResult<()> {
        let previous = self.account.snapshot_import_failure_countdown.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |remaining| remaining.checked_sub(1),
        );
        if previous == Ok(1) {
            return Err(MemoryRuntimeError::AllocationFailed {
                object: None,
                requested,
                alignment,
                space: crate::MemorySpace::Host,
            });
        }
        Ok(())
    }

    pub(crate) fn reserve(&self, bytes: u64) -> MemoryRuntimeResult<ManagedMemoryCharge> {
        self.reserve_capacity(bytes)
    }

    /// Admits backing capacity before materialization. The reservation is an
    /// exclusive, transferable claim on this account, not an observation of
    /// physical bytes. Dropping unused capacity releases it.
    pub fn reserve_capacity(&self, bytes: u64) -> MemoryRuntimeResult<ManagedMemoryReservation> {
        self.account
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(bytes)
                    .filter(|next| *next <= self.account.limit)
            })
            .map_err(|current| MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: current.checked_add(bytes).unwrap_or(u64::MAX),
                limit: self.account.limit,
            })?;
        Ok(ManagedMemoryReservation {
            budget: self.clone(),
            bytes,
        })
    }
}

/// Non-cloneable byte ownership. Splitting moves an already admitted charge;
/// it does not reserve or allocate and never charges shared arena aliases twice.
#[derive(Debug)]
pub struct ManagedMemoryReservation {
    budget: ManagedMemoryBudget,
    bytes: u64,
}

pub(crate) type ManagedMemoryCharge = ManagedMemoryReservation;

impl ManagedMemoryReservation {
    pub fn capacity_bytes(&self) -> u64 {
        self.bytes
    }

    pub fn budget(&self) -> ManagedMemoryBudget {
        self.budget.clone()
    }

    /// Changes admitted capacity, reserving any growth before changing this
    /// token. Rejected growth leaves both the token and account unchanged.
    pub fn resize_capacity(&mut self, bytes: u64) -> MemoryRuntimeResult<()> {
        if bytes > self.bytes {
            let extra = self.budget.reserve_capacity(bytes - self.bytes)?;
            self.merge_capacity(extra)
        } else {
            self.shrink(bytes);
            Ok(())
        }
    }

    pub fn split_capacity(&mut self, bytes: u64) -> MemoryRuntimeResult<Self> {
        self.bytes = self.bytes.checked_sub(bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "configured memory reservation split",
                current: self.bytes,
                change: bytes,
            },
        )?;
        Ok(Self {
            budget: self.budget.clone(),
            bytes,
        })
    }

    /// Moves another reservation from the same account into this one. A
    /// reservation from another account is rejected, never rehomed; because
    /// this method consumes it, the rejected token releases its own capacity.
    pub fn merge_capacity(&mut self, mut other: Self) -> MemoryRuntimeResult<()> {
        if self.budget != other.budget {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "cannot merge reservations from different memory budgets".into(),
            });
        }
        let bytes = self.bytes.checked_add(other.bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "configured memory reservation merge",
                current: self.bytes,
                change: other.bytes,
            },
        )?;
        self.bytes = bytes;
        other.bytes = 0;
        Ok(())
    }

    pub(crate) fn split(&mut self, bytes: u64) -> MemoryRuntimeResult<Self> {
        self.split_capacity(bytes)
    }

    pub(crate) fn shrink(&mut self, bytes: u64) {
        let released = self
            .bytes
            .checked_sub(bytes)
            .expect("a retained charge may only shrink after construction");
        self.release(released);
        self.bytes = bytes;
    }

    fn release(&self, bytes: u64) {
        let result =
            self.budget
                .account
                .used
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                    current.checked_sub(bytes)
                });
        assert!(result.is_ok(), "configured memory charge underflow");
    }
}

impl Drop for ManagedMemoryReservation {
    fn drop(&mut self) {
        self.release(self.bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_memory_account_checks_overflow_and_retains_split_ownership() {
        let budget = ManagedMemoryBudget::new(u64::MAX);
        let mut reservation = budget.reserve(u64::MAX).unwrap();
        assert!(matches!(
            budget.reserve(1),
            Err(MemoryRuntimeError::BudgetExceeded { .. })
        ));
        let retained = reservation.split(17).unwrap();
        assert_eq!(budget.used_bytes(), u64::MAX);
        drop(reservation);
        assert_eq!(budget.used_bytes(), 17);
        drop(retained);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn retained_memory_charge_releases_only_unconstructed_tail() {
        let budget = ManagedMemoryBudget::new(23);
        let mut retained = budget.reserve(23).unwrap();
        retained.shrink(7);
        assert_eq!(budget.used_bytes(), 7);
        let second = budget.reserve(16).unwrap();
        assert!(budget.reserve(1).is_err());
        drop(retained);
        assert_eq!(budget.used_bytes(), 16);
        drop(second);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn capacity_reservations_grow_split_merge_and_release_without_losing_admission() {
        let budget = ManagedMemoryBudget::new(40);
        let mut reservation = budget.reserve_capacity(24).unwrap();
        assert_eq!(reservation.budget(), budget);
        reservation.resize_capacity(40).unwrap();
        assert_eq!(reservation.capacity_bytes(), 40);
        assert_eq!(budget.used_bytes(), 40);
        assert!(reservation.resize_capacity(41).is_err());
        assert_eq!(reservation.capacity_bytes(), 40);
        assert_eq!(budget.used_bytes(), 40);
        assert!(reservation.split_capacity(41).is_err());
        assert_eq!(reservation.capacity_bytes(), 40);

        let retained = reservation.split_capacity(17).unwrap();
        assert_eq!(reservation.capacity_bytes(), 23);
        assert_eq!(retained.capacity_bytes(), 17);
        assert_eq!(budget.used_bytes(), 40);
        reservation.resize_capacity(7).unwrap();
        assert_eq!(budget.used_bytes(), 24);
        reservation.merge_capacity(retained).unwrap();
        assert_eq!(reservation.capacity_bytes(), 24);
        assert_eq!(budget.used_bytes(), 24);
        drop(reservation);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn capacity_reservations_cannot_merge_different_accounts() {
        let left = ManagedMemoryBudget::new(16);
        let right = ManagedMemoryBudget::new(16);
        let mut reservation = left.reserve_capacity(12).unwrap();
        let foreign = right.reserve_capacity(8).unwrap();
        assert!(matches!(
            reservation.merge_capacity(foreign),
            Err(MemoryRuntimeError::CandidateValidationFailed { .. })
        ));
        assert_eq!(reservation.capacity_bytes(), 12);
        assert_eq!(left.used_bytes(), 12);
        assert_eq!(right.used_bytes(), 0);
        drop(reservation);
        assert_eq!(left.used_bytes(), 0);
    }
}
