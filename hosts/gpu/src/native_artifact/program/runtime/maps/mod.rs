//! Native owner namespace.

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "admission_tests.rs"]
pub(in crate::native_artifact::program::runtime) mod admission_tests;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "ascii/mod.rs"]
pub(in crate::native_artifact) mod ascii;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "binding/mod.rs"]
pub(in crate::native_artifact) mod binding;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "closed_tests/mod.rs"]
pub(in crate::native_artifact::program::runtime) mod closed_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "frozen_tests.rs"]
pub(in crate::native_artifact::program::runtime) mod frozen_tests;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "key_domain/mod.rs"]
pub(in crate::native_artifact) mod key_domain;
#[path = "owner/mod.rs"]
pub(in crate::native_artifact::program::runtime) mod owner;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "packed/mod.rs"]
pub(in crate::native_artifact) mod packed;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "string/mod.rs"]
pub(in crate::native_artifact) mod string;
