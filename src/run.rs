use crate::*;
use mech_core::*;
use mech_syntax::*;
use mech_interpreter::Interpreter;
use std::time::Instant;
use std::ffi::OsStr;
use std::path::Path;

#[macro_export]
macro_rules! print_tree {
  ($tree:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $tree.pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $tree);
  };
}

#[macro_export]
macro_rules! print_symbols {
  ($intrp:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $intrp.pretty_print_symbols());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $intrp.symbols());
  };
}

#[macro_export]
macro_rules! print_plan {
  ($intrp:expr) => {
    #[cfg(feature = "pretty_print")]
    println!("{}", $intrp.plan().pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", $intrp.plan());
  };
}

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

pub use mech_program::{Program as MechProgram, ProgramConfig as MechProgramConfig, ProgramEnvironment as MechProgramEnvironment};

pub fn configure_mech_program(program: &mut MechProgram, tree_flag: bool, debug_flag: bool, time_flag: bool, trace_flag: bool) {
  program.set_environment(MechProgramEnvironment {
    trace_enabled: trace_flag,
    debug_enabled: debug_flag,
    time_enabled: time_flag,
    print_tree: tree_flag,
    rounds_per_step: program.environment().rounds_per_step,
  });
}

fn run_mech_program_code_with_fs(program: &mut MechProgram, code: &MechFileSystem) -> MResult<Value> {
  let sources = code.sources();
  let sources = sources.read().unwrap();
  for (_, source) in sources.sources_iter() {
    let now = Instant::now();
    let result = program.run_source(source);
    if program.environment().time_enabled {
      let cycle_duration = now.elapsed().as_nanos() as f64;
      println!("Cycle Time: {} ns", cycle_duration);
    }
    match result {
      Ok(value) => return Ok(value),
      Err(err) => return Err(err),
    }
  }
  Ok(Value::Empty)
}

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

pub fn run_paths_or_inline(program: &mut MechProgram, paths: &[String]) -> MResult<Value> {
  let any_look_like_paths = paths.iter().any(|p| is_intended_path(p));
  if paths.is_empty() {
    return Ok(Value::Empty);
  }
  if any_look_like_paths {
    run_mech_program_paths(program, paths)
  } else {
    program.interpreter.clear();
    let joined = paths.join(" ");
    program.run_program(joined.trim())
  }
}

fn is_intended_path(s: &str) -> bool {
  if s.trim().is_empty() { return false; }
  let path = Path::new(s);
  if s.starts_with("./") || s.starts_with(".\\") ||
    s.starts_with("../") || s.starts_with("..\\") ||
    s.starts_with('/') || s.starts_with('\\') {
    return true;
  }
  if s.len() > 2 && s.as_bytes()[1] == b':' {
    return true;
  }
  if s.contains('/') || s.contains('\\') {
    return true;
  }
  if let Some(ext) = path.extension().and_then(OsStr::to_str) {
    matches!(ext, "mec" | "🤖" | "mecb" | "mdoc" | "mpkg" | "m" | "csv" | "tsv" | "txt" | "md" | "json" | "toml" | "yaml" | "html" | "htm" | "css" | "js" | "wasm" | "png" | "jpg" | "jpeg" | "gif" | "svg" | "bmp" | "ico")
  } else {
    false
  }
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
}
