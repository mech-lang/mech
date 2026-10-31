//! Canonical managed-program owners.
#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) mod prepare;
pub(in crate::native_artifact) mod protocol;

#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod application;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) mod compiler;
#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) mod image;
#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) mod limits;
#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) mod linked;
#[cfg(feature = "canonical-native-program-runtime")]
pub(in crate::native_artifact) mod runtime;

#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) mod codegen;
