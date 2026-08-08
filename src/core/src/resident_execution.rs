//! Dependency-neutral identities for the experimental resident executor.

pub use crate::semantic_identity::{CellSlotId, InstanceEpoch, SlotIndex};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_are_transparent_dependency_free_scalars() {
        assert_eq!(
            core::mem::size_of::<CellSlotId>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<SlotIndex>(),
            core::mem::size_of::<u32>()
        );
        assert_eq!(
            core::mem::size_of::<InstanceEpoch>(),
            core::mem::size_of::<u64>()
        );
        assert_eq!(InstanceEpoch(41).checked_next(), Ok(InstanceEpoch(42)));
        assert!(InstanceEpoch(u64::MAX).checked_next().is_err());
    }
}
