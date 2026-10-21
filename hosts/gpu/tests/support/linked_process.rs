//! Shared owned-process, source-pin and runtime-closure checks for linked tests.
use mech_build::process::ContainedChild;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(super) fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
pub(super) fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(super) fn pin(path: &Path) -> String {
    hash(&fs::read(path).unwrap())
}

pub(super) fn assert_no_ambient_cargo_config() {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|value| PathBuf::from(value).join(".cargo")));
    if let Some(directory) = cargo_home {
        assert!(
            !directory.join("config").exists() && !directory.join("config.toml").exists(),
            "ambient Cargo configuration is unsupported"
        );
    }
}

pub(super) fn bounded(command: &mut Command, evidence: &Path, label: &str, seconds: u64) -> Output {
    let stdout = evidence.join(format!("{label}.stdout"));
    let stderr = evidence.join(format!("{label}.stderr"));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(File::create(&stdout).unwrap()))
        .stderr(Stdio::from(File::create(&stderr).unwrap()));
    let mut child = ContainedChild::spawn(command).unwrap();
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
            panic!("{label} exceeded deadline; see {}", stderr.display());
        }
        thread::sleep(Duration::from_millis(20));
    };
    child.terminate_tree().unwrap();
    Output {
        status,
        stdout: fs::read(stdout).unwrap(),
        stderr: fs::read(stderr).unwrap(),
    }
}

pub(super) fn clean(evidence: &Path, commit: &str, label: &str) {
    let head = bounded(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(workspace()),
        evidence,
        &format!("{label}-head"),
        15,
    );
    assert!(head.status.success());
    assert_eq!(String::from_utf8(head.stdout).unwrap().trim(), commit);
    let status = bounded(
        Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=all"])
            .current_dir(workspace()),
        evidence,
        &format!("{label}-status"),
        15,
    );
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "requires a clean immutable checkout"
    );
}

pub(super) fn closure(metadata: &Value, stdout: &[u8]) -> Vec<Value> {
    let packages = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p))
        .collect::<BTreeMap<_, _>>();
    let root = metadata["resolve"]["root"].as_str().unwrap();
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    let mut names = BTreeSet::new();
    for line in stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let message: Value = serde_json::from_slice(line).unwrap();
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let id = message["package_id"].as_str().unwrap();
        let package = packages[id];
        let name = package["name"].as_str().unwrap();
        names.insert(name.to_owned());
        assert_ne!(name, "mech-syntax", "runtime contains parser");
        assert!(
            !name.starts_with("cranelift-"),
            "runtime contains code generator"
        );
        if name.starts_with("mech-") {
            assert!(package["source"].is_null(), "registry Mech crate {name}");
            assert!(
                Path::new(package["manifest_path"].as_str().unwrap())
                    .canonicalize()
                    .unwrap()
                    .starts_with(workspace().canonicalize().unwrap())
            );
        }
        let forbidden: &[&str] = match name {
            "mech-gpu" => &[
                "canonical-native",
                "jit",
                "aot",
                "sequence-cli",
                "native-build",
                "fused-cli",
            ],
            "mech-engine" | "mech-stdlib" => &[
                "source",
                "semantic-compiler",
                "compiler",
                "full_compiler",
                "standard_compiler",
                "native-sequence",
            ],
            _ => &[],
        };
        for feature in message["features"].as_array().unwrap() {
            assert!(
                !forbidden.contains(&feature.as_str().unwrap()),
                "runtime enabled {name}/{feature}"
            );
        }
        if seen.insert((
            id.to_owned(),
            message["target"].to_string(),
            message["features"].to_string(),
        )) {
            result.push(json!({"name":name, "id":id, "root":id==root, "target":message["target"], "features":message["features"], "filenames":message["filenames"]}));
        }
    }
    for name in ["mech-core", "mech-engine", "mech-gpu", "mech-stdlib"] {
        assert!(names.contains(name));
    }
    assert!(result.iter().any(|unit| unit["root"] == true));
    result
}
