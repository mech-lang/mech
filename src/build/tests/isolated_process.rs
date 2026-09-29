#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

#[allow(
    dead_code,
    reason = "shared owner-process support exposes helpers selected by the native integration targets"
)]
#[path = "support/isolated.rs"]
mod isolated;

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use isolated::{CommandContext, run_json_command};

const PARENT_DEATH_HELPER_ROOT: &str = "MECH_NATIVE_PARENT_DEATH_HELPER_ROOT";
#[cfg(windows)]
const TIMEOUT_DESCENDANT_ROOT: &str = "MECH_NATIVE_TIMEOUT_DESCENDANT_ROOT";

fn context(stage: &'static str) -> CommandContext<'static> {
    CommandContext {
        case: "wrapper-contract",
        profile: "test-profile",
        stage,
    }
}

#[test]
fn timed_out_child_tree_is_terminated_and_reaped() {
    let temporary = tempfile::tempdir().unwrap();
    let ready = temporary.path().join("timeout-descendant-ready");
    let release = temporary.path().join("timeout-descendant-release");
    let leaked = temporary.path().join("timeout-descendant-leaked");
    let error = run_json_command::<serde_json::Value>(
        hanging_child(&ready, &release, &leaked),
        context("hang"),
        Duration::from_secs(5),
        temporary.path(),
    )
    .unwrap_err();
    assert!(error.contains("timed out"), "{error}");
    assert!(error.contains("case=wrapper-contract"), "{error}");
    assert!(error.contains("profile=test-profile"), "{error}");
    assert!(error.contains("stdout="), "{error}");
    assert!(error.contains("stderr="), "{error}");
    assert!(
        ready.exists(),
        "the timeout expired before the descendant reported readiness"
    );
    std::fs::write(&release, b"release").unwrap();

    thread::sleep(Duration::from_secs(2));
    assert!(
        !leaked.exists(),
        "a descendant survived the wrapper timeout and wrote {}",
        leaked.display()
    );
}

#[test]
fn unsuccessful_child_reports_status_progress_and_log_paths() {
    let temporary = tempfile::tempdir().unwrap();
    let error = run_json_command::<serde_json::Value>(
        unsuccessful_child(),
        context("failure"),
        Duration::from_secs(5),
        temporary.path(),
    )
    .unwrap_err();
    assert!(error.contains("exited unsuccessfully"), "{error}");
    assert!(error.contains("status="), "{error}");
    assert!(error.contains("expected child failure"), "{error}");
    assert!(error.contains("stdout="), "{error}");
    assert!(error.contains("stderr="), "{error}");
}

#[cfg(unix)]
#[test]
fn failed_leader_cannot_leave_a_pipe_inheriting_descendant() {
    let temporary = tempfile::tempdir().unwrap();
    let marker = temporary.path().join("leaked-after-parent-exit");
    let started = Instant::now();
    let error = run_json_command::<serde_json::Value>(
        failing_parent_with_term_ignoring_descendant(&marker),
        context("failed-parent-tree"),
        Duration::from_secs(5),
        temporary.path(),
    )
    .unwrap_err();
    assert!(error.contains("exited unsuccessfully"), "{error}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "pipe reader waited for the leaked descendant: {:?}",
        started.elapsed()
    );

    thread::sleep(Duration::from_millis(1_200));
    assert!(
        !marker.exists(),
        "a TERM-ignoring descendant survived its failed leader and wrote {}",
        marker.display()
    );
}

#[test]
fn parent_death_watchdog_kills_the_owned_child_group() {
    let temporary = tempfile::tempdir().unwrap();
    let ready = temporary.path().join("parent-death-ready");
    let leaked = temporary.path().join("parent-death-leaked");
    let mut helper = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "parent_death_helper_process", "--nocapture"])
        .env(PARENT_DEATH_HELPER_ROOT, temporary.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() && Instant::now() < deadline {
        if let Some(status) = helper.try_wait().unwrap() {
            panic!("parent-death helper exited before readiness: {status}");
        }
        thread::sleep(Duration::from_millis(20));
    }
    if !ready.exists() {
        let _ = helper.kill();
        let _ = helper.wait();
        panic!("parent-death helper never became ready");
    }
    let _ = helper.kill();
    let _ = helper.wait();

    thread::sleep(Duration::from_millis(2_500));
    assert!(
        !leaked.exists(),
        "the wrapper parent died but its owned child group wrote {}",
        leaked.display()
    );
}

#[test]
fn parent_death_helper_process() {
    let Some(root) = std::env::var_os(PARENT_DEATH_HELPER_ROOT) else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let ready = root.join("parent-death-ready");
    let leaked = root.join("parent-death-leaked");
    let _: serde_json::Value = run_json_command(
        parent_death_child(&ready, &leaked),
        context("parent-death-helper"),
        Duration::from_secs(30),
        &root,
    )
    .unwrap();
}

#[cfg(windows)]
#[test]
#[ignore = "launched as a contained descendant by the wrapper contract"]
fn windows_parent_death_descendant() {
    let root = std::path::PathBuf::from(
        std::env::var_os(PARENT_DEATH_HELPER_ROOT)
            .expect("parent-death descendant requires its marker root"),
    );
    std::fs::write(root.join("parent-death-ready"), b"ready").unwrap();
    thread::sleep(Duration::from_secs(2));
    std::fs::write(root.join("parent-death-leaked"), b"leaked").unwrap();
    thread::sleep(Duration::from_secs(30));
}

#[cfg(windows)]
#[test]
#[ignore = "launched as a contained descendant by the wrapper contract"]
fn windows_timeout_descendant() {
    let root = std::path::PathBuf::from(
        std::env::var_os(TIMEOUT_DESCENDANT_ROOT)
            .expect("timeout descendant requires its marker root"),
    );
    std::fs::write(root.join("timeout-descendant-ready"), b"ready").unwrap();
    let release = root.join("timeout-descendant-release");
    while !release.exists() {
        thread::sleep(Duration::from_millis(20));
    }
    std::fs::write(root.join("timeout-descendant-leaked"), b"leaked").unwrap();
    thread::sleep(Duration::from_secs(30));
}

#[test]
fn malformed_json_is_rejected_without_losing_the_structured_stdout_log() {
    let temporary = tempfile::tempdir().unwrap();
    let error = run_json_command::<serde_json::Value>(
        malformed_json_child(),
        context("malformed-json"),
        Duration::from_secs(5),
        temporary.path(),
    )
    .unwrap_err();
    assert!(
        error.contains("malformed structured JSON output"),
        "{error}"
    );
    assert!(error.contains("stdout_tail=not-json"), "{error}");
    assert!(error.contains("stdout="), "{error}");
}

#[cfg(unix)]
fn hanging_child(ready: &Path, release: &Path, leaked: &Path) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(
            "(printf ready > \"$1\"; while test ! -e \"$2\"; do sleep 0.02; done; printf leaked > \"$3\") & wait",
        )
        .arg("native-owner-wrapper")
        .arg(ready)
        .arg(release)
        .arg(leaked);
    command
}

#[cfg(windows)]
fn hanging_child(_ready: &Path, _release: &Path, leaked: &Path) -> Command {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "param($testBinary); $child = Start-Process -FilePath $testBinary -ArgumentList @('--exact', 'windows_timeout_descendant', '--ignored', '--nocapture') -PassThru; $child.WaitForExit(); exit $child.ExitCode",
        ])
        .arg(std::env::current_exe().unwrap())
        .env(
            TIMEOUT_DESCENDANT_ROOT,
            leaked.parent().expect("timeout leak marker has a parent"),
        );
    command
}

#[cfg(unix)]
fn unsuccessful_child() -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", "printf 'expected child failure\\n' >&2; exit 7"]);
    command
}

#[cfg(unix)]
fn failing_parent_with_term_ignoring_descendant(marker: &Path) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("(trap '' TERM; sleep 1; printf leaked > \"$1\"; sleep 30) & exit 7")
        .arg("native-owner-wrapper")
        .arg(marker);
    command
}

#[cfg(unix)]
fn parent_death_child(ready: &Path, leaked: &Path) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("printf ready > \"$1\"; sleep 2; printf leaked > \"$2\"; sleep 30")
        .arg("native-owner-wrapper")
        .arg(ready)
        .arg(leaked);
    command
}

#[cfg(windows)]
fn parent_death_child(_ready: &Path, _leaked: &Path) -> Command {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "param($testBinary); $child = Start-Process -FilePath $testBinary -ArgumentList @('--exact', 'windows_parent_death_descendant', '--ignored', '--nocapture') -PassThru; $child.WaitForExit(); exit $child.ExitCode",
        ])
        .arg(std::env::current_exe().unwrap());
    command
}

#[cfg(windows)]
fn unsuccessful_child() -> Command {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "[Console]::Error.WriteLine('expected child failure'); exit 7",
    ]);
    command
}

#[cfg(unix)]
fn malformed_json_child() -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", "printf not-json"]);
    command
}

#[cfg(windows)]
fn malformed_json_child() -> Command {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "[Console]::Out.Write('not-json')",
    ]);
    command
}
