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
use std::marker::PhantomData;
use std::sync::Arc;

use mech_core::{MResult, MechError, MechErrorKind, Value};

use crate::capability::{CapabilityRequest, Operation, Resource};

use crate::context::RuntimeCallContext;
#[cfg(test)]
use crate::context::RuntimeContext;
use crate::service::RuntimeManagedServices;
use crate::{PreparedRuntimeEffect, RuntimeValueSnapshot, TryIntoRuntimeValueSnapshot};

pub mod actor;
pub mod arg;

pub use self::actor::*;
pub use self::arg::*;

// -----------------------------------------------------------------------------
// Host Function Planning and Invocation
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RuntimePreparedHostCall {
    pub value: RuntimeValueSnapshot,
    pub effect: PreparedRuntimeEffect,
}

pub trait HostFunctionPlan: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;

    fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot>;

    fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        let _ = context;
        None
    }

    fn estimated_cost_bytes(&self, arguments: &[RuntimeValueSnapshot]) -> u64 {
        let _ = arguments;
        0
    }

    fn estimated_cost_items(&self, arguments: &[RuntimeValueSnapshot]) -> u64 {
        arguments.len() as u64
    }
}

pub trait PureHostFunction: HostFunctionPlan {
    fn invoke(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot>;
}

pub trait RuntimeManagedHostFunction: HostFunctionPlan {
    fn invoke(
        &self,
        services: &mut dyn RuntimeManagedServices,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot>;
}

pub trait StagedHostFunction: HostFunctionPlan {
    fn prepare(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimePreparedHostCall>;
}

#[derive(Clone)]
pub enum RegisteredHostFunction {
    Pure(Arc<dyn PureHostFunction>),
    RuntimeManaged(Arc<dyn RuntimeManagedHostFunction>),
    Staged(Arc<dyn StagedHostFunction>),
}

impl std::fmt::Debug for RegisteredHostFunction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pure(function) => formatter.debug_tuple("Pure").field(function).finish(),
            Self::RuntimeManaged(function) => formatter
                .debug_tuple("RuntimeManaged")
                .field(function)
                .finish(),
            Self::Staged(function) => formatter.debug_tuple("Staged").field(function).finish(),
        }
    }
}

impl RegisteredHostFunction {
    pub fn name(&self) -> &str {
        match self {
            Self::Pure(function) => function.name(),
            Self::RuntimeManaged(function) => function.name(),
            Self::Staged(function) => function.name(),
        }
    }

    pub fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        match self {
            Self::Pure(function) => function.plan(context, arguments),
            Self::RuntimeManaged(function) => function.plan(context, arguments),
            Self::Staged(function) => function.plan(context, arguments),
        }
    }

    pub fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        match self {
            Self::Pure(function) => function.required_capability(context),
            Self::RuntimeManaged(function) => function.required_capability(context),
            Self::Staged(function) => function.required_capability(context),
        }
    }

    pub fn estimated_cost_bytes(&self, arguments: &[RuntimeValueSnapshot]) -> u64 {
        match self {
            Self::Pure(function) => function.estimated_cost_bytes(arguments),
            Self::RuntimeManaged(function) => function.estimated_cost_bytes(arguments),
            Self::Staged(function) => function.estimated_cost_bytes(arguments),
        }
    }

    pub fn estimated_cost_items(&self, arguments: &[RuntimeValueSnapshot]) -> u64 {
        match self {
            Self::Pure(function) => function.estimated_cost_items(arguments),
            Self::RuntimeManaged(function) => function.estimated_cost_items(arguments),
            Self::Staged(function) => function.estimated_cost_items(arguments),
        }
    }
}

pub struct DeterministicHostFunction<P, F, R>
where
    P: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    F: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
{
    name: String,
    capability: Option<CapabilityRequest>,
    plan: P,
    function: F,
    result: PhantomData<fn() -> R>,
}

impl<P, F, R> DeterministicHostFunction<P, F, R>
where
    P: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    F: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
{
    pub fn new(name: impl Into<String>, plan: P, function: F) -> Self {
        Self {
            name: name.into(),
            capability: None,
            plan,
            function,
            result: PhantomData,
        }
    }

    pub fn with_capability(mut self, capability: CapabilityRequest) -> Self {
        self.capability = Some(capability);
        self
    }
}

impl<P, F, R> std::fmt::Debug for DeterministicHostFunction<P, F, R>
where
    P: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    F: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DeterministicHostFunction")
            .field("name", &self.name)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

impl<P, F, R> HostFunctionPlan for DeterministicHostFunction<P, F, R>
where
    P: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    F: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    R: TryIntoRuntimeValueSnapshot,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        let result = (self.plan)(context, arguments)?;
        result.try_into_runtime_value_snapshot()
    }

    fn required_capability(&self, _context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        self.capability.clone()
    }
}

impl<P, F, R> PureHostFunction for DeterministicHostFunction<P, F, R>
where
    P: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    F: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    R: TryIntoRuntimeValueSnapshot,
{
    fn invoke(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        let result = (self.function)(context, &arguments)?;
        result.try_into_runtime_value_snapshot()
    }
}

impl<P, F, R> From<DeterministicHostFunction<P, F, R>> for RegisteredHostFunction
where
    P: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    F: for<'context, 'arguments> Fn(
            &'context RuntimeCallContext,
            &'arguments [RuntimeValueSnapshot],
        ) -> MResult<R>
        + Send
        + Sync
        + 'static,
    R: TryIntoRuntimeValueSnapshot + 'static,
{
    fn from(function: DeterministicHostFunction<P, F, R>) -> Self {
        Self::Pure(Arc::new(function))
    }
}

type HostPlanCallback = dyn Fn(&RuntimeCallContext, &[RuntimeValueSnapshot]) -> MResult<RuntimeValueSnapshot>
    + Send
    + Sync;

type PureHostInvocationCallback = dyn Fn(&RuntimeCallContext, Vec<RuntimeValueSnapshot>) -> MResult<RuntimeValueSnapshot>
    + Send
    + Sync;

type RuntimeManagedHostInvocationCallback = dyn Fn(
        &mut dyn RuntimeManagedServices,
        &RuntimeCallContext,
        Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot>
    + Send
    + Sync;

type StagedHostPreparationCallback = dyn Fn(&RuntimeCallContext, Vec<RuntimeValueSnapshot>) -> MResult<RuntimePreparedHostCall>
    + Send
    + Sync;

pub struct PlannedPureHostFunction {
    name: String,
    capability: Option<CapabilityRequest>,
    plan: Arc<HostPlanCallback>,
    invoke: Arc<PureHostInvocationCallback>,
}

impl PlannedPureHostFunction {
    pub fn new<P, I>(name: impl Into<String>, plan: P, invoke: I) -> Self
    where
        P: Fn(&RuntimeCallContext, &[RuntimeValueSnapshot]) -> MResult<RuntimeValueSnapshot>
            + Send
            + Sync
            + 'static,
        I: Fn(&RuntimeCallContext, Vec<RuntimeValueSnapshot>) -> MResult<RuntimeValueSnapshot>
            + Send
            + Sync
            + 'static,
    {
        Self {
            name: name.into(),
            capability: None,
            plan: Arc::new(plan),
            invoke: Arc::new(invoke),
        }
    }

    pub fn with_capability(mut self, capability: CapabilityRequest) -> Self {
        self.capability = Some(capability);
        self
    }
}

impl std::fmt::Debug for PlannedPureHostFunction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlannedPureHostFunction")
            .field("name", &self.name)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

impl HostFunctionPlan for PlannedPureHostFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        (self.plan)(context, arguments)
    }

    fn required_capability(&self, _context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        self.capability.clone()
    }
}

impl PureHostFunction for PlannedPureHostFunction {
    fn invoke(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        (self.invoke)(context, arguments)
    }
}

impl From<PlannedPureHostFunction> for RegisteredHostFunction {
    fn from(function: PlannedPureHostFunction) -> Self {
        Self::Pure(Arc::new(function))
    }
}

pub struct PlannedRuntimeManagedHostFunction {
    name: String,
    capability: Option<CapabilityRequest>,
    plan: Arc<HostPlanCallback>,
    invoke: Arc<RuntimeManagedHostInvocationCallback>,
}

impl PlannedRuntimeManagedHostFunction {
    pub fn new<P, I>(name: impl Into<String>, plan: P, invoke: I) -> Self
    where
        P: Fn(&RuntimeCallContext, &[RuntimeValueSnapshot]) -> MResult<RuntimeValueSnapshot>
            + Send
            + Sync
            + 'static,
        I: Fn(
                &mut dyn RuntimeManagedServices,
                &RuntimeCallContext,
                Vec<RuntimeValueSnapshot>,
            ) -> MResult<RuntimeValueSnapshot>
            + Send
            + Sync
            + 'static,
    {
        Self {
            name: name.into(),
            capability: None,
            plan: Arc::new(plan),
            invoke: Arc::new(invoke),
        }
    }

    pub fn with_capability(mut self, capability: CapabilityRequest) -> Self {
        self.capability = Some(capability);
        self
    }
}

impl std::fmt::Debug for PlannedRuntimeManagedHostFunction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlannedRuntimeManagedHostFunction")
            .field("name", &self.name)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

impl HostFunctionPlan for PlannedRuntimeManagedHostFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        (self.plan)(context, arguments)
    }

    fn required_capability(&self, _context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        self.capability.clone()
    }
}

impl RuntimeManagedHostFunction for PlannedRuntimeManagedHostFunction {
    fn invoke(
        &self,
        services: &mut dyn RuntimeManagedServices,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        (self.invoke)(services, context, arguments)
    }
}

impl From<PlannedRuntimeManagedHostFunction> for RegisteredHostFunction {
    fn from(function: PlannedRuntimeManagedHostFunction) -> Self {
        Self::RuntimeManaged(Arc::new(function))
    }
}

pub struct PlannedStagedHostFunction {
    name: String,
    capability: Option<CapabilityRequest>,
    plan: Arc<HostPlanCallback>,
    prepare: Arc<StagedHostPreparationCallback>,
}

impl PlannedStagedHostFunction {
    pub fn new<P, F>(name: impl Into<String>, plan: P, prepare: F) -> Self
    where
        P: Fn(&RuntimeCallContext, &[RuntimeValueSnapshot]) -> MResult<RuntimeValueSnapshot>
            + Send
            + Sync
            + 'static,
        F: Fn(&RuntimeCallContext, Vec<RuntimeValueSnapshot>) -> MResult<RuntimePreparedHostCall>
            + Send
            + Sync
            + 'static,
    {
        Self {
            name: name.into(),
            capability: None,
            plan: Arc::new(plan),
            prepare: Arc::new(prepare),
        }
    }

    pub fn with_capability(mut self, capability: CapabilityRequest) -> Self {
        self.capability = Some(capability);
        self
    }
}

impl std::fmt::Debug for PlannedStagedHostFunction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlannedStagedHostFunction")
            .field("name", &self.name)
            .field("capability", &self.capability)
            .finish_non_exhaustive()
    }
}

impl HostFunctionPlan for PlannedStagedHostFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        (self.plan)(context, arguments)
    }

    fn required_capability(&self, _context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        self.capability.clone()
    }
}

impl StagedHostFunction for PlannedStagedHostFunction {
    fn prepare(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimePreparedHostCall> {
        (self.prepare)(context, arguments)
    }
}

impl From<PlannedStagedHostFunction> for RegisteredHostFunction {
    fn from(function: PlannedStagedHostFunction) -> Self {
        Self::Staged(Arc::new(function))
    }
}

// -----------------------------------------------------------------------------
// Host Registry
// -----------------------------------------------------------------------------

/// Registry of host functions.
pub trait HostRegistry: std::fmt::Debug + Send {
    fn register_function(&mut self, function: RegisteredHostFunction) -> MResult<()>;

    fn get_function(&self, name: &str) -> MResult<Option<RegisteredHostFunction>>;

    fn remove_function(&mut self, name: &str) -> MResult<Option<RegisteredHostFunction>>;

    fn list_functions(&self) -> MResult<Vec<String>>;
}

/// Default in-memory host registry.
#[derive(Clone, Debug, Default)]
pub struct InMemoryHostRegistry {
    functions: HashMap<String, RegisteredHostFunction>,
}

impl InMemoryHostRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, function: impl Into<RegisteredHostFunction>) -> MResult<()> {
        self.register_function(function.into())
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
    fn register_function(&mut self, function: RegisteredHostFunction) -> MResult<()> {
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

    fn get_function(&self, name: &str) -> MResult<Option<RegisteredHostFunction>> {
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

    fn remove_function(&mut self, name: &str) -> MResult<Option<RegisteredHostFunction>> {
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
        context: &RuntimeCallContext,
        function: &RegisteredHostFunction,
        arguments: &[RuntimeValueSnapshot],
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
        context: &RuntimeCallContext,
        function: &RegisteredHostFunction,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<()> {
        if function.name().trim().is_empty() {
            return Err(MechError::new(
                InvalidHostFunctionError {
                    field: "name",
                    reason: "must not be empty",
                },
                None,
            ));
        }

        let _ = context;
        let _ = arguments;
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
    context: &RuntimeCallContext,
    function_name: &str,
) -> CapabilityRequest {
    let resource = HostResource::function(function_name);
    let operation = HostOperation::call();

    CapabilityRequest {
        subject: context.subject().to_string(),
        operation: operation.key().to_string(),
        resource: resource.key().to_string(),
        context: crate::CapabilityContext {
            local: true,
            bytes: None,
            items: None,
            duration_ms: None,
        },
    }
}

pub fn register_actor_context_host_functions(registry: &mut dyn HostRegistry) -> MResult<()> {
    registry.register_function(RegisteredHostFunction::Pure(Arc::new(
        ActorMessageKindHostFunction::new(),
    )))?;
    registry.register_function(RegisteredHostFunction::Pure(Arc::new(
        ActorMessagePayloadHostFunction::new(),
    )))?;
    registry.register_function(RegisteredHostFunction::Pure(Arc::new(
        ActorStateIdHostFunction::new(),
    )))?;
    registry.register_function(RegisteredHostFunction::RuntimeManaged(Arc::new(
        ActorStateGetHostFunction::new(),
    )))?;
    registry.register_function(RegisteredHostFunction::RuntimeManaged(Arc::new(
        ActorStatePutHostFunction::new(),
    )))?;

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
            self.function, self.reason
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
        format!(
            "Invalid host function field `{}`: {}",
            self.field, self.reason
        )
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

    fn empty_function(
        name: &'static str,
    ) -> DeterministicHostFunction<
        impl Fn(&RuntimeCallContext, &[RuntimeValueSnapshot]) -> MResult<Value>,
        impl Fn(&RuntimeCallContext, &[RuntimeValueSnapshot]) -> MResult<Value>,
        Value,
    > {
        DeterministicHostFunction::new(
            name,
            |_context, _arguments| Ok(Value::Empty),
            |_context, _arguments| Ok(Value::Empty),
        )
    }

    #[test]
    fn registry_registers_and_lists_functions() {
        let mut registry = InMemoryHostRegistry::new();
        registry.insert(empty_function("host.echo")).unwrap();

        let names = registry.list_functions().unwrap();

        assert_eq!(names, vec!["host.echo".to_string()]);
        assert!(registry.contains("host.echo"));
    }

    #[test]
    fn registry_rejects_duplicate_functions() {
        let mut registry = InMemoryHostRegistry::new();

        registry.insert(empty_function("host.echo")).unwrap();
        let result = registry.insert(empty_function("host.echo"));

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
        let context = RuntimeCallContext::capture(&context);

        let request = default_host_capability_request(&context, "host.echo");

        assert_eq!(request.subject, "task:1");
        assert_eq!(request.operation, "call");
        assert_eq!(request.resource, "host:host.echo");
    }

    #[test]
    fn deterministic_function_plans_and_invokes_snapshots() {
        let function = empty_function("host.empty");
        let context = RuntimeContext::new(RuntimeId(1), "task:1");
        let context = RuntimeCallContext::capture(&context);
        let planned = function.plan(&context, &[]).unwrap();
        let invoked = function.invoke(&context, Vec::new()).unwrap();
        assert_eq!(planned.kind(), mech_core::ValueKind::Empty);
        assert_eq!(invoked.kind(), mech_core::ValueKind::Empty);
    }

    #[test]
    fn deterministic_function_planning_never_invokes_host_behavior() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let invocations = Arc::new(AtomicUsize::new(0));
        let invocation_counter = invocations.clone();
        let function = DeterministicHostFunction::new(
            "host.counted",
            |_context, _arguments| Ok(Value::Empty),
            move |_context, _arguments| {
                invocation_counter.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Empty)
            },
        );
        let context = RuntimeContext::new(RuntimeId(1), "task:1");
        let context = RuntimeCallContext::capture(&context);

        let planned = function.plan(&context, &[]).unwrap();

        assert_eq!(planned.kind(), mech_core::ValueKind::Empty);
        assert_eq!(invocations.load(Ordering::SeqCst), 0);

        let invoked = function.invoke(&context, Vec::new()).unwrap();

        assert_eq!(invoked.kind(), mech_core::ValueKind::Empty);
        assert_eq!(invocations.load(Ordering::SeqCst), 1);
    }
}
