//! The bound closed-map Owner callback has a separate domain from GlobalHelper.
//! Ignored lanes remain ignored. Entry operations 13/14 keep conservative full
//! frame synchronization. Precise effects cover only: Refill (validation reads,
//! fuel/entry-known/entry-pointer writes), InsertMissingPacked (validation/key/
//! entry/update reads, storage/growth/flag writes), PrepareAndSelectRange and
//! failures 8/9 (no packed-frame effects), and failures 10–12 (validation reads,
//! no writes). Other operations have no precise memory-effect proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum OwnerOperation {
    Refill,
    PrepareRange,
    SelectRange,
    ContainsKey,
    ReadKey,
    InsertKey,
    PrepareAndSelectRange,
    InsertMissingPacked,
    MissingKeyFailure,
    InvalidPackedOperation,
    RangeCallDenied,
    ReadCallDenied,
    WriteCallDenied,
    EnterPackedSsa,
    EnterGuardedLoop,
    Unknown(u64),
}
impl OwnerOperation {
    // Unknown values retain the old late failure after existing key checks.
    pub const fn decode(raw: u64) -> Self {
        match raw {
            0 => Self::Refill,
            1 => Self::PrepareRange,
            2 => Self::SelectRange,
            3 => Self::ContainsKey,
            4 => Self::ReadKey,
            5 => Self::InsertKey,
            6 => Self::PrepareAndSelectRange,
            7 => Self::InsertMissingPacked,
            8 => Self::MissingKeyFailure,
            9 => Self::InvalidPackedOperation,
            10 => Self::RangeCallDenied,
            11 => Self::ReadCallDenied,
            12 => Self::WriteCallDenied,
            13 => Self::EnterPackedSsa,
            14 => Self::EnterGuardedLoop,
            value => Self::Unknown(value),
        }
    }
    pub const fn raw(self) -> u64 {
        match self {
            Self::Refill => 0,
            Self::PrepareRange => 1,
            Self::SelectRange => 2,
            Self::ContainsKey => 3,
            Self::ReadKey => 4,
            Self::InsertKey => 5,
            Self::PrepareAndSelectRange => 6,
            Self::InsertMissingPacked => 7,
            Self::MissingKeyFailure => 8,
            Self::InvalidPackedOperation => 9,
            Self::RangeCallDenied => 10,
            Self::ReadCallDenied => 11,
            Self::WriteCallDenied => 12,
            Self::EnterPackedSsa => 13,
            Self::EnterGuardedLoop => 14,
            Self::Unknown(value) => value,
        }
    }
}
