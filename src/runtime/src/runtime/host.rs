// ---------------------------------------------------------------------------
// Host Calls
// ---------------------------------------------------------------------------

use super::*;

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
    context.validate()?;
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


#[derive(Clone, Copy)]
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
      unsafe {
        (&mut *target.runtime).call_host_with_context(
          &mut *target.context,
          HostCall::new(&self.host_name, arguments.clone()),
        )
      }
    })?;

    Ok(Box::new(RuntimeHostNativeFunction {
      name: self.mech_name.clone(),
      value,
    }))
  }
}

#[derive(Clone, Debug)]
pub struct RuntimeHostNativeFunction {
  pub name: String,
  pub value: Value,
}

impl MechFunctionImpl for RuntimeHostNativeFunction {
  fn solve(&self) {
    // The runtime host call already ran during native function compilation.
  }

  fn out(&self) -> Value {
    self.value.clone()
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