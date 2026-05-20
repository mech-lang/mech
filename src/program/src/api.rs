use crate::*;
use mech_core::{hash_str, MResult, MechSourceCode, ParsedProgram, Value};
use mech_interpreter::Interpreter;
use mech_syntax::parser;

#[derive(Debug, Clone, Default)]
pub struct ProgramDiagnostics {
  pub parse_time_ns: Option<u128>,
  pub execution_time_ns: Option<u128>,
}

#[derive(Debug)]
pub struct ProgramResult {
  pub value: Value,
  pub diagnostics: ProgramDiagnostics,
}

#[derive(Debug, Clone)]
pub struct ProgramEngineConfig {
  pub name: String,
  pub rounds_per_step: usize,
  pub trace_enabled: bool,
}

impl Default for ProgramEngineConfig {
  fn default() -> Self {
    Self { name: "program".into(), rounds_per_step: 10_000, trace_enabled: false }
  }
}

pub struct ProgramEngine {
  interpreter: Interpreter,
  fs: MechFileSystem,
}

impl ProgramEngine {
  pub fn new(config: ProgramEngineConfig) -> Self {
    let id = hash_str(&format!("program/{}", config.name));
    let mut interpreter = Interpreter::new(id, config.rounds_per_step);
    interpreter.set_trace_enabled(config.trace_enabled);
    Self { interpreter, fs: MechFileSystem::new() }
  }

  pub fn run_string(&mut self, source: &str) -> MResult<ProgramResult> {
    let parse_start = std::time::Instant::now();
    let tree = parser::parse(source.trim())?;
    let parse_time_ns = parse_start.elapsed().as_nanos();

    let run_start = std::time::Instant::now();
    let value = self.interpreter.interpret(&tree)?;
    let execution_time_ns = run_start.elapsed().as_nanos();

    Ok(ProgramResult { value, diagnostics: ProgramDiagnostics { parse_time_ns: Some(parse_time_ns), execution_time_ns: Some(execution_time_ns) } })
  }

  pub fn run_bytecode_program(&mut self, program: &ParsedProgram) -> MResult<ProgramResult> {
    let run_start = std::time::Instant::now();
    let value = self.interpreter.run_program(program)?;
    let execution_time_ns = run_start.elapsed().as_nanos();
    Ok(ProgramResult { value, diagnostics: ProgramDiagnostics { parse_time_ns: None, execution_time_ns: Some(execution_time_ns) } })
  }

  pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<ProgramResult> {
    match source {
      MechSourceCode::String(s) => self.run_string(s),
      MechSourceCode::ByteCode(bc_program) => self.run_bytecode_program(&ParsedProgram::from_bytes(bc_program)?),
      MechSourceCode::Program(code_vec) => {
        let mut value = Value::Empty;
        let run_start = std::time::Instant::now();
        for c in code_vec {
          if let MechSourceCode::Tree(tree) = c {
            value = self.interpreter.interpret(tree)?;
          }
        }
        Ok(ProgramResult { value, diagnostics: ProgramDiagnostics { parse_time_ns: None, execution_time_ns: Some(run_start.elapsed().as_nanos()) } })
      }
      x => todo!("Todo: support source code type: {:?}", x),
    }
  }

  pub fn watch_source(&mut self, path: &str) -> MResult<()> {
    self.fs.watch_source(path)?;
    Ok(())
  }

  pub fn run_paths(&mut self, paths: &[String]) -> MResult<ProgramResult> {
    for path in paths {
      self.fs.watch_source(path)?;
    }
    self.run()
  }

  pub fn run(&mut self) -> MResult<ProgramResult> {
    let sources = self.fs.sources();
    let sources = sources.read().unwrap();
    let mut result = ProgramResult { value: Value::Empty, diagnostics: ProgramDiagnostics::default() };
    for (_, source) in sources.sources_iter() {
      result = self.run_source(source)?;
    }
    Ok(result)
  }

  pub fn interpreter(&self) -> &Interpreter { &self.interpreter }
  pub fn interpreter_mut(&mut self) -> &mut Interpreter { &mut self.interpreter }
  pub fn into_interpreter(self) -> Interpreter { self.interpreter }
  pub fn set_trace_enabled(&mut self, enabled: bool) { self.interpreter.set_trace_enabled(enabled); }
}
