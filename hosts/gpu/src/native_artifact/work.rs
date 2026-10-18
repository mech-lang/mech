//! One cumulative, non-resettable native invocation work ledger.
use super::{NativeExecutionError, NativeExecutionPhase};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORK_LEDGER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct NativeWorkMeter {
    identity: u64,
    used: u64,
    limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
impl NativeWorkMeter {
    pub fn new(limit: u64) -> Result<Self, NativeExecutionError> {
        let identity = NEXT_WORK_LEDGER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| NativeExecutionError::Descriptor {
                reason: "native work ledger identity exhausted",
            })?;
        Ok(Self {
            identity,
            used: 0,
            limit,
        })
    }
    pub(super) const fn identity(&self) -> u64 {
        self.identity
    }
    pub const fn used(&self) -> u64 {
        self.used
    }
    pub const fn limit(&self) -> u64 {
        self.limit
    }
    pub(super) fn charge(
        &mut self,
        phase: NativeExecutionPhase,
        amount: u64,
    ) -> Result<(), NativeExecutionError> {
        let required = self
            .used
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
        self.used = required;
        Ok(())
    }
}
