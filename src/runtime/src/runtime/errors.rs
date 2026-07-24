
// Runtime Errors
// -----------------------------------------------------------------------------

// This module defines error types used by the runtime. These errors capture various failure modes that can occur during the runtime's operation, such as module resolution failures, dependency cycles, record not found errors, invalid operations, and more.

// See /src/core/src/error.rs for the base error types and traits used by these runtime errors.

use super::*;

#[derive(Debug, Clone)]
pub struct RuntimeModuleDependencyCycleError {
  pub cycle: Vec<String>,
}

impl MechErrorKind for RuntimeModuleDependencyCycleError {
  fn name(&self) -> &str {
    "RuntimeModuleDependencyCycle"
  }

  fn message(&self) -> String {
    format!(
      "module dependency cycle detected: {}",
      self.cycle.join(" -> "),
    )
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeRecordNotFoundError {
  pub record_type: &'static str,
  pub id: String,
}

impl MechErrorKind for RuntimeRecordNotFoundError {
  fn name(&self) -> &str {
    "RuntimeRecordNotFound"
  }

  fn message(&self) -> String {
    format!("{} record not found: {}", self.record_type, self.id)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeInvalidOperationError {
  pub operation: &'static str,
  pub reason: String,
}

impl MechErrorKind for RuntimeInvalidOperationError {
  fn name(&self) -> &str {
    "RuntimeInvalidOperation"
  }

  fn message(&self) -> String {
    format!("Invalid runtime operation `{}`: {}", self.operation, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleExportNotFound {
  pub dependency: String,
  pub export: String,
}

impl MechErrorKind for RuntimeModuleExportNotFound {
  fn name(&self) -> &str {
    "RuntimeModuleExportNotFound"
  }

  fn message(&self) -> String {
    format!("module `{}` does not export `{}`", self.dependency, self.export)
  }
}

#[derive(Debug, Clone)]
pub struct UnknownAddressTarget {
  pub target: String,
}

impl MechErrorKind for UnknownAddressTarget {
  fn name(&self) -> &str {
    "UnknownAddressTarget"
  }

  fn message(&self) -> String {
    format!("unknown address target `{}`", self.target)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleImportConflict {
  pub binding: String,
  pub first_import: String,
  pub second_import: String,
}

impl MechErrorKind for RuntimeModuleImportConflict {
  fn name(&self) -> &str {
    "RuntimeModuleImportConflict"
  }

  fn message(&self) -> String {
    format!(
      "import binding conflict for `{}` between `{}` and `{}`",
      self.binding, self.first_import, self.second_import
    )
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleImportEdgeInvalid {
  pub module: ModuleVersionId,
  pub reason: String,
}

impl MechErrorKind for RuntimeModuleImportEdgeInvalid {
  fn name(&self) -> &str {
    "RuntimeModuleImportEdgeInvalid"
  }

  fn message(&self) -> String {
    format!(
      "module `{}` has invalid import edges: {}",
      self.module,
      self.reason,
    )
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleDependencyMissingError {
  pub module: String,
  pub specifier: String,
  pub referrer: Option<String>,
}

impl MechErrorKind for RuntimeModuleDependencyMissingError {
  fn name(&self) -> &str {
    "RuntimeModuleDependencyMissing"
  }

  fn message(&self) -> String {
    match &self.referrer {
      Some(referrer) => format!(
        "module `{}` declared dependency `{}` (referrer `{}`) but it could not be resolved",
        self.module,
        self.specifier,
        referrer,
      ),
      None => format!(
        "module `{}` declared dependency `{}` but it could not be resolved",
        self.module,
        self.specifier,
      ),
    }
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeRootModuleSourceNotFound {
  pub specifier: String,
}

impl MechErrorKind for RuntimeRootModuleSourceNotFound {
  fn name(&self) -> &str {
    "RuntimeRootModuleSourceNotFound"
  }

  fn message(&self) -> String {
    format!("root module source `{}` could not be resolved", self.specifier)
  }
}


#[derive(Debug, Clone)]
pub struct RuntimeProgramHostNotActiveError {
  pub function: String,
}

impl MechErrorKind for RuntimeProgramHostNotActiveError {
  fn name(&self) -> &str {
    "RuntimeProgramHostNotActive"
  }

  fn message(&self) -> String {
    format!(
      "Runtime host function `{}` was called without an active runtime context",
      self.function,
    )
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeHostFunctionNotBytecodeCompilableError {
  pub function: String,
}

impl MechErrorKind for RuntimeHostFunctionNotBytecodeCompilableError {
  fn name(&self) -> &str {
    "RuntimeHostFunctionNotBytecodeCompilable"
  }

  fn message(&self) -> String {
    format!(
      "Runtime host function `{}` cannot be compiled to bytecode yet",
      self.function,
    )
  }
}
#[derive(Debug, Clone)]
pub struct ActivationScopeEffectWithRegisterUnsupported;
impl MechErrorKind for ActivationScopeEffectWithRegisterUnsupported {
  fn name(&self) -> &str { "ActivationScopeEffectWithRegisterUnsupported" }
  fn message(&self) -> String { "activation scopes cannot mix local register writes and context sends".to_string() }
}

#[derive(Debug, Clone)]
pub struct RuntimeIsolatedActivationSendUnsupported;
impl MechErrorKind for RuntimeIsolatedActivationSendUnsupported {
  fn name(&self) -> &str { "RuntimeIsolatedActivationSendUnsupported" }
  fn message(&self) -> String { "activation-scoped context sends require retained live registration".to_string() }
}

#[derive(Debug, Clone)]
pub struct RuntimeActivationEffectBarrierInvariantError { pub reason: String }
impl MechErrorKind for RuntimeActivationEffectBarrierInvariantError {
  fn name(&self) -> &str { "RuntimeActivationEffectBarrierInvariant" }
  fn message(&self) -> String { self.reason.clone() }
}
