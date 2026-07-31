use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::outcome::{CliOutcome, RootFlags};
use crate::generate_uuid;
use clap::{Arg, ArgAction, ArgMatches, Command};
use colored::*;
use mech_core::*;
use mech_program::*;

pub(crate) fn command() -> Command {
    Command::new("build")
        .about("Build Mech program into a binary.")
        .arg(
            Arg::new("mech_build_file_paths")
                .help("Source .mec and .mecb files")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Print debug info")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output_path")
                .short('o')
                .long("out")
                .help("Destination folder.")
                .required(false),
        )
}

fn is_bytecode_source_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("mecb"))
        .unwrap_or(false)
}

pub(crate) fn validate_build_bytecode_inputs(paths: &[String]) -> MResult<usize> {
    let bytecode_count = paths
        .iter()
        .filter(|path| is_bytecode_source_path(path))
        .count();
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

pub(crate) struct BuildOptions {
    pub paths: Vec<String>,
    pub output_path: PathBuf,
    pub debug: bool,
    pub trace: bool,
    pub time: bool,
    pub rounds_per_step: usize,
}

impl BuildOptions {
    pub(crate) fn from_matches(
        root: RootFlags,
        _root_matches: &ArgMatches,
        matches: &ArgMatches,
    ) -> MResult<Self> {
        Ok(Self {
            paths: matches
                .get_many::<String>("mech_build_file_paths")
                .map_or(vec![], |files| files.map(|file| file.to_string()).collect()),
            output_path: PathBuf::from(
                matches
                    .get_one::<String>("output_path")
                    .cloned()
                    .unwrap_or(".".to_string()),
            ),
            debug: root.debug || matches.get_flag("debug"),
            trace: root.trace,
            time: root.time,
            rounds_per_step: root.rounds_per_step.unwrap_or(10_000),
        })
    }
}

pub(crate) fn run(options: BuildOptions) -> MResult<CliOutcome> {
    let mech_paths = options.paths;
    let output_path = options.output_path;
    let debug_flag = options.debug;
    let time_flag = options.time;
    let trace_flag = options.trace;
    let rounds_per_step = options.rounds_per_step;
    if output_path != PathBuf::from(".") {
        fs::create_dir_all(&output_path)?;
        println!(
            "{} Directory created: {}",
            "[Created]".truecolor(153, 221, 85),
            output_path.display()
        );
    }

    let bytecode_count = validate_build_bytecode_inputs(&mech_paths)?;
    let bytecode = if bytecode_count == 1 {
        match mech_runtime::read_runtime_source_file(Path::new(&mech_paths[0]))? {
            MechSourceCode::ByteCode(bytecode) => bytecode,
            _ => unreachable!("bytecode input should load as MechSourceCode::ByteCode"),
        }
    } else {
        let uuid = generate_uuid();
        let mut program = MechProgram::new(MechProgramConfig {
            name: format!("program-{}", uuid),
            environment: MechProgramEnvironment::default(),
        });
        program.configure(debug_flag, trace_flag, time_flag, rounds_per_step);
        for path in mech_paths {
            let source = mech_runtime::read_runtime_source_file(Path::new(&path))?;
            let _ = program.run_source(&source)?;
        }
        let bytecode = program.interpreter_mut().compile()?;
        if debug_flag {
            println!(
                "{} Bytecode Size: {:#?} bytes",
                "[Debug]".truecolor(246, 192, 78),
                &program.interpreter().context
            );
        }
        bytecode
    };

    let output_file = output_path.join("output.mecb");
    let mut f = std::fs::File::create(&output_file)?;
    f.write_all(&bytecode)?;
    f.flush()?;
    println!(
        "{} Mech bytecode written to: {}",
        "[Output]".truecolor(153, 221, 85),
        output_file.display()
    );
    Ok(CliOutcome::success())
}
