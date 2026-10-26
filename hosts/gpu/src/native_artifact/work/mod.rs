//! One cumulative, non-resettable native invocation work ledger.
use crate::native_artifact::{NativeExecutionError, NativeExecutionPhase};
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
#[path = "tests.rs"]
pub(in crate::native_artifact::work) mod tests;
impl NativeWorkMeter {
    /// Attach a meter to a consumed parent-backed worker loan. This does not
    /// promote a scalar ledger, mint root fuel or copy thread-local scopes.
    pub fn from_worker_budget(
        library: NativeLibraryWorkBudget,
        memory: &ManagedMemoryBudget,
    ) -> Result<Self, NativeExecutionError> {
        if NativeLibraryWorkBudget::has_active_scope() || !library.is_worker_budget() {
            return Err(NativeExecutionError::Descriptor {
                reason: "worker work meter requires an inactive parent-backed worker budget",
            });
        }
        let admitted = library.memory_budget();
        if admitted != *memory {
            return Err(NativeExecutionError::Descriptor {
                reason: "worker work meter cannot change memory account",
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
            limit: library.limits().0,
            library: Some(library),
            memory: Some(memory.with_limits(admitted.limit_bytes(), admitted.limit_nodes())),
        })
    }

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
    pub(in crate::native_artifact) const fn identity(&self) -> u64 {
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

    pub(in crate::native_artifact) fn check_memory_budget(
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
    pub(in crate::native_artifact) fn bind_memory_budget(
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
    pub(in crate::native_artifact) fn charge(
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
