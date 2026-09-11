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
            }),
        }
    }

    pub fn limit_bytes(&self) -> u64 {
        self.account.limit
    }

    pub fn used_bytes(&self) -> u64 {
        self.account.used.load(Ordering::Acquire)
    }

    pub(crate) fn reserve(&self, bytes: u64) -> MemoryRuntimeResult<ManagedMemoryCharge> {
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
        Ok(ManagedMemoryCharge {
            budget: self.clone(),
            bytes,
        })
    }
}

/// Non-cloneable byte ownership. Splitting moves an already admitted charge;
/// it does not reserve or allocate and never charges shared arena aliases twice.
#[derive(Debug)]
pub(crate) struct ManagedMemoryCharge {
    budget: ManagedMemoryBudget,
    bytes: u64,
}

impl ManagedMemoryCharge {
    pub(crate) fn split(&mut self, bytes: u64) -> MemoryRuntimeResult<Self> {
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

impl Drop for ManagedMemoryCharge {
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
}
