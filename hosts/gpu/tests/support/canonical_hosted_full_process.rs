//! Test-driver owned byte snapshots and contained actual linked processes.
use super::config::FileSpec;
use mech_build::process::ContainedChild;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn hex(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(super) fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 16384];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex(hash.finalize()))
}
// Clone copies path/hash metadata, never the immutable input payload.
#[derive(Clone, Debug)]
pub(super) struct Frozen {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
}
impl Frozen {
    pub(super) fn check(&self) -> Result<(), String> {
        if fs::metadata(&self.path).map_err(|e| e.to_string())?.len() != self.bytes
            || hash_file(&self.path)? != self.sha256
        {
            return Err(format!("owned input changed: {}", self.path.display()));
        }
        Ok(())
    }
    pub(super) fn evidence(&self, requested: &Path) -> Value {
        json!({"requested_path":requested, "owned_path":self.path, "sha256":self.sha256,
            "actual_bytes":self.bytes, "read_only":true})
    }
}
/// Hash the bytes actually copied, not a hash-then-reopen external workload.
pub(super) fn freeze(spec: &FileSpec, path: &Path, cap: u64) -> Result<Frozen, String> {
    let mut source = File::open(&spec.path).map_err(|e| e.to_string())?;
    if !source.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("workload input must be a regular file".into());
    }
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 16384];
    loop {
        let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        bytes = bytes
            .checked_add(n as u64)
            .filter(|n| *n <= cap)
            .ok_or("actual file exceeds explicit finite cap")?;
        target.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        hash.update(&buffer[..n]);
    }
    target.flush().map_err(|e| e.to_string())?;
    drop(target);
    let digest = hex(hash.finalize());
    if digest != spec.sha256 {
        return Err(format!(
            "actual copied SHA256 differs: {}",
            spec.path.display()
        ));
    }
    finish_frozen(path, digest, bytes)
}
fn finish_frozen(path: &Path, digest: String, bytes: u64) -> Result<Frozen, String> {
    let mut permissions = fs::metadata(path).map_err(|e| e.to_string())?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).map_err(|e| e.to_string())?;
    let frozen = Frozen {
        path: path.to_owned(),
        sha256: digest,
        bytes,
    };
    frozen.check()?;
    Ok(frozen)
}
/// Persist a driver-derived launch with the same immutable-file guarantees.
pub(super) fn freeze_bytes(path: &Path, bytes: &[u8], cap: u64) -> Result<Frozen, String> {
    let count = u64::try_from(bytes.len()).map_err(|_| "derived file size overflows")?;
    if count > cap {
        return Err("derived file exceeds explicit finite cap".into());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.flush())
        .map_err(|e| e.to_string())?;
    drop(file);
    finish_frozen(path, hex(Sha256::digest(bytes)), count)
}
pub(super) fn equal_files(left: &Path, right: &Path) -> Result<bool, String> {
    if fs::metadata(left).map_err(|e| e.to_string())?.len()
        != fs::metadata(right).map_err(|e| e.to_string())?.len()
    {
        return Ok(false);
    }
    let mut left = File::open(left).map_err(|e| e.to_string())?;
    let mut right = File::open(right).map_err(|e| e.to_string())?;
    let mut a = [0u8; 16384];
    let mut b = [0u8; 16384];
    loop {
        let n = left.read(&mut a).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(true);
        }
        right.read_exact(&mut b[..n]).map_err(|e| e.to_string())?;
        if a[..n] != b[..n] {
            return Ok(false);
        }
    }
}
pub(super) fn small_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("report/launch exceeds 64 KiB".into());
    }
    Ok(bytes)
}
#[derive(Debug)]
pub(super) struct Capture {
    pub exit: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub elapsed_ns: u64,
    pub status: String,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub report: PathBuf,
}
pub(super) fn invoke(
    binary: &Path,
    launch: &Path,
    input: &Path,
    directory: &Path,
    label: &str,
    seconds: u64,
) -> Result<Capture, String> {
    let stdout = directory.join(format!("{label}.stdout"));
    let stderr = directory.join(format!("{label}.stderr"));
    let report = directory.join(format!("{label}.report.json"));
    let mut command = Command::new(binary);
    command
        .arg(launch)
        .arg(&report)
        .arg("--")
        .env_remove("MECH_NATIVE_SECURITY")
        .stdin(Stdio::from(File::open(input).map_err(|e| e.to_string())?))
        .stdout(Stdio::from(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stdout)
                .map_err(|e| e.to_string())?,
        ))
        .stderr(Stdio::from(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stderr)
                .map_err(|e| e.to_string())?,
        ));
    let start = Instant::now();
    let mut child = ContainedChild::spawn(&mut command).map_err(|e| e.to_string())?;
    let deadline = start + Duration::from_secs(seconds);
    let mut timed_out = false;
    let status = 'wait: loop {
        if let Some(status) = child.child_mut().try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            child.terminate_tree().map_err(|e| e.to_string())?;
            let reap = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(status) = child.child_mut().try_wait().map_err(|e| e.to_string())? {
                    break 'wait status;
                }
                if Instant::now() >= reap {
                    return Err("timed-out child did not reap".into());
                }
                thread::sleep(Duration::from_millis(1));
            }
        }
        thread::sleep(Duration::from_millis(1));
    };
    let elapsed_ns =
        u64::try_from(start.elapsed().as_nanos()).map_err(|_| "elapsed duration overflows")?;
    child.terminate_tree().map_err(|e| e.to_string())?;
    Ok(Capture {
        exit: status.code(),
        success: status.success(),
        status: format!("{status:?}"),
        timed_out,
        elapsed_ns,
        stdout,
        stderr,
        report,
    })
}
