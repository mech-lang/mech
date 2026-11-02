//! Native owner namespace.

pub(in crate::native_artifact) mod admission;

#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) mod admission_tests;
pub(in crate::native_artifact) mod ascii;
pub(in crate::native_artifact) mod binding;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) mod closed_tests;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) mod frozen_tests;
pub(in crate::native_artifact) mod key_domain;
pub(in crate::native_artifact::program::runtime) mod owner;
pub(in crate::native_artifact) mod packed;
pub(in crate::native_artifact) mod string;
