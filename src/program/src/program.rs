
use crate::*;
use mech_core::{hash_str, MResult, MechSourceCode, Value, ParsedProgram, PrettyPrint, CompileCtx};
use mech_interpreter::Interpreter;
use mech_syntax::parser;


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
  interpreter: Interpreter,
}

pub type MechProgram = Program;
pub type MechProgramConfig = ProgramConfig;
pub type MechProgramEnvironment = ProgramEnvironment;

impl Program {
  pub fn new(config: ProgramConfig) -> Self {
    let id = hash_str(&format!("program/{}", config.name));
    let mut interpreter = Interpreter::new(id);
    interpreter.set_trace_enabled(config.environment.trace_enabled);
    Self { config, interpreter }
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
    self.interpreter.interpret(&tree)
  }

  pub fn run_bytecode_program(&mut self, program: &ParsedProgram) -> MResult<Value> {
    self.interpreter.run_program(program)
  }

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

  pub fn interpreter(&self) -> &Interpreter {
    &self.interpreter
  }

  pub fn interpreter_mut(&mut self) -> &mut Interpreter {
    &mut self.interpreter
  }

  pub fn into_interpreter(self) -> Interpreter {
    self.interpreter
  }
}

pub fn configure_mech_program(program: &mut Program, tree_flag: bool, debug_flag: bool, time_flag: bool, trace_flag: bool) {
  program.set_environment(ProgramEnvironment {
    trace_enabled: trace_flag,
    debug_enabled: debug_flag,
    time_enabled: time_flag,
    print_tree: tree_flag,
    rounds_per_step: program.environment().rounds_per_step,
  });
}

pub fn run_mech_program_paths(program: &mut Program, paths: &[String]) -> MResult<Value> {
  let mut mechfs = MechFileSystem::new();
  for path in paths {
    mechfs.watch_source(path)?;
  }
  let sources = mechfs.sources();
  let sources = sources.read().unwrap();
  for (_, source) in sources.sources_iter() {
    return program.run_source(source);
  }
  Ok(Value::Empty)
}
