//! Filesystem paths shared by already materialized Cargo project families.
use super::{
    MResult, Path, PathBuf, ensure_directory_beneath, materialization_boundary,
    require_regular_file_if_present, write_generated_file,
};

/// A borrowed path view, not a build plan, application kind or input proof.
/// Construction performs no IO and authenticates no source or generated file.
/// The common lock/build helpers retain their own regular-file and isolation
/// checks; each generator retains its own source and Object pin authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CargoProjectPaths<'a> {
    root: &'a Path,
}

impl<'a> CargoProjectPaths<'a> {
    pub const fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub const fn root(&self) -> &'a Path {
        self.root
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("Cargo.toml")
    }

    pub fn lockfile_path(&self) -> PathBuf {
        self.root.join("Cargo.lock")
    }

    // The SAME generated-project descendant checks and atomic writer as the
    // original GeneratedNativeProject method. This does not materialize or
    // reinterpret either family's source, metadata, bytecode or Object files.
    pub(crate) fn materialize_lockfile_seed(&self, seed: &[u8]) -> MResult<()> {
        let boundary = materialization_boundary(self.root)?;
        ensure_directory_beneath(&boundary, self.root, "generated project root")?;
        require_regular_file_if_present(&self.lockfile_path(), "generated Cargo lockfile")?;
        write_generated_file(&self.lockfile_path(), seed)
    }
}

#[cfg(test)]
#[path = "source_cargo_paths_tests.rs"]
mod tests;
