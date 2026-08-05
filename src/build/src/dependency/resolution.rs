#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceResolutionPatch {
    pub package: &'static str,
    pub package_relative_path: &'static str,
    pub manifest_relative_path: &'static str,
}

pub(crate) const WORKSPACE_RESOLUTION_PATCHES: &[WorkspaceResolutionPatch] =
    &[WorkspaceResolutionPatch {
        package: "mech-syntax",
        package_relative_path: "src/syntax",
        manifest_relative_path: "src/syntax/Cargo.toml",
    }];
