use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use colored::*;
use mech_core::*;
use mech_program::*;
use crate::generate_uuid;

fn is_bytecode_source_path(path: &str) -> bool {
  Path::new(path)
    .extension()
    .and_then(|extension| extension.to_str())
    .map(|extension| extension.eq_ignore_ascii_case("mecb"))
    .unwrap_or(false)
}

fn validate_build_bytecode_inputs(paths: &[String]) -> MResult<usize> {
  let bytecode_count = paths.iter().filter(|path| is_bytecode_source_path(path)).count();
  if bytecode_count > 0 && bytecode_count != paths.len() {
    return Err(MechError::new(
      GenericError {
        msg: "Cannot mix bytecode (.mecb) inputs with source inputs in `mech build`; build bytecode inputs separately or rebuild from source.".to_string(),
      },
      None,
    ).with_compiler_loc());
  }
  if bytecode_count > 1 {
    return Err(MechError::new(
      GenericError {
        msg: "Cannot combine multiple bytecode (.mecb) inputs in one `mech build` invocation.".to_string(),
      },
      None,
    ).with_compiler_loc());
  }
  Ok(bytecode_count)
}

pub(crate) fn run(
  root_matches: &ArgMatches,
  matches: &ArgMatches,
  tree_flag: bool,
  time_flag: bool,
  trace_flag: bool,
  root_rounds_per_step: Option<usize>,
) -> MResult<()> {
  let mech_paths: Vec<String> = matches
    .get_many::<String>("mech_build_file_paths")
    .map_or(vec![], |files| files.map(|file| file.to_string()).collect());
  let output_path = PathBuf::from(matches.get_one::<String>("output_path").cloned().unwrap_or(".".to_string()));
  let debug_flag = root_matches.get_flag("debug") || matches.get_flag("debug");
  let rounds_per_step = root_rounds_per_step.unwrap_or(10_000);
  if output_path != PathBuf::from(".") {
    match fs::create_dir_all(&output_path) {
      Ok(_) => println!("{} Directory created: {}", "[Created]".truecolor(153,221,85), output_path.display()),
      Err(err) => println!("Error creating directory: {:?}", err),
    }
  }

  let bytecode_count = validate_build_bytecode_inputs(&mech_paths)?;
  let bytecode = if bytecode_count == 1 {
    match mech_runtime::read_runtime_source_file(Path::new(&mech_paths[0]))? {
      MechSourceCode::ByteCode(bytecode) => bytecode,
      _ => unreachable!("bytecode input should load as MechSourceCode::ByteCode"),
    }
  } else {
    let uuid = generate_uuid();
    let mut program = MechProgram::new(MechProgramConfig { name: format!("program-{}", uuid), environment: MechProgramEnvironment::default() });
    let _ = tree_flag;
    program.configure(debug_flag, trace_flag, time_flag, rounds_per_step);
    for path in mech_paths {
      let source = mech_runtime::read_runtime_source_file(Path::new(&path))?;
      let _ = program.run_source(&source)?;
    }
    let bytecode = program.interpreter_mut().compile()?;
    if debug_flag {
      println!("{} Bytecode Size: {:#?} bytes", "[Debug]".truecolor(246,192,78), &program.interpreter().context);
    }
    bytecode
  };

  let output_file = output_path.join("output.mecb");
  let mut f = std::fs::File::create(&output_file)?;
  f.write_all(&bytecode)?;
  f.flush()?;
  println!("{} Mech bytecode written to: {}", "[Output]".truecolor(153,221,85), output_file.display());
  Ok(())
}
