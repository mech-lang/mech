use clap::{Arg, ArgAction, ArgMatches, Command};
use mech_core::*;

use crate::cli::outcome::{CliOutcome, RootFlags};
use crate::run_mech_tests;

pub(crate) fn command() -> Command {
    Command::new("test")
        .about("Validate program invariants.")
        .arg(
            Arg::new("mech_test_file_paths")
                .help("Source .mec and .mecb files or directories")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("output_path")
                .short('o')
                .long("out")
                .help("Write test output to .json or .mec.")
                .required(false),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Print verbose pass/fail details.")
                .action(ArgAction::SetTrue)
                .required(false),
        )
}

pub(crate) struct TestOptions {
    pub paths: Vec<String>,
    pub output_path: Option<String>,
    pub verbose: bool,
    pub tree: bool,
    pub debug: bool,
    pub time: bool,
    pub trace: bool,
}

impl TestOptions {
    pub(crate) fn from_matches(root: RootFlags, matches: &ArgMatches) -> MResult<Self> {
        Ok(Self {
            paths: matches
                .get_many::<String>("mech_test_file_paths")
                .map_or(vec![".".to_string()], |files| {
                    files.map(|file| file.to_string()).collect()
                }),
            output_path: matches.get_one::<String>("output_path").cloned(),
            verbose: matches.get_flag("verbose"),
            tree: root.tree,
            debug: root.debug,
            time: root.time,
            trace: root.trace,
        })
    }
}

pub(crate) fn run(options: TestOptions) -> MResult<CliOutcome> {
    let code = run_mech_tests(
        options.paths,
        options.tree,
        options.debug,
        options.time,
        options.trace,
        options.output_path,
        options.verbose,
    )?;
    Ok(CliOutcome::exit(code))
}
