// Host Calls
// -----------------------------------------------------------------------------

// This file defines the logic for handling host calls in the Mech runtime. Host calls are a mechanism for Mech programs to interact with the host environment, allowing them to call functions that are implemented outside of the Mech program itself, typically in Rust. This is a crucial part of the runtime, as it enables Mech to be extended and embedded in other programming environments.

// The runtime provides the following host methods:

// - `MechRuntimeBuilder::host_function`: Registers a planned host function
//   before the runtime is built.
// - `call_host`: Executes a host call by name with the provided arguments. It emits events for the start, completion, and failure of the host call, allowing for observability of host interactions. It also checks the host policy to ensure that the call is allowed and charges the appropriate costs based on the function's estimated cost. A version of the function that accepts a MechRuntimeContext is also provided.

// Furthermore, this file defines two structs:

// `RuntimeHostNativeFunctionCompiler`, which allows for host functions to be registered as native function compilers in the Mech program, enabling them to be called directly from Mech code. The `RuntimeHostNativeFunction` struct represents a compiled host function that can be executed within the Mech program.

// For example, a function to compute an affine transformation could be registered as a host function, and then called from Mech code like this:
/*
  let runtime = MechRuntime::builder().host_function(DeterministicHostFunction::new(
    "demo/math/affine",
    |_context, args| {
      host_call3(
        "demo/math/affine",
        &args,
        |x: f64, scale: f64, offset: f64| {
          (x * scale) + offset
        },
      )
    },
  ))?.build()?;
*/
// Then in Mech:
/*
  result := demo/math/affine(2.0, 3.0, 4.0);
*/


use super::*;
use super::extension::invoke_extension;
use super::execution::{
  ACTIVATION_EFFECT_BARRIER_NAME,
  ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
  ActivationEffectBarrierCompiler,
  ActivationEffectPayloadCaptureCompiler,
};
use mech_core::{
  GuardFunctionSafety, MechExecutionServices, Ref,
  ValueKind,
};
use crate::{
  RegisteredHostFunction, RuntimeCallContext,
  RuntimeValueSnapshot,
};

impl MechRuntime {

  fn install_runtime_program_host_compilers(
    program: &mut MechProgram,
    context: RuntimeCallContext,
    functions: Vec<RegisteredHostFunction>,
  ) {
    program.register_native_function_compiler(
      ACTIVATION_EFFECT_BARRIER_NAME,
      Arc::new(ActivationEffectBarrierCompiler),
    );
    program.register_native_function_compiler(
      ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
      Arc::new(ActivationEffectPayloadCaptureCompiler),
    );
    for function in functions {
      let name = function.name().to_string();
      program.register_native_function_compiler(
        name.clone(),
        Arc::new(RuntimeHostNativeFunctionCompiler::new(
          name.clone(),
          name,
          context.clone(),
          function,
        )),
      );
    }
  }

  pub(super) fn register_retained_program_host_functions(
    &mut self,
    context: &RuntimeContext,
  ) -> MResult<()> {
    let functions =
      self.registered_host_functions()?;
    Self::install_runtime_program_host_compilers(
      &mut self.program,
      RuntimeCallContext::capture(context),
      functions,
    );
    Ok(())
  }

  pub(super) fn register_runtime_program_host_functions(
    &mut self,
    context: &mut RuntimeContext,
    program: &mut MechProgram,
  ) -> MResult<()> {
    let functions = self.registered_host_functions()?;
    Self::install_runtime_program_host_compilers(
      program,
      RuntimeCallContext::capture(context),
      functions,
    );
    Ok(())
  }

  fn registered_host_functions(
    &self,
  ) -> MResult<Vec<RegisteredHostFunction>> {
    self
      .host_registry
      .list_functions()?
      .into_iter()
      .map(|name| {
        self
          .host_registry
          .get_function(&name)?
          .ok_or_else(|| {
            MechError::new(
              HostFunctionNotFoundError { name },
              None,
            )
          })
      })
      .collect()
  }

  pub fn call_host(
    &mut self,
    call: HostCall,
  ) -> MResult<RuntimeValueSnapshot> {
    let mut context = self.runtime_context()?;
    self.call_host_with_context(&mut context, call)
  }

  pub fn call_host_with_context(
    &mut self,
    context: &mut RuntimeContext,
    call: HostCall,
  ) -> MResult<RuntimeValueSnapshot> {
    self
      .call_host_value_with_context(context, call)
      .map(|value| RuntimeValueSnapshot::capture(&value))
  }

  pub(crate) fn call_host_value_with_context(
    &mut self,
    context: &mut RuntimeContext,
    call: HostCall,
  ) -> MResult<Value> {
    self.ensure_runtime_mutation_allowed("call_host_with_context")?;
    self.validate_context_for_runtime(context)?;
    call.validate()?;
    let implicit = context.transaction.is_none();
    let transaction_id = if implicit {
      Some(self.begin_transaction(context)?)
    } else {
      context.transaction
    };
    let result = self.with_runtime_execution_session(
      context,
      |session| {
        session.invoke_native(&call.name, &call.args)
      },
    );
    if !implicit {
      return result;
    }
    match result {
      Ok(value) => {
        self.commit_runtime_transaction(context)?;
        Ok(value)
      }
      Err(error) => {
        let original = format!("{error:?}");
        match self.abort_runtime_transaction(
          context,
          format!("host call `{}` failed", call.name),
        ) {
          Ok(()) => Err(error),
          Err(cleanup_error) => Err(self.poison_program_operation(
            "call_host_with_context",
            transaction_id,
            original,
            vec![format!(
              "implicit host transaction cleanup failed: {cleanup_error:?}",
            )],
          )),
        }
      }
    }
  }
}


#[derive(Clone, Debug)]
pub struct RuntimeHostNativeFunctionCompiler {
  pub mech_name: String,
  pub host_name: String,
  pub context: RuntimeCallContext,
  pub function: RegisteredHostFunction,
}

impl RuntimeHostNativeFunctionCompiler {
  pub fn new(
    mech_name: impl Into<String>,
    host_name: impl Into<String>,
    context: RuntimeCallContext,
    function: RegisteredHostFunction,
  ) -> Self {
    Self {
      mech_name: mech_name.into(),
      host_name: host_name.into(),
      context,
      function,
    }
  }
}

impl NativeFunctionCompiler for RuntimeHostNativeFunctionCompiler {
  fn guard_safety(&self) -> GuardFunctionSafety {
    GuardFunctionSafety::Unsupported
  }

  fn compile(
    &self,
    arguments: &Vec<Value>,
  ) -> MResult<Box<dyn mech_core::MechFunction>> {
    let argument_snapshots = arguments
      .iter()
      .map(RuntimeValueSnapshot::capture)
      .collect::<Vec<_>>();
    let planned = invoke_extension(
      format!("host function `{}`", self.host_name),
      "plan",
      || {
        self.function
          .plan(&self.context, &argument_snapshots)
      },
    )?;
    Ok(Box::new(RuntimeHostNativeFunction {
      name: self.mech_name.clone(),
      host_name: self.host_name.clone(),
      arguments: arguments.clone(),
      value: Ref::new(
        planned.into_value().deep_snapshot(),
      ),
    }))
  }
}

#[derive(Clone, Debug)]
pub struct RuntimeHostNativeFunction {
  pub name: String,
  pub host_name: String,
  pub arguments: Vec<Value>,
  pub value: Ref<Value>,
}

#[derive(Debug, Clone)]
pub struct RuntimeHostOutputUpdateError {
  pub function: String,
  pub expected: ValueKind,
  pub actual: ValueKind,
  pub reason: String,
}
impl MechErrorKind for RuntimeHostOutputUpdateError {
  fn name(&self) -> &str { "RuntimeHostOutputUpdateError" }
  fn message(&self) -> String {
    format!(
      "host function `{}` returned unsupported or incompatible output kind {:?}; expected {:?}: {}",
      self.function,
      self.actual,
      self.expected,
      self.reason,
    )
  }
}

impl RuntimeHostNativeFunction {
  fn update_output(&self, next: Value) -> MResult<()> {
    let expected = self.value.borrow().kind();
    let actual = next.kind();
    if expected == ValueKind::Empty {
      *self.value.borrow_mut() = next.deep_snapshot();
      return Ok(());
    }
    mech_program::apply_stable_value_update(self.value.clone(), next)
      .map(|_| ())
      .map_err(|error| {
        MechError::new(
          RuntimeHostOutputUpdateError {
            function: self.name.clone(),
            expected,
            actual,
            reason: format!("{error:?}"),
          },
          None,
        )
      })
  }

  fn solve_inner(
    &self,
    services: &mut dyn mech_core::MechExecutionServices,
  ) -> MResult<()> {
    let next = services.invoke_native(
      &self.host_name,
      &self.arguments,
    )?;
    self.update_output(next)
  }
}

impl MechFunctionImpl for RuntimeHostNativeFunction {
  fn solve(&self) {
    let mut services = mech_core::NoMechExecutionServices;
    if let Err(error) = self.solve_inner(&mut services) {
      eprintln!(
        "[Mech Runtime Host Error] function `{}` failed during solve; preserving previous output: {:?}",
        self.name,
        error,
      );
    }
  }

  fn solve_result(&self) -> MResult<()> {
    let mut services = mech_core::NoMechExecutionServices;
    self.solve_inner(&mut services)
  }

  fn solve_result_with(
    &self,
    services: &mut dyn mech_core::MechExecutionServices,
  ) -> MResult<()> {
    self.solve_inner(services)
  }

  fn solve_reactive_with(
    &self,
    services: &mut dyn mech_core::MechExecutionServices,
  ) -> MResult<mech_core::ReactiveSolveStatus> {
    self.solve_inner(services)?;
    Ok(mech_core::ReactiveSolveStatus::Changed)
  }

  fn out(&self) -> Value {
    self.value.borrow().clone()
  }

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(vec![
      Value::MutableReference(self.value.clone()),
    ])
  }

  fn to_string(&self) -> String {
    format!("RuntimeHostNativeFunction::{}", self.name)
  }
}

impl MechFunctionCompiler for RuntimeHostNativeFunction {
  fn compile(
    &self,
    _ctx: &mut CompileCtx,
  ) -> MResult<Register> {
    Err(MechError::new(
      RuntimeHostFunctionNotBytecodeCompilableError {
        function: self.name.clone(),
      },
      None,
    ))
  }
}

#[cfg(test)]
mod transaction_tests {
  use super::*;
  use std::sync::{Arc, Mutex};
  use std::sync::atomic::{AtomicUsize, Ordering};
  use crate::{
    BasicCapability, BasicConstraints, BasicOperation, BasicResource,
    BasicSubject, Capability, CapabilityDecision, CapabilityRequest,
    PlannedPureHostFunction, PlannedRuntimeManagedHostFunction,
    PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeAfterCommitEffect,
    RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimePreparedHostCall, RuntimeTransactionalEffect,
  };

  #[derive(Debug)]
  struct RecordingHostEffect {
    log: Arc<Mutex<Vec<String>>>,
    entry: String,
  }

  impl RuntimeAfterCommitEffect for RecordingHostEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::HostFunction {
          name: "demo/staged".to_string(),
        },
        "deliver",
      )
    }

    fn deliver(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push(self.entry.clone());
      Ok(())
    }
  }

  fn grant_host_call(runtime: &mut MechRuntime, name: &str) {
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
      .grant_capability(Arc::new(BasicCapability::new(
        CapabilityId(700),
        &BasicSubject::new(&subject),
        &BasicResource::new(format!("host:{name}")),
        [BasicOperation::new("call")],
      )))
      .unwrap();
  }

  fn grant_limited_host_call(
    runtime: &mut MechRuntime,
    id: CapabilityId,
    name: &str,
  ) {
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
      .grant_capability(Arc::new(
        BasicCapability::new(
          id,
          &BasicSubject::new(&subject),
          &BasicResource::new(format!("host:{name}")),
          [BasicOperation::new("call")],
        )
        .with_constraints(
          BasicConstraints::default().with_max_uses(1),
        ),
      ))
      .unwrap();
  }

  #[derive(Debug)]
  struct PreviewUnsupportedCapability {
    id: CapabilityId,
    subject: String,
    resource: String,
  }

  impl Capability for PreviewUnsupportedCapability {
    fn id(&self) -> CapabilityId {
      self.id
    }

    fn subject_key(&self) -> &str {
      &self.subject
    }

    fn validate(&self) -> MResult<()> {
      Ok(())
    }

    fn check(
      &self,
      request: &CapabilityRequest,
    ) -> MResult<CapabilityDecision> {
      Ok(if request.subject == self.subject
        && request.operation == "call"
        && request.resource == self.resource
      {
        CapabilityDecision::allow()
      } else {
        CapabilityDecision::deny("request does not match")
      })
    }
  }

  #[derive(Debug)]
  struct PreviewLifecycleEffect {
    log: Arc<Mutex<Vec<String>>>,
  }

  #[derive(Debug)]
  struct CountingAfterCommitEffect {
    deliveries: Arc<AtomicUsize>,
  }

  impl RuntimeAfterCommitEffect for CountingAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::HostFunction {
          name: "demo/staged-limited".to_string(),
        },
        "deliver",
      )
    }

    fn deliver(&mut self) -> MResult<()> {
      self.deliveries.fetch_add(1, Ordering::SeqCst);
      Ok(())
    }
  }

  impl RuntimeTransactionalEffect for PreviewLifecycleEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::HostFunction {
          name: "demo/staged-lifecycle".to_string(),
        },
        "preview-lifecycle",
      )
    }

    fn prepare(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push("prepare".to_string());
      Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push("commit".to_string());
      Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push("abort".to_string());
      Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "preview_lifecycle_abort",
          reason: "abort must not run for preview-only effects".to_string(),
        },
        None,
      ))
    }
  }

  #[test]
  fn staged_host_call_returns_value_before_effect_delivery() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedStagedHostFunction::new(
        "demo/staged",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::String(Ref::new("provisional".to_string())).into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          Ok(RuntimePreparedHostCall {
            value: Value::String(Ref::new("provisional".to_string())).into(),
            effect: PreparedRuntimeEffect::AfterCommit(Box::new(
              RecordingHostEffect {
                log: effect_log.clone(),
                entry: "delivered".to_string(),
              },
            )),
          })
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_host_call(&mut runtime, "demo/staged");
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let value = runtime
      .call_host_with_context(
        &mut context,
        HostCall::new("demo/staged", Vec::new()),
      )
      .unwrap();

    assert_eq!(
      value.as_value(),
      &Value::String(Ref::new("provisional".to_string())),
    );
    assert!(log.lock().unwrap().is_empty());

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(
      log.lock().unwrap().as_slice(),
      &["delivered".to_string()],
    );
  }

  #[test]
  fn planned_pure_host_runs_inside_implicit_and_explicit_transactions() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedPureHostFunction::new(
        "demo/pure",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::F64(Ref::new(42.0)).into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          callback_calls.fetch_add(1, Ordering::SeqCst);
          Ok(Value::F64(Ref::new(42.0)).into())
        },
      ))
      .unwrap();
    let mut runtime = runtime.build().unwrap();
    grant_host_call(&mut runtime, "demo/pure");

    runtime.run_string("implicit := demo/pure()").unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .run_string_with_context(
        &mut context,
        "explicit := demo/pure()",
      )
      .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
  }

  #[test]
  fn planning_never_invokes_a_host_callback() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let callback_invocations = invocations.clone();
    let runtime = MechRuntime::builder()
      .host_function(PlannedPureHostFunction::new(
        "demo/plan-only",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::Empty.into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          callback_invocations.fetch_add(1, Ordering::SeqCst);
          Ok(Value::Empty.into())
        },
      ))
      .unwrap()
      .build()
      .unwrap();

    assert_eq!(invocations.load(Ordering::SeqCst), 0);
    assert!(runtime.program.root_symbol_value("missing").is_err());
  }

  #[test]
  fn pure_host_planning_does_not_consume_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedPureHostFunction::new(
        "demo/pure-limited",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::F64(Ref::new(1.0)).into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          callback_calls.fetch_add(1, Ordering::SeqCst);
          Ok(Value::F64(Ref::new(1.0)).into())
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_limited_host_call(
      &mut runtime,
      CapabilityId(710),
      "demo/pure-limited",
    );

    runtime
      .run_string("pure-limited-result := demo/pure-limited()")
      .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(runtime
      .call_host(HostCall::new("demo/pure-limited", Vec::new()))
      .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn runtime_managed_planning_does_not_consume_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(
        PlannedRuntimeManagedHostFunction::new(
          "demo/managed-limited",
          |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
            Ok(Value::F64(Ref::new(1.0)).into())
          },
          move |_services, _context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
            callback_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Value::F64(Ref::new(1.0)).into())
          },
        ),
      )
      .unwrap()
      .build()
      .unwrap();
    grant_limited_host_call(
      &mut runtime,
      CapabilityId(711),
      "demo/managed-limited",
    );

    runtime
      .run_string("managed-limited-result := demo/managed-limited()")
      .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(runtime
      .call_host(HostCall::new("demo/managed-limited", Vec::new()))
      .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn staged_planning_does_not_consume_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let deliveries = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let delivered = deliveries.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedStagedHostFunction::new(
        "demo/staged-limited",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::F64(Ref::new(1.0)).into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          callback_calls.fetch_add(1, Ordering::SeqCst);
          let delivered = delivered.clone();
          Ok(RuntimePreparedHostCall {
            value: Value::F64(Ref::new(1.0)).into(),
            effect: PreparedRuntimeEffect::AfterCommit(Box::new(
              CountingAfterCommitEffect {
                deliveries: delivered,
              },
            )),
          })
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_limited_host_call(
      &mut runtime,
      CapabilityId(712),
      "demo/staged-limited",
    );

    runtime
      .run_string("staged-limited-result := demo/staged-limited()")
      .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(deliveries.load(Ordering::SeqCst), 1);
    assert!(runtime
      .call_host(HostCall::new("demo/staged-limited", Vec::new()))
      .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn custom_capability_without_preview_contract_fails_closed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedPureHostFunction::new(
        "demo/unsupported-preview",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::Empty.into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          callback_calls.fetch_add(1, Ordering::SeqCst);
          Ok(Value::Empty.into())
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
      .grant_capability(Arc::new(PreviewUnsupportedCapability {
        id: CapabilityId(713),
        subject,
        resource: "host:demo/unsupported-preview".to_string(),
      }))
      .unwrap();
    let error = runtime
      .run_string(
        "unsupported-preview-result := demo/unsupported-preview()",
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
  }

  #[test]
  fn staged_planning_does_not_create_effects() {
    let lifecycle = Arc::new(Mutex::new(Vec::new()));
    let effect_log = lifecycle.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedStagedHostFunction::new(
        "demo/staged-lifecycle",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::F64(Ref::new(1.0)).into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          Ok(RuntimePreparedHostCall {
            value: Value::F64(Ref::new(1.0)).into(),
            effect: PreparedRuntimeEffect::Transactional(Box::new(
              PreviewLifecycleEffect {
                log: effect_log.clone(),
              },
            )),
          })
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    assert!(lifecycle.lock().unwrap().is_empty());
    grant_host_call(&mut runtime, "demo/staged-lifecycle");

    runtime
      .run_string(
        "staged-lifecycle-result := demo/staged-lifecycle()",
      )
      .unwrap();

    assert_eq!(
      lifecycle.lock().unwrap().as_slice(),
      &["prepare".to_string(), "commit".to_string()],
    );
  }

  #[test]
  fn failed_later_operation_discards_only_its_staged_host_effect() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedStagedHostFunction::new(
        "demo/staged",
        |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
          Ok(Value::String(Ref::new("provisional".to_string())).into())
        },
        move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
          Ok(RuntimePreparedHostCall {
            value: Value::String(Ref::new("provisional".to_string())).into(),
            effect: PreparedRuntimeEffect::AfterCommit(Box::new(
              RecordingHostEffect {
                log: effect_log.clone(),
                entry: "delivered".to_string(),
              },
            )),
          })
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_host_call(&mut runtime, "demo/staged");
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    runtime
      .run_string_with_context(
        &mut context,
        "first := demo/staged()",
      )
      .unwrap();
    let failed = runtime.run_string_with_context(
      &mut context,
      "discarded := demo/staged()\nbroken := missing + 1",
    );

    assert!(failed.is_err());
    assert!(runtime.program.root_symbol_value("first").is_ok());
    assert!(runtime.program.root_symbol_value("discarded").is_err());
    assert!(log.lock().unwrap().is_empty());

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(
      log.lock().unwrap().as_slice(),
      &["delivered".to_string()],
    );
  }

  #[test]
  fn runtime_managed_planning_does_not_duplicate_staged_mutation() {
    let observed_ids = Arc::new(Mutex::new(Vec::new()));
    let callback_ids = observed_ids.clone();
    let mut runtime = MechRuntime::builder()
      .host_function(
        PlannedRuntimeManagedHostFunction::new(
          "demo/runtime-managed",
          |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
            Ok(Value::String(Ref::new("planned".to_string())).into())
          },
          move |services, _context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
            let id = services.allocate_object_id()?;
            callback_ids.lock().unwrap().push(id);
            services.put_object(
              ObjectRecord::text(id, "preview-test", "value"),
            )?;
            Ok(Value::String(Ref::new(id.to_string())).into())
          },
        ),
      )
      .unwrap()
      .build()
      .unwrap();
    grant_host_call(&mut runtime, "demo/runtime-managed");

    runtime
      .run_string("result := demo/runtime-managed()")
      .unwrap();

    let ids = observed_ids.lock().unwrap().clone();
    assert_eq!(ids.len(), 1);
    assert!(runtime.store().get_object(ids[0]).unwrap().is_some());
  }

  #[test]
  fn host_planning_panics_are_converted_without_invocation() {
    let plan_calls = Arc::new(AtomicUsize::new(0));
    let invoke_calls = Arc::new(AtomicUsize::new(0));
    let plan_count = plan_calls.clone();
    let invoke_count = invoke_calls.clone();
    let runtime = MechRuntime::builder().build().unwrap();
    let context = RuntimeCallContext::capture(
      &runtime.runtime_context().unwrap(),
    );
    let compiler = RuntimeHostNativeFunctionCompiler::new(
      "sealed/plan-panic",
      "sealed/plan-panic",
      context,
      PlannedPureHostFunction::new(
        "sealed/plan-panic",
        move |_context, _arguments| {
          plan_count.fetch_add(1, Ordering::SeqCst);
          panic!("deliberate host plan panic");
        },
        move |_context, _arguments| {
          invoke_count.fetch_add(1, Ordering::SeqCst);
          Ok(Value::Empty.into())
        },
      )
      .into(),
    );

    let error = match compiler.compile(&Vec::new()) {
      Ok(_) => panic!("planning panic should be converted to an error"),
      Err(error) => error,
    };

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate host plan panic"));
    assert_eq!(plan_calls.load(Ordering::SeqCst), 1);
    assert_eq!(invoke_calls.load(Ordering::SeqCst), 0);
  }

  #[test]
  fn pure_host_panic_rolls_back_and_restores_program_and_guard() {
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedPureHostFunction::new(
        "sealed/pure-panic",
        |_context, _arguments| {
          Ok(Value::F64(Ref::new(1.0)).into())
        },
        |_context, _arguments| {
          panic!("deliberate pure host panic");
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_host_call(&mut runtime, "sealed/pure-panic");
    runtime.run_string("panic-anchor := 1.0").unwrap();

    let error = runtime
      .run_string("discarded := sealed/pure-panic()")
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate pure host panic"));
    assert!(runtime.program.root_symbol_value("panic-anchor").is_ok());
    assert!(runtime.program.root_symbol_value("discarded").is_err());
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
  }

  #[test]
  fn runtime_managed_host_panic_is_an_ordinary_rollback_failure() {
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedRuntimeManagedHostFunction::new(
        "sealed/managed-panic",
        |_context, _arguments| {
          Ok(Value::F64(Ref::new(1.0)).into())
        },
        |_services, _context, _arguments| {
          panic!("deliberate runtime-managed host panic");
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_host_call(&mut runtime, "sealed/managed-panic");

    let error = runtime
      .run_string("discarded := sealed/managed-panic()")
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}")
      .contains("deliberate runtime-managed host panic"));
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
  }

  #[test]
  fn staged_host_prepare_panic_stages_no_effect() {
    let mut runtime = MechRuntime::builder()
      .host_function(PlannedStagedHostFunction::new(
        "sealed/staged-panic",
        |_context, _arguments| {
          Ok(Value::F64(Ref::new(1.0)).into())
        },
        |_context, _arguments| {
          panic!("deliberate staged host prepare panic");
        },
      ))
      .unwrap()
      .build()
      .unwrap();
    grant_host_call(&mut runtime, "sealed/staged-panic");

    let error = runtime
      .run_string("discarded := sealed/staged-panic()")
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}")
      .contains("deliberate staged host prepare panic"));
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
  }
}

#[cfg(test)]
mod checkpoint_tests {
  use super::*;

  #[test]
  fn runtime_host_native_function_output_round_trips_through_program_checkpoint() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let plan = program.interpreter().plan();
    let value = Ref::new(Value::Empty);
    let value_address = value.addr();
    plan.add_function(Box::new(RuntimeHostNativeFunction {
      name: "test/host".to_string(),
      host_name: "test/host".to_string(),
      arguments: Vec::new(),
      value: value.clone(),
    }));
    let checkpoint = program.checkpoint().unwrap();
    let replacement = Ref::new(Value::Index(Ref::new(99)));
    *value.borrow_mut() = Value::MutableReference(replacement);

    program.restore(checkpoint).unwrap();

    assert_eq!(value.addr(), value_address);
    assert_eq!(*value.borrow(), Value::Empty);
    assert!(program.checkpoint().is_ok());
  }
}
