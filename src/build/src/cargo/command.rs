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

/// A direct, shell-free Cargo invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoInvocation {
    arguments: Vec<OsString>,
    current_dir: Option<PathBuf>,
}

impl CargoInvocation {
    pub fn generate_lockfile(manifest_path: impl AsRef<Path>, offline: bool) -> Self {
        let mut arguments = vec![
            OsString::from("generate-lockfile"),
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

/// Generate the frozen project's lockfile with a direct Cargo invocation.
pub fn generate_project_lockfile(project: &GeneratedNativeProject, offline: bool) -> MResult<()> {
    require_regular_generated_file(&project.manifest_path(), "Cargo manifest")?;
    reject_non_regular_generated_file_if_present(&project.lockfile_path(), "Cargo lockfile")?;

    let invocation = CargoInvocation::generate_lockfile(project.manifest_path(), offline);
    let output = invocation.output()?;
    if !output.status.success() {
        return Err(cargo_error(cargo_failure_reason(
            &invocation,
            &output,
            None,
        )));
    }
    require_regular_generated_file(&project.lockfile_path(), "generated Cargo lockfile")?;
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

    let invocation = CargoInvocation::build(
        project.manifest_path(),
        binary_name,
        target,
        target_dir,
        release,
        offline,
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

fn reject_non_regular_generated_file_if_present(path: &Path, label: &str) -> MResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(project_error(format!(
                "{label} `{}` is not a regular file",
                path.display()
            )))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(project_error(format!(
            "failed to inspect {label} `{}`: {error}",
            path.display()
        ))),
    }
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

    fn arguments(invocation: &CargoInvocation) -> Vec<String> {
        invocation
            .arguments()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn generate_lockfile_is_a_direct_offline_cargo_invocation() {
        let invocation = CargoInvocation::generate_lockfile("project/Cargo.toml", true);
        assert_eq!(
            arguments(&invocation),
            [
                "generate-lockfile",
                "--manifest-path",
                "project/Cargo.toml",
                "--offline",
            ]
        );
        assert_eq!(invocation.command().get_program(), "cargo");
    }

    #[test]
    fn build_arguments_match_the_frozen_contract() {
        let invocation = CargoInvocation::build(
            "project/Cargo.toml",
            "phase1-app",
            Some("x86_64-unknown-linux-gnu"),
            "target/mech-native/cargo-target",
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            arguments(&invocation),
            [
                "build",
                "--manifest-path",
                "project/Cargo.toml",
                "--bin",
                "phase1-app",
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
                "name = \"phase1-cargo-test\"\n",
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

        generate_project_lockfile(&project, true).unwrap();
        assert!(project.lockfile_path().is_file());
        let first_lock = fs::read(project.lockfile_path()).unwrap();
        generate_project_lockfile(&project, true).unwrap();
        assert_eq!(fs::read(project.lockfile_path()).unwrap(), first_lock);

        let artifact = build_native_project(
            &project,
            "phase1-cargo-test",
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
        let error = generate_project_lockfile(&project, true).unwrap_err();
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

        let error = generate_project_lockfile(&project, true).unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
        assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
    }
}
