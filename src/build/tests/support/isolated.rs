use std::path::{Path, PathBuf};
use std::process::Command;

use mech_build::NativeBuildPlan;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerProfile {
    Standard,
    Fixed,
}

impl OwnerProfile {
    fn cargo_feature(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Fixed => "fixed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunnerAction {
    Plan,
    Generate,
    Build,
    BuildOnly,
}

impl RunnerAction {
    fn argument(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Generate => "generate",
            Self::Build => "build",
            Self::BuildOnly => "build-only",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OwnerRunnerResult {
    pub plan: NativeBuildPlan,
    pub project_root: Option<PathBuf>,
    pub cargo_manifest: Option<String>,
    pub build_plan_json: Option<String>,
    pub catalog_source: Option<String>,
    pub runtime_source: Option<String>,
    pub executable: Option<PathBuf>,
    pub stdout: Option<String>,
    pub poisoned_output_seed: bool,
    pub poisoned_output_seed_count: usize,
}

pub fn fixture_path(file: &str) -> PathBuf {
    workspace_root()
        .join("tests/architecture/bytecode-v1/phase1")
        .join(file)
}

pub fn run_owner(
    profile: OwnerProfile,
    action: RunnerAction,
    case: &str,
    fixture: impl AsRef<Path>,
    binary_name: &str,
    poison_output_seed: bool,
) -> OwnerRunnerResult {
    let workspace = workspace_root();
    let output = Command::new("cargo")
        .arg("+nightly-2026-03-03")
        .arg("run")
        .arg("--quiet")
        .arg("--locked")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(workspace.join("tests/fixtures/native-build-owner-runner/Cargo.toml"))
        .arg("--target-dir")
        .arg(workspace.join("target/phase1-fixtures/cargo-target"))
        .arg("--no-default-features")
        .arg("--features")
        .arg(profile.cargo_feature())
        .arg("--")
        .arg(action.argument())
        .arg(case)
        .arg(fixture.as_ref())
        .arg(binary_name)
        .arg(if poison_output_seed { "poison" } else { "raw" })
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("isolated owner runner must start");
    assert!(
        output.status.success(),
        "isolated {profile:?} owner runner failed for {case}: status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "isolated owner runner emitted invalid JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    })
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mech-build lives at <workspace>/src/build")
        .to_path_buf()
}
