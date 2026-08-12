use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mech_core::MResult;

use super::{NativeBuildArtifact, parse_cargo_build_messages};
use crate::error::{NativeBuildErrorKind, native_build_error};
use crate::project::{
    GeneratedNativeProject, validate_project_binary_name, validate_project_target_triple,
};

pub const NATIVE_RUST_TOOLCHAIN: &str = "nightly-2026-03-03";

/// A direct, shell-free Cargo invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoInvocation {
    arguments: Vec<OsString>,
    current_dir: Option<PathBuf>,
}

impl CargoInvocation {
    pub fn resolve_lockfile(manifest_path: impl AsRef<Path>, offline: bool) -> Self {
        let mut arguments = vec![
            OsString::from(format!("+{NATIVE_RUST_TOOLCHAIN}")),
            OsString::from("metadata"),
            OsString::from("--format-version"),
            OsString::from("1"),
            OsString::from("--manifest-path"),
            manifest_path.as_ref().as_os_str().to_owned(),
        ];
        if offline {
            arguments.push(OsString::from("--offline"));
        }
        Self {
            arguments,
            current_dir: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build(
        manifest_path: impl AsRef<Path>,
        binary_name: &str,
        target: Option<&str>,
        target_dir: impl AsRef<Path>,
        release: bool,
        offline: bool,
    ) -> MResult<Self> {
        validate_project_binary_name(binary_name)?;
        if let Some(target) = target {
            validate_project_target_triple(target)?;
        }

        let mut arguments = vec![
            OsString::from(format!("+{NATIVE_RUST_TOOLCHAIN}")),
            OsString::from("build"),
            OsString::from("--manifest-path"),
            manifest_path.as_ref().as_os_str().to_owned(),
            OsString::from("--bin"),
            OsString::from(binary_name),
            OsString::from("--message-format=json-render-diagnostics"),
            OsString::from("--locked"),
            OsString::from("--target-dir"),
            target_dir.as_ref().as_os_str().to_owned(),
        ];
        if release {
            arguments.push(OsString::from("--release"));
        }
        if let Some(target) = target {
            arguments.push(OsString::from("--target"));
            arguments.push(OsString::from(target));
        }
        if offline {
            arguments.push(OsString::from("--offline"));
        }

        Ok(Self {
            arguments,
            current_dir: None,
        })
    }

    pub fn with_current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn arguments(&self) -> impl ExactSizeIterator<Item = &OsStr> {
        self.arguments.iter().map(OsString::as_os_str)
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new("cargo");
        command.args(&self.arguments);
        for (key, _) in std::env::vars_os() {
            if is_ambient_cargo_build_input(&key) {
                command.env_remove(key);
            }
        }
        if let Some(current_dir) = &self.current_dir {
            command.current_dir(current_dir);
        }
        command
    }

    fn output(&self) -> MResult<Output> {
        self.command().output().map_err(|error| {
            cargo_error(format!(
                "failed to start `{}`: {error}",
                self.display_command()
            ))
        })
    }

    fn display_command(&self) -> String {
        std::iter::once(OsStr::new("cargo"))
            .chain(self.arguments())
            .map(|argument| argument.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Cargo and rustc accept a broad ambient configuration surface. Generated
/// applications deliberately inherit only the cache/install locations needed
/// to find the pinned toolchain and already-fetched crates.
fn is_ambient_cargo_build_input(key: &OsStr) -> bool {
    let key = key.to_string_lossy();
    (key.starts_with("CARGO_") && key != "CARGO_HOME")
        || (key.starts_with("RUST") && key != "RUSTUP_HOME")
}

fn isolated_cargo_working_directory(manifest_path: &Path) -> MResult<PathBuf> {
    let manifest = fs::canonicalize(manifest_path).map_err(|error| {
        cargo_error(format!(
            "generated Cargo manifest `{}` cannot be resolved: {error}",
            manifest_path.display(),
        ))
    })?;
    manifest
        .ancestors()
        .last()
        .map(Path::to_path_buf)
        .ok_or_else(|| cargo_error("generated Cargo manifest has no filesystem root"))
}

fn reject_ambient_cargo_configuration() -> MResult<()> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")));
    let Some(cargo_home) = cargo_home else {
        return Ok(());
    };
    for name in ["config.toml", "config"] {
        let path = cargo_home.join(name);
        if path.exists() {
            return Err(cargo_error(format!(
                "ambient Cargo configuration `{}` is unsupported for deterministic native builds",
                path.display(),
            )));
        }
    }
    Ok(())
}

fn isolate_cargo_invocation(
    invocation: CargoInvocation,
    manifest_path: &Path,
) -> MResult<CargoInvocation> {
    reject_ambient_cargo_configuration()?;
    Ok(invocation.with_current_dir(isolated_cargo_working_directory(manifest_path)?))
}

/// Derive the exact project lockfile from a frozen resolution seed.
pub fn generate_project_lockfile(
    project: &GeneratedNativeProject,
    resolution_seed: &[u8],
    offline: bool,
) -> MResult<()> {
    require_regular_generated_file(&project.manifest_path(), "Cargo manifest")?;
    let frozen_registry_packages = frozen_resolution_packages(resolution_seed)?;
    project.materialize_lockfile_seed(resolution_seed)?;

    let manifest_path = project.manifest_path();
    let invocation = isolate_cargo_invocation(
        CargoInvocation::resolve_lockfile(&manifest_path, offline),
        &manifest_path,
    )?;
    let output = invocation.output()?;
    if !output.status.success() {
        return Err(cargo_error(cargo_failure_reason(
            &invocation,
            &output,
            None,
        )));
    }
    require_regular_generated_file(&project.lockfile_path(), "generated Cargo lockfile")?;
    let resolved_lockfile = fs::read(project.lockfile_path()).map_err(|error| {
        cargo_error(format!("failed to read generated Cargo lockfile: {error}"))
    })?;
    validate_frozen_registry_resolution(&frozen_registry_packages, &resolved_lockfile)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FrozenRegistryPackage {
    name: String,
    version: String,
    source: String,
    checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RegistryCompatibilityLine {
    name: String,
    source: String,
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
}

impl RegistryCompatibilityLine {
    fn new(package: &FrozenRegistryPackage) -> MResult<Self> {
        let version = package
            .version
            .parse::<cargo_metadata::semver::Version>()
            .map_err(|error| {
                cargo_error(format!(
                    "resolution seed contains invalid package version `{}`: {error}",
                    package.version,
                ))
            })?;
        Ok(Self {
            name: package.name.clone(),
            source: package.source.clone(),
            major: version.major,
            minor: (version.major == 0).then_some(version.minor),
            patch: (version.major == 0 && version.minor == 0).then_some(version.patch),
        })
    }
}

fn registry_packages(bytes: &[u8], label: &str) -> MResult<BTreeSet<FrozenRegistryPackage>> {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| cargo_error(format!("{label} is not UTF-8: {error}")))?;
    let document = source
        .parse::<toml_edit::DocumentMut>()
        .map_err(|error| cargo_error(format!("{label} is not valid Cargo lock TOML: {error}")))?;
    let packages = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables);
    let mut registry = BTreeSet::new();
    for package in packages.into_iter().flatten() {
        let Some(source) = package.get("source").and_then(toml_edit::Item::as_str) else {
            continue;
        };
        if !source.starts_with("registry+") {
            return Err(cargo_error(format!(
                "{label} contains unsupported non-registry dependency source `{source}`"
            )));
        }
        let field = |name: &str| {
            package
                .get(name)
                .and_then(toml_edit::Item::as_str)
                .map(str::to_owned)
                .ok_or_else(|| cargo_error(format!("{label} package lacks `{name}`")))
        };
        registry.insert(FrozenRegistryPackage {
            name: field("name")?,
            version: field("version")?,
            source: source.to_owned(),
            checksum: field("checksum")?,
        });
    }
    Ok(registry)
}

fn frozen_resolution_packages(seed: &[u8]) -> MResult<BTreeSet<FrozenRegistryPackage>> {
    let packages = registry_packages(seed, "resolution seed")?;
    let mut compatibility_lines = BTreeMap::new();
    for package in &packages {
        let line = RegistryCompatibilityLine::new(package)?;
        if let Some(other) = compatibility_lines.insert(line, package) {
            return Err(cargo_error(format!(
                "resolution seed permits interchangeable registry versions {} {} and {}",
                package.name, other.version, package.version,
            )));
        }
    }
    Ok(packages)
}

fn validate_frozen_registry_resolution(
    frozen_registry_packages: &BTreeSet<FrozenRegistryPackage>,
    resolved: &[u8],
) -> MResult<()> {
    for package in registry_packages(resolved, "generated Cargo lockfile")? {
        if !frozen_registry_packages.contains(&package) {
            return Err(cargo_error(format!(
                "generated Cargo lockfile selected unfrozen registry package {} {}",
                package.name, package.version,
            )));
        }
    }
    Ok(())
}

/// Build one materialized generated project and return the exact executable
/// path reported by Cargo's `CompilerArtifact` message.
#[allow(clippy::too_many_arguments)]
pub fn build_native_project(
    project: &GeneratedNativeProject,
    binary_name: &str,
    target: Option<&str>,
    target_dir: &Path,
    release: bool,
    offline: bool,
) -> MResult<NativeBuildArtifact> {
    require_regular_generated_file(&project.manifest_path(), "Cargo manifest")?;
    require_regular_generated_file(&project.lockfile_path(), "Cargo lockfile")?;

    let manifest_path = project.manifest_path();
    let invocation = isolate_cargo_invocation(
        CargoInvocation::build(
            &manifest_path,
            binary_name,
            target,
            target_dir,
            release,
            offline,
        )?,
        &manifest_path,
    )?;
    let output = invocation.output()?;
    let messages = parse_cargo_build_messages(Cursor::new(&output.stdout))?;
    if !output.status.success() || messages.build_finished == Some(false) {
        return Err(cargo_error(cargo_failure_reason(
            &invocation,
            &output,
            Some(messages.diagnostics_summary()),
        )));
    }
    messages.select_binary(binary_name)
}

fn require_regular_generated_file(path: &Path, label: &str) -> MResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        project_error(format!(
            "{label} `{}` is unavailable: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(project_error(format!(
            "{label} `{}` is not a regular file",
            path.display()
        )));
    }
    Ok(())
}

fn cargo_failure_reason(
    invocation: &CargoInvocation,
    output: &Output,
    parsed_diagnostics: Option<String>,
) -> String {
    let mut details = Vec::new();
    if let Some(diagnostics) = parsed_diagnostics.filter(|diagnostics| !diagnostics.is_empty()) {
        details.push(diagnostics);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !stderr.is_empty() {
        details.push(stderr);
    }
    let suffix = if details.is_empty() {
        String::new()
    } else {
        format!(": {}", details.join("\n"))
    };
    format!(
        "`{}` exited with status {}{suffix}",
        invocation.display_command(),
        output.status
    )
}

fn cargo_error(reason: impl Into<String>) -> mech_core::MechError {
    native_build_error(
        NativeBuildErrorKind::NativeCargoFailed {
            reason: reason.into(),
        },
        None,
    )
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
    use super::*;
    use crate::project::GeneratedSourceSet;

    const MINIMAL_LOCK_SEED: &[u8] =
        b"# This file is automatically @generated by Cargo.\nversion = 4\n";

    fn arguments(invocation: &CargoInvocation) -> Vec<String> {
        invocation
            .arguments()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn lockfile_resolution_is_a_direct_offline_cargo_invocation() {
        let invocation = CargoInvocation::resolve_lockfile("project/Cargo.toml", true);
        assert_eq!(
            arguments(&invocation),
            [
                "+nightly-2026-03-03",
                "metadata",
                "--format-version",
                "1",
                "--manifest-path",
                "project/Cargo.toml",
                "--offline",
            ]
        );
        assert_eq!(invocation.command().get_program(), "cargo");
    }

    #[test]
    fn ambient_cargo_and_rust_build_settings_are_removed() {
        for key in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "RUSTC_WRAPPER",
            "CARGO_BUILD_TARGET",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        ] {
            assert!(is_ambient_cargo_build_input(OsStr::new(key)), "{key}");
        }
        for key in ["CARGO_HOME", "RUSTUP_HOME", "PATH", "HOME"] {
            assert!(!is_ambient_cargo_build_input(OsStr::new(key)), "{key}");
        }
    }

    #[test]
    fn cargo_runs_from_the_filesystem_root_not_manifest_ancestors() {
        let temporary = tempfile::tempdir().unwrap();
        let project = minimal_project(&temporary.path().join("parent/project"));
        project.materialize().unwrap();
        let root = isolated_cargo_working_directory(&project.manifest_path()).unwrap();
        assert_eq!(
            root,
            project
                .manifest_path()
                .canonicalize()
                .unwrap()
                .ancestors()
                .last()
                .unwrap()
        );
        assert!(!root.starts_with(temporary.path()));
    }

    #[test]
    fn generated_resolution_cannot_introduce_a_registry_version_absent_from_the_seed() {
        let resolved = br#"# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "dependency"
version = "2.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
"#;
        let frozen = frozen_resolution_packages(MINIMAL_LOCK_SEED).unwrap();
        let error = validate_frozen_registry_resolution(&frozen, resolved).unwrap_err();
        assert_eq!(error.kind_name(), "NativeCargoFailed");
        assert!(error.kind_message().contains("unfrozen registry package"));
    }

    #[test]
    fn resolution_seed_cannot_permit_two_semver_interchangeable_versions() {
        let seed = br#"# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "dependency"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[package]]
name = "dependency"
version = "1.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
"#;
        let error = frozen_resolution_packages(seed).unwrap_err();
        assert_eq!(error.kind_name(), "NativeCargoFailed");
        assert!(
            error
                .kind_message()
                .contains("interchangeable registry versions")
        );
    }

    #[test]
    fn build_arguments_match_the_frozen_contract() {
        let invocation = CargoInvocation::build(
            "project/Cargo.toml",
            "native-app",
            Some("x86_64-unknown-linux-gnu"),
            "target/mech-native/cargo-target",
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            arguments(&invocation),
            [
                "+nightly-2026-03-03",
                "build",
                "--manifest-path",
                "project/Cargo.toml",
                "--bin",
                "native-app",
                "--message-format=json-render-diagnostics",
                "--locked",
                "--target-dir",
                "target/mech-native/cargo-target",
                "--release",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--offline",
            ]
        );
    }

    #[test]
    fn hostile_request_strings_never_become_arguments() {
        for binary in ["app; touch owned", "$(whoami)", "--release"] {
            assert!(
                CargoInvocation::build("Cargo.toml", binary, None, "target", false, false).is_err()
            );
        }
        for target in ["x86_64;touch-owned", "$(whoami)", "--target=x"] {
            assert!(
                CargoInvocation::build(
                    "Cargo.toml",
                    "safe-app",
                    Some(target),
                    "target",
                    false,
                    false,
                )
                .is_err()
            );
        }
    }

    fn minimal_project(root: &Path) -> GeneratedNativeProject {
        let mut sources = GeneratedSourceSet::new();
        sources
            .insert(
                "src/main.rs",
                "fn main() { println!(\"cargo-execution-ok\"); }\n",
            )
            .unwrap();
        sources.insert("src/catalog.rs", "// catalog\n").unwrap();
        sources
            .insert("src/native_numeric.rs", "// numeric\n")
            .unwrap();
        sources.insert("src/runtime.rs", "// runtime\n").unwrap();
        GeneratedNativeProject::new(
            root,
            concat!(
                "[package]\n",
                "name = \"mech-native-cargo-execution-test\"\n",
                "version = \"0.0.0\"\n",
                "edition = \"2024\"\n",
                "\n",
                "[[bin]]\n",
                "name = \"native-cargo-test\"\n",
                "path = \"src/main.rs\"\n",
                "\n",
                "[workspace]\n",
            ),
            "{}",
            Vec::new(),
            sources,
        )
    }

    #[test]
    fn direct_cargo_execution_generates_lockfile_and_selects_reported_binary() {
        let temporary = tempfile::tempdir().unwrap();
        let project = minimal_project(&temporary.path().join("project"));
        let target_dir = temporary.path().join("shared-cargo-target");
        project.materialize().unwrap();

        generate_project_lockfile(&project, MINIMAL_LOCK_SEED, true).unwrap();
        assert!(project.lockfile_path().is_file());
        let first_lock = fs::read(project.lockfile_path()).unwrap();
        generate_project_lockfile(&project, MINIMAL_LOCK_SEED, true).unwrap();
        assert_eq!(fs::read(project.lockfile_path()).unwrap(), first_lock);

        let artifact = build_native_project(
            &project,
            "native-cargo-test",
            None,
            &target_dir,
            false,
            true,
        )
        .unwrap();
        assert!(artifact.executable.is_file());
        assert!(artifact.executable.starts_with(&target_dir));
        assert_ne!(artifact.executable, target_dir.join("guessed-path"));
    }

    #[test]
    fn cargo_helpers_require_materialized_regular_inputs() {
        let temporary = tempfile::tempdir().unwrap();
        let project = minimal_project(&temporary.path().join("missing"));
        let error = generate_project_lockfile(&project, MINIMAL_LOCK_SEED, true).unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
    }

    #[cfg(unix)]
    #[test]
    fn lockfile_generation_rejects_a_symlink_destination() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let project = minimal_project(&temporary.path().join("project"));
        project.materialize().unwrap();
        let outside = temporary.path().join("outside-lock");
        fs::write(&outside, "outside").unwrap();
        symlink(&outside, project.lockfile_path()).unwrap();

        let error = generate_project_lockfile(&project, MINIMAL_LOCK_SEED, true).unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
        assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
    }
}
