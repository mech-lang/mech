//! Typed private helper domains. Raw u64 lanes remain the unchanged transport.
#[cfg(feature = "canonical-native-program-runtime")]
mod owner;
#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) use crate::native_artifact::program::protocol::owner::OwnerOperation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct OutputIndex(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum GlobalHelper {
    Library,
    Move,
    Collection,
    Output(OutputIndex),
    WindowPrepare,
    WindowFinish,
    StringMapPrepare,
    StringMapFinish,
    BorrowedStringsPrepare,
    BorrowedStringFailure,
    ClosedTailBegin,
    ClosedTailEnd,
    PrefixPause,
    PrefixBorrowedResume,
    PrefixTailResume,
    PrefixScanned,
    JoinMapPrepare,
    JoinMapFinish,
    StructuralObservation,
    MapOwnerBegin,
    MapOwnerCall,
    MapOwnerEnd,
    MapOwnerSpan,
    ParallelRun,
    ParallelTake,
    ParallelYield,
    ParallelEnter,
    FrozenMapCopy,
    FrozenMapRead,
    NumericWritePrepare,
    NumericSegmentBegin,
    NumericSegmentEnd,
}
impl GlobalHelper {
    // Decoding classifies only. Site, phase, lane, admission and cursor checks
    // remain at their original runtime points, including unknown output tags.
    pub const fn decode(raw: u64) -> Self {
        match raw {
            0 => Self::Library,
            1 => Self::Move,
            2 => Self::Collection,
            value if value == u64::MAX - 1 => Self::WindowPrepare,
            value if value == u64::MAX - 2 => Self::WindowFinish,
            value if value == u64::MAX - 3 => Self::StringMapPrepare,
            value if value == u64::MAX - 4 => Self::StringMapFinish,
            value if value == u64::MAX - 5 => Self::BorrowedStringsPrepare,
            value if value == u64::MAX - 6 => Self::BorrowedStringFailure,
            value if value == u64::MAX - 7 => Self::ClosedTailBegin,
            value if value == u64::MAX - 8 => Self::ClosedTailEnd,
            value if value == u64::MAX - 9 => Self::PrefixPause,
            value if value == u64::MAX - 10 => Self::PrefixBorrowedResume,
            value if value == u64::MAX - 11 => Self::PrefixTailResume,
            value if value == u64::MAX - 12 => Self::PrefixScanned,
            value if value == u64::MAX - 13 => Self::JoinMapPrepare,
            value if value == u64::MAX - 14 => Self::JoinMapFinish,
            value if value == u64::MAX - 15 => Self::StructuralObservation,
            value if value == u64::MAX - 16 => Self::MapOwnerBegin,
            value if value == u64::MAX - 17 => Self::MapOwnerCall,
            value if value == u64::MAX - 18 => Self::MapOwnerEnd,
            value if value == u64::MAX - 19 => Self::MapOwnerSpan,
            value if value == u64::MAX - 20 => Self::ParallelRun,
            value if value == u64::MAX - 21 => Self::ParallelTake,
            value if value == u64::MAX - 22 => Self::ParallelYield,
            value if value == u64::MAX - 23 => Self::ParallelEnter,
            value if value == u64::MAX - 24 => Self::FrozenMapCopy,
            value if value == u64::MAX - 25 => Self::FrozenMapRead,
            value if value == u64::MAX - 26 => Self::NumericWritePrepare,
            value if value == u64::MAX - 27 => Self::NumericSegmentBegin,
            value if value == u64::MAX - 28 => Self::NumericSegmentEnd,
            value => Self::Output(OutputIndex(value - 3)),
        }
    }
    pub const fn raw(self) -> u64 {
        match self {
            Self::Library => 0,
            Self::Move => 1,
            Self::Collection => 2,
            Self::Output(OutputIndex(index)) => index + 3,
            Self::WindowPrepare => u64::MAX - 1,
            Self::WindowFinish => u64::MAX - 2,
            Self::StringMapPrepare => u64::MAX - 3,
            Self::StringMapFinish => u64::MAX - 4,
            Self::BorrowedStringsPrepare => u64::MAX - 5,
            Self::BorrowedStringFailure => u64::MAX - 6,
            Self::ClosedTailBegin => u64::MAX - 7,
            Self::ClosedTailEnd => u64::MAX - 8,
            Self::PrefixPause => u64::MAX - 9,
            Self::PrefixBorrowedResume => u64::MAX - 10,
            Self::PrefixTailResume => u64::MAX - 11,
            Self::PrefixScanned => u64::MAX - 12,
            Self::JoinMapPrepare => u64::MAX - 13,
            Self::JoinMapFinish => u64::MAX - 14,
            Self::StructuralObservation => u64::MAX - 15,
            Self::MapOwnerBegin => u64::MAX - 16,
            Self::MapOwnerCall => u64::MAX - 17,
            Self::MapOwnerEnd => u64::MAX - 18,
            Self::MapOwnerSpan => u64::MAX - 19,
            Self::ParallelRun => u64::MAX - 20,
            Self::ParallelTake => u64::MAX - 21,
            Self::ParallelYield => u64::MAX - 22,
            Self::ParallelEnter => u64::MAX - 23,
            Self::FrozenMapCopy => u64::MAX - 24,
            Self::FrozenMapRead => u64::MAX - 25,
            Self::NumericWritePrepare => u64::MAX - 26,
            Self::NumericSegmentBegin => u64::MAX - 27,
            Self::NumericSegmentEnd => u64::MAX - 28,
        }
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn output(index: u64) -> Option<Self> {
        let raw = index.checked_add(3)?;
        match Self::decode(raw) {
            value @ Self::Output(_) => Some(value),
            _ => None,
        }
    }
}

#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) mod abi;
