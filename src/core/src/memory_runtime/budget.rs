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
    snapshot_nodes: AtomicU64,
    retained_nodes: AtomicU64,
    snapshot_import_failure_countdown: AtomicU64,
}

/// A configured limit on admitted Mech-owned backing, including unconsumed
/// reservations and retained owners. Clones share one account across domains,
/// so replacement activation includes the old and candidate storage together.
/// This is not a process-RSS limit and installs no default host quota.
#[derive(Clone, Debug)]
pub struct ManagedMemoryBudget {
    account: Arc<ManagedMemoryBudgetAccount>,
    byte_limit: u64,
    node_limit: u64,
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
                snapshot_nodes: AtomicU64::new(0),
                retained_nodes: AtomicU64::new(0),
                snapshot_import_failure_countdown: AtomicU64::new(0),
            }),
            byte_limit: limit_bytes,
            node_limit: u64::MAX,
        }
    }

    pub fn limit_bytes(&self) -> u64 {
        self.byte_limit.min(self.account.limit)
    }

    /// A tighter admission view of this SAME account. Every reservation checks
    /// the shared live total atomically, so concurrent workers cannot each spend
    /// a stale observation of remaining capacity. Cloning/restricting a view
    /// never renews it. Identity equality continues to mean account identity.
    /// All participants in a bounded parallel group must receive this view;
    /// this does not revoke an independently retained, less restrictive view.
    pub fn with_limits(&self, bytes: u64, nodes: u64) -> Self {
        Self {
            account: self.account.clone(),
            byte_limit: self.limit_bytes().min(bytes),
            node_limit: self.node_limit.min(nodes),
        }
    }

    pub fn limit_nodes(&self) -> u64 {
        self.node_limit
    }

    /// Combined live admitted snapshot nodes and private builder/frame claims.
    /// Private owners reserve their staging geometry before materialization;
    /// publishing while staging is alive conservatively counts both owners.
    pub fn retained_nodes(&self) -> u64 {
        self.account.retained_nodes.load(Ordering::Acquire)
    }

    pub fn used_bytes(&self) -> u64 {
        self.account.used.load(Ordering::Acquire)
    }

    /// Sum of canonical node witnesses held by live admitted snapshot owners.
    /// Ordinary aliases reuse their owner's claim; independent imports may
    /// conservatively count shared data more than once. This reads no Values.
    /// Unadmitted snapshots, private builders and native frame metadata are
    /// excluded. This is resource metadata, not a quota or execution authority.
    pub fn retained_snapshot_nodes(&self) -> u64 {
        self.account.snapshot_nodes.load(Ordering::Acquire)
    }

    pub(crate) fn retain_snapshot_nodes(
        &self,
        nodes: u64,
    ) -> MemoryRuntimeResult<SnapshotNodeCharge> {
        self.claim_nodes(nodes, "admitted snapshot node census")?;
        // The combined claim was acquired first, so this subset cannot wrap.
        self.account
            .snapshot_nodes
            .fetch_add(nodes, Ordering::AcqRel);
        Ok(SnapshotNodeCharge {
            budget: self.clone(),
            nodes,
        })
    }

    fn claim_nodes(&self, nodes: u64, dimension: &'static str) -> MemoryRuntimeResult<()> {
        self.account
            .retained_nodes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(nodes)
                    .filter(|next| *next <= self.node_limit)
            })
            .map_err(|current| match current.checked_add(nodes) {
                None => MemoryRuntimeError::AccountingInvariantViolation {
                    dimension,
                    current,
                    change: nodes,
                },
                Some(requested) => MemoryRuntimeError::BudgetExceeded {
                    operation: Some("retained node admission".into()),
                    requested,
                    limit: self.node_limit,
                },
            })?;
        Ok(())
    }

    /// Exclusive transferable claim for private state that is not yet an
    /// admitted Value. It shares the canonical snapshot node ceiling atomically.
    pub fn reserve_nodes(&self, nodes: u64) -> MemoryRuntimeResult<ManagedNodeReservation> {
        self.claim_nodes(nodes, "admitted private node reservation")?;
        Ok(ManagedNodeReservation {
            budget: self.clone(),
            nodes,
        })
    }

    /// One-shot account-scoped fixture for immutable import allocation tests.
    /// Zero fails the next allocation. One lets registration succeed and
    /// rejects a replacement wrapper when one is needed. Exclusive unadmitted
    /// roots reuse their wrapper and leave that next failure armed.
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
                    .filter(|next| *next <= self.limit_bytes())
            })
            .map_err(|current| MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: current.checked_add(bytes).unwrap_or(u64::MAX),
                limit: self.limit_bytes(),
            })?;
        Ok(ManagedMemoryReservation {
            budget: self.clone(),
            bytes,
        })
    }
}

/// One immutable snapshot owner's node witness. Kept inside its ownership
/// wrapper rather than the weak import registry, so registry links cannot
/// prolong the census after the last actual admitted owner retires.
#[derive(Debug)]
pub(crate) struct SnapshotNodeCharge {
    budget: ManagedMemoryBudget,
    nodes: u64,
}
impl SnapshotNodeCharge {
    pub(crate) fn nodes(&self) -> u64 {
        self.nodes
    }
}
impl Drop for SnapshotNodeCharge {
    fn drop(&mut self) {
        let previous = self
            .budget
            .account
            .snapshot_nodes
            .fetch_sub(self.nodes, Ordering::AcqRel);
        debug_assert!(previous >= self.nodes, "snapshot node census underflow");
        release_nodes(&self.budget, self.nodes);
    }
}

fn release_nodes(budget: &ManagedMemoryBudget, nodes: u64) {
    let previous = budget
        .account
        .retained_nodes
        .fetch_sub(nodes, Ordering::AcqRel);
    assert!(previous >= nodes, "retained node reservation underflow");
}

/// Non-cloneable ownership of private retained-node geometry. Growth is admitted
/// before mutation; failed growth preserves both the token and shared census.
#[derive(Debug)]
pub struct ManagedNodeReservation {
    budget: ManagedMemoryBudget,
    nodes: u64,
}
impl ManagedNodeReservation {
    pub fn nodes(&self) -> u64 {
        self.nodes
    }
    pub fn budget(&self) -> ManagedMemoryBudget {
        self.budget.clone()
    }
    pub fn resize(&mut self, nodes: u64) -> MemoryRuntimeResult<()> {
        if nodes > self.nodes {
            self.budget
                .claim_nodes(nodes - self.nodes, "admitted private node reservation")?;
        } else {
            release_nodes(&self.budget, self.nodes - nodes);
        }
        self.nodes = nodes;
        Ok(())
    }
    /// Transfer an already admitted same-account claim without another atomic
    /// debit. This permits a fallible builder to keep pending growth separate
    /// until all construction succeeds, then commit the claim with its owner.
    pub fn merge(&mut self, mut other: Self) -> MemoryRuntimeResult<()> {
        if self.budget != other.budget {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "cannot merge nodes from different memory budgets".into(),
            });
        }
        let nodes = self.nodes.checked_add(other.nodes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retained node reservation merge",
                current: self.nodes,
                change: other.nodes,
            },
        )?;
        self.nodes = nodes;
        self.budget = self
            .budget
            .with_limits(other.budget.limit_bytes(), other.budget.limit_nodes());
        other.nodes = 0;
        Ok(())
    }
}
impl Drop for ManagedNodeReservation {
    fn drop(&mut self) {
        release_nodes(&self.budget, self.nodes);
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
        // Moving capacity between same-account views must not widen the view
        // later used for growth, even when the receiver began less restricted.
        self.budget = self
            .budget
            .with_limits(other.budget.limit_bytes(), other.budget.limit_nodes());
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
    fn capped_views_share_identity_and_never_widen_admission() {
        let root = ManagedMemoryBudget::new(100);
        let capped = root.with_limits(30, 12);
        assert_eq!(root, capped);
        assert_eq!(capped.with_limits(90, 90).limit_bytes(), 30);
        assert_eq!(capped.with_limits(90, 90).limit_nodes(), 12);
        let old = root.reserve_capacity(20).unwrap();
        let mut next = capped.reserve_capacity(10).unwrap();
        assert!(next.resize_capacity(11).is_err());
        assert_eq!(root.used_bytes(), 30);
        drop(old);
        next.resize_capacity(30).unwrap();
        assert!(capped.reserve_capacity(1).is_err());
        drop(next);
        assert_eq!(root.used_bytes(), 0);
    }

    #[test]
    fn merging_capacity_preserves_the_tighter_view_for_later_growth() {
        let root = ManagedMemoryBudget::new(100);
        let capped = root.with_limits(10, 7);
        let mut wide = root.reserve_capacity(4).unwrap();
        wide.merge_capacity(capped.reserve_capacity(6).unwrap())
            .unwrap();
        assert_eq!(wide.budget().limit_bytes(), 10);
        assert_eq!(wide.budget().limit_nodes(), 7);
        assert!(wide.resize_capacity(11).is_err());
        let split = wide.split_capacity(4).unwrap();
        assert_eq!(split.budget().limit_bytes(), 10);
        drop((wide, split));
        assert_eq!(root.used_bytes(), 0);
    }

    #[test]
    fn private_and_snapshot_nodes_share_one_atomic_ceiling() {
        let root = ManagedMemoryBudget::new(100);
        let capped = root.with_limits(100, 10);
        let snapshot = capped.retain_snapshot_nodes(4).unwrap();
        let mut private = capped.reserve_nodes(6).unwrap();
        assert_eq!(root.retained_snapshot_nodes(), 4);
        assert_eq!(root.retained_nodes(), 10);
        assert!(private.resize(7).is_err());
        assert!(capped.retain_snapshot_nodes(1).is_err());
        assert_eq!(private.nodes(), 6);
        assert_eq!(root.retained_nodes(), 10);
        private.resize(3).unwrap();
        let published = capped.retain_snapshot_nodes(3).unwrap();
        assert_eq!(root.retained_snapshot_nodes(), 7);
        assert_eq!(root.retained_nodes(), 10);
        drop(private);
        assert_eq!(root.retained_nodes(), 7);
        drop((snapshot, published));
        assert_eq!(root.retained_nodes(), 0);
        assert_eq!(root.retained_snapshot_nodes(), 0);
    }

    #[test]
    fn concurrent_capped_admissions_cannot_each_spend_stale_remaining_capacity() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ManagedMemoryBudget>();
        assert_send_sync::<ManagedNodeReservation>();
        let root = ManagedMemoryBudget::new(1000);
        let capped = root.with_limits(12, 8);
        let ready = std::sync::Arc::new(std::sync::Barrier::new(9));
        let release = std::sync::Arc::new(std::sync::Barrier::new(9));
        let threads = (0..8)
            .map(|_| {
                let capped = capped.clone();
                let ready = ready.clone();
                let release = release.clone();
                std::thread::spawn(move || {
                    let bytes = capped.reserve_capacity(3).ok();
                    let nodes = capped.reserve_nodes(2).ok();
                    ready.wait();
                    release.wait();
                    drop((bytes, nodes));
                })
            })
            .collect::<std::vec::Vec<_>>();
        ready.wait();
        assert_eq!(root.used_bytes(), 12);
        assert_eq!(root.retained_nodes(), 8);
        release.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(root.used_bytes(), 0);
        assert_eq!(root.retained_nodes(), 0);
    }

    #[test]
    fn private_node_overflow_is_denied_without_mutating_the_claim() {
        let budget = ManagedMemoryBudget::new(0);
        let mut full = budget.reserve_nodes(u64::MAX).unwrap();
        assert!(matches!(
            budget.reserve_nodes(1),
            Err(MemoryRuntimeError::AccountingInvariantViolation { .. })
        ));
        full.resize(0).unwrap();
        assert_eq!(budget.retained_nodes(), 0);
    }

    #[test]
    fn pending_private_claims_merge_without_renewal_or_rehoming() {
        let budget = ManagedMemoryBudget::new(100);
        let view = budget.with_limits(80, 3);
        let mut owner = budget.reserve_nodes(1).unwrap();
        owner.merge(view.reserve_nodes(2).unwrap()).unwrap();
        assert_eq!(owner.nodes(), 3);
        assert_eq!(budget.retained_nodes(), 3);
        assert!(owner.resize(4).is_err());
        let other = ManagedMemoryBudget::new(100);
        assert!(owner.merge(other.reserve_nodes(1).unwrap()).is_err());
        assert_eq!(owner.nodes(), 3);
        assert_eq!(other.retained_nodes(), 0);
        drop(owner);
        assert_eq!(budget.retained_nodes(), 0);
    }

    #[test]
    fn snapshot_node_census_overflow_preserves_account_and_retires_claim() {
        let budget = ManagedMemoryBudget::new(0);
        let other = ManagedMemoryBudget::new(0);
        let claim = budget.retain_snapshot_nodes(u64::MAX).unwrap();
        assert_eq!(claim.nodes(), u64::MAX);
        assert_eq!(budget.clone().retained_snapshot_nodes(), u64::MAX);
        assert_eq!(other.retained_snapshot_nodes(), 0);
        assert!(matches!(
            budget.retain_snapshot_nodes(1),
            Err(MemoryRuntimeError::AccountingInvariantViolation { .. })
        ));
        assert_eq!(budget.retained_snapshot_nodes(), u64::MAX);
        assert_eq!(
            budget.used_bytes(),
            0,
            "node metadata does not reserve payload bytes"
        );
        drop(claim);
        assert_eq!(budget.retained_snapshot_nodes(), 0);
    }

    #[cfg(any(not(feature = "no_std"), feature = "std"))]
    #[test]
    fn snapshot_node_census_shares_atomic_claims_across_threads() {
        let budget = ManagedMemoryBudget::new(0);
        let ready = std::sync::Arc::new(std::sync::Barrier::new(9));
        let retire = std::sync::Arc::new(std::sync::Barrier::new(9));
        let threads = (0..8)
            .map(|_| {
                let budget = budget.clone();
                let ready = ready.clone();
                let retire = retire.clone();
                std::thread::spawn(move || {
                    let claim = budget.retain_snapshot_nodes(7).unwrap();
                    ready.wait();
                    retire.wait();
                    drop(claim);
                })
            })
            .collect::<std::vec::Vec<_>>();
        ready.wait();
        let live = budget.retained_snapshot_nodes();
        retire.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(live, 56);
        assert_eq!(budget.retained_snapshot_nodes(), 0);
    }

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
