#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

#[allow(
    dead_code,
    reason = "shared owner-process support exposes helpers selected by the native integration targets"
)]
#[path = "support/isolated.rs"]
mod isolated;

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use isolated::{CommandContext, ProcessBoundary, run_json_command, run_json_command_observed};

const FIXTURE_ROOT: &str = "MECH_NATIVE_PROCESS_FIXTURE_ROOT";
const FIXTURE_MODE: &str = "MECH_NATIVE_PROCESS_FIXTURE_MODE";
const WRAPPER_SCENARIO: &str = "MECH_NATIVE_WRAPPER_SCENARIO";
const SAFETY_DEADLINE: Duration = Duration::from_secs(60);

fn context(stage: &'static str) -> CommandContext<'static> {
    CommandContext {
        case: "wrapper-contract",
        profile: "test-profile",
        stage,
    }
}

fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + SAFETY_DEADLINE;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_file(path: &Path) {
    wait_until(&path.display().to_string(), || path.is_file());
}

fn scheduling_delay() {
    if let Ok(delay) = std::env::var("MECH_NATIVE_PROCESS_TEST_COORDINATOR_DELAY_MS") {
        // Delay the coordinator, never the cleanup oracle. Participants remain
        // held at explicit gates throughout this deliberate scheduling stress.
        thread::sleep(Duration::from_millis(delay.parse().unwrap()));
    }
}

fn record_pid(path: &Path, pid: u32) -> io::Result<()> {
    let pending = path.with_extension("pending");
    std::fs::write(&pending, pid.to_string())?;
    std::fs::rename(pending, path)
}

fn fixture_command(root: &Path, test: &str, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--ignored", "--nocapture"])
        .env(FIXTURE_ROOT, root)
        .env(FIXTURE_MODE, mode);
    command
}

struct ObservedProcess {
    pid: u32,
    #[cfg(unix)]
    started: String,
    #[cfg(windows)]
    handle: std::os::windows::io::OwnedHandle,
}

impl ObservedProcess {
    fn from_pid_file(path: &Path) -> Self {
        wait_file(path);
        let pid = std::fs::read_to_string(path).unwrap().parse().unwrap();
        #[cfg(unix)]
        let started = unix_process_identity(pid)
            .unwrap_or_else(|| panic!("process {pid} exited before its identity was captured"))
            .0;
        #[cfg(windows)]
        let handle = {
            use std::os::windows::io::{FromRawHandle, OwnedHandle};
            use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};
            // SAFETY: OpenProcess returns a new owned synchronization handle;
            // retaining it pins process identity even if the PID is reused.
            let raw = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
            assert!(
                !raw.is_null(),
                "OpenProcess({pid}): {}",
                io::Error::last_os_error()
            );
            unsafe { OwnedHandle::from_raw_handle(raw) }
        };
        Self {
            pid,
            #[cfg(unix)]
            started,
            #[cfg(windows)]
            handle,
        }
    }

    fn has_exited(&self) -> bool {
        #[cfg(unix)]
        {
            match unix_process_identity(self.pid) {
                None => true,
                Some((started, state)) => started != self.started || state.starts_with('Z'),
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
            use windows_sys::Win32::System::Threading::WaitForSingleObject;
            // SAFETY: this synchronization handle remains owned while used.
            let status = unsafe { WaitForSingleObject(self.handle.as_raw_handle(), 0) };
            assert!(status == WAIT_OBJECT_0 || status == WAIT_TIMEOUT);
            status == WAIT_OBJECT_0
        }
    }

    fn assert_exited(&self) {
        wait_until(&format!("process {} to exit", self.pid), || {
            self.has_exited()
        });
    }
}

#[cfg(unix)]
fn unix_process_identity(pid: u32) -> Option<(String, String)> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "lstart=", "-o", "stat="])
        .output()
        .unwrap();
    if !output.status.success() {
        assert_eq!(output.status.code(), Some(1), "ps failed: {output:?}");
        return None;
    }
    let output = String::from_utf8(output.stdout).unwrap();
    let fields = output.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 6, "unexpected process identity: {output}");
    Some((fields[..5].join(" "), fields[5].to_owned()))
}

fn observe_target_tree(root: &Path) -> (ObservedProcess, ObservedProcess) {
    (
        ObservedProcess::from_pid_file(&root.join("target-ready")),
        ObservedProcess::from_pid_file(&root.join("descendant-ready")),
    )
}

#[test]
fn timed_out_child_tree_is_terminated_and_reaped() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_owned();
    let (send, receive) = mpsc::channel();
    let worker_root = root.clone();
    let worker = thread::spawn(move || {
        let result = run_json_command_observed::<serde_json::Value>(
            fixture_command(&worker_root, "controlled_target_process", "timeout"),
            context("hang"),
            Duration::from_secs(2),
            &worker_root,
            |boundary, _| {
                if boundary == ProcessBoundary::ApplicationReleased {
                    wait_file(&worker_root.join("target-ready"));
                    wait_file(&worker_root.join("descendant-ready"));
                    wait_file(&worker_root.join("start-timeout"));
                }
                Ok(())
            },
        );
        send.send(result).unwrap();
    });
    let (target, descendant) = observe_target_tree(&root);
    scheduling_delay();
    std::fs::write(root.join("start-timeout"), b"start").unwrap();
    let error = receive.recv_timeout(SAFETY_DEADLINE).unwrap().unwrap_err();
    worker.join().unwrap();
    assert!(error.contains("timed out"), "{error}");
    for detail in [
        "case=wrapper-contract",
        "profile=test-profile",
        "stdout=",
        "stderr=",
    ] {
        assert!(error.contains(detail), "{error}");
    }
    target.assert_exited();
    descendant.assert_exited();
    std::fs::write(root.join("descendant-release"), b"release").unwrap();
    assert!(!root.join("descendant-leaked").exists());
}

#[test]
fn failed_leader_cannot_leave_a_pipe_inheriting_descendant() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_owned();
    let worker_root = root.clone();
    let (send, receive) = mpsc::channel();
    let worker = thread::spawn(move || {
        send.send(run_json_command::<serde_json::Value>(
            fixture_command(&worker_root, "controlled_target_process", "failed-leader"),
            context("failed-parent-tree"),
            SAFETY_DEADLINE,
            &worker_root,
        ))
        .unwrap();
    });
    let (target, descendant) = observe_target_tree(&root);
    scheduling_delay();
    std::fs::write(root.join("leader-exit"), b"exit").unwrap();
    // Completion proves both inherited-pipe readers joined. This deadline is a
    // safety bound, not a short elapsed-time performance assertion.
    let error = receive.recv_timeout(SAFETY_DEADLINE).unwrap().unwrap_err();
    worker.join().unwrap();
    assert!(error.contains("exited unsuccessfully"), "{error}");
    target.assert_exited();
    descendant.assert_exited();
    assert!(!root.join("descendant-leaked").exists());
}

struct WrapperGuard(Child);

impl WrapperGuard {
    fn kill_and_reap(&mut self) {
        self.0.kill().unwrap();
        self.0.wait().unwrap();
    }
}

impl Drop for WrapperGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wrapper(root: &Path, scenario: &str) -> WrapperGuard {
    let mut command = fixture_command(root, "parent_death_helper_process", "wrapper");
    WrapperGuard(
        command
            .env(WRAPPER_SCENARIO, scenario)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

#[cfg(unix)]
#[test]
fn wrapper_death_before_supervision_registration_is_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut wrapper = wrapper(root, "before-registration");
    let launcher = ObservedProcess::from_pid_file(&root.join("startup-blocked"));
    assert!(!root.join("supervision-armed").exists());
    scheduling_delay();
    wrapper.kill_and_reap();
    launcher.assert_exited();
    assert!(!root.join("target-ready").exists());
    assert!(!root.join("descendant-ready").exists());
}

#[cfg(unix)]
#[test]
fn wrapper_death_after_supervision_registration_before_release_is_contained() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut wrapper = wrapper(root, "after-registration");
    let launcher = ObservedProcess::from_pid_file(&root.join("supervision-armed"));
    scheduling_delay();
    wrapper.kill_and_reap();
    launcher.assert_exited();
    assert!(!root.join("target-ready").exists());
    assert!(!root.join("descendant-ready").exists());
}

#[test]
fn parent_death_watchdog_kills_the_owned_child_group() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut wrapper = wrapper(root, "application-ready");
    wait_file(&root.join("supervision-armed"));
    let (target, descendant) = observe_target_tree(root);
    scheduling_delay();
    wrapper.kill_and_reap();
    target.assert_exited();
    descendant.assert_exited();
    assert!(!root.join("descendant-leaked").exists());
}

#[test]
#[ignore = "launched as a wrapper by the parent-death contract"]
fn parent_death_helper_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).unwrap());
    let scenario = std::env::var(WRAPPER_SCENARIO).unwrap();
    let _: serde_json::Value = run_json_command_observed(
        fixture_command(&root, "controlled_target_process", "parent-death"),
        context("parent-death-helper"),
        SAFETY_DEADLINE,
        &root,
        |boundary, pid| {
            match boundary {
                ProcessBoundary::StartupBlocked => {
                    record_pid(&root.join("startup-blocked"), pid)?;
                    if scenario == "before-registration" {
                        wait_file(&root.join("startup-release"));
                    }
                }
                ProcessBoundary::SupervisionArmed => {
                    record_pid(&root.join("supervision-armed"), pid)?;
                    if scenario == "after-registration" {
                        wait_file(&root.join("startup-release"));
                    }
                }
                ProcessBoundary::ApplicationReleased => {}
            }
            Ok(())
        },
    )
    .unwrap();
}

#[test]
#[ignore = "launched as the contained target by process contract tests"]
fn controlled_target_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).unwrap());
    let mode = std::env::var(FIXTURE_MODE).unwrap();
    record_pid(&root.join("target-ready"), std::process::id()).unwrap();
    #[cfg(unix)]
    let mut command = {
        let mut command = Command::new("sh");
        command
            .args([
                "-c",
                "trap '' TERM; exec \"$@\"",
                "term-ignoring-descendant",
            ])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "controlled_descendant_process",
                "--ignored",
                "--nocapture",
            ])
            .env(FIXTURE_ROOT, &root);
        command
    };
    #[cfg(windows)]
    let mut command = fixture_command(&root, "controlled_descendant_process", "descendant");
    let mut descendant = command.spawn().unwrap();
    wait_file(&root.join("descendant-ready"));
    if mode == "failed-leader" {
        wait_file(&root.join("leader-exit"));
        std::process::exit(7);
    }
    descendant.wait().unwrap();
}

#[test]
#[ignore = "launched as a pipe-holding descendant by process contract tests"]
fn controlled_descendant_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).unwrap());
    record_pid(&root.join("descendant-ready"), std::process::id()).unwrap();
    wait_file(&root.join("descendant-release"));
    std::fs::write(root.join("descendant-leaked"), b"leaked").unwrap();
}

#[test]
fn unsuccessful_child_reports_status_progress_and_log_paths() {
    let temporary = tempfile::tempdir().unwrap();
    let error = run_json_command::<serde_json::Value>(
        unsuccessful_child(),
        context("failure"),
        SAFETY_DEADLINE,
        temporary.path(),
    )
    .unwrap_err();
    for detail in [
        "exited unsuccessfully",
        "status=",
        "expected child failure",
        "stdout=",
        "stderr=",
    ] {
        assert!(error.contains(detail), "{error}");
    }
}

#[test]
fn malformed_json_is_rejected_without_losing_the_structured_stdout_log() {
    let temporary = tempfile::tempdir().unwrap();
    let error = run_json_command::<serde_json::Value>(
        malformed_json_child(),
        context("malformed-json"),
        SAFETY_DEADLINE,
        temporary.path(),
    )
    .unwrap_err();
    for detail in [
        "malformed structured JSON output",
        "stdout_tail=not-json",
        "stdout=",
    ] {
        assert!(error.contains(detail), "{error}");
    }
}

#[cfg(unix)]
fn unsuccessful_child() -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", "printf 'expected child failure\\n' >&2; exit 7"]);
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
