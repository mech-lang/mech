//! Compiled application launchers. Generated mains supply build-owned bindings.
#[path = "arguments.rs"]
pub(in crate::native_artifact::launch) mod arguments;
#[path = "scalar.rs"]
pub(in crate::native_artifact::launch) mod scalar;
pub use crate::native_artifact::launch::scalar::{
    BuildOwnedIterationBinding, BuildOwnedMatchBinding, BuildOwnedScalarBinding,
    run_iteration_application, run_match_application, run_scalar_application,
};

#[cfg(feature = "canonical-native-program-runtime")]
#[path = "managed.rs"]
pub(in crate::native_artifact::launch) mod managed;
#[cfg(feature = "canonical-native-program-runtime")]
#[path = "program.rs"]
pub(in crate::native_artifact::launch) mod program;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::launch::managed::run_program_application;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::launch::program::BuildOwnedProgramBinding;
#[cfg(feature = "canonical-native-program-host-runtime")]
#[path = "hosted/mod.rs"]
pub(in crate::native_artifact::launch) mod hosted;
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
pub use crate::native_artifact::launch::hosted::run_benchmark_program_application;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use crate::native_artifact::launch::hosted::run_hosted_program_application;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use crate::native_artifact::launch::program::BuildOwnedHostedBinding;
