pub(crate) enum CliOutcome {
    Success,
    Exit(i32),
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
    pub trace: bool,
    pub rounds_per_step: Option<usize>,
}
