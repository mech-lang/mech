//! Dependency-neutral identities for the experimental resident executor.

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CellSlotId(pub u32);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SlotIndex(pub u32);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct InstanceEpoch(pub u64);

impl InstanceEpoch {
    /// Returns the next unique resident epoch, or `None` after `u64::MAX`.
    #[inline]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

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
        assert_eq!(InstanceEpoch(41).checked_next(), Some(InstanceEpoch(42)));
        assert_eq!(InstanceEpoch(u64::MAX).checked_next(), None);
    }
}
