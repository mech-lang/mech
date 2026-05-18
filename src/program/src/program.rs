use mech_core::{hash_str, Core, MResult, MechSourceCode, ParsedProgram, Value};
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

  pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<Value> {
    self.interpreter.set_trace_enabled(self.config.environment.trace_enabled);
    match source {
      MechSourceCode::String(s) => self.run_program(s),
      MechSourceCode::ByteCode(bc_program) => {
        self.interpreter.run_program(&ParsedProgram::from_bytes(bc_program)?)
      }
      MechSourceCode::Program(code_vec) => {
        for c in code_vec {
          if let MechSourceCode::Tree(tree) = c {
            return self.interpreter.interpret(tree);
          }
        }
        Ok(Value::Empty)
      }
      _ => Ok(Value::Empty),
    }
  }

  pub fn set_environment(&mut self, environment: ProgramEnvironment) {
    self.config.environment = environment;
    self.interpreter.set_trace_enabled(self.config.environment.trace_enabled);
  }

  pub fn environment(&self) -> &ProgramEnvironment {
    &self.config.environment
  }

  pub fn into_interpreter(self) -> Interpreter {
    self.interpreter
  }
}
