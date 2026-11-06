//! Native owner namespace.

#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod ascii_map;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod borrowed;
#[cfg(feature = "canonical-native-program")]
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod bound_shim;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod cfg_cleanup;
pub(in crate::native_artifact) mod hooks;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod join_map;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod map_owner;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod packed_loop;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod packed_map;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod parallel;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod prefix;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod scalar_loop;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod string_map;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod tag_threading;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod tail;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod window;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod word_collections;

#[cfg(feature = "canonical-native-program")]
mod numeric_branches;
mod numeric_fuel;
#[cfg(feature = "canonical-native-program")]
mod numeric_read_packs;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod numeric_segments;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod numeric_words;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod ordinary_helpers;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod readonly_numeric;

#[cfg(feature = "canonical-native-program")]
mod traversal_facts;

mod numeric_loop_prepayment;
mod numeric_loop_windows;
#[cfg(feature = "canonical-native-program")]
mod numeric_traversal;
mod ordered_reduction;
