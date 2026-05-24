//! Host integration boundary.
//!
//! `host.rs` defines how embedders expose native functionality to the Mech
//! runtime without baking those APIs into the interpreter.
//!
//! Host functions are runtime-facing capabilities:
//!
//! - filesystem APIs
//! - database APIs
//! - UI calls
//! - clocks / timers
//! - network calls
//! - device APIs
//! - GPU calls
//! - application-specific functions
//!
//! Host functions should be capability-checked before invocation.

use std::collections::HashMap;
use std::sync::Arc;

use mech_core::{MResult, MechError, MechErrorKind, Value};

use crate::capability::{
  CapabilityRequest, Operation, Resource,
};

use crate::context::RuntimeContext;

use crate::service::RuntimeServices;

pub mod actor;
pub mod arg;

pub use self::actor::*;
pub use self::arg::*;

// -----------------------------------------------------------------------------
// Host Function
// -----------------------------------------------------------------------------

/// A callable host function.
///
/// This trait is the embedder boundary. Implementations can wrap closures,
/// native functions, database handles, UI calls, device APIs, etc.
pub trait HostFunction: std::fmt::Debug + Send + Sync {
  /// Stable host-visible name.
  ///
  /// Examples:
  ///
  /// - `clock.now`
  /// - `fs.read`
  /// - `db.query`
  /// - `ui.render`
  /// - `net.fetch`
  fn name(&self) -> &str;

  /// Optional resource key used for capability checks.
  ///
  /// If this returns `None`, the default resource key is `host:<name>`.
  fn resource(&self) -> Option<&dyn Resource> {
    None
  }

  /// Optional operation key used for capability checks.
  ///
  /// If this returns `None`, the default operation key is `call`.
  fn operation(&self) -> Option<&dyn Operation> {
    None
  }

  /// Optional explicit capability request.
  ///
  /// If this returns `Some`, the runtime should check this request directly.
  /// If this returns `None`, the registry builds a request from context subject,
  /// operation, and resource.
  fn required_capability(&self, context: &RuntimeContext) -> Option<CapabilityRequest> {
    let _ = context;
    None
  }

  /// Estimated bytes charged before the call.
  ///
  /// Implementations can override this for predictable costs. Dynamic output
  /// costs can be charged by host code through the context as needed.
  fn estimated_cost_bytes(&self, args: &[Value]) -> u64 {
    let _ = args;
    0
  }

  /// Estimated item count charged before the call.
  fn estimated_cost_items(&self, args: &[Value]) -> u64 {
    args.len() as u64
  }

  /// Invoke the host function.
  fn call(
    &self,
    services: &mut dyn RuntimeServices,
    context: &mut RuntimeContext,
    args: Vec<Value>,
  ) -> MResult<Value>;
}

// -----------------------------------------------------------------------------
// Closure Host Function
// -----------------------------------------------------------------------------

/// Simple closure-backed host function.
///
/// Useful for tests and embedding small APIs.
pub struct ClosureHostFunction<F>
where
  F: Fn(&mut dyn RuntimeServices, &mut RuntimeContext, Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
{
  name: String,
  capability: Option<CapabilityRequest>,
  function: F,
}

impl<F> std::fmt::Debug for ClosureHostFunction<F>
where
  F: Fn(&mut dyn RuntimeServices, &mut RuntimeContext, Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ClosureHostFunction")
      .field("name", &self.name)
      .field("capability", &self.capability)
      .field("function", &"<closure>")
      .finish()
  }
}

impl<F> ClosureHostFunction<F>
where
  F: Fn(&mut dyn RuntimeServices, &mut RuntimeContext, Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
{
  pub fn new(
    name: impl Into<String>,
    function: F,
  ) -> Self {
    Self {
      name: name.into(),
      capability: None,
      function,
    }
  }

  pub fn with_capability(
    mut self,
    capability: CapabilityRequest,
  ) -> Self {
    self.capability = Some(capability);
    self
  }
}

impl<F> HostFunction for ClosureHostFunction<F>
where
  F: Fn(&mut dyn RuntimeServices, &mut RuntimeContext, Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn required_capability(&self, context: &RuntimeContext) -> Option<CapabilityRequest> {
    let _ = context;
    self.capability.clone()
  }

  fn call(&self, services: &mut dyn RuntimeServices, context: &mut RuntimeContext, args: Vec<Value>) -> MResult<Value> {
    (self.function)(services, context, args)
  }
}

// -----------------------------------------------------------------------------
// Host Registry
// -----------------------------------------------------------------------------

/// Registry of host functions.
pub trait HostRegistry: std::fmt::Debug + Send {
  fn register_function(
    &mut self,
    function: Arc<dyn HostFunction>,
  ) -> MResult<()>;

  fn get_function(&self, name: &str) -> MResult<Option<Arc<dyn HostFunction>>>;

  fn remove_function(&mut self, name: &str) -> MResult<Option<Arc<dyn HostFunction>>>;

  fn list_functions(&self) -> MResult<Vec<String>>;
}

/// Default in-memory host registry.
#[derive(Clone, Debug, Default)]
pub struct InMemoryHostRegistry {
  functions: HashMap<String, Arc<dyn HostFunction>>,
}

impl InMemoryHostRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn insert(
    &mut self,
    function: impl HostFunction + 'static,
  ) -> MResult<()> {
    self.register_function(Arc::new(function))
  }

  pub fn contains(&self, name: &str) -> bool {
    self.functions.contains_key(name)
  }

  pub fn len(&self) -> usize {
    self.functions.len()
  }

  pub fn is_empty(&self) -> bool {
    self.functions.is_empty()
  }
}

impl HostRegistry for InMemoryHostRegistry {
  fn register_function(
    &mut self,
    function: Arc<dyn HostFunction>,
  ) -> MResult<()> {
    let name = function.name().to_string();

    if name.trim().is_empty() {
      return Err(MechError::new(
        InvalidHostFunctionError {
          field: "name",
          reason: "must not be empty",
        },
        None,
      ));
    }

    if self.functions.contains_key(&name) {
      return Err(MechError::new(
        HostFunctionAlreadyExistsError { name },
        None,
      ));
    }

    self.functions.insert(name, function);
    Ok(())
  }

  fn get_function(&self, name: &str) -> MResult<Option<Arc<dyn HostFunction>>> {
    if name.trim().is_empty() {
      return Err(MechError::new(
        InvalidHostFunctionError {
          field: "name",
          reason: "must not be empty",
        },
        None,
      ));
    }

    Ok(self.functions.get(name).cloned())
  }

  fn remove_function(&mut self, name: &str) -> MResult<Option<Arc<dyn HostFunction>>> {
    if name.trim().is_empty() {
      return Err(MechError::new(
        InvalidHostFunctionError {
          field: "name",
          reason: "must not be empty",
        },
        None,
      ));
    }

    Ok(self.functions.remove(name))
  }

  fn list_functions(&self) -> MResult<Vec<String>> {
    let mut names: Vec<String> = self.functions.keys().cloned().collect();
    names.sort();
    Ok(names)
  }
}

// -----------------------------------------------------------------------------
// Host Call Policy
// -----------------------------------------------------------------------------

/// Policy object used by the runtime before invoking a host function.
///
/// This trait exists so embedders can provide stricter policies later:
///
/// - deny unregistered host functions
/// - require explicit capabilities
/// - block host calls during deterministic replay
/// - restrict host calls on remote nodes
/// - audit or rate limit host calls
pub trait HostCallPolicy: std::fmt::Debug + Send + Sync {
  fn validate_call(
    &self,
    context: &RuntimeContext,
    function: &dyn HostFunction,
    args: &[Value],
  ) -> MResult<()>;
}

/// Default permissive policy.
///
/// It validates the context and charges the function's estimated costs. It does
/// not itself check capabilities; that is the runtime's job because it owns the
/// CapabilityKernel.
#[derive(Clone, Debug, Default)]
pub struct DefaultHostCallPolicy;

impl HostCallPolicy for DefaultHostCallPolicy {
  fn validate_call(
    &self,
    context: &RuntimeContext,
    function: &dyn HostFunction,
    args: &[Value],
  ) -> MResult<()> {
    context.validate()?;

    if function.name().trim().is_empty() {
      return Err(MechError::new(
        InvalidHostFunctionError {
          field: "name",
          reason: "must not be empty",
        },
        None,
      ));
    }

    let _ = args;
    Ok(())
  }
}

/// Utility functions for performing a host call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostCall {
  pub name: String,
  pub args: Vec<Value>,
}

impl HostCall {
  pub fn new(name: impl Into<String>, args: Vec<Value>) -> Self {
    Self {
      name: name.into(),
      args,
    }
  }

  pub fn validate(&self) -> MResult<()> {
    if self.name.trim().is_empty() {
      return Err(MechError::new(
        InvalidHostCallFieldError {
          field: "name",
          reason: "must not be empty",
        },
        None,
      ));
    }

    Ok(())
  }
}

// -----------------------------------------------------------------------------
// Default Resource / Operation Keys
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostResource {
  key: String,
}

impl HostResource {
  pub fn new(key: impl Into<String>) -> Self {
    Self { key: key.into() }
  }

  pub fn function(name: &str) -> Self {
    Self::new(format!("host:{}", name))
  }
}

impl Resource for HostResource {
  fn key(&self) -> &str {
    &self.key
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostOperation {
  key: String,
}

impl HostOperation {
  pub fn new(key: impl Into<String>) -> Self {
    Self { key: key.into() }
  }

  pub fn call() -> Self {
    Self::new("call")
  }
}

impl Operation for HostOperation {
  fn key(&self) -> &str {
    &self.key
  }
}

pub fn default_host_capability_request(
  context: &RuntimeContext,
  function_name: &str,
) -> CapabilityRequest {
  let resource = HostResource::function(function_name);
  let operation = HostOperation::call();

  context.capability_request(&operation, &resource)
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn context_payload_cost_unavailable() -> u64 {
  0
}

pub fn register_actor_context_host_functions(
  registry: &mut dyn HostRegistry,
) -> MResult<()> {
  registry.register_function(Arc::new(ActorMessageKindHostFunction::new()))?;
  registry.register_function(Arc::new(ActorMessagePayloadHostFunction::new()))?;
  registry.register_function(Arc::new(ActorStateIdHostFunction::new()))?;
  registry.register_function(Arc::new(ActorStateGetHostFunction::new()))?;
  registry.register_function(Arc::new(ActorStatePutHostFunction::new()))?;

  Ok(())
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HostInvalidContextError {
  pub function: String,
  pub reason: String,
}

impl MechErrorKind for HostInvalidContextError {
  fn name(&self) -> &str {
    "HostInvalidContext"
  }

  fn message(&self) -> String {
    format!(
      "Host function `{}` cannot run in this context: {}",
      self.function,
      self.reason
    )
  }
}

#[derive(Debug, Clone)]
pub struct InvalidHostFunctionError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidHostFunctionError {
  fn name(&self) -> &str {
    "InvalidHostFunction"
  }

  fn message(&self) -> String {
    format!("Invalid host function field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidHostCallError {
  pub function: String,
  pub reason: String,
}

impl MechErrorKind for InvalidHostCallError {
  fn name(&self) -> &str {
    "InvalidHostCall"
  }

  fn message(&self) -> String {
    format!("Invalid host call `{}`: {}", self.function, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidHostCallFieldError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidHostCallFieldError {
  fn name(&self) -> &str {
    "InvalidHostCall"
  }

  fn message(&self) -> String {
    format!("Invalid host call field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct HostFunctionAlreadyExistsError {
  pub name: String,
}

impl MechErrorKind for HostFunctionAlreadyExistsError {
  fn name(&self) -> &str {
    "HostFunctionAlreadyExists"
  }

  fn message(&self) -> String {
    format!("Host function already exists: {}", self.name)
  }
}

#[derive(Debug, Clone)]
pub struct HostFunctionNotFoundError {
  pub name: String,
}

impl MechErrorKind for HostFunctionNotFoundError {
  fn name(&self) -> &str {
    "HostFunctionNotFound"
  }

  fn message(&self) -> String {
    format!("Host function not found: {}", self.name)
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  use crate::id::RuntimeId;
  use crate::service::NoRuntimeServices;

  #[test]
  fn registry_registers_and_lists_functions() {
    let mut registry = InMemoryHostRegistry::new();

    registry
      .insert(ClosureHostFunction::new(
        "host.echo",
        |_services, _ctx, args| {
          Ok(args.into_iter().next().unwrap_or(Value::Empty))
        },
      ))
      .unwrap();

    let names = registry.list_functions().unwrap();

    assert_eq!(names, vec!["host.echo".to_string()]);
    assert!(registry.contains("host.echo"));
  }

  #[test]
  fn registry_rejects_duplicate_functions() {
    let mut registry = InMemoryHostRegistry::new();

    registry
      .insert(ClosureHostFunction::new(
        "host.echo",
        |_services, _ctx, _args| Ok(Value::Empty),
      ))
      .unwrap();

    let result = registry.insert(ClosureHostFunction::new(
      "host.echo",
      |_services, _ctx, _args| Ok(Value::Empty),
    ));

    assert!(result.is_err());
  }

  #[test]
  fn host_call_validates_name() {
    let call = HostCall::new("host.echo", Vec::new());
    assert!(call.validate().is_ok());

    let call = HostCall::new("", Vec::new());
    assert!(call.validate().is_err());
  }

  #[test]
  fn default_host_capability_request_uses_context_subject() {
    let context = RuntimeContext::new(RuntimeId(1), "task:1");

    let request = default_host_capability_request(&context, "host.echo");

    assert_eq!(request.subject, "task:1");
    assert_eq!(request.operation, "call");
    assert_eq!(request.resource, "host:host.echo");
  }

  #[test]
  fn closure_function_calls() {
    let function = ClosureHostFunction::new(
      "host.empty",
      |_services, _ctx, _args| Ok(Value::Empty),
    );

    let mut services = NoRuntimeServices;
    let mut context = RuntimeContext::new(RuntimeId(1), "task:1");

    let result = function
      .call(&mut services, &mut context, Vec::new())
      .unwrap();

    assert_eq!(result, Value::Empty);
  }
}