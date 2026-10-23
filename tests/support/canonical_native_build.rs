//! Shared qualification helpers; no public application/runtime API.
use mech_build::process::ContainedChild;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) enum CaseDirectory {
    Temporary(tempfile::TempDir),
    Retained(PathBuf),
}
impl CaseDirectory {
    pub(super) fn path(&self) -> &Path {
        match self {
            Self::Temporary(owner) => owner.path(),
            Self::Retained(path) => path,
        }
    }
}
pub(super) fn case_directory(label: &str) -> CaseDirectory {
    if let Some(root) = std::env::var_os("MECH_CANONICAL_CLI_EVIDENCE_DIR") {
        let root = PathBuf::from(root);
        assert!(root.is_absolute() && root.file_name().is_some());
        fs::create_dir_all(&root).unwrap();
        let owner = tempfile::Builder::new()
            .prefix(label)
            .tempdir_in(root)
            .unwrap();
        let path = owner.keep();
        eprintln!(
            "CANONICAL_CLI_EVIDENCE case={label} path={}",
            path.display()
        );
        CaseDirectory::Retained(path)
    } else {
        CaseDirectory::Temporary(tempfile::tempdir().unwrap())
    }
}
pub(super) fn complete(root: &Path) {
    fs::write(
        root.join("qualification-result.json"),
        b"{\"schema\":\"mech.public-native-cli-qualification.v1\",\"passed\":true}\n",
    )
    .unwrap();
}

pub(super) fn bounded(command: &mut Command, root: &Path, label: &str, seconds: u64) -> Output {
    let stdout = root.join(format!("{label}.stdout"));
    let stderr = root.join(format!("{label}.stderr"));
    let receipt = root.join(format!("{label}.result.json"));
    assert!(!stdout.exists() && !stderr.exists() && !receipt.exists());
    fs::write(
        &receipt,
        serde_json::to_vec_pretty(&json!({
            "state":"launch-requested", "stdout":&stdout, "stderr":&stderr,
            "deadline_seconds":seconds, "qualification_only":true
        }))
        .unwrap(),
    )
    .unwrap();
    command
        .stdout(Stdio::from(File::create(&stdout).unwrap()))
        .stderr(Stdio::from(File::create(&stderr).unwrap()));
    let mut child = ContainedChild::spawn(command).unwrap();
    fs::write(
        &receipt,
        serde_json::to_vec_pretty(&json!({
            "state":"running", "pid":child.child_mut().id(), "qualification_only":true
        }))
        .unwrap(),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let status = loop {
        if let Some(status) = child.child_mut().try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.terminate_tree().unwrap();
            let reap = Instant::now() + Duration::from_secs(5);
            while child.child_mut().try_wait().unwrap().is_none() && Instant::now() < reap {
                thread::sleep(Duration::from_millis(20));
            }
            fs::write(
                &receipt,
                serde_json::to_vec_pretty(&json!({
                    "state":"timed-out", "deadline_seconds":seconds, "qualification_only":true
                }))
                .unwrap(),
            )
            .unwrap();
            panic!("{label} exceeded its explicit qualification deadline");
        }
        thread::sleep(Duration::from_millis(20));
    };
    child.terminate_tree().unwrap();
    let output = Output {
        status,
        stdout: fs::read(stdout).unwrap(),
        stderr: fs::read(stderr).unwrap(),
    };
    fs::write(
        &receipt,
        serde_json::to_vec_pretty(&json!({
            "state":"completed", "exit_code":status.code(), "success":status.success(),
            "stdout_sha256":hash(&output.stdout), "stderr_sha256":hash(&output.stderr),
            "qualification_only":true
        }))
        .unwrap(),
    )
    .unwrap();
    output
}
pub(super) fn success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
pub(super) fn cli(root: &Path, input: &Path, output: &Path, native: bool) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
    command
        .current_dir(root)
        .args(["--no-config", "build"])
        .arg(input)
        .args(["--name", "public-knucleotide", "--out"])
        .arg(output)
        .arg("--workspace-root")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--offline");
    if native {
        command.args([
            "--backend",
            "cranelift",
            "--profile",
            "debug",
            "--keep-project",
        ]);
    } else {
        command.args(["--emit", "bytecode"]);
    }
    command.stdin(Stdio::null());
    command
}
pub(super) fn launch() -> Value {
    json!({
        "grants":[
            {"target":"cli/stdin","operations":["read"],"paths":["text"]},
            {"target":"cli/stdout","operations":["write"],"paths":["text"]}
        ],
        "max_ports":16, "input_text_bytes":1048576, "output_text_bytes":1048576,
        "memory_bytes":268435456, "work":1000000000,
        "transport_bytes":16777216, "native_stack_bytes":1048576,
        "library":{
            "output_elements":1000000, "output_bytes":134217728,
            "temporary_bytes":134217728, "cloned_bytes":134217728,
            "retained_nodes":1000000, "compute_work":134217728, "comparison_work":1000000
        }
    })
}
pub(super) fn invoke(binary: &Path, policy: &Path, report: &Path, input: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .arg(policy)
        .arg(report)
        .arg("--")
        .stdin(Stdio::from(File::open(input).unwrap()));
    command
}
pub(super) fn project(output: &Path) -> PathBuf {
    PathBuf::from(format!("{}.project", output.display()))
}
pub(super) fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(super) fn validate_report(report: &Value, manifest: &Value, launch_bytes: &[u8]) {
    assert_eq!(
        report["route"],
        "canonical-artifact-program-object-secured-cli"
    );
    assert_eq!(report["program_revision"], manifest["program_revision"]);
    assert_eq!(report["launch_sha256"], hash(launch_bytes));
    assert_eq!(report["native_aot"], true);
    assert_eq!(report["source_parser"], false);
    assert_eq!(report["jit"], false);
    assert_eq!(report["resident_body_evaluation"], false);
    assert_eq!(report["hosted_phases"], "admit-activate-capture-resume");
    assert_eq!(report["shutdown"], "completed");
}
