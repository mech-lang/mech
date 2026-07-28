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
    |_context, _args| Ok(value_f64(0.0)),
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


use super::extension::invoke_extension;
use super::execution::{
  ACTIVATION_EFFECT_BARRIER_NAME,
  ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
  ActivationEffectBarrierCompiler,
  ActivationEffectPayloadCaptureCompiler,
};
use super::{
  MechRuntime,
  RuntimeHostFunctionNotBytecodeCompilableError,
};
use mech_core::{
  CompileCtx,
  GuardFunctionSafety,
  MResult,
  MechError,
  MechErrorKind,
  MechExecutionServices,
  MechFunctionCompiler,
  MechFunctionImpl,
  NativeFunctionCompiler,
  Ref,
  Register,
  Value,
  ValueKind,
};
use mech_program::MechProgram;
use crate::{
  HostCall,
  HostFunctionNotFoundError,
  RegisteredHostFunction,
  RuntimeCallContext,
  RuntimeContext,
  RuntimeValueSnapshot,
};
use std::sync::Arc;

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
#[path = "host/tests/transaction/mod.rs"]
mod transaction_tests;

#[cfg(test)]
#[path = "host/tests/checkpoint.rs"]
mod checkpoint_tests;
