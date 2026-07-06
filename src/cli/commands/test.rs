use clap::{Arg, ArgAction, ArgMatches, Command};
use mech_core::*;

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

pub(crate) fn run(
    matches: &ArgMatches,
    tree_flag: bool,
    debug_flag: bool,
    time_flag: bool,
    trace_flag: bool,
) -> MResult<i32> {
    let mech_paths: Vec<String> = matches
        .get_many::<String>("mech_test_file_paths")
        .map_or(vec![".".to_string()], |files| {
            files.map(|file| file.to_string()).collect()
        });

    let output_path = matches.get_one::<String>("output_path").cloned();
    let verbose = matches.get_flag("verbose");

    run_mech_tests(
        mech_paths,
        tree_flag,
        debug_flag,
        time_flag,
        trace_flag,
        output_path,
        verbose,
    )
}
