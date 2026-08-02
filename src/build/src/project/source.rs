use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use mech_core::MResult;

use crate::error::{NativeBuildErrorKind, native_build_error};

/// Deterministically ordered Rust sources for a generated native project.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeneratedSourceSet {
    files: BTreeMap<String, String>,
}

impl GeneratedSourceSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        relative_path: impl AsRef<Path>,
        source: impl Into<String>,
    ) -> MResult<()> {
        let relative_path = validate_source_path(relative_path.as_ref())?;
        if self
            .files
            .insert(relative_path.clone(), source.into())
            .is_some()
        {
            return project_invalid(format!("duplicate generated source `{relative_path}`"));
        }
        Ok(())
    }

    pub fn get(&self, relative_path: &str) -> Option<&str> {
        self.files.get(relative_path).map(String::as_str)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&str, &str)> + ExactSizeIterator {
        self.files
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_str()))
    }

    pub fn into_files(self) -> BTreeMap<String, String> {
        self.files
    }
}

/// In-memory description plus destination of one deterministic generated
/// native Cargo project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedNativeProject {
    pub root: PathBuf,
    pub cargo_manifest: String,
    pub build_plan_json: String,
    pub bytecode: Vec<u8>,
    pub sources: BTreeMap<String, String>,
}

impl GeneratedNativeProject {
    pub fn new(
        root: impl Into<PathBuf>,
        cargo_manifest: impl Into<String>,
        build_plan_json: impl Into<String>,
        bytecode: Vec<u8>,
        sources: GeneratedSourceSet,
    ) -> Self {
        Self {
            root: root.into(),
            cargo_manifest: cargo_manifest.into(),
            build_plan_json: build_plan_json.into(),
            bytecode,
            sources: sources.into_files(),
        }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("Cargo.toml")
    }

    pub fn lockfile_path(&self) -> PathBuf {
        self.root.join("Cargo.lock")
    }

    pub fn build_plan_path(&self) -> PathBuf {
        self.root.join("build-plan.json")
    }

    pub fn bytecode_path(&self) -> PathBuf {
        self.root.join("program.mecb")
    }
}

fn validate_source_path(path: &Path) -> MResult<String> {
    if path.extension().is_none_or(|extension| extension != "rs") {
        return project_invalid(format!(
            "generated source `{}` must have a `.rs` extension",
            path.display()
        ));
    }
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => {
                let Some(component) = component.to_str() else {
                    return project_invalid("generated source paths must be UTF-8");
                };
                components.push(component);
            }
            _ => {
                return project_invalid(format!(
                    "generated source path `{}` must be relative",
                    path.display()
                ));
            }
        }
    }
    if components.first().copied() != Some("src") || components.len() < 2 {
        return project_invalid("generated Rust sources must live below `src/`");
    }
    Ok(components.join("/"))
}

fn project_invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(native_build_error(
        NativeBuildErrorKind::NativeProjectInvalid {
            reason: reason.into(),
        },
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_sources_are_safe_sorted_and_unique() {
        let mut sources = GeneratedSourceSet::new();
        sources.insert("src/runtime.rs", "runtime").unwrap();
        sources.insert("src/catalog.rs", "catalog").unwrap();
        sources.insert("src/main.rs", "main").unwrap();
        assert_eq!(
            sources.iter().map(|(path, _)| path).collect::<Vec<_>>(),
            ["src/catalog.rs", "src/main.rs", "src/runtime.rs"]
        );
        assert!(sources.insert("src/main.rs", "duplicate").is_err());
        assert!(sources.insert("../main.rs", "escape").is_err());
        assert!(sources.insert("Cargo.toml", "not Rust").is_err());
    }
}
