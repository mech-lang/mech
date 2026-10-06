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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Barrier, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use isolated::{
    CommandContext, OwnerProfile, OwnerRunnerPreparation, ProcessBoundary,
    query_pinned_host_target, run_json_command, run_json_command_observed,
};

const FIXTURE_ROOT: &str = "MECH_NATIVE_PROCESS_FIXTURE_ROOT";
const FIXTURE_MODE: &str = "MECH_NATIVE_PROCESS_FIXTURE_MODE";
const WRAPPER_SCENARIO: &str = "MECH_NATIVE_WRAPPER_SCENARIO";
const FIXTURE_FAILURE_PREFIX: &str = "fixture-failure-";
#[cfg(unix)]
const DISABLE_PARENT_DEATH: &str = "MECH_NATIVE_TEST_DISABLE_PARENT_DEATH";
#[cfg(unix)]
const ORACLE_ROOT: &str = "MECH_NATIVE_PARENT_DEATH_ORACLE_ROOT";
const FIXTURE_EXPIRY_MS: &str = "MECH_NATIVE_FIXTURE_EXPIRY_MS";
const SAFETY_DEADLINE: Duration = Duration::from_secs(60);

fn context(stage: &'static str) -> CommandContext<'static> {
    CommandContext {
        case: "wrapper-contract",
        profile: "test-profile",
        stage,
    }
}

fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
    wait_with_deadline(description, SAFETY_DEADLINE, &mut condition)
        .unwrap_or_else(|error| panic!("{error}"));
}

fn wait_with_deadline(
    description: &str,
    timeout: Duration,
    mut condition: impl FnMut() -> bool,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while !condition() {
        if Instant::now() >= deadline {
            return Err(format!(
                "timed out after {timeout:?} waiting for {description}"
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn wait_file(path: &Path) {
    wait_until(&path.display().to_string(), || path.is_file());
}

fn fixture_failure(root: &Path, reason: &str) -> ! {
    std::fs::write(
        root.join(format!("{FIXTURE_FAILURE_PREFIX}{}", std::process::id())),
        reason,
    )
    .expect("record fixture failure before exiting");
    panic!("fixture safety failure: {reason}");
}

fn wait_fixture_file(root: &Path, name: &str) {
    wait_fixture_file_with_deadline(root, name, SAFETY_DEADLINE);
}

fn wait_fixture_file_with_deadline(root: &Path, name: &str, timeout: Duration) {
    let path = root.join(name);
    if let Err(error) = wait_with_deadline(&path.display().to_string(), timeout, || path.is_file())
    {
        fixture_failure(root, &format!("fixture deadline expired: {error}"));
    }
}

fn assert_no_fixture_failures(root: &Path) {
    let failures = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(FIXTURE_FAILURE_PREFIX)
        })
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "fixture safety failure: {failures:?}");
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

#[test]
#[cfg(unix)]
fn process_group_signals_preserve_complete_multidigit_targets() {
    for process_group in [12, 103, 997, 10_001, 1_443_247] {
        for (force, signal) in [(false, "-TERM"), (true, "-KILL")] {
            let command = isolated::process_group_signal_command(process_group, force);
            assert_eq!(command.get_program(), "kill");
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [signal, "--", &format!("-{process_group}")].map(std::ffi::OsStr::new),
            );
        }
    }
}

#[test]
fn cold_profile_builds_share_one_budget_and_keep_separate_cached_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let preparation = OwnerRunnerPreparation::new();
    let active_builds = AtomicUsize::new(0);
    let peak_builds = AtomicUsize::new(0);
    let started = Barrier::new(3);
    let profiles = [
        (OwnerProfile::Standard, "standard"),
        (OwnerProfile::Full, "full"),
        (OwnerProfile::Fixed, "fixed"),
    ];
    thread::scope(|scope| {
        let preparation = &preparation;
        let started = &started;
        let active_builds = &active_builds;
        let peak_builds = &peak_builds;
        let workers = profiles
            .iter()
            .map(|&(profile, label)| {
                let expected = temporary.path().join(label);
                scope.spawn(move || {
                    started.wait();
                    let executable = preparation
                        .get_or_build(profile, "profile-budget", || {
                            let active = active_builds.fetch_add(1, Ordering::SeqCst) + 1;
                            peak_builds.fetch_max(active, Ordering::SeqCst);
                            // Keep each simulated compiler active briefly while
                            // the other profile initializers contend for admission.
                            thread::sleep(Duration::from_millis(100));
                            active_builds.fetch_sub(1, Ordering::SeqCst);
                            Ok(expected.clone())
                        })
                        .unwrap();
                    assert_eq!(executable, expected);
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap();
        }
    });
    assert_eq!(peak_builds.load(Ordering::SeqCst), 1);
    assert_eq!(active_builds.load(Ordering::SeqCst), 0);
    for (profile, label) in profiles {
        assert_eq!(
            preparation
                .get_or_build(profile, "warm-profile", || {
                    panic!("cached profile {label} must not start another build")
                })
                .unwrap(),
            temporary.path().join(label),
        );
    }
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
        Self::from_pid(pid)
    }

    fn from_pid(pid: u32) -> Self {
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
    assert_no_fixture_failures(&root);
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
    assert_no_fixture_failures(&root);
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
    assert_no_fixture_failures(root);
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
    assert_no_fixture_failures(root);
    assert!(!root.join("target-ready").exists());
    assert!(!root.join("descendant-ready").exists());
}

#[test]
fn parent_death_watchdog_kills_the_owned_child_group() {
    let temporary = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    let root = std::env::var_os(ORACLE_ROOT)
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_owned());
    #[cfg(not(unix))]
    let root = temporary.path().to_owned();
    let mut wrapper = wrapper(&root, "application-ready");
    wait_file(&root.join("supervision-armed"));
    let (target, descendant) = observe_target_tree(&root);
    #[cfg(unix)]
    if std::env::var_os(DISABLE_PARENT_DEATH).is_some() {
        wait_file(&root.join("parent-death-disabled"));
        // The negative self-check captures both process identities before the
        // coordinator can kill the wrapper or start fixture self-expiry.
        wait_file(&root.join("oracle-observation-armed"));
    }
    scheduling_delay();
    wrapper.kill_and_reap();
    std::fs::write(root.join("wrapper-terminated"), b"terminated").unwrap();
    target.assert_exited();
    descendant.assert_exited();
    assert_no_fixture_failures(&root);
    assert!(!root.join("descendant-leaked").exists());
}

#[cfg(unix)]
#[test]
fn parent_death_oracle_rejects_fixture_self_expiry() {
    for delay_ms in [0, 5000] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().to_owned();
        let worker_root = root.clone();
        let (send, receive) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "parent_death_watchdog_kills_the_owned_child_group",
                    "--nocapture",
                ])
                .env(DISABLE_PARENT_DEATH, "1")
                .env(ORACLE_ROOT, &worker_root)
                .env(FIXTURE_EXPIRY_MS, "1000")
                .env(
                    "MECH_NATIVE_PROCESS_TEST_COORDINATOR_DELAY_MS",
                    delay_ms.to_string(),
                );
            send.send(run_json_command::<serde_json::Value>(
                command,
                context("parent-death-negative-oracle"),
                SAFETY_DEADLINE,
                &worker_root,
            ))
            .unwrap();
        });
        let (target, descendant) = observe_target_tree(&root);
        wait_file(&root.join("parent-death-disabled"));
        std::fs::write(root.join("oracle-observation-armed"), b"armed").unwrap();
        let error = receive.recv_timeout(SAFETY_DEADLINE).unwrap().unwrap_err();
        worker.join().unwrap();
        assert!(error.contains("exited unsuccessfully"), "{error}");
        let stderr_logs = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                let name = path.file_name().unwrap().to_string_lossy();
                name.contains("parent-death-negative-oracle") && name.ends_with(".stderr.log")
            })
            .collect::<Vec<_>>();
        assert_eq!(stderr_logs.len(), 1);
        let diagnostics = std::fs::read_to_string(&stderr_logs[0]).unwrap();
        assert!(
            diagnostics.contains("fixture safety failure"),
            "{diagnostics}"
        );
        assert!(
            diagnostics.contains("fixture deadline expired"),
            "{diagnostics}"
        );
        let descendant_failure = std::fs::read_to_string(
            root.join(format!("{FIXTURE_FAILURE_PREFIX}{}", descendant.pid)),
        )
        .unwrap();
        assert!(descendant_failure.contains("fixture deadline expired"));
        assert!(descendant_failure.contains("descendant-release"));
        assert!(root.join("wrapper-terminated").is_file());
        target.assert_exited();
        descendant.assert_exited();
        assert!(!root.join("descendant-leaked").exists());
    }
}

#[cfg(unix)]
fn disable_parent_death_after_release(target_pid: u32, root: &Path) {
    // Test-only mutation: stop the already-armed watchdog only after the real
    // startup protocol releases the application. No wrapper implementation or
    // startup behavior is replaced by this negative validation.
    let output = Command::new("ps")
        .args(["-ax", "-o", "pid=", "-o", "ppid=", "-o", "pgid="])
        .output()
        .unwrap();
    assert!(output.status.success());
    let own_pid = std::process::id();
    let watchdogs = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let fields = line
                .split_whitespace()
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(fields.len(), 3);
            let [pid, parent, group] = fields[..] else {
                unreachable!()
            };
            // At this startup boundary the wrapper owns exactly two grouped
            // children: the target and its separately grouped watchdog. The
            // observing ps process inherits our group and cannot match.
            (parent == own_pid && pid == group && pid != target_pid).then_some(pid)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        watchdogs.len(),
        1,
        "expected the wrapper's one owned watchdog"
    );
    let watchdog = ObservedProcess::from_pid(watchdogs[0]);
    let status = Command::new("/bin/kill")
        .args(["-KILL", "--", &watchdog.pid.to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    watchdog.assert_exited();
    std::fs::write(
        root.join("parent-death-disabled"),
        b"disabled after release",
    )
    .unwrap();
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
                        wait_fixture_file(&root, "startup-release");
                    }
                }
                ProcessBoundary::SupervisionArmed => {
                    record_pid(&root.join("supervision-armed"), pid)?;
                    if scenario == "after-registration" {
                        wait_fixture_file(&root, "startup-release");
                    }
                }
                ProcessBoundary::ApplicationReleased => {
                    #[cfg(unix)]
                    if std::env::var_os(DISABLE_PARENT_DEATH).is_some() {
                        disable_parent_death_after_release(pid, &root);
                    }
                }
            }
            Ok(())
        },
    )
    .unwrap_or_else(|error| fixture_failure(&root, &format!("wrapper fixture failed: {error}")));
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
    wait_fixture_file(&root, "descendant-ready");
    if mode == "failed-leader" {
        wait_fixture_file(&root, "leader-exit");
        std::process::exit(7);
    }
    let status = descendant.wait().unwrap_or_else(|error| {
        fixture_failure(&root, &format!("descendant wait failed: {error}"))
    });
    if !status.success() {
        fixture_failure(
            &root,
            &format!("descendant exited unsuccessfully: {status}"),
        );
    }
}

#[test]
#[ignore = "launched as a pipe-holding descendant by process contract tests"]
fn controlled_descendant_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).unwrap());
    record_pid(&root.join("descendant-ready"), std::process::id()).unwrap();
    let timeout = match std::env::var(FIXTURE_EXPIRY_MS) {
        Ok(timeout) => {
            // The negative oracle exercises ordinary fixture self-expiry, but
            // starts its shortened clock only after observed wrapper death.
            // A delayed coordinator cannot cause an earlier fixture exit.
            wait_fixture_file(&root, "wrapper-terminated");
            Duration::from_millis(timeout.parse().unwrap())
        }
        Err(std::env::VarError::NotPresent) => SAFETY_DEADLINE,
        Err(error) => fixture_failure(&root, &format!("fixture deadline configuration: {error}")),
    };
    wait_fixture_file_with_deadline(&root, "descendant-release", timeout);
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
fn host_target_discovery_timeout_retains_stage_progress_and_logs() {
    let temporary = tempfile::tempdir().unwrap();
    let error = query_pinned_host_target(
        fixture_command(
            temporary.path(),
            "host_target_discovery_fixture_process",
            "hang",
        ),
        context("discover-host-target"),
        Duration::from_secs(5),
        temporary.path(),
    )
    .unwrap_err();
    for detail in [
        "timed out",
        "stage=discover-host-target",
        "pinned host discovery deliberately stalled",
        "stdout=",
        "stderr=",
    ] {
        assert!(error.contains(detail), "{error}");
    }
    assert_discovery_stderr_retained(
        temporary.path(),
        "pinned host discovery deliberately stalled",
    );
    assert_no_fixture_failures(temporary.path());
}

#[test]
fn unsuccessful_host_target_discovery_retains_status_progress_and_logs() {
    let temporary = tempfile::tempdir().unwrap();
    let error = query_pinned_host_target(
        fixture_command(
            temporary.path(),
            "host_target_discovery_fixture_process",
            "fail",
        ),
        context("discover-host-target"),
        SAFETY_DEADLINE,
        temporary.path(),
    )
    .unwrap_err();
    for detail in [
        "exited unsuccessfully",
        "stage=discover-host-target",
        "status=",
        "pinned host discovery deliberately failed",
        "stdout=",
        "stderr=",
    ] {
        assert!(error.contains(detail), "{error}");
    }
    assert_discovery_stderr_retained(
        temporary.path(),
        "pinned host discovery deliberately failed",
    );
}

fn assert_discovery_stderr_retained(root: &Path, detail: &str) {
    let stderr_logs = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.contains("discover-host-target") && name.ends_with(".stderr.log")
        })
        .collect::<Vec<_>>();
    assert_eq!(stderr_logs.len(), 1);
    let retained = std::fs::read_to_string(&stderr_logs[0]).unwrap();
    assert!(retained.contains(detail), "{retained}");
}

#[test]
#[ignore = "injected pinned-host discovery command, isolated from process-global PATH"]
fn host_target_discovery_fixture_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ROOT).unwrap());
    match std::env::var(FIXTURE_MODE).unwrap().as_str() {
        "hang" => {
            eprintln!("pinned host discovery deliberately stalled");
            wait_fixture_file(&root, "host-discovery-release");
        }
        "fail" => {
            eprintln!("pinned host discovery deliberately failed");
            std::process::exit(7);
        }
        mode => panic!("unexpected host-discovery fixture mode {mode}"),
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
