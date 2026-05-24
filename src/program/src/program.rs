use crate::*;

use mech_core::{
  hash_str, CompileCtx, MResult, MechError, MechErrorKind, MechSourceCode,
  ParsedProgram, Value,
};

use mech_interpreter::Interpreter;
use mech_syntax::parser;

#[derive(Debug, Clone)]
pub struct MechProgramEnvironment {
  pub trace_enabled: bool,
  pub debug_enabled: bool,
  pub profile_enabled: bool,
  pub rounds_per_step: usize,
}

impl Default for MechProgramEnvironment {
  fn default() -> Self {
    Self {
      trace_enabled: false,
      debug_enabled: false,
      profile_enabled: false,
      rounds_per_step: 10_000,
    }
  }
}

pub trait ProgramHostBridge {
  fn call_host(
    &mut self,
    name: &str,
    args: Vec<Value>,
  ) -> MResult<Value>;
}

#[derive(Debug, Clone)]
pub struct ProgramHostCall {
  pub name: String,
  pub args: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct MechProgramConfig {
  pub name: String,
  pub environment: MechProgramEnvironment,
}

impl Default for MechProgramConfig {
  fn default() -> Self {
    Self {
      name: "program".into(),
      environment: MechProgramEnvironment::default(),
    }
  }
}

pub struct MechProgram {
  pub config: MechProgramConfig,
  interpreter: Interpreter,
}

impl MechProgram {
  pub fn new(config: MechProgramConfig) -> Self {
    let id = hash_str(&format!("program/{}", config.name));
    let mut interpreter = Interpreter::new(id, config.environment.rounds_per_step);

    interpreter.set_trace_enabled(config.environment.trace_enabled);

    Self {
      config,
      interpreter,
    }
  }

  pub fn register_native_function_compiler(&mut self, name: impl Into<String>, compiler: Arc<dyn NativeFunctionCompiler>) {
    self
      .interpreter
      .functions()
      .borrow_mut()
      .insert_function_compiler(name, compiler);
  }

  pub fn run_string_with_host(
    &mut self,
    source: &str,
    host: &mut dyn ProgramHostBridge,
  ) -> MResult<Value> {
    if let Some(calls) = parse_program_host_calls(source)? {
      let mut value = Value::Empty;

      for call in calls {
        value = host.call_host(&call.name, call.args)?;
      }

      return Ok(value);
    }

    self.run_string(source)
  }

  pub fn run_source_with_host(
    &mut self,
    source: &MechSourceCode,
    host: &mut dyn ProgramHostBridge,
  ) -> MResult<Value> {
    match source {
      MechSourceCode::String(source) => {
        self.run_string_with_host(source, host)
      }
      other => self.run_source(other),
    }
  }

  pub fn from_environment(
    name: impl Into<String>,
    environment: MechProgramEnvironment,
  ) -> Self {
    Self::new(MechProgramConfig {
      name: name.into(),
      environment,
    })
  }

  pub fn environment(&self) -> &MechProgramEnvironment {
    &self.config.environment
  }

  pub fn set_environment(&mut self, environment: MechProgramEnvironment) {
    self.config.environment = environment;
    self.apply_environment();
  }

  pub fn configure(
    &mut self,
    debug_enabled: bool,
    trace_enabled: bool,
    profile_enabled: bool,
    rounds_per_step: usize,
  ) {
    self.set_environment(MechProgramEnvironment {
      trace_enabled,
      debug_enabled,
      profile_enabled,
      rounds_per_step,
    });
  }

  fn apply_environment(&mut self) {
    self
      .interpreter
      .set_trace_enabled(self.config.environment.trace_enabled);
  }

  pub fn interpreter(&self) -> &Interpreter {
    &self.interpreter
  }

  pub fn interpreter_mut(&mut self) -> &mut Interpreter {
    &mut self.interpreter
  }

  pub fn into_interpreter(self) -> Interpreter {
    self.interpreter
  }

  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let tree = parser::parse(source.trim())?;
    self.run_tree(&tree)
  }

  pub fn run_tree(&mut self, tree: &mech_core::Program) -> MResult<Value> {
    self.interpreter.interpret(tree)
  }

  pub fn run_bytecode(&mut self, bytecode: &[u8]) -> MResult<Value> {
    let parsed = ParsedProgram::from_bytes(&bytecode.to_vec())?;
    self.run_bytecode_program(&parsed)
  }

  pub fn run_bytecode_program(&mut self, program: &ParsedProgram) -> MResult<Value> {
    self.interpreter.run_program(program)
  }

  pub fn run_program(&mut self, source: &str) -> MResult<Value> {
    self.run_profiled_string(source)
  }

  pub fn run_profiled_string(&mut self, source: &str) -> MResult<Value> {
    let now = std::time::Instant::now();
    let result = self.run_string(source);

    if self.config.environment.profile_enabled {
      let cycle_duration = now.elapsed().as_nanos() as f64;
      println!("Cycle Time: {} ns", cycle_duration);
    }

    result
  }

  pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<Value> {
    match source {
      MechSourceCode::String(source) => self.run_string(source),
      MechSourceCode::Tree(tree) => self.run_tree(tree),
      MechSourceCode::ByteCode(bytecode) => self.run_bytecode(bytecode),
      MechSourceCode::Program(sources) => self.run_sources(sources),
      unsupported => Err(MechError::new(
        UnsupportedProgramSourceError {
          source_kind: format!("{:?}", unsupported),
        },
        None,
      )),
    }
  }

  pub fn run_sources(&mut self, sources: &[MechSourceCode]) -> MResult<Value> {
    let mut value = Value::Empty;

    for source in sources {
      value = self.run_source(source)?;
    }

    Ok(value)
  }

  #[cfg(feature = "compiler")]
  pub fn compile_bytecode(&mut self) -> MResult<Vec<u8>> {
    let state_brrw = self.interpreter.state.borrow();
    let mut plan_brrw = state_brrw.plan.borrow_mut();

    let mut ctx = CompileCtx::new();

    for step in plan_brrw.iter() {
      step.compile(&mut ctx)?;
    }

    let bytes = ctx.compile()?;
    self.interpreter.context = Some(ctx);

    Ok(bytes)
  }
}

#[derive(Debug, Clone)]
pub struct UnsupportedProgramSourceError {
  pub source_kind: String,
}

impl MechErrorKind for UnsupportedProgramSourceError {
  fn name(&self) -> &str {
    "UnsupportedProgramSource"
  }

  fn message(&self) -> String {
    format!("Unsupported program source: {}", self.source_kind)
  }
}