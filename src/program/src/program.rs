use mech_core::{hash_str, Core, MResult, Value};
use mech_interpreter::Interpreter;
use mech_syntax::{compiler::Compiler, parser};

#[derive(Debug, Clone)]
pub struct ProgramConfig {
  pub name: String,
  pub rounds_per_step: usize,
}

impl Default for ProgramConfig {
  fn default() -> Self {
    Self { name: "program".into(), rounds_per_step: 1 }
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
    Self { config, core: Core::new(), interpreter: Interpreter::new(id) }
  }

  pub fn compile_program(&mut self, source: &str) -> MResult<()> {
    let mut compiler = Compiler::new();
    let sections = compiler.compile_str(source)?;
    self.core.load_sections(sections)?;
    Ok(())
  }

  pub fn run_program(&mut self, source: &str) -> MResult<Value> {
    let tree = parser::parse(source.trim())?;
    self.interpreter.interpret(&tree)
  }
}
