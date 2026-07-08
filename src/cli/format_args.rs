use std::path::PathBuf;

use clap::ArgMatches;
use mech_core::*;

use crate::cli::outcome::RootFlags;
use crate::cli::resources::WebResourceDefaults;

pub(crate) struct FormatOptions {
    pub html: bool,
    pub stylesheet_paths: Vec<String>,
    pub shim_path: String,
    pub output_arg: Option<String>,
    pub output_path: PathBuf,
    pub mech_paths: Vec<String>,
    pub resources: WebResourceDefaults,
}

impl FormatOptions {
    pub(crate) fn from_matches(
        _root: RootFlags,
        _root_matches: &ArgMatches,
        matches: &ArgMatches,
        resources: WebResourceDefaults,
    ) -> MResult<Self> {
        let output_arg = matches.get_one::<String>("output_path").cloned();
        Ok(Self {
            html: matches.get_flag("html"),
            stylesheet_paths: matches
                .get_many::<String>("stylesheet")
                .map_or(vec![], |paths| paths.map(|path| path.to_string()).collect()),
            shim_path: matches
                .get_one::<String>("shim")
                .cloned()
                .unwrap_or("".to_string()),
            output_path: PathBuf::from(output_arg.clone().unwrap_or(".".to_string())),
            output_arg,
            mech_paths: matches
                .get_many::<String>("mech_format_file_paths")
                .map_or(vec![], |files| files.map(|file| file.to_string()).collect()),
            resources,
        })
    }
}
