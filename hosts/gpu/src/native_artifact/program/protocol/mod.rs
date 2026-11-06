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
    AdmitMove,
    AdmitStructuralObservation,
    AdmitNumericSegmentBegin,
    MapOwnerConstructorCheck,
    BorrowedRecordMapProject,
    BorrowedRecordMapRead,
    Utf8ReadPrepare,
    Utf8ReadFinish,
    UniformFillPrepare,
    UniformFillFinish,
    NumericMapPrepare,
    NumericMapFinish,
    HistogramPrepare,
    HistogramFinish,
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
            value if value == u64::MAX - 29 => Self::AdmitMove,
            value if value == u64::MAX - 30 => Self::AdmitStructuralObservation,
            value if value == u64::MAX - 31 => Self::AdmitNumericSegmentBegin,
            value if value == u64::MAX - 32 => Self::MapOwnerConstructorCheck,
            value if value == u64::MAX - 33 => Self::BorrowedRecordMapProject,
            value if value == u64::MAX - 34 => Self::BorrowedRecordMapRead,
            value if value == u64::MAX - 35 => Self::Utf8ReadPrepare,
            value if value == u64::MAX - 36 => Self::Utf8ReadFinish,
            value if value == u64::MAX - 37 => Self::UniformFillPrepare,
            value if value == u64::MAX - 38 => Self::UniformFillFinish,
            value if value == u64::MAX - 41 => Self::NumericMapPrepare,
            value if value == u64::MAX - 42 => Self::NumericMapFinish,
            value if value == u64::MAX - 39 => Self::HistogramPrepare,
            value if value == u64::MAX - 40 => Self::HistogramFinish,
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
            Self::AdmitMove => u64::MAX - 29,
            Self::AdmitStructuralObservation => u64::MAX - 30,
            Self::AdmitNumericSegmentBegin => u64::MAX - 31,
            Self::MapOwnerConstructorCheck => u64::MAX - 32,
            Self::BorrowedRecordMapProject => u64::MAX - 33,
            Self::BorrowedRecordMapRead => u64::MAX - 34,
            Self::Utf8ReadPrepare => u64::MAX - 35,
            Self::Utf8ReadFinish => u64::MAX - 36,
            Self::UniformFillPrepare => u64::MAX - 37,
            Self::UniformFillFinish => u64::MAX - 38,
            Self::NumericMapPrepare => u64::MAX - 41,
            Self::NumericMapFinish => u64::MAX - 42,
            Self::HistogramPrepare => u64::MAX - 39,
            Self::HistogramFinish => u64::MAX - 40,
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

#[cfg(all(test, feature = "canonical-native-program"))]
#[test]
fn collection_helpers_have_distinct_roundtrip_codes_and_reserve_output_space() {
    let tags = [
        GlobalHelper::UniformFillPrepare,
        GlobalHelper::UniformFillFinish,
        GlobalHelper::HistogramPrepare,
        GlobalHelper::HistogramFinish,
        GlobalHelper::NumericMapPrepare,
        GlobalHelper::NumericMapFinish,
    ];
    let mut codes = std::collections::BTreeSet::new();
    for tag in tags {
        assert!(codes.insert(tag.raw()));
        assert_eq!(GlobalHelper::decode(tag.raw()), tag);
        assert!(GlobalHelper::output(tag.raw() - 3).is_none());
    }
}
