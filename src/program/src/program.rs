use mech_core::{Core, MResult};
use mech_syntax::compiler::Compiler;

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
}

impl Program {
  pub fn new(config: ProgramConfig) -> Self {
    Self { config, core: Core::new() }
  }

  pub fn compile_program(&mut self, source: &str) -> MResult<()> {
    let mut compiler = Compiler::new();
    let sections = compiler.compile_str(source)?;
    self.core.load_sections(sections)?;
    Ok(())
  }
}
