//! Neutral word/aggregate/control emission. No packed-map strategy dependency.
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod aggregates;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod bit_counts;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod scalar_selects;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod unsigned_guard_facts;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod boolean_branches;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod ssa_words;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod terminators;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod word_access;

pub(in crate::native_artifact) mod memory_regions;
