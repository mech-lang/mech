use std::path::{Component, Path, PathBuf};

use mech_core::MResult;

use crate::error::{NativeBuildErrorKind, native_build_error};

/// A trusted package location within a Mech source workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePackage {
    pub package: String,
    pub crate_name: String,
    pub relative_path: PathBuf,
    pub embedded_resources: Vec<PathBuf>,
}

impl WorkspacePackage {
    pub fn new(
        package: impl Into<String>,
        crate_name: impl Into<String>,
        relative_path: impl Into<PathBuf>,
    ) -> MResult<Self> {
        let package = package.into();
        let crate_name = crate_name.into();
        validate_package_name(&package)?;
        validate_crate_name(&crate_name)?;
        let relative_path = validate_workspace_relative_path(relative_path.into())?;

        Ok(Self {
            package,
            crate_name,
            relative_path,
            embedded_resources: Vec::new(),
        })
    }

    /// Add a compile-time resource path relative to this package's root.
    pub fn with_embedded_resource(mut self, path: impl Into<PathBuf>) -> MResult<Self> {
        self.add_embedded_resource(path)?;
        Ok(self)
    }

    /// Add a compile-time resource path relative to this package's root.
    pub fn add_embedded_resource(&mut self, path: impl Into<PathBuf>) -> MResult<()> {
        let path = validate_workspace_relative_path(path.into())?;
        match self.embedded_resources.binary_search(&path) {
            Ok(_) => {}
            Err(index) => self.embedded_resources.insert(index, path),
        }
        Ok(())
    }

    pub(crate) fn manifest_relative_path(&self) -> PathBuf {
        self.relative_path.join("Cargo.toml")
    }

    pub(crate) fn source_relative_path(&self) -> PathBuf {
        self.relative_path.join("src")
    }

    pub(crate) fn build_script_relative_path(&self) -> PathBuf {
        self.relative_path.join("build.rs")
    }

    pub(crate) fn resource_relative_path(&self, resource: &Path) -> PathBuf {
        self.relative_path.join(resource)
    }
}

/// Validate and normalize a path that will be persisted relative to the
/// trusted workspace root.
pub fn validate_workspace_relative_path(path: impl AsRef<Path>) -> MResult<PathBuf> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() || path.to_str().is_none() {
        return invalid_workspace_path(path, "path must be non-empty UTF-8");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => {
                if component == ".git" || component == "target" {
                    return invalid_workspace_path(path, "path may not enter `.git` or `target`");
                }
                normalized.push(component);
            }
            Component::CurDir => {
                return invalid_workspace_path(path, "`.` components are not allowed");
            }
            Component::ParentDir => {
                return invalid_workspace_path(path, "`..` components are not allowed");
            }
            Component::RootDir | Component::Prefix(_) => {
                return invalid_workspace_path(path, "absolute paths are not allowed");
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return invalid_workspace_path(path, "path must contain a normal component");
    }
    Ok(normalized)
}

pub(crate) fn workspace_path_string(path: &Path) -> MResult<String> {
    let normalized = validate_workspace_relative_path(path)?;
    let components = normalized
        .components()
        .map(|component| match component {
            Component::Normal(component) => component.to_str().map(str::to_owned),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            native_build_error(
                NativeBuildErrorKind::NativeWorkspaceInputInvalid {
                    path: path.to_path_buf(),
                    reason: "path is not UTF-8".into(),
                },
                None,
            )
        })?;
    Ok(components.join("/"))
}

fn validate_package_name(package: &str) -> MResult<()> {
    if valid_identifier(package, true) {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeIdentifierInvalid {
                kind: "package name",
                value: package.to_string(),
            },
            None,
        ))
    }
}

fn validate_crate_name(crate_name: &str) -> MResult<()> {
    if valid_identifier(crate_name, false) {
        Ok(())
    } else {
        Err(native_build_error(
            NativeBuildErrorKind::NativeIdentifierInvalid {
                kind: "crate name",
                value: crate_name.to_string(),
            },
            None,
        ))
    }
}

fn valid_identifier(value: &str, hyphen: bool) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || (hyphen && character == '-')
        })
}

fn invalid_workspace_path<T>(path: &Path, reason: impl Into<String>) -> MResult<T> {
    Err(native_build_error(
        NativeBuildErrorKind::NativeWorkspaceInputInvalid {
            path: path.to_path_buf(),
            reason: reason.into(),
        },
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_paths_are_relative_and_portable() {
        assert_eq!(
            validate_workspace_relative_path("machines/math/src/lib.rs").unwrap(),
            PathBuf::from("machines/math/src/lib.rs")
        );
        assert_eq!(
            workspace_path_string(Path::new("machines/math/src/lib.rs")).unwrap(),
            "machines/math/src/lib.rs"
        );

        for invalid in [
            "",
            ".",
            "../math",
            "machines/../math",
            "/tmp/math",
            "target/x",
        ] {
            assert!(
                validate_workspace_relative_path(invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn package_resources_are_sorted_and_deduplicated() {
        let mut package = WorkspacePackage::new("mech-math", "mech_math", "machines/math").unwrap();
        package.add_embedded_resource("assets/z.bin").unwrap();
        package.add_embedded_resource("assets/a.bin").unwrap();
        package.add_embedded_resource("assets/z.bin").unwrap();
        assert_eq!(
            package.embedded_resources,
            [PathBuf::from("assets/a.bin"), PathBuf::from("assets/z.bin")]
        );
    }
}
