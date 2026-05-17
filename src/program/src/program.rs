use mech_core::{hash_str, Core, MResult, Value};
use mech_interpreter::Interpreter;
use mech_syntax::{compiler::Compiler, parser};

#[derive(Debug, Clone)]
pub struct ProgramEnvironment {
  pub trace_enabled: bool,
  pub debug_enabled: bool,
  pub time_enabled: bool,
  pub print_tree: bool,
  pub rounds_per_step: usize,
}

impl Default for ProgramEnvironment {
  fn default() -> Self {
    Self {
      trace_enabled: false,
      debug_enabled: false,
      time_enabled: false,
      print_tree: false,
      rounds_per_step: 1,
    }
  }
}

#[derive(Debug, Clone)]
pub struct ProgramConfig {
  pub name: String,
  pub environment: ProgramEnvironment,
}

impl Default for ProgramConfig {
  fn default() -> Self {
    Self { name: "program".into(), environment: ProgramEnvironment::default() }
  }
}

pub struct Program {
  pub config: ProgramConfig,
  pub core: Core,
  pub interpreter: Interpreter,
}

impl Program {
  pub fn new(config: ProgramConfig) -> Self {
    let id = hash_str(&format!("program/{}", config.name));
    let mut interpreter = Interpreter::new(id);
    interpreter.set_trace_enabled(config.environment.trace_enabled);
    Self { config, core: Core::new(), interpreter }
  }

  pub fn compile_program(&mut self, source: &str) -> MResult<()> {
    let mut compiler = Compiler::new();
    let sections = compiler.compile_str(source)?;
    self.core.load_sections(sections)?;
    Ok(())
  }

  pub fn run_program(&mut self, source: &str) -> MResult<Value> {
    self.interpreter.set_trace_enabled(self.config.environment.trace_enabled);
    let tree = parser::parse(source.trim())?;
    self.interpreter.interpret(&tree)
  }

  pub fn set_environment(&mut self, environment: ProgramEnvironment) {
    self.config.environment = environment;
    self.interpreter.set_trace_enabled(self.config.environment.trace_enabled);
  }

  pub fn environment(&self) -> &ProgramEnvironment {
    &self.config.environment
  }
}
