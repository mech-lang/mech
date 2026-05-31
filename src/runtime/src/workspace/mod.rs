mod errors;
mod config;
mod load;
mod snapshot;
mod refresh;
mod workspace;
mod discovery;

pub use self::errors::*;
pub use self::config::*;
use self::load::*;
pub use self::snapshot::*;
pub use self::refresh::*;
pub use self::workspace::*;
pub use self::discovery::*;

use std::{
  collections::{BTreeMap, BTreeSet, VecDeque},
  hash::{DefaultHasher, Hash, Hasher},
  path::{Path, PathBuf},
  time::SystemTime,
};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::{MechRuntime, ModuleBuildOptions, ModuleVersionId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeWorkspaceDiagnosticSeverity {
  Error,
  Warning,
  Info,
}

fn hash_content(content: Vec<u8>) -> u64 {
  let mut hasher = DefaultHasher::new();
  content.hash(&mut hasher);
  hasher.finish()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(windows))]
  #[test]
  fn file_uri_path_converts_unix_file_uri() {
    assert_eq!(
      file_uri_path("file:///tmp/project/main.mec").unwrap(),
      PathBuf::from("/tmp/project/main.mec"),
    );
  }

  #[cfg(windows)]
  #[test]
  fn file_uri_path_converts_windows_drive_file_uri() {
    assert_eq!(
      file_uri_path("file:///C:/Users/cmont/project/main.mec").unwrap(),
      PathBuf::from(r"C:\Users\cmont\project\main.mec"),
    );
  }

  #[cfg(windows)]
  #[test]
  fn file_uri_path_converts_windows_extended_file_uri() {
    assert_eq!(
      file_uri_path("file:////?/C:/Users/cmont/project/main.mec").unwrap(),
      PathBuf::from(r"\\?\C:\Users\cmont\project\main.mec"),
    );
  }

  #[test]
  fn file_uri_path_rejects_non_file_uri() {
    assert!(file_uri_path("http://example.com/main.mec").is_none());
  }

  #[test]
  fn workspace_target_specifier_canonicalizes_absolute_local_path() {
    let root = std::env::temp_dir().join(format!(
      "mech-runtime-workspace-absolute-target-{}",
      SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("main.mec");
    std::fs::write(&target, "result := true\n").unwrap();
    let canonical_root = root.canonicalize().unwrap();
    let canonical_target = target.canonicalize().unwrap();

    assert_eq!(
      workspace_target_specifier(
        &canonical_root,
        target.to_string_lossy().as_ref(),
      ).unwrap(),
      canonical_target.to_string_lossy(),
    );
  }

  #[test]
  fn workspace_target_specifier_passes_through_uri() {
    assert_eq!(
      workspace_target_specifier(Path::new("unused"), "memory://main.mec").unwrap(),
      "memory://main.mec",
    );
  }
}
