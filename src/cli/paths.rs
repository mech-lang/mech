use std::io;
use std::path::{Component, Path, PathBuf};
use mech_core::*;

pub(crate) fn source_extension(path: &Path) -> Option<String> {
  path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

pub(crate) fn extension_allowed(path: &Path, allowed_extensions: &[&str]) -> bool {
  source_extension(path)
    .map(|ext| allowed_extensions.iter().any(|allowed| *allowed == ext))
    .unwrap_or(false)
}

pub(crate) fn unsupported_source_path_error(path: &Path, allowed_extensions: &[&str]) -> MechError {
  MechError::new(
    GenericError {
      msg: format!(
        "Unsupported source extension for `{}`; expected one of: {}",
        path.display(),
        allowed_extensions.join(", "),
      ),
    },
    None,
  ).with_compiler_loc()
}

pub(crate) fn absolute_path(path: &Path) -> MResult<PathBuf> {
  Ok(if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir()?.join(path) })
}

pub(crate) fn normalized_existing_or_absolute(path: &Path) -> MResult<PathBuf> {
  let absolute = absolute_path(path)?;
  Ok(if absolute.exists() { absolute.canonicalize()? } else { absolute })
}

pub(crate) fn paths_equivalent(a: &Path, b: &Path) -> MResult<bool> {
  Ok(normalized_existing_or_absolute(a)? == normalized_existing_or_absolute(b)?)
}

pub(crate) fn validate_safe_relative_path(path: &Path) -> MResult<()> {
  if path.is_absolute() || path.components().any(|component| matches!(component, Component::ParentDir)) {
    return Err(MechError::new(GenericError { msg: format!("rejected unsafe relative path: {}", path.display()) }, None).with_compiler_loc());
  }
  Ok(())
}

pub(crate) fn relative_to_base(logical_path: &Path, base_dir: &Path, project_dir: &Path) -> MResult<PathBuf> {
  let relative = if let Ok(relative) = logical_path.strip_prefix(base_dir) {
    relative
  } else if let Ok(relative) = logical_path.strip_prefix(project_dir) {
    relative
  } else {
    return Err(MechError::new(GenericError { msg: format!("source is outside project/config root: {}", logical_path.display()) }, None).with_compiler_loc());
  };
  validate_safe_relative_path(relative)?;
  Ok(relative.to_path_buf())
}

pub(crate) fn is_directory_symlink(path: &Path) -> io::Result<bool> {
  Ok(std::fs::symlink_metadata(path)?.file_type().is_symlink() && path.canonicalize().map(|target| target.is_dir()).unwrap_or(false))
}

pub(crate) fn canonicalize_for_read(path: &Path) -> MResult<PathBuf> {
  Ok(path.canonicalize()?)
}
