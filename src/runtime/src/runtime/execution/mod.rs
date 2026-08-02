//! Runtime execution topology.
//!
//! Source execution, queries, reactive turns, context preflight, activation
//! effects, module execution, live registration, host input, persistent sends,
//! and input-driver lifecycle each live in their owning module.

mod activation_effects;
#[cfg(feature = "source")]
mod context_preflight;
mod host_input;
mod input_drivers;
mod live_registration;
#[cfg(feature = "source")]
mod module;
#[cfg(feature = "source")]
mod module_environment;
#[cfg(feature = "source")]
mod persistent_send;
mod query;
mod reactive;
#[cfg(feature = "source")]
mod source;
#[cfg(feature = "source")]
mod source_reconstruction;

#[cfg(test)]
pub(super) use activation_effects::snapshot_runtime_value;
pub(super) use activation_effects::{
    ACTIVATION_EFFECT_BARRIER_NAME, ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
    ActivationEffectBarrierSpecializer, ActivationEffectPayloadCaptureSpecializer,
};
#[allow(unused_imports)]
#[cfg(feature = "source")]
pub use context_preflight::RuntimeAddressedAssignmentUnsupported;
#[cfg(feature = "source")]
use context_preflight::{
    AddressedReadPreflight, RuntimeProgramTarget, identifier_from_str, resolve_runtime_value,
    single_code_program,
};
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
    CapabilityId, MechRuntime, ObjectId, ObjectRecord, ResourceBudgetExceededError, RuntimeConfig,
    RuntimeEventKind,
};
#[cfg(all(test, feature = "compiler"))]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
#[cfg(test)]
use mech_core::{MResult, MechFunctionImpl, MechSourceCode, Value, hash_str};

#[cfg(test)]
mod tests;
