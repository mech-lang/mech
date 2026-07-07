use mech_core::*;

pub(crate) enum CliOutcome {
    Success,
    Exit(i32),
    #[cfg(feature = "repl")]
    EnterRepl(crate::cli::commands::repl::ReplStartup),
}

impl CliOutcome {
    pub(crate) fn success() -> Self {
        CliOutcome::Success
    }

    pub(crate) fn exit(code: i32) -> Self {
        CliOutcome::Exit(code)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RootFlags {
    pub debug: bool,
    pub tree: bool,
    pub trace: bool,
    pub time: bool,
    pub repl: bool,
    pub rounds_per_step: Option<usize>,
}
