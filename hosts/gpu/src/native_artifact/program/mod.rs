//! Canonical managed-program owners.
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "prepare/mod.rs"]
pub(in crate::native_artifact) mod prepare;
#[path = "protocol/mod.rs"]
pub(in crate::native_artifact) mod protocol;

#[cfg(feature = "canonical-native-program")]
#[path = "application.rs"]
pub(in crate::native_artifact) mod application;
#[cfg(feature = "canonical-native-program")]
#[path = "compiler.rs"]
pub(in crate::native_artifact) mod compiler;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "image/mod.rs"]
pub(in crate::native_artifact) mod image;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "linked.rs"]
pub(in crate::native_artifact) mod linked;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "runtime/mod.rs"]
pub(in crate::native_artifact) mod runtime;

#[cfg(feature = "canonical-native")]
#[path = "codegen/mod.rs"]
pub(in crate::native_artifact) mod codegen;
