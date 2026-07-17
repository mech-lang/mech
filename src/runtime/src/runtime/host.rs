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
use mech_core::Ref;

impl MechRuntime {

  pub(super) fn register_runtime_program_host_functions(
    &mut self,
    _context: &mut RuntimeContext,
    program: &mut MechProgram,
  ) -> MResult<()> {
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

      function.call(self, context, call.args)
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
        (&mut *target.runtime).call_host_with_context(
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

impl MechFunctionImpl for RuntimeHostNativeFunction {
  fn solve(&self) {
    let result = ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
      let Some(target) = *slot.borrow() else {
        return Err(MechError::new(
          RuntimeProgramHostNotActiveError {
            function: self.name.clone(),
          },
          None,
        ));
      };

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
    });

    match result {
      Ok(value) => {
        let mut current = self.value.borrow_mut();
        match (&mut *current, value) {
          #[cfg(feature = "f64")]
          (Value::F64(current), Value::F64(next)) => {
            *current.borrow_mut() = *next.borrow();
          }
          (_, next) => {
            *current = next;
          }
        }
      },
      Err(error) => {
        eprintln!(
          "[Mech Runtime Host Error] function `{}` failed during solve; preserving previous output: {:?}",
          self.name,
          error,
        );
      }
    }
  }

  fn out(&self) -> Value {
    self.value.borrow().clone()
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
