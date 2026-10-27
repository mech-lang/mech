//! Native owner namespace.

#[cfg(feature = "canonical-native-program")]
#[path = "ascii_map.rs"]
pub(in crate::native_artifact) mod ascii_map;
#[cfg(feature = "canonical-native-program")]
#[path = "borrowed/mod.rs"]
pub(in crate::native_artifact) mod borrowed;
#[cfg(feature = "canonical-native-program")]
#[cfg(feature = "canonical-native-program")]
#[path = "bound_shim/mod.rs"]
pub(in crate::native_artifact) mod bound_shim;
#[cfg(feature = "canonical-native-program")]
#[path = "cfg_cleanup/mod.rs"]
pub(in crate::native_artifact) mod cfg_cleanup;
#[cfg(feature = "canonical-native")]
#[path = "hooks/mod.rs"]
pub(in crate::native_artifact) mod hooks;
#[cfg(feature = "canonical-native-program")]
#[path = "join_map/mod.rs"]
pub(in crate::native_artifact) mod join_map;
#[cfg(feature = "canonical-native-program")]
#[path = "map_owner.rs"]
pub(in crate::native_artifact) mod map_owner;
#[cfg(feature = "canonical-native-program")]
#[path = "packed_loop/mod.rs"]
pub(in crate::native_artifact) mod packed_loop;
#[cfg(feature = "canonical-native-program")]
#[path = "packed_map/mod.rs"]
pub(in crate::native_artifact) mod packed_map;
#[cfg(feature = "canonical-native-program")]
#[path = "parallel.rs"]
pub(in crate::native_artifact) mod parallel;
#[cfg(feature = "canonical-native-program")]
#[path = "prefix/mod.rs"]
pub(in crate::native_artifact) mod prefix;
#[cfg(feature = "canonical-native-program")]
#[path = "scalar_loop.rs"]
pub(in crate::native_artifact) mod scalar_loop;
#[cfg(feature = "canonical-native-program")]
#[path = "string_map/mod.rs"]
pub(in crate::native_artifact) mod string_map;
#[cfg(feature = "canonical-native-program")]
#[path = "tag_threading/mod.rs"]
pub(in crate::native_artifact) mod tag_threading;
#[cfg(feature = "canonical-native-program")]
#[path = "tail.rs"]
pub(in crate::native_artifact) mod tail;
#[cfg(feature = "canonical-native-program")]
#[path = "window/mod.rs"]
pub(in crate::native_artifact) mod window;
