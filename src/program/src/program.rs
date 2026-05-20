
use crate::*;
use crate::api::{ProgramDiagnostics, ProgramEngine, ProgramEngineConfig, ProgramResult};
use mech_core::{MResult, MechSourceCode, Value, ParsedProgram, PrettyPrint, CompileCtx};
use mech_interpreter::Interpreter;
use mech_syntax::parser;


#[derive(Debug, Clone)]
pub struct MechProgramEnvironment {
  pub trace_enabled: bool,
  pub debug_enabled: bool,
  pub time_enabled: bool,
  pub print_tree: bool,
  pub rounds_per_step: usize,
}

impl Default for MechProgramEnvironment {
  fn default() -> Self {
    Self {
      trace_enabled: false,
      debug_enabled: false,
      time_enabled: false,
      print_tree: false,
      rounds_per_step: 10_000,
    }
  }
}

#[derive(Debug, Clone)]
pub struct MechProgramConfig {
  pub name: String,
  pub environment: MechProgramEnvironment,
}

impl Default for MechProgramConfig {
  fn default() -> Self {
    Self { name: "program".into(), environment: MechProgramEnvironment::default() }
  }
}

pub struct MechProgram {
  pub config: MechProgramConfig,
  engine: ProgramEngine,
}

impl MechProgram {
  pub fn new(config: MechProgramConfig) -> Self {
    let engine = ProgramEngine::new(ProgramEngineConfig {
      name: config.name.clone(),
      rounds_per_step: config.environment.rounds_per_step,
      trace_enabled: config.environment.trace_enabled,
    });
    Self { config, engine }
  }

  #[cfg(feature = "compiler")]
  pub fn compile_bytecode(&mut self) -> MResult<Vec<u8>> {
    let mut ctx = CompileCtx::new();
    {
      let state_brrw = self.engine.interpreter().state.borrow();
      let plan_brrw = state_brrw.plan.borrow_mut();
      for step in plan_brrw.iter() {
        step.compile(&mut ctx)?;
      }
    }
    let bytes = ctx.compile()?;
    self.engine.interpreter_mut().context = Some(ctx);
    Ok(bytes)
  }


/*
  pub fn run_mech_code(
    intrp: &mut Interpreter,
    code: &MechFileSystem,
    tree_flag: bool,
    debug_flag: bool,
    time_flag: bool,
    trace_flag: bool,
  ) -> MResult<Value> {
    intrp.set_trace_enabled(trace_flag);
    let sources = code.sources();
    let sources = sources.read().unwrap();
    for (file, source) in sources.sources_iter() {
      match source {
        MechSourceCode::Program(code_vec) => {
          for c in code_vec {
            match c {
              MechSourceCode::Tree(tree) => {
                if tree_flag {
                  print_tree!(tree);
                }
                let now = Instant::now();
                let result = intrp.interpret(tree);
                let elapsed_time = now.elapsed();
                let cycle_duration = elapsed_time.as_nanos() as f64;
                if time_flag {
                  println!("Cycle Time: {} ns", cycle_duration);
                }
                if debug_flag {
                  print_symbols!(intrp);
                  print_plan!(intrp);
                  print_bytecode(code);
                }
                return result;
              }
              _ => todo!(),
            }
          }
        }
        MechSourceCode::String(s) => {
          let now = Instant::now();
          let parse_result = parser::parse(&s.trim());
          let elapsed_time = now.elapsed();
          let parse_duration = elapsed_time.as_nanos() as f64;
          match parse_result {
            Ok(tree) => {
              if tree_flag {
                print_tree!(tree);
              }
              let now = Instant::now();
              let result = intrp.interpret(&tree);
              let elapsed_time = now.elapsed();
              let cycle_duration = elapsed_time.as_nanos() as f64;
              if time_flag {
                println!("Parse Time: {} ns", parse_duration);
              }
              if time_flag {
                println!("Cycle Time: {} ns", cycle_duration);
              }
              if debug_flag {
                print_symbols!(intrp);
                print_plan!(intrp);
                print_bytecode(code);
              }
              return result;
            }
            Err(err) => return Err(err),
          }
        }
        MechSourceCode::ByteCode(bc_program) => {
          let now = Instant::now();
          let result = intrp.run_program(&ParsedProgram::from_bytes(bc_program)?);
          let elapsed_time = now.elapsed();
          let cycle_duration = elapsed_time.as_nanos() as f64;
          if time_flag {
            println!("Cycle Time: {} ns", cycle_duration);
          }
          if debug_flag {
            print_symbols!(intrp);
            print_plan!(intrp);
            print_bytecode(code);
          }
          return result;
        }
        x => todo!("Unsupported source code type: {:?}", x),
      }
    }
    Ok(Value::Empty)
  }


  pub fn configure_mech_program(program: &mut MechProgram, tree_flag: bool, debug_flag: bool, time_flag: bool, trace_flag: bool) {
    program.set_environment(MechProgramEnvironment {
      trace_enabled: trace_flag,
      debug_enabled: debug_flag,
      time_enabled: time_flag,
      print_tree: tree_flag,
      rounds_per_step: program.environment().rounds_per_step,
    });
  }*/

  /*fn run(&mut self) -> MResult<Value> {
    let sources = self.fs.sources();
    let sources = sources.read().unwrap();
    for (_, source) in sources.sources_iter() {
      let now = Instant::now();
      let result = self.run_source(source);
      if self.config.environment.time_enabled {
        let cycle_duration = now.elapsed().as_nanos() as f64;
        println!("Cycle Time: {} ns", cycle_duration);
      }
      match result {
        Ok(value) => return Ok(value),
        Err(err) => return Err(err),
      }
    }
    Ok(Value::Empty)
  }*/
/*
  pub fn run_mech_program_code(program: &mut MechProgram, source_code: &MechSourceCode) -> MResult<Value> {
    let mut mechfs = MechFileSystem::new();
    mechfs.add_code(source_code)?;
    run_mech_program_code_with_fs(program, &mechfs)
  }

  pub fn run_mech_program_paths(program: &mut MechProgram, paths: &[String]) -> MResult<Value> {
    let mut mechfs = MechFileSystem::new();
    for path in paths {
      mechfs.watch_source(path)?;
    }
    run_mech_program_code_with_fs(program, &mechfs)
  }

  fn print_bytecode(fs: &MechFileSystem) {
    let sources = fs.sources();
    let sources = sources.read().unwrap();
    for (file, source) in sources.sources_iter() {
      match source {
        MechSourceCode::ByteCode(bc_program) => {
          println!("Bytecode for file: {}", file);
          let program = ParsedProgram::from_bytes(bc_program).unwrap();
          println!("{:#?}", program);
        }
        _ => {}
      }
    }
  }*/


  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let tree = parser::parse(source.trim())?;
    if self.config.environment.print_tree {
      print_tree!(tree);
    }
    self.engine.interpreter_mut().interpret(&tree)
  }

  pub fn run_bytecode_program(&mut self, program: &ParsedProgram) -> MResult<Value> {
    self.engine.run_bytecode_program(program).map(|r| r.value)
  }

  #[deprecated(note = "Use ProgramEngine::run_string and inspect diagnostics instead of CLI printing concerns.")]
  pub fn run_program(&mut self, source: &str) -> MResult<Value> {
    let now = std::time::Instant::now();
    let result = self.run_string(source);
    if self.config.environment.time_enabled {
      let cycle_duration = now.elapsed().as_nanos() as f64;
      println!("Cycle Time: {} ns", cycle_duration);
    }
    result
  }

  pub fn run_source(&mut self, source: &MechSourceCode) -> MResult<Value> {
    match source {
      MechSourceCode::String(s) => self.run_string(s),
      MechSourceCode::ByteCode(bc_program) => {
        self.engine.run_bytecode_program(&ParsedProgram::from_bytes(bc_program)?).map(|r| r.value)
      }
      MechSourceCode::Program(code_vec) => {
        let mut value = Value::Empty;
        for c in code_vec {
          if let MechSourceCode::Tree(tree) = c {
            value = self.engine.interpreter_mut().interpret(tree)?;
          }
        }
        Ok(value)
      }
      x => todo!("Todo: support source code type: {:?}", x),
    }
  }

  pub fn set_environment(&mut self, environment: MechProgramEnvironment) {
    self.config.environment = environment;
    self.engine.set_trace_enabled(self.config.environment.trace_enabled);
  }

  pub fn environment(&self) -> &MechProgramEnvironment {
    &self.config.environment
  }

  pub fn interpreter(&self) -> &Interpreter {
    self.engine.interpreter()
  }

  pub fn interpreter_mut(&mut self) -> &mut Interpreter {
    self.engine.interpreter_mut()
  }

  pub fn into_interpreter(self) -> Interpreter {
    self.engine.into_interpreter()
  }

  pub fn watch_source(&mut self, path: &str) -> MResult<()> {
    self.engine.watch_source(path)?;
    Ok(())
  }

  pub fn run_paths(&mut self, paths: &[String]) -> MResult<Value> {
    for path in paths {
      self.engine.watch_source(path)?;
    }
    self.run()
  }

  pub fn run(&mut self) -> MResult<Value> {
    self.engine.run().map(|r| r.value)
  }

  #[deprecated(note = "Use ProgramEngine and a CLI adapter for presentation concerns.")]
  pub fn configure(&mut self, tree_flag: bool, debug_flag: bool, time_flag: bool, trace_flag: bool, rounds_per_step: usize) {
    self.set_environment(MechProgramEnvironment {
      trace_enabled: trace_flag,
      debug_enabled: debug_flag,
      time_enabled: time_flag,
      print_tree: tree_flag,
      rounds_per_step: rounds_per_step,
    });
  }

}



#[derive(Debug, Clone, Default)]
pub struct MechCliOptions {
  pub print_tree: bool,
  pub print_symbols: bool,
  pub print_plan: bool,
  pub print_timing: bool,
}

pub struct MechCliAdapter {
  pub options: MechCliOptions,
}

impl MechCliAdapter {
  pub fn run_string(&self, program: &mut MechProgram, source: &str) -> MResult<ProgramResult> {
    if self.options.print_tree {
      let tree = parser::parse(source.trim())?;
      print_tree!(tree);
    }
    let result = program.engine.run_string(source)?;
    self.print_diagnostics(program, &result.diagnostics);
    Ok(result)
  }

  pub fn run_paths(&self, program: &mut MechProgram, paths: &[String]) -> MResult<ProgramResult> {
    let result = program.engine.run_paths(paths)?;
    self.print_diagnostics(program, &result.diagnostics);
    Ok(result)
  }

  fn print_diagnostics(&self, program: &MechProgram, diagnostics: &ProgramDiagnostics) {
    if self.options.print_timing {
      if let Some(parse) = diagnostics.parse_time_ns { println!("Parse Time: {} ns", parse); }
      if let Some(exec) = diagnostics.execution_time_ns { println!("Cycle Time: {} ns", exec); }
    }
    if self.options.print_symbols { print_symbols!(program.interpreter()); }
    if self.options.print_plan { print_plan!(program.interpreter()); }
  }
}
