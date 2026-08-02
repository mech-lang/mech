use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use mech_core::MResult;

use crate::project::{validate_project_binary_name, validate_project_target_triple};

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
