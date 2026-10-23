//! One cumulative, non-resettable native invocation work ledger.
use super::{NativeExecutionError, NativeExecutionPhase};
use mech_core::{ManagedMemoryBudget, MemoryBudgetDimension, ResidentKernelError};
use mech_engine::native_artifact::NativeLibraryWorkBudget;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORK_LEDGER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct NativeWorkMeter {
    identity: u64,
    used: u64,
    limit: u64,
    library: Option<NativeLibraryWorkBudget>,
    memory: Option<ManagedMemoryBudget>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_tail_fuel_observes_the_same_spent_account_and_declines_nested_scopes() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut work = NativeWorkMeter::new(1000).unwrap();
        work.charge(NativeExecutionPhase::Setup, 7).unwrap();
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
            .unwrap();
        assert_eq!(library.unscoped_remaining(), Some((929, 100)));
        library.charge(29, 3).unwrap();
        assert_eq!(library.clone().unscoped_remaining(), Some((900, 97)));
        library
            .with_scope(|| {
                assert_eq!(library.unscoped_remaining(), None);
                work.charge(NativeExecutionPhase::Completion, 10).unwrap();
            })
            .unwrap();
        assert_eq!(library.unscoped_remaining(), Some((890, 97)));
        library.restrict(500, 50).unwrap();
        assert_eq!(library.unscoped_remaining(), Some((390, 47)));
        assert_eq!(work.used(), 110);
    }

    #[test]
    fn repeated_phases_share_one_limit_and_denial_does_not_reset_it() {
        let mut work = NativeWorkMeter::new(100).unwrap();
        work.charge(NativeExecutionPhase::Setup, 35).unwrap();
        work.charge(NativeExecutionPhase::Node(mech_core::NodeId::new(9)), 40)
            .unwrap();
        assert_eq!(work.used(), 75);
        assert_eq!(
            work.charge(NativeExecutionPhase::Completion, 26),
            Err(NativeExecutionError::WorkLimit {
                phase: NativeExecutionPhase::Completion,
                required: 101,
                limit: 100
            })
        );
        assert_eq!(work.used(), 75);
        work.charge(NativeExecutionPhase::Completion, 25).unwrap();
        assert_eq!(work.used(), 100);
    }

    #[test]
    fn cumulative_overflow_is_a_denial_without_wrapping_the_ledger() {
        let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
        work.charge(NativeExecutionPhase::Setup, u64::MAX - 1)
            .unwrap();
        assert_eq!(
            work.charge(NativeExecutionPhase::Completion, 2),
            Err(NativeExecutionError::WorkLimit {
                phase: NativeExecutionPhase::Completion,
                required: u64::MAX,
                limit: u64::MAX
            })
        );
        assert_eq!(work.used(), u64::MAX - 1);
    }

    #[test]
    fn promotion_preserves_scalar_work_and_nested_library_work_is_immediately_visible() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut work = NativeWorkMeter::new(100).unwrap();
        let identity = work.identity();
        work.charge(NativeExecutionPhase::Setup, 7).unwrap();
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
            .unwrap();
        assert_eq!(work.identity(), identity);
        assert_eq!(library.used(), (71, 0));
        let backing = memory.used_bytes();
        assert!(backing > 0);
        library
            .with_scope(|| {
                library.charge(20, 3).unwrap();
                assert_eq!(work.used(), 91);
                work.charge(NativeExecutionPhase::Completion, 9).unwrap();
            })
            .unwrap();
        assert_eq!(library.used(), (100, 3));
        assert_eq!(work.used(), 100);
        assert!(matches!(
            work.charge(NativeExecutionPhase::Completion, 1),
            Err(NativeExecutionError::WorkLimit {
                required: 101,
                limit: 100,
                ..
            })
        ));
        assert!(library.charge(0, 2).is_err());
        assert_eq!(library.used(), (100, 3));
        drop(work);
        assert_eq!(memory.used_bytes(), backing);
        drop(library);
        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn repeated_library_access_cannot_refill_limits_or_change_memory_identity() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut work = NativeWorkMeter::new(200).unwrap();
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
            .unwrap();
        library.charge(5, 2).unwrap();
        let repeated = work
            .shared_library_budget(NativeExecutionPhase::Setup, 3, &memory.clone())
            .unwrap();
        assert_eq!(library.used(), (77, 2));
        assert_eq!(repeated.used(), (77, 2));
        assert!(
            work.shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
                .is_err()
        );
        assert!(
            work.shared_library_budget(
                NativeExecutionPhase::Setup,
                3,
                &ManagedMemoryBudget::new(4096)
            )
            .is_err()
        );
        assert_eq!(work.used(), 77);
        repeated.charge(1, 1).unwrap();
        assert_eq!(work.used(), 78);
        assert!(library.charge(0, 1).is_err());
        assert_eq!(library.used(), (78, 3));
    }

    #[test]
    fn failed_repeated_promotion_preserves_existing_comparison_allowance() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut work = NativeWorkMeter::new(100).unwrap();
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
            .unwrap();
        work.charge(NativeExecutionPhase::Setup, 30).unwrap();
        let backing = memory.used_bytes();
        assert!(matches!(
            work.shared_library_budget(NativeExecutionPhase::Setup, 5, &memory),
            Err(NativeExecutionError::WorkLimit {
                required: 102,
                limit: 100,
                ..
            })
        ));
        assert_eq!(library.used(), (94, 0));
        assert_eq!(memory.used_bytes(), backing);
        library.charge(0, 10).unwrap();
        work.charge(NativeExecutionPhase::Completion, 6).unwrap();
        assert_eq!(library.used(), (100, 10));
    }

    #[test]
    fn active_library_account_rejects_new_and_preexisting_independent_inline_meters() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut caller = NativeWorkMeter::new(1000).unwrap();
        let mut independent = NativeWorkMeter::new(1000).unwrap();
        let library = caller
            .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
            .unwrap();
        library
            .with_scope(|| {
                assert!(matches!(
                    NativeWorkMeter::new(1000),
                    Err(NativeExecutionError::Descriptor { .. })
                ));
                assert!(matches!(
                    independent.charge(NativeExecutionPhase::Setup, 1),
                    Err(NativeExecutionError::LibraryWork {
                        error: ResidentKernelError::InvalidInput,
                        ..
                    })
                ));
                assert_eq!(independent.used(), 0);
                caller.charge(NativeExecutionPhase::Setup, 1).unwrap();
            })
            .unwrap();
        assert_eq!(library.used(), (65, 0));
        independent.charge(NativeExecutionPhase::Setup, 1).unwrap();
        assert_eq!(independent.used(), 1);
        assert!(NativeWorkMeter::new(1000).is_ok());
    }

    #[test]
    fn promotion_denials_precede_allocation_and_never_refund_prior_work() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut denied = NativeWorkMeter::new(63).unwrap();
        assert!(matches!(
            denied.shared_library_budget(NativeExecutionPhase::Setup, 4, &memory),
            Err(NativeExecutionError::WorkLimit {
                required: 64,
                limit: 63,
                ..
            })
        ));
        assert_eq!(denied.used(), 0);
        assert_eq!(memory.used_bytes(), 0);
        let mut work = NativeWorkMeter::new(1000).unwrap();
        let empty = ManagedMemoryBudget::new(0);
        assert!(matches!(
            work.shared_library_budget(NativeExecutionPhase::Setup, 4, &empty),
            Err(NativeExecutionError::Memory { .. })
        ));
        assert_eq!(work.used(), 64);
        assert_eq!(empty.used_bytes(), 0);
        let library = work
            .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
            .unwrap();
        assert_eq!(library.used(), (128, 0));
    }

    #[test]
    fn a_different_shared_ledger_cannot_replace_a_live_library_caller() {
        let memory = ManagedMemoryBudget::new(4096);
        let mut caller = NativeWorkMeter::new(1000).unwrap();
        let mut other = NativeWorkMeter::new(1000).unwrap();
        let caller_library = caller
            .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
            .unwrap();
        let other_library = other
            .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
            .unwrap();
        caller_library
            .with_scope(|| {
                assert!(matches!(
                    other.charge(NativeExecutionPhase::Completion, 1),
                    Err(NativeExecutionError::LibraryWork {
                        error: ResidentKernelError::InvalidInput,
                        ..
                    })
                ));
                assert_eq!(other_library.used(), (64, 0));
                caller.charge(NativeExecutionPhase::Completion, 1).unwrap();
            })
            .unwrap();
        assert_eq!(caller.used(), 65);
    }
}
impl NativeWorkMeter {
    pub fn new(limit: u64) -> Result<Self, NativeExecutionError> {
        if NativeLibraryWorkBudget::has_active_scope() {
            return Err(NativeExecutionError::Descriptor {
                reason: "independent native work ledger cannot replace an active invocation",
            });
        }
        let identity = NEXT_WORK_LEDGER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| NativeExecutionError::Descriptor {
                reason: "native work ledger identity exhausted",
            })?;
        Ok(Self {
            identity,
            used: 0,
            limit,
            library: None,
            memory: None,
        })
    }
    pub(super) const fn identity(&self) -> u64 {
        self.identity
    }
    pub fn used(&self) -> u64 {
        self.library
            .as_ref()
            .map_or(self.used, |budget| budget.used().0)
    }
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    pub(super) fn check_memory_budget(
        &self,
        memory: &ManagedMemoryBudget,
    ) -> Result<(), NativeExecutionError> {
        if self.memory.as_ref().is_some_and(|bound| bound != memory) {
            return Err(NativeExecutionError::Descriptor {
                reason: "native work ledger cannot change memory account",
            });
        }
        Ok(())
    }

    /// Bind only after successful construction. A failed reservation has no
    /// live arena/account to retain, but its spent work is never refunded.
    pub(super) fn bind_memory_budget(
        &mut self,
        memory: &ManagedMemoryBudget,
    ) -> Result<(), NativeExecutionError> {
        self.check_memory_budget(memory)?;
        if self.memory.is_none() {
            self.memory = Some(memory.clone());
        }
        Ok(())
    }

    /// Promote the inline scalar ledger to the maintained library's shared
    /// account, preserving every prior debit and this ledger's opaque identity.
    /// This is accounting only, never a BoundCall or permission to execute.
    /// Repeated calls may tighten the comparison limit but cannot refill it or
    /// move the invocation onto another memory account.
    pub fn shared_library_budget(
        &mut self,
        phase: NativeExecutionPhase,
        comparison_limit: u64,
        memory: &ManagedMemoryBudget,
    ) -> Result<NativeLibraryWorkBudget, NativeExecutionError> {
        self.check_memory_budget(memory)?;
        if let Some(budget) = &self.library {
            if budget.memory_budget() != *memory {
                return Err(NativeExecutionError::Descriptor {
                    reason: "native work ledger cannot change memory account",
                });
            }
            budget
                .restrict_and_charge(self.limit, comparison_limit, 8, 0)
                .map_err(|error| Self::library_error(phase, error))?;
            return Ok(self.library.as_ref().unwrap().clone());
        }
        // Charge the fixed-size metadata setup before reservation/allocation.
        // Frozen scalar-only ABI calls retain their inline storage and costs.
        self.charge(phase, 64)?;
        let budget = NativeLibraryWorkBudget::new(self.limit, comparison_limit, memory)?;
        budget
            .charge(self.used, 0)
            .map_err(|error| Self::library_error(phase, error))?;
        self.bind_memory_budget(memory)?;
        self.library = Some(budget.clone());
        Ok(budget)
    }

    fn library_error(
        phase: NativeExecutionPhase,
        error: ResidentKernelError,
    ) -> NativeExecutionError {
        match error {
            ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComputeWork,
                required,
                limit,
                ..
            } => NativeExecutionError::WorkLimit {
                phase,
                required,
                limit,
            },
            error => NativeExecutionError::LibraryWork { phase, error },
        }
    }
    pub(super) fn charge(
        &mut self,
        phase: NativeExecutionPhase,
        amount: u64,
    ) -> Result<(), NativeExecutionError> {
        if self.library.is_none() && NativeLibraryWorkBudget::has_active_scope() {
            return Err(Self::library_error(
                phase,
                ResidentKernelError::InvalidInput,
            ));
        }
        let required = self
            .used()
            .checked_add(amount)
            .ok_or(NativeExecutionError::WorkLimit {
                phase,
                required: u64::MAX,
                limit: self.limit,
            })?;
        if required > self.limit {
            return Err(NativeExecutionError::WorkLimit {
                phase,
                required,
                limit: self.limit,
            });
        }
        if let Some(budget) = &self.library {
            budget
                .charge(amount, 0)
                .map_err(|error| Self::library_error(phase, error))?;
        } else {
            self.used = required;
        }
        Ok(())
    }
}
