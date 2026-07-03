use clap::ArgMatches;
use mech_core::*;

use crate::run_mech_tests;

pub(crate) fn run(
  matches: &ArgMatches,
  tree_flag: bool,
  debug_flag: bool,
  time_flag: bool,
  trace_flag: bool,
) -> MResult<i32> {
  let mech_paths: Vec<String> = matches
    .get_many::<String>("mech_test_file_paths")
    .map_or(vec![".".to_string()], |files| files.map(|file| file.to_string()).collect());

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
