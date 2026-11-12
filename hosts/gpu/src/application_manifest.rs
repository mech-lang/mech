//! Render the source workspace's local dependency overrides for exported apps.
use std::{
    fmt::{self, Write},
    fs,
    path::Path,
};

#[derive(Debug)]
pub(crate) enum WorkspaceResolutionError {
    Invalid(&'static str),
    Io(std::io::Error),
    Encoding(String),
}
impl fmt::Display for WorkspaceResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Io(error) => fmt::Display::fmt(error, f),
            Self::Encoding(message) => f.write_str(message),
        }
    }
}
impl From<std::io::Error> for WorkspaceResolutionError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

fn write_local_patch(
    result: &mut String,
    workspace: &Path,
    package: &str,
    relative: &str,
) -> Result<(), WorkspaceResolutionError> {
    let path = workspace.join(relative).canonicalize()?;
    if !path.starts_with(workspace) || !path.join("Cargo.toml").is_file() {
        return Err(WorkspaceResolutionError::Invalid(
            "workspace registry override is outside the pinned workspace",
        ));
    }
    let path = path.to_str().ok_or(WorkspaceResolutionError::Invalid(
        "workspace registry override path is not Unicode",
    ))?;
    let json_string =
        |value: &str| serde_json::to_string(value).expect("string serialization cannot fail");
    writeln!(
        result,
        "{} = {{ path = {} }}",
        json_string(package),
        json_string(path)
    )
    .expect("String formatting cannot fail");
    Ok(())
}

/// Cargo ignores a dependency workspace's patch table. Retain the actual
/// workspace's local resolution in the generated standalone workspace too.
/// Legacy launchers also supply local fallbacks for their registry dependencies.
pub(crate) fn workspace_resolution_patches(
    workspace: &Path,
    local_fallbacks: &[(&str, &str)],
) -> Result<String, WorkspaceResolutionError> {
    let source = fs::read_to_string(workspace.join("Cargo.toml"))?;
    let manifest = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| WorkspaceResolutionError::Encoding(error.to_string()))?;
    let patches = manifest
        .get("patch")
        .and_then(|patch| patch.get("crates-io"))
        .map(|patches| {
            patches
                .as_table_like()
                .ok_or(WorkspaceResolutionError::Invalid(
                    "workspace registry overrides must be a table",
                ))
        })
        .transpose()?;
    if patches.is_none() && local_fallbacks.is_empty() {
        return Ok(String::new());
    }
    let mut result = String::from(
        "
[patch.crates-io]
",
    );
    if let Some(patches) = patches {
        for (package, specification) in patches.iter() {
            let table = specification
                .as_table_like()
                .ok_or(WorkspaceResolutionError::Invalid(
                    "workspace registry override must be a local path",
                ))?;
            if table.len() != 1 {
                return Err(WorkspaceResolutionError::Invalid(
                    "workspace registry override must contain only a local path",
                ));
            }
            let relative = table.get("path").and_then(toml_edit::Item::as_str).ok_or(
                WorkspaceResolutionError::Invalid("workspace registry override has no local path"),
            )?;
            write_local_patch(&mut result, workspace, package, relative)?;
        }
    }
    for &(package, relative) in local_fallbacks {
        if patches.and_then(|patches| patches.get(package)).is_none() {
            write_local_patch(&mut result, workspace, package, relative)?;
        }
    }
    Ok(result)
}
