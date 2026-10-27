//! Neutral word/aggregate/control emission. No packed-map strategy dependency.
#[cfg(feature = "canonical-native-program")]
#[path = "aggregates/mod.rs"]
pub(in crate::native_artifact) mod aggregates;
#[cfg(feature = "canonical-native-program")]
#[path = "ssa_words.rs"]
pub(in crate::native_artifact) mod ssa_words;
#[cfg(feature = "canonical-native-program")]
#[path = "terminators.rs"]
pub(in crate::native_artifact) mod terminators;
#[cfg(feature = "canonical-native-program")]
#[path = "word_access.rs"]
pub(in crate::native_artifact) mod word_access;

#[path = "memory_regions.rs"]
pub(in crate::native_artifact) mod memory_regions;
