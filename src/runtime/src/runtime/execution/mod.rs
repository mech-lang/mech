//! Runtime execution topology.
//!
//! Source execution, queries, reactive turns, context preflight, module
//! execution, live registration, host input, and input-driver lifecycle each
//! live in their owning module.

mod audit;
mod bytecode;
#[cfg(feature = "source")]
mod context_preflight;
mod host_input;
mod input_drivers;
mod live_registration;
#[cfg(feature = "source")]
mod module;
#[cfg(feature = "source")]
mod module_environment;
mod query;
mod reactive;
#[cfg(feature = "source")]
mod source;
#[cfg(feature = "source")]
mod source_reconstruction;

#[cfg(feature = "source")]
use context_preflight::AddressedReadPreflight;
#[allow(unused_imports)]
#[cfg(feature = "source")]
pub use context_preflight::RuntimeAddressedAssignmentUnsupported;
#[cfg(feature = "source")]
pub(in crate::runtime) use context_preflight::RuntimeProgramTarget;
#[cfg(feature = "source")]
pub(super) use module::IntegrityEvaluationCollector;
#[cfg(feature = "source")]
use module_environment::{
    PreparedModuleScopeExecution, ProgramEnvironmentOverlay, RuntimeAddressTarget,
    context_registry_for_scope, execution_scope_for_extracted_module_source, exports_for_scope,
    materialize_function_imports_for_scope, merge_module_environment,
    resolve_runtime_address_target,
};
#[cfg(feature = "source")]
use source_reconstruction::module_source_for_scope;

#[cfg(test)]
use crate::{
    CapabilityId, MechRuntime, ResourceBudgetExceededError, RuntimeConfig, RuntimeEventKind,
};
#[cfg(all(test, feature = "compiler"))]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
#[cfg(test)]
use mech_core::{LegacyValue, MResult, MechFunctionImpl};

#[cfg(test)]
mod tests;
