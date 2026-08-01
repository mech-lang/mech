//! Runtime execution topology.
//!
//! Source execution, queries, reactive turns, context preflight, activation
//! effects, module execution, live registration, host input, persistent sends,
//! and input-driver lifecycle each live in their owning module.

mod activation_effects;
mod context_preflight;
mod host_input;
mod input_drivers;
mod live_registration;
mod module;
mod module_environment;
mod persistent_send;
mod query;
mod reactive;
mod source;
mod source_reconstruction;

pub(super) use activation_effects::{
    ACTIVATION_EFFECT_BARRIER_NAME, ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
    ActivationEffectBarrierCompiler, ActivationEffectPayloadCaptureCompiler,
};
#[allow(unused_imports)]
pub use context_preflight::RuntimeAddressedAssignmentUnsupported;
use context_preflight::{
    AddressedReadPreflight, RuntimeProgramTarget, identifier_from_str, resolve_runtime_value,
    single_code_program, snapshot_runtime_value,
};
pub(super) use module::IntegrityEvaluationCollector;
use module_environment::{
    PreparedModuleScopeExecution, ProgramEnvironmentOverlay, RuntimeAddressTarget,
    context_registry_for_scope, execution_scope_for_extracted_module_source, exports_for_scope,
    materialize_function_imports_for_scope, merge_module_environment,
    resolve_runtime_address_target,
};
use source_reconstruction::module_source_for_scope;

#[cfg(test)]
use crate::{
    CapabilityId, MechRuntime, ObjectId, ObjectRecord, ResourceBudgetExceededError, RuntimeConfig,
    RuntimeEventKind,
};
#[cfg(test)]
use mech_core::{
    BytecodeCompilerContext, MResult, MechFunctionCompiler, MechFunctionImpl, MechSourceCode,
    Register, Value, hash_str,
};

#[cfg(test)]
mod tests;
