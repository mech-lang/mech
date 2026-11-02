//! Neutral word/aggregate/control emission. No packed-map strategy dependency.
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod aggregates;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod ssa_words;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod terminators;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod word_access;

pub(in crate::native_artifact) mod memory_regions;
