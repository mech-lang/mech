// Host Calls
// -----------------------------------------------------------------------------

// This file defines the logic for handling host calls in the Mech runtime. Host calls are a mechanism for Mech programs to interact with the host environment, allowing them to call functions that are implemented outside of the Mech program itself, typically in Rust. This is a crucial part of the runtime, as it enables Mech to be extended and embedded in other programming environments.

// The runtime provides the following host methods:

// - `register_mech_host_function`: Registers a new host function that can be called from Mech programs. The function must implement the `HostFunction` trait, which defines how the function is called and what arguments it accepts.
// - `call_host`: Executes a host call by name with the provided arguments. It emits events for the start, completion, and failure of the host call, allowing for observability of host interactions. It also checks the host policy to ensure that the call is allowed and charges the appropriate costs based on the function's estimated cost. A version of the function that accepts a MechRuntimeContext is also provided.

// Furthermore, this file defines two structs:

// `RuntimeHostNativeFunctionCompiler`, which allows for host functions to be registered as native function compilers in the Mech program, enabling them to be called directly from Mech code. The `RuntimeHostNativeFunction` struct represents a compiled host function that can be executed within the Mech program.

// For example, a function to compute an affine transformation could be registered as a host function, and then called from Mech code like this:
/*
  runtime.register_mech_host_function(ClosureHostFunction::new(
    "demo/math/affine",
    |_services, _context, args| {
      host_call3(
        "demo/math/affine",
        &args,
        |x: f64, scale: f64, offset: f64| {
          (x * scale) + offset
        },
      )
    },
  ))?;
*/
// Then in Mech:
/*
  result := demo/math/affine(2.0, 3.0, 4.0);
*/


use super::*;
use super::execution::{
  ACTIVATION_EFFECT_BARRIER_NAME,
  ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
  ActivationEffectBarrierCompiler,
  ActivationEffectPayloadCaptureCompiler,
};
use mech_core::{
  GuardFunctionSafety, Ref, ValueKind,
};
use crate::{
  HostFunction, HostFunctionTransactionMode,
  HostFunctionTransactionUnsupportedError, RuntimePreparedHostCall,
};

impl MechRuntime {

  pub(super) fn register_runtime_program_host_functions(
    &mut self,
    _context: &mut RuntimeContext,
    program: &mut MechProgram,
  ) -> MResult<()> {
    program.register_native_function_compiler(
      ACTIVATION_EFFECT_BARRIER_NAME,
      Arc::new(ActivationEffectBarrierCompiler),
    );
    program.register_native_function_compiler(
      ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
      Arc::new(ActivationEffectPayloadCaptureCompiler),
    );
    for name in self.host_registry.list_functions()? {
      program.register_native_function_compiler(
        name.clone(),
        Arc::new(RuntimeHostNativeFunctionCompiler::new(
          name.clone(),
          name,
        )),
      );
    }

    Ok(())
  }

  pub fn register_mech_host_function(
    &mut self,
    function: impl HostFunction + 'static,
  ) -> MResult<()> {
    self.ensure_runtime_healthy("register_mech_host_function")?;
    self.reject_effect_reentrancy("register_mech_host_function")?;
    let name = function.name().to_string();

    self
      .host_registry
      .register_function(Arc::new(function))?;

    self.program.register_native_function_compiler(
      name.clone(),
      Arc::new(RuntimeHostNativeFunctionCompiler::new(
        name.clone(),
        name,
      )),
    );

    Ok(())
  }

  pub fn call_host(&mut self, call: HostCall) -> MResult<Value> {
    let mut context = self.runtime_context()?;
    self.call_host_with_context(&mut context, call)
  }

  fn preview_runtime_managed_host_function(
    &mut self,
    context: &mut RuntimeContext,
    function: &dyn HostFunction,
    args: Vec<Value>,
  ) -> MResult<Value> {
    let transaction_id = Self::context_transaction_id(context)?;
    let context_checkpoint = RuntimeContextCheckpoint::capture(context);
    let (store, effect_mark) = {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      (transaction.store.clone(), transaction.effects.mark())
    };

    let result = function.preview_call(self, context, args);
    let mut cleanup_failures = Vec::new();
    self.active_effect_phase = Some(ActiveRuntimeEffectPhase::Aborting);
    match self.active_transactions.get_mut(&transaction_id) {
      Some(transaction) => {
        cleanup_failures.extend(Self::describe_effect_failures(
          transaction.effects.rollback_to(effect_mark),
        ));
        transaction.store = store;
      }
      None => cleanup_failures.push(format!(
        "runtime-managed host preview lost transaction {}",
        transaction_id,
      )),
    }
    self.active_effect_phase = None;
    context_checkpoint.restore_preserving_consumption(context);
    if let Err(error) = self.validate_context_for_runtime(context) {
      cleanup_failures.push(format!(
        "runtime-managed host preview context restore failed: {:?}",
        error,
      ));
    }

    if cleanup_failures.is_empty() {
      return result;
    }

    let original_error = match result {
      Ok(_) => format!(
        "runtime-managed host function `{}` preview cleanup failed",
        function.name(),
      ),
      Err(error) => format!("{:?}", error),
    };
    Err(self.poison_program_operation(
      "preview_runtime_managed_host_function",
      Some(transaction_id),
      original_error,
      cleanup_failures,
    ))
  }

  fn preview_host_call_with_context(
    &mut self,
    context: &mut RuntimeContext,
    call: HostCall,
  ) -> MResult<Value> {
    self.ensure_runtime_healthy("preview_host_call_with_context")?;
    self.reject_effect_reentrancy("preview_host_call_with_context")?;
    self.validate_context_for_runtime(context)?;
    call.validate()?;

    let Some(function) = self.host_registry.get_function(&call.name)? else {
      return Err(MechError::new(
        HostFunctionNotFoundError {
          name: call.name,
        },
        None,
      ));
    };

    self
      .host_policy
      .validate_call(context, function.as_ref(), &call.args)?;
    let capability_request = function
      .required_capability(context)
      .unwrap_or_else(|| {
        default_host_capability_request(context, function.name())
      });
    self.check_capability_with_context(context, &capability_request)?;

    match function.transaction_mode() {
      HostFunctionTransactionMode::Pure => {
        function.preview_call(self, context, call.args)
      }
      HostFunctionTransactionMode::RuntimeManaged => {
        self.preview_runtime_managed_host_function(
          context,
          function.as_ref(),
          call.args,
        )
      }
      HostFunctionTransactionMode::Staged => {
        let RuntimePreparedHostCall { value, effect } =
          function.stage_call(self, context, call.args)?;
        if let Err(error) = self.discard_unstaged_runtime_effect(effect) {
          return Err(self.poison_program_operation(
            "preview_host_call_with_context",
            context.transaction,
            format!(
              "staged host function `{}` preview cleanup failed",
              function.name(),
            ),
            vec![format!("{:?}", error)],
          ));
        }
        Ok(value)
      }
      HostFunctionTransactionMode::ImmediateOnly => {
        Err(MechError::new(
          HostFunctionTransactionUnsupportedError {
            function: function.name().to_string(),
            mode: HostFunctionTransactionMode::ImmediateOnly,
          },
          None,
        ))
      }
    }
  }

  pub fn call_host_with_context(
    &mut self,
    context: &mut RuntimeContext,
    call: HostCall,
  ) -> MResult<Value> {
    self.validate_context_for_runtime(context)?;
    call.validate()?;

    let name = call.name.clone();

    self.emit_event_to_context(
      context,
      RuntimeEventKind::HostCallStarted {
        name: name.clone(),
      },
    )?;

    let Some(function) = self.host_registry.get_function(&call.name)? else {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::HostCallFailed {
          name: name.clone(),
          message: "host function not found".to_string(),
        },
      )?;

      return Err(MechError::new(
        HostFunctionNotFoundError {
          name,
        },
        None,
      ));
    };

    let result = (|| -> MResult<Value> {
      let transaction_mode = function.transaction_mode();
      if context.transaction.is_some()
        && transaction_mode == HostFunctionTransactionMode::ImmediateOnly
      {
        return Err(MechError::new(
          HostFunctionTransactionUnsupportedError {
            function: function.name().to_string(),
            mode: transaction_mode,
          },
          None,
        ));
      }

      self
        .host_policy
        .validate_call(context, function.as_ref(), &call.args)?;

      context.charge_items(function.estimated_cost_items(&call.args))?;
      context.charge_bytes(function.estimated_cost_bytes(&call.args))?;

      let capability_request = function
        .required_capability(context)
        .unwrap_or_else(|| {
          default_host_capability_request(context, function.name())
        });

      self.check_capability_with_context(context, &capability_request)?;

      match function.transaction_mode() {
        HostFunctionTransactionMode::Staged => {
          let RuntimePreparedHostCall { value, effect } =
            function.stage_call(self, context, call.args)?;
          if context.transaction.is_some() {
            self.stage_runtime_effect_with_context(context, effect)?;
          } else {
            let cost = effect.cost();
            context.charge_bytes(cost.bytes)?;
            context.charge_items(cost.items)?;
            self.execute_runtime_effect_immediately(effect)?;
          }
          Ok(value)
        }
        HostFunctionTransactionMode::Pure
        | HostFunctionTransactionMode::RuntimeManaged
        | HostFunctionTransactionMode::ImmediateOnly => {
          function.call(self, context, call.args)
        }
      }
    })();

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::HostCallCompleted {
            name,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::HostCallFailed {
            name,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }
}


#[derive(Clone, Copy, Debug)]
pub struct RuntimeProgramHostTarget {
  pub runtime: *mut MechRuntime,
  pub context: *mut RuntimeContext,
}

#[derive(Clone, Debug)]
pub struct RuntimeHostNativeFunctionCompiler {
  pub mech_name: String,
  pub host_name: String,
}

impl RuntimeHostNativeFunctionCompiler {
  pub fn new(
    mech_name: impl Into<String>,
    host_name: impl Into<String>,
  ) -> Self {
    Self {
      mech_name: mech_name.into(),
      host_name: host_name.into(),
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
    let value = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      let target = slot.borrow().ok_or_else(|| {
        MechError::new(
          RuntimeProgramHostNotActiveError {
            function: self.mech_name.clone(),
          },
          None,
        )
      })?;

      // Safety: this target is installed only around `program.run_string(...)`
      // in `run_string_with_context`. During that call the `MechProgram` has
      // been moved out of `self`, so calling back into the runtime does not
      // alias `self.program`.
      let value = unsafe {
        (&mut *target.runtime).preview_host_call_with_context(
          &mut *target.context,
          HostCall::new(&self.host_name, arguments.clone()),
        )
      }?;
      Ok::<Value, MechError>(value)
    })?;

    Ok(Box::new(RuntimeHostNativeFunction {
      name: self.mech_name.clone(),
      host_name: self.host_name.clone(),
      arguments: arguments.clone(),
      value: Ref::new(value),
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

  fn solve_inner(&self) -> MResult<()> {
    let next = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      let target = slot.borrow().ok_or_else(|| {
        MechError::new(
          RuntimeProgramHostNotActiveError {
            function: self.name.clone(),
          },
          None,
        )
      })?;

      // Safety: callers install the active runtime-program host target around
      // program execution/stepping. Runtime host functions intentionally do not
      // retain the original context pointer because persisted programs may be
      // solved later with a different active RuntimeContext.
      unsafe {
        (&mut *target.runtime).call_host_with_context(
          &mut *target.context,
          HostCall::new(&self.host_name, self.arguments.clone()),
        )
      }
    })?;

    self.update_output(next)
  }
}

impl MechFunctionImpl for RuntimeHostNativeFunction {
  fn solve(&self) {
    if let Err(error) = self.solve_inner() {
      eprintln!(
        "[Mech Runtime Host Error] function `{}` failed during solve; preserving previous output: {:?}",
        self.name,
        error,
      );
    }
  }

  fn solve_result(&self) -> MResult<()> {
    self.solve_inner()
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
    BasicCapability, BasicOperation, BasicResource, BasicSubject,
    ClosureHostFunction,
    PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectMetadata, RuntimeEffectSource,
    StagedClosureHostFunction,
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

  #[test]
  fn staged_host_call_returns_value_before_effect_delivery() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    grant_host_call(&mut runtime, "demo/staged");
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    runtime
      .register_mech_host_function(StagedClosureHostFunction::new(
        "demo/staged",
        move |_services, _context, _args| {
          Ok(RuntimePreparedHostCall {
            value: Value::String(Ref::new("provisional".to_string())),
            effect: PreparedRuntimeEffect::AfterCommit(Box::new(
              RecordingHostEffect {
                log: effect_log.clone(),
                entry: "delivered".to_string(),
              },
            )),
          })
        },
      ))
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    let value = runtime
      .call_host_with_context(
        &mut context,
        HostCall::new("demo/staged", Vec::new()),
      )
      .unwrap();

    assert_eq!(
      value,
      Value::String(Ref::new("provisional".to_string())),
    );
    assert!(log.lock().unwrap().is_empty());

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(
      log.lock().unwrap().as_slice(),
      &["delivered".to_string()],
    );
  }

  #[test]
  fn immediate_only_host_is_rejected_before_transactional_callback() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    grant_host_call(&mut runtime, "demo/immediate");
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    runtime
      .register_mech_host_function(ClosureHostFunction::new(
        "demo/immediate",
        move |_services, _context, _args| {
          callback_calls.fetch_add(1, Ordering::SeqCst);
          Ok(Value::Empty)
        },
      ))
      .unwrap();

    runtime
      .call_host(HostCall::new("demo/immediate", Vec::new()))
      .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let error = runtime
      .call_host_with_context(
        &mut context,
        HostCall::new("demo/immediate", Vec::new()),
      )
      .unwrap_err();

    assert_eq!(
      error.kind_name(),
      "HostFunctionTransactionUnsupported",
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    runtime
      .abort_runtime_transaction(&mut context, "discard rejection test")
      .unwrap();

    let implicit_error = runtime
      .run_string("result := demo/immediate()")
      .unwrap_err();
    assert_eq!(
      implicit_error.kind_name(),
      "HostFunctionTransactionUnsupported",
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
  }

  #[test]
  fn pure_host_runs_inside_implicit_and_explicit_transactions() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    grant_host_call(&mut runtime, "demo/pure");
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    runtime
      .register_mech_host_function(ClosureHostFunction::new_pure(
        "demo/pure",
        move |_services, _context, _args| {
          callback_calls.fetch_add(1, Ordering::SeqCst);
          Ok(Value::F64(Ref::new(42.0)))
        },
      ))
      .unwrap();

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

    assert_eq!(calls.load(Ordering::SeqCst), 4);
  }

  #[test]
  fn failed_later_operation_discards_only_its_staged_host_effect() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    grant_host_call(&mut runtime, "demo/staged");
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    runtime
      .register_mech_host_function(StagedClosureHostFunction::new(
        "demo/staged",
        move |_services, _context, _args| {
          Ok(RuntimePreparedHostCall {
            value: Value::String(Ref::new("provisional".to_string())),
            effect: PreparedRuntimeEffect::AfterCommit(Box::new(
              RecordingHostEffect {
                log: effect_log.clone(),
                entry: "delivered".to_string(),
              },
            )),
          })
        },
      ))
      .unwrap();
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
  fn runtime_managed_preview_does_not_duplicate_staged_mutation() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    grant_host_call(&mut runtime, "demo/runtime-managed");
    let observed_ids = Arc::new(Mutex::new(Vec::new()));
    let callback_ids = observed_ids.clone();
    runtime
      .register_mech_host_function(
        ClosureHostFunction::new_runtime_managed(
          "demo/runtime-managed",
          move |services, context, _args| {
            let id = services.next_object_id();
            callback_ids.lock().unwrap().push(id);
            services.put_object_with_context(
              context,
              ObjectRecord::text(id, "preview-test", "value"),
            )?;
            Ok(Value::String(Ref::new(id.to_string())))
          },
        ),
      )
      .unwrap();

    runtime
      .run_string("result := demo/runtime-managed()")
      .unwrap();

    let ids = observed_ids.lock().unwrap().clone();
    assert_eq!(ids.len(), 2);
    assert!(runtime.store().get_object(ids[0]).unwrap().is_none());
    assert!(runtime.store().get_object(ids[1]).unwrap().is_some());
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
