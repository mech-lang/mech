use std::sync::Arc;

use mech_core::{
  BytecodeCompilerContext, CompileCtx, MResult, MechError, MechErrorKind,
  MechFunctionCompiler, MechFunctionImpl, NativeFunctionCompiler, Register, Value,
};

pub type NativeClosure =
  dyn Fn(Vec<Value>) -> MResult<Value> + Send + Sync + 'static;

#[derive(Clone)]
pub struct ClosureNativeFunctionCompiler {
  name: String,
  function: Arc<NativeClosure>,
}

impl ClosureNativeFunctionCompiler {
  pub fn new(
    name: impl Into<String>,
    function: impl Fn(Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
  ) -> Self {
    Self {
      name: name.into(),
      function: Arc::new(function),
    }
  }
}

impl NativeFunctionCompiler for ClosureNativeFunctionCompiler {
  fn compile(
    &self,
    arguments: &Vec<Value>,
  ) -> MResult<Box<dyn mech_core::MechFunction>> {
    let value = (self.function)(arguments.clone())?;

    Ok(Box::new(ClosureNativeFunction {
      name: self.name.clone(),
      value,
    }))
  }
}

#[derive(Clone, Debug)]
pub struct ClosureNativeFunction {
  name: String,
  value: Value,
}

impl MechFunctionImpl for ClosureNativeFunction {
  fn solve(&self) {
    // Pure closure functions are executed once during native function compilation.
  }

  fn out(&self) -> Value {
    self.value.clone()
  }

  fn to_string(&self) -> String {
    format!("ClosureNativeFunction::{}", self.name)
  }

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
  }
}

impl MechFunctionCompiler for ClosureNativeFunction {
  fn compile(
    &self,
    _ctx: &mut dyn BytecodeCompilerContext,
  ) -> MResult<Register> {
    Err(MechError::new(
      ClosureNativeFunctionNotBytecodeCompilableError {
        function: self.name.clone(),
      },
      None,
    ))
  }
}

#[derive(Debug, Clone)]
pub struct ClosureNativeFunctionNotBytecodeCompilableError {
  pub function: String,
}

impl MechErrorKind for ClosureNativeFunctionNotBytecodeCompilableError {
  fn name(&self) -> &str {
    "ClosureNativeFunctionNotBytecodeCompilable"
  }

  fn message(&self) -> String {
    format!(
      "Native closure function `{}` cannot be compiled to bytecode yet",
      self.function,
    )
  }
}
