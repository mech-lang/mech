use std::path::{Path, PathBuf};

/// Executable path reported by Cargo for the generated native binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeBuildArtifact {
    pub executable: PathBuf,
}

impl NativeBuildArtifact {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
}
