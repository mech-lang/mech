//! Compiled application launchers. Generated mains supply build-owned bindings.
mod arguments;
mod scalar;
pub use scalar::{
    BuildOwnedIterationBinding, BuildOwnedMatchBinding, BuildOwnedScalarBinding,
    run_iteration_application, run_match_application, run_scalar_application,
};

#[cfg(feature = "canonical-native-program-runtime")]
mod managed;
#[cfg(feature = "canonical-native-program-runtime")]
mod program;
#[cfg(feature = "canonical-native-program-runtime")]
pub use managed::run_program_application;
#[cfg(feature = "canonical-native-program-runtime")]
pub use program::BuildOwnedProgramBinding;
#[cfg(feature = "canonical-native-program-host-runtime")]
mod hosted;
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
pub use hosted::run_benchmark_program_application;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use hosted::run_hosted_program_application;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use program::BuildOwnedHostedBinding;
