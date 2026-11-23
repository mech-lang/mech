//! Synchronous prepaid lexical cleanup on the existing invocation ledger.
use super::*;

/// A non-cloneable loan, not another work account. No helper can borrow the
/// owning invocation until this exact paid amount has been consumed and closed.
pub(in crate::native_artifact) struct PaidRetirements<'a> {
    meter: &'a mut NativeWorkMeter,
    remaining: u64,
}

impl NativeWorkMeter {
    pub(in crate::native_artifact) fn try_retirement_batch(
        &mut self,
        amount: u64,
    ) -> Option<PaidRetirements<'_>> {
        if self.limit.checked_sub(self.used())? < amount {
            return None;
        }
        if let Some(library) = &self.library {
            // Shared worker spending can change between preview and debit.
            // Active call/observer scopes keep their original ordered charges.
            if library.is_worker_budget() || library.unscoped_remaining()?.0 < amount {
                return None;
            }
        } else if NativeLibraryWorkBudget::has_active_scope() {
            return None;
        }
        // The synchronous unscoped account cannot change in this interval.
        // A rejection is non-spending and selects the original ordered path.
        self.charge(NativeExecutionPhase::Setup, amount).ok()?;
        Some(PaidRetirements {
            meter: self,
            remaining: amount,
        })
    }
}

impl PaidRetirements<'_> {
    pub(in crate::native_artifact) fn meter(&self) -> &NativeWorkMeter {
        self.meter
    }
    pub(in crate::native_artifact) fn pay(
        &mut self,
        amount: u64,
    ) -> Result<(), NativeExecutionError> {
        self.remaining =
            self.remaining
                .checked_sub(amount)
                .ok_or(NativeExecutionError::Descriptor {
                    reason: "lexical retirement exceeds its paid amount",
                })?;
        Ok(())
    }
    pub(in crate::native_artifact) fn finish(self) -> Result<(), NativeExecutionError> {
        if self.remaining != 0 {
            return Err(NativeExecutionError::Descriptor {
                reason: "lexical retirement did not consume its paid amount",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
