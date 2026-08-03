use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

    /// Materialize the frozen native-project input set below [`Self::root`].
    ///
    /// Every final file is written through a sibling temporary file and then
    /// renamed into place. Existing files with identical bytes are retained,
    /// making repeated generation idempotent. Symlinks and non-directory path
    /// components at the generated-project boundary are rejected rather than
    /// followed.
    pub fn materialize(&self) -> MResult<()> {
        const REQUIRED_SOURCES: [&str; 3] = ["src/catalog.rs", "src/main.rs", "src/runtime.rs"];
        let actual_sources = self.sources.keys().map(String::as_str).collect::<Vec<_>>();
        if actual_sources != REQUIRED_SOURCES {
            return project_invalid(format!(
                "generated source set must be exactly {:?}, found {:?}",
                REQUIRED_SOURCES, actual_sources
            ));
        }

        let boundary = materialization_boundary(&self.root)?;
        ensure_directory_beneath(&boundary, &self.root, "generated project root")?;
        let source_root = self.root.join("src");
        ensure_directory_beneath(&self.root, &source_root, "generated source directory")?;
        require_exact_existing_entries(
            &self.root,
            "generated project root",
            &[
                "Cargo.lock",
                "Cargo.toml",
                "build-plan.json",
                "program.mecb",
                "src",
            ],
        )?;
        require_exact_existing_entries(
            &source_root,
            "generated source directory",
            &["catalog.rs", "main.rs", "runtime.rs"],
        )?;
        require_regular_file_if_present(&self.lockfile_path(), "generated Cargo lockfile")?;

        write_generated_file(&self.manifest_path(), self.cargo_manifest.as_bytes())?;
        write_generated_file(&self.build_plan_path(), self.build_plan_json.as_bytes())?;
        write_generated_file(&self.bytecode_path(), &self.bytecode)?;
        for (relative_path, source) in &self.sources {
            let relative_path = validate_source_path(Path::new(relative_path))?;
            write_generated_file(&self.root.join(relative_path), source.as_bytes())?;
        }
        Ok(())
    }
}

static NEXT_TEMPORARY_FILE: AtomicU64 = AtomicU64::new(0);

/// Return the caller-controlled boundary above the frozen generated-project
/// suffix. Symlinks at or above this boundary are trusted as part of the
/// workspace path; every generated descendant is checked without following a
/// symlink.
fn materialization_boundary(root: &Path) -> MResult<PathBuf> {
    let projects = root.parent();
    let mech_native = projects.and_then(Path::parent);
    let target = mech_native.and_then(Path::parent);
    let workspace = target.and_then(Path::parent);
    if projects.is_some_and(|path| path.file_name().is_some_and(|name| name == "projects"))
        && mech_native
            .is_some_and(|path| path.file_name().is_some_and(|name| name == "mech-native"))
        && target.is_some_and(|path| path.file_name().is_some_and(|name| name == "target"))
    {
        return workspace
            .map(Path::to_path_buf)
            .ok_or_else(|| project_error("generated project root has no workspace boundary"));
    }

    root.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| project_error("generated project root has no parent boundary"))
}

fn ensure_directory_beneath(boundary: &Path, path: &Path, label: &str) -> MResult<()> {
    let relative = path.strip_prefix(boundary).map_err(|_| {
        project_error(format!(
            "{label} `{}` is outside materialization boundary `{}`",
            path.display(),
            boundary.display()
        ))
    })?;
    let boundary_metadata = fs::metadata(boundary).map_err(|error| {
        project_error(format!(
            "failed to inspect materialization boundary `{}`: {error}",
            boundary.display()
        ))
    })?;
    if !boundary_metadata.is_dir() {
        return project_invalid(format!(
            "materialization boundary `{}` is not a directory",
            boundary.display()
        ));
    }

    let mut current = boundary.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return project_invalid(format!(
                "{label} `{}` has an invalid path component",
                path.display()
            ));
        };
        current.push(component);
        ensure_directory_component(&current, label)?;
    }
    Ok(())
}

fn ensure_directory_component(path: &Path, label: &str) -> MResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return project_invalid(format!(
                "{label} `{}` must not be a symlink",
                path.display()
            ));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return project_invalid(format!("{label} `{}` is not a directory", path.display()));
        }
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return project_invalid(format!(
                "failed to inspect {label} `{}`: {error}",
                path.display()
            ));
        }
    }

    match fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(project_error(format!(
                "failed to create {label} `{}`: {error}",
                path.display()
            )));
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        project_error(format!(
            "failed to inspect created {label} `{}`: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return project_invalid(format!(
            "created {label} `{}` is not a real directory",
            path.display()
        ));
    }
    Ok(())
}

fn write_generated_file(path: &Path, bytes: &[u8]) -> MResult<()> {
    let parent = path.parent().ok_or_else(|| {
        project_error(format!(
            "generated file `{}` has no parent directory",
            path.display()
        ))
    })?;
    require_directory(parent, "generated file parent")?;

    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return project_invalid(format!(
                "generated file `{}` must not be a symlink",
                path.display()
            ));
        }
        Ok(metadata) if !metadata.is_file() => {
            return project_invalid(format!(
                "generated file `{}` is not a regular file",
                path.display()
            ));
        }
        Ok(_) => {
            let existing = fs::read(path).map_err(|error| {
                project_error(format!(
                    "failed to read generated file `{}`: {error}",
                    path.display()
                ))
            })?;
            if existing == bytes {
                return Ok(());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return project_invalid(format!(
                "failed to inspect generated file `{}`: {error}",
                path.display()
            ));
        }
    }

    let temporary = temporary_sibling(path)?;
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                ) =>
            {
                let metadata = fs::symlink_metadata(path)?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(error);
                }
                fs::remove_file(path)?;
                fs::rename(&temporary, path)
            }
            Err(error) => Err(error),
        }
    })();

    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return project_invalid(format!(
            "failed to write generated file `{}`: {error}",
            path.display()
        ));
    }
    Ok(())
}

fn require_directory(path: &Path, label: &str) -> MResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        project_error(format!(
            "failed to inspect {label} `{}`: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return project_invalid(format!(
            "{label} `{}` is not a real directory",
            path.display()
        ));
    }
    Ok(())
}

fn require_exact_existing_entries(path: &Path, label: &str, allowed: &[&str]) -> MResult<()> {
    let mut unexpected = Vec::new();
    let entries = fs::read_dir(path).map_err(|error| {
        project_error(format!(
            "failed to inspect {label} `{}`: {error}",
            path.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            project_error(format!(
                "failed to inspect an entry in {label} `{}`: {error}",
                path.display()
            ))
        })?;
        let name = entry.file_name().into_string().map_err(|_| {
            project_error(format!(
                "{label} `{}` contains a non-UTF-8 entry",
                path.display()
            ))
        })?;
        if !allowed.contains(&name.as_str()) {
            unexpected.push(name);
        }
    }
    unexpected.sort();
    if !unexpected.is_empty() {
        return project_invalid(format!(
            "{label} `{}` contains unexpected entries: {}",
            path.display(),
            unexpected.join(", ")
        ));
    }
    Ok(())
}

fn require_regular_file_if_present(path: &Path, label: &str) -> MResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            project_invalid(format!(
                "{label} `{}` is not a regular file",
                path.display()
            ))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(project_error(format!(
            "failed to inspect {label} `{}`: {error}",
            path.display()
        ))),
    }
}

fn temporary_sibling(path: &Path) -> MResult<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        project_error(format!(
            "generated file `{}` has no parent directory",
            path.display()
        ))
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            project_error(format!(
                "generated file name `{}` is not UTF-8",
                path.display()
            ))
        })?;
    let sequence = NEXT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".{name}.mech-tmp-{}-{sequence}",
        std::process::id()
    )))
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
    Err(project_error(reason))
}

fn project_error(reason: impl Into<String>) -> mech_core::MechError {
    native_build_error(
        NativeBuildErrorKind::NativeProjectInvalid {
            reason: reason.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use super::*;

    fn generated_project(root: &Path) -> GeneratedNativeProject {
        let mut sources = GeneratedSourceSet::new();
        sources.insert("src/main.rs", "fn main() {}\n").unwrap();
        sources.insert("src/catalog.rs", "// catalog\n").unwrap();
        sources.insert("src/runtime.rs", "// runtime\n").unwrap();
        GeneratedNativeProject::new(
            root,
            "[package]\nname = \"generated\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
            "{\"schema\":\"mech.native-build-plan.v1\"}",
            b"MECH bytecode".to_vec(),
            sources,
        )
    }

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

    #[test]
    fn materialization_writes_the_exact_frozen_input_set_idempotently() {
        let temporary = tempfile::tempdir().unwrap();
        let project = generated_project(&temporary.path().join("project"));

        project.materialize().unwrap();
        project.materialize().unwrap();

        assert_eq!(
            fs::read_to_string(project.manifest_path()).unwrap(),
            project.cargo_manifest
        );
        assert_eq!(
            fs::read_to_string(project.build_plan_path()).unwrap(),
            project.build_plan_json
        );
        assert_eq!(fs::read(project.bytecode_path()).unwrap(), project.bytecode);
        for (relative_path, source) in &project.sources {
            assert_eq!(
                fs::read_to_string(project.root.join(relative_path)).unwrap(),
                *source
            );
        }

        let entries = fs::read_dir(&project.root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            entries,
            BTreeSet::from([
                "Cargo.toml".to_owned(),
                "build-plan.json".to_owned(),
                "program.mecb".to_owned(),
                "src".to_owned(),
            ])
        );
    }

    #[test]
    fn materialization_revalidates_the_public_source_map() {
        let temporary = tempfile::tempdir().unwrap();
        let mut project = generated_project(&temporary.path().join("project"));
        project
            .sources
            .insert("src/extra.rs".to_owned(), "// extra".to_owned());
        assert_eq!(
            project.materialize().unwrap_err().kind_name(),
            "NativeProjectInvalid"
        );
        assert!(!project.root.exists());
    }

    #[test]
    fn materialization_rejects_existing_non_directories() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("project");
        fs::write(&root, "not a directory").unwrap();
        let project = generated_project(&root);
        assert_eq!(
            project.materialize().unwrap_err().kind_name(),
            "NativeProjectInvalid"
        );
    }

    #[test]
    fn materialization_rejects_stale_files_and_directories() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("project");
        fs::create_dir_all(root.join("src/stale")).unwrap();
        fs::write(root.join("build.rs"), "panic!(\"must not execute\");\n").unwrap();

        let error = generated_project(&root).materialize().unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
        assert!(!root.join("Cargo.toml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn materialization_rejects_symlink_boundaries_and_files() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let outside = temporary.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let root_link = temporary.path().join("root-link");
        symlink(&outside, &root_link).unwrap();
        assert_eq!(
            generated_project(&root_link)
                .materialize()
                .unwrap_err()
                .kind_name(),
            "NativeProjectInvalid"
        );

        let root = temporary.path().join("project");
        fs::create_dir(&root).unwrap();
        symlink(&outside, root.join("src")).unwrap();
        assert_eq!(
            generated_project(&root)
                .materialize()
                .unwrap_err()
                .kind_name(),
            "NativeProjectInvalid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn materialization_rejects_symlinks_in_existing_layout_ancestors() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("workspace");
        let outside = temporary.path().join("outside");
        let digest = "a".repeat(64);
        fs::create_dir_all(workspace.join("target")).unwrap();
        fs::create_dir_all(outside.join("projects").join(&digest)).unwrap();
        symlink(&outside, workspace.join("target/mech-native")).unwrap();

        let root = workspace.join("target/mech-native/projects").join(&digest);
        let error = generated_project(&root).materialize().unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
        assert!(
            !outside
                .join("projects")
                .join(digest)
                .join("Cargo.toml")
                .exists()
        );
    }
}
