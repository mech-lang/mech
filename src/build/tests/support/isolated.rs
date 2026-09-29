use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
#[cfg(windows)]
use std::os::windows::process::ChildExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
#[cfg(unix)]
use std::{os::fd::OwnedFd, os::unix::net::UnixStream};

use mech_build::NativeBuildPlan;
use serde::Deserialize;
use serde::de::DeserializeOwned;

const OWNER_BUILD_TIMEOUT_ENV: &str = "MECH_NATIVE_OWNER_BUILD_TIMEOUT_SECS";
const OWNER_RUN_TIMEOUT_ENV: &str = "MECH_NATIVE_OWNER_RUN_TIMEOUT_SECS";
const DEFAULT_OWNER_BUILD_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const DEFAULT_OWNER_RUN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DEFAULT_OWNER_BUILD_ACTION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(50);
const PROCESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const TERMINATION_GRACE: Duration = Duration::from_millis(500);
const WATCHDOG_JOIN_GRACE: Duration = Duration::from_secs(2);
const READER_JOIN_GRACE: Duration = Duration::from_secs(5);

static STANDARD_RUNNER: OnceLock<Result<PathBuf, String>> = OnceLock::new();
static FULL_RUNNER: OnceLock<Result<PathBuf, String>> = OnceLock::new();
static FIXED_RUNNER: OnceLock<Result<PathBuf, String>> = OnceLock::new();
static NEXT_LOG_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerProfile {
    Standard,
    Full,
    Fixed,
}

impl OwnerProfile {
    fn cargo_feature(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Full => "full",
            Self::Fixed => "fixed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Full => "full",
            Self::Fixed => "fixed",
        }
    }

    fn build_cache(self) -> &'static OnceLock<Result<PathBuf, String>> {
        match self {
            Self::Standard => &STANDARD_RUNNER,
            Self::Full => &FULL_RUNNER,
            Self::Fixed => &FIXED_RUNNER,
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

    fn timeout(self) -> Duration {
        let default = match self {
            Self::Plan | Self::Generate => DEFAULT_OWNER_RUN_TIMEOUT,
            Self::Build | Self::BuildOnly => DEFAULT_OWNER_BUILD_ACTION_TIMEOUT,
        };
        timeout_from_env(OWNER_RUN_TIMEOUT_ENV, default)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandContext<'a> {
    pub case: &'a str,
    pub profile: &'a str,
    pub stage: &'a str,
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
        .join("tests/architecture/bytecode-v1")
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
    let executable =
        owner_runner(profile, case, &workspace).unwrap_or_else(|error| panic!("{error}"));
    let mut command = Command::new(executable);
    command
        .arg(action.argument())
        .arg(case)
        .arg(fixture.as_ref())
        .arg(binary_name)
        .arg(if poison_output_seed { "poison" } else { "raw" })
        .env("CARGO_TERM_COLOR", "never");
    run_json_command(
        command,
        CommandContext {
            case,
            profile: profile.label(),
            stage: action.argument(),
        },
        action.timeout(),
        &owner_log_root(&workspace),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

fn owner_runner(profile: OwnerProfile, case: &str, workspace: &Path) -> Result<PathBuf, String> {
    profile
        .build_cache()
        .get_or_init(|| build_owner_runner(profile, case, workspace))
        .clone()
}

fn build_owner_runner(
    profile: OwnerProfile,
    case: &str,
    workspace: &Path,
) -> Result<PathBuf, String> {
    let host_target = pinned_nightly_host_target()?;
    let target_dir = workspace
        .join("target/bytecode-v1-fixtures/owner-runner")
        .join(profile.label());
    let mut command = Command::new("cargo");
    command
        .arg("+nightly-2026-03-03")
        .arg("build")
        .arg("--locked")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(workspace.join("tests/fixtures/native-build-owner-runner/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--target")
        .arg(&host_target)
        .arg("--no-default-features")
        .arg("--features")
        .arg(profile.cargo_feature())
        .arg("--bin")
        .arg("native-build-owner-runner")
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_TERM_COLOR", "never");
    let context = CommandContext {
        case,
        profile: profile.label(),
        stage: "build-owner-runner",
    };
    let output = run_command(
        command,
        context,
        timeout_from_env(OWNER_BUILD_TIMEOUT_ENV, DEFAULT_OWNER_BUILD_TIMEOUT),
        &owner_log_root(workspace),
    )?;
    require_success(&output, context)?;

    let executable = target_dir.join(&host_target).join("debug").join(format!(
        "native-build-owner-runner{}",
        std::env::consts::EXE_SUFFIX
    ));
    if !executable.is_file() {
        return Err(format_process_error(
            context,
            &output,
            format!(
                "successful build did not retain executable {}",
                executable.display()
            ),
        ));
    }
    eprintln!(
        "MECH_NATIVE_OWNER_PROGRESS case={case} profile={} stage=build-owner-runner progress=ready executable={} elapsed={:?}",
        profile.label(),
        executable.display(),
        output.elapsed,
    );
    Ok(executable)
}

fn pinned_nightly_host_target() -> Result<String, String> {
    let output = Command::new("rustc")
        .args(["+nightly-2026-03-03", "--print", "host-tuple"])
        .output()
        .map_err(|error| format!("failed to query pinned nightly host target: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "failed to query pinned nightly host target: status={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let target = String::from_utf8(output.stdout)
        .map_err(|error| format!("pinned nightly host target was not UTF-8: {error}"))?;
    let target = target.trim();
    if target.is_empty()
        || !target
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(format!(
            "pinned nightly returned invalid host target {target:?}"
        ));
    }
    Ok(target.to_owned())
}

pub fn run_json_command<T: DeserializeOwned>(
    command: Command,
    context: CommandContext<'_>,
    timeout: Duration,
    log_root: &Path,
) -> Result<T, String> {
    run_json_command_observed(command, context, timeout, log_root, |_, _| Ok(()))
}

/// Observable startup boundaries used by containment contract tests. Observers
/// may hold a boundary; the execution deadline starts after application release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessBoundary {
    StartupBlocked,
    SupervisionArmed,
    ApplicationReleased,
}

pub fn run_json_command_observed<T: DeserializeOwned>(
    command: Command,
    context: CommandContext<'_>,
    timeout: Duration,
    log_root: &Path,
    mut observer: impl FnMut(ProcessBoundary, u32) -> io::Result<()>,
) -> Result<T, String> {
    let output = run_command_observed(command, context, timeout, log_root, &mut observer)?;
    require_success(&output, context)?;
    serde_json::from_slice(&output.stdout).map_err(|error| {
        format_process_error(
            context,
            &output,
            format!("malformed structured JSON output: {error}"),
        )
    })
}

struct CapturedProcess {
    status: ExitStatus,
    stdout: Vec<u8>,
    command: String,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    last_progress: String,
    elapsed: Duration,
}

fn run_command(
    command: Command,
    context: CommandContext<'_>,
    timeout: Duration,
    log_root: &Path,
) -> Result<CapturedProcess, String> {
    run_command_observed(command, context, timeout, log_root, &mut |_, _| Ok(()))
}

fn run_command_observed(
    mut command: Command,
    context: CommandContext<'_>,
    timeout: Duration,
    log_root: &Path,
    observer: &mut impl FnMut(ProcessBoundary, u32) -> io::Result<()>,
) -> Result<CapturedProcess, String> {
    std::fs::create_dir_all(log_root).map_err(|error| {
        format!(
            "native owner process could not create log root {}: {error}",
            log_root.display()
        )
    })?;
    let sequence = NEXT_LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let log_stem = format!(
        "{}-{}-{}-{}-{sequence}",
        sanitize_log_component(context.case),
        sanitize_log_component(context.profile),
        sanitize_log_component(context.stage),
        std::process::id(),
    );
    let stdout_path = log_root.join(format!("{log_stem}.stdout.log"));
    let stderr_path = log_root.join(format!("{log_stem}.stderr.log"));
    let stdout_log = File::create(&stdout_path)
        .map_err(|error| format!("failed to create {}: {error}", stdout_path.display()))?;
    let stderr_log = File::create(&stderr_path)
        .map_err(|error| format!("failed to create {}: {error}", stderr_path.display()))?;

    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let command_text = command_text(&command);
    eprintln!(
        "MECH_NATIVE_OWNER_PROGRESS case={} profile={} stage={} progress=spawn timeout={timeout:?} command={command_text} stdout={} stderr={}",
        context.case,
        context.profile,
        context.stage,
        stdout_path.display(),
        stderr_path.display(),
    );
    let mut process = ProcessTree::spawn(command, observer).map_err(|error| {
        format!(
            "native owner process tree setup failed: case={} profile={} stage={} command={} error={error} stdout={} stderr={}",
            context.case,
            context.profile,
            context.stage,
            command_text,
            stdout_path.display(),
            stderr_path.display(),
        )
    })?;
    let stdout = process
        .child_mut()
        .stdout
        .take()
        .ok_or_else(|| "native owner process stdout was not piped".to_owned())?;
    let stderr = process
        .child_mut()
        .stderr
        .take()
        .ok_or_else(|| "native owner process stderr was not piped".to_owned())?;
    let last_progress = Arc::new(Mutex::new("spawned child process".to_owned()));
    let stdout_reader = thread::spawn(move || capture_stdout(stdout, stdout_log));
    let stderr_reader = {
        let last_progress = Arc::clone(&last_progress);
        thread::spawn(move || stream_stderr(stderr, stderr_log, last_progress))
    };
    let started = Instant::now();
    let mut next_heartbeat = PROCESS_HEARTBEAT_INTERVAL;

    let status = loop {
        match process.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => {}
            Ok(None) => {
                let elapsed = started.elapsed();
                let progress = read_last_progress(&last_progress);
                eprintln!(
                    "MECH_NATIVE_OWNER_PROGRESS case={} profile={} stage={} progress=timeout elapsed={elapsed:?} last={progress:?}",
                    context.case, context.profile, context.stage,
                );
                let status = process.terminate().map_err(|error| {
                    format!(
                        "native owner process timed out and could not be reaped: case={} profile={} stage={} command={} elapsed={elapsed:?} last_progress={progress:?} error={error} stdout={} stderr={}",
                        context.case,
                        context.profile,
                        context.stage,
                        command_text,
                        stdout_path.display(),
                        stderr_path.display(),
                    )
                })?;
                let stdout = join_stdout(stdout_reader)?;
                join_stderr(stderr_reader)?;
                return Err(format!(
                    "native owner process timed out: case={} profile={} stage={} command={} deadline={timeout:?} elapsed={elapsed:?} status={} last_progress={progress:?} stdout={} stderr={} stdout_tail={}",
                    context.case,
                    context.profile,
                    context.stage,
                    command_text,
                    status,
                    stdout_path.display(),
                    stderr_path.display(),
                    output_tail(&stdout),
                ));
            }
            Err(error) => {
                let progress = read_last_progress(&last_progress);
                let _ = process.terminate();
                let _ = join_stdout(stdout_reader);
                let _ = join_stderr(stderr_reader);
                return Err(format!(
                    "native owner process wait failed: case={} profile={} stage={} command={} last_progress={progress:?} error={error} stdout={} stderr={}",
                    context.case,
                    context.profile,
                    context.stage,
                    command_text,
                    stdout_path.display(),
                    stderr_path.display(),
                ));
            }
        }
        let elapsed = started.elapsed();
        if elapsed >= next_heartbeat {
            let progress = read_last_progress(&last_progress);
            eprintln!(
                "MECH_NATIVE_OWNER_PROGRESS case={} profile={} stage={} progress=waiting elapsed={elapsed:?} last={progress:?}",
                context.case, context.profile, context.stage,
            );
            next_heartbeat += PROCESS_HEARTBEAT_INTERVAL;
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    };
    // A command that exits while a descendant retains either pipe would make
    // the reader joins below unbounded. The executable contract never permits
    // descendants to outlive their owner, so close the whole process group
    // before reaping the observed leader status.
    process.kill_remaining_group().map_err(|error| {
        format!(
            "native owner process descendants could not be terminated: case={} profile={} stage={} command={} error={error} stdout={} stderr={}",
            context.case,
            context.profile,
            context.stage,
            command_text,
            stdout_path.display(),
            stderr_path.display(),
        )
    })?;
    process.reap().map_err(|error| {
        format!(
            "native owner process exited but could not be reaped: case={} profile={} stage={} command={} error={error} stdout={} stderr={}",
            context.case,
            context.profile,
            context.stage,
            command_text,
            stdout_path.display(),
            stderr_path.display(),
        )
    })?;
    let stdout = join_stdout(stdout_reader)?;
    join_stderr(stderr_reader)?;
    let elapsed = started.elapsed();
    let last_progress = read_last_progress(&last_progress);
    eprintln!(
        "MECH_NATIVE_OWNER_PROGRESS case={} profile={} stage={} progress=complete elapsed={elapsed:?} status={status} last={last_progress:?} stdout={} stderr={}",
        context.case,
        context.profile,
        context.stage,
        stdout_path.display(),
        stderr_path.display(),
    );
    Ok(CapturedProcess {
        status,
        stdout,
        command: command_text,
        stdout_path,
        stderr_path,
        last_progress,
        elapsed,
    })
}

fn require_success(output: &CapturedProcess, context: CommandContext<'_>) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format_process_error(
            context,
            output,
            "process exited unsuccessfully",
        ))
    }
}

fn format_process_error(
    context: CommandContext<'_>,
    output: &CapturedProcess,
    reason: impl AsRef<str>,
) -> String {
    format!(
        "native owner process failed: reason={} case={} profile={} stage={} command={} elapsed={:?} status={} last_progress={:?} stdout={} stderr={} stdout_tail={}",
        reason.as_ref(),
        context.case,
        context.profile,
        context.stage,
        output.command,
        output.elapsed,
        output.status,
        output.last_progress,
        output.stdout_path.display(),
        output.stderr_path.display(),
        output_tail(&output.stdout),
    )
}

fn capture_stdout(mut input: impl Read, mut log: File) -> io::Result<Vec<u8>> {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        log.write_all(&buffer[..read])?;
        captured.extend_from_slice(&buffer[..read]);
    }
    log.flush()?;
    Ok(captured)
}

fn stream_stderr(
    input: impl Read,
    mut log: File,
    last_progress: Arc<Mutex<String>>,
) -> io::Result<()> {
    let mut input = BufReader::new(input);
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = input.read_until(b'\n', &mut line)?;
        if read == 0 {
            break;
        }
        log.write_all(&line)?;
        io::stderr().write_all(&line)?;
        io::stderr().flush()?;
        let progress = String::from_utf8_lossy(&line).trim().to_owned();
        if !progress.is_empty() {
            *last_progress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = progress;
        }
    }
    log.flush()
}

fn join_stdout(reader: thread::JoinHandle<io::Result<Vec<u8>>>) -> Result<Vec<u8>, String> {
    wait_for_reader(&reader, "stdout")?;
    reader
        .join()
        .map_err(|_| "native owner stdout reader panicked".to_owned())?
        .map_err(|error| format!("native owner stdout reader failed: {error}"))
}

fn join_stderr(reader: thread::JoinHandle<io::Result<()>>) -> Result<(), String> {
    wait_for_reader(&reader, "stderr")?;
    reader
        .join()
        .map_err(|_| "native owner stderr reader panicked".to_owned())?
        .map_err(|error| format!("native owner stderr reader failed: {error}"))
}

fn wait_for_reader<T>(reader: &thread::JoinHandle<T>, stream: &str) -> Result<(), String> {
    let deadline = Instant::now() + READER_JOIN_GRACE;
    while !reader.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    if reader.is_finished() {
        Ok(())
    } else {
        Err(format!(
            "native owner {stream} reader did not finish within {READER_JOIN_GRACE:?}"
        ))
    }
}

fn read_last_progress(progress: &Arc<Mutex<String>>) -> String {
    progress
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn output_tail(output: &[u8]) -> String {
    const LIMIT: usize = 2048;
    let start = output.len().saturating_sub(LIMIT);
    String::from_utf8_lossy(&output[start..]).into_owned()
}

fn timeout_from_env(name: &str, default: Duration) -> Duration {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs)
            .unwrap_or_else(|| panic!("{name} must be a positive integer number of seconds")),
        Err(std::env::VarError::NotPresent) => default,
        Err(error) => panic!("failed to read {name}: {error}"),
    }
}

fn owner_log_root(workspace: &Path) -> PathBuf {
    workspace.join("target/bytecode-v1-fixtures/owner-runner/logs")
}

fn sanitize_log_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn command_text(command: &Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|argument| format!("{:?}", argument))
        .collect::<Vec<_>>()
        .join(" ")
}

struct ProcessTree {
    child: Option<Child>,
    process_id: u32,
    #[cfg(unix)]
    startup_writer: Option<UnixStream>,
    #[cfg(unix)]
    watchdog_writer: Option<UnixStream>,
    #[cfg(unix)]
    watchdog: Option<Child>,
    #[cfg(windows)]
    job: Option<OwnedHandle>,
}

impl ProcessTree {
    fn spawn(
        command: Command,
        observer: &mut impl FnMut(ProcessBoundary, u32) -> io::Result<()>,
    ) -> io::Result<Self> {
        #[cfg(unix)]
        let (mut command, startup_writer) = startup_launcher(&command)?;
        #[cfg(not(unix))]
        let mut command = command;
        configure_process_group(&mut command);
        let child = command.spawn()?;
        let process_id = child.id();
        let mut process = Self {
            child: Some(child),
            process_id,
            #[cfg(unix)]
            startup_writer: Some(startup_writer),
            #[cfg(unix)]
            watchdog_writer: None,
            #[cfg(unix)]
            watchdog: None,
            #[cfg(windows)]
            job: None,
        };
        observer(ProcessBoundary::StartupBlocked, process_id)?;
        #[cfg(unix)]
        {
            let (writer, watchdog) = spawn_parent_death_watchdog(process_id)?;
            process.watchdog_writer = Some(writer);
            process.watchdog = Some(watchdog);
        }
        #[cfg(windows)]
        {
            process.job = Some(create_kill_on_close_job(process.child_mut())?);
        }
        observer(ProcessBoundary::SupervisionArmed, process_id)?;
        #[cfg(unix)]
        {
            // The launcher cannot exec the application until the watchdog has
            // acknowledged its parent-death channel. EOF before this release
            // makes the launcher exit, including if this wrapper is SIGKILLed.
            let mut startup = process.startup_writer.take().expect("startup barrier");
            startup.write_all(b"run\n")?;
        }
        #[cfg(windows)]
        resume_contained_child(process.child_mut())?;
        observer(ProcessBoundary::ApplicationReleased, process_id)?;
        Ok(process)
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child
            .as_mut()
            .expect("process guard retains its child")
            .try_wait()
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child
            .as_mut()
            .expect("process guard retains its child")
    }

    fn kill_remaining_group(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            drop(self.startup_writer.take());
            // EOF is also delivered when this entire test process is killed,
            // allowing the separately grouped watchdog to clean descendants
            // even though Rust destructors cannot run.
            drop(self.watchdog_writer.take());
            let watchdog_result = self
                .watchdog
                .take()
                .map(|mut watchdog| wait_for_watchdog(&mut watchdog))
                .transpose();
            // Keep a direct signal as a best-effort fallback if the watchdog's
            // target group disappeared or changed while the parent was live.
            terminate_process_tree(self.process_id, true);
            let _ = watchdog_result?;
        }
        #[cfg(windows)]
        {
            // Closing the last job handle enforces KILL_ON_JOB_CLOSE even if
            // the original process already exited and taskkill lost its PID.
            drop(self.job.take());
        }
        #[cfg(not(any(unix, windows)))]
        terminate_process_tree(self.process_id, true);
        Ok(())
    }

    fn reap(&mut self) -> io::Result<ExitStatus> {
        self.child
            .take()
            .expect("process guard retains its child")
            .wait()
    }

    fn terminate(&mut self) -> io::Result<ExitStatus> {
        terminate_process_tree(self.process_id, false);
        let deadline = Instant::now() + TERMINATION_GRACE;
        while Instant::now() < deadline {
            if self.try_wait()?.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        // The leader may exit on TERM while a descendant ignores it and keeps
        // stdout/stderr open. Force the owned group before any reader join.
        self.kill_remaining_group()?;
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
        self.reap()
    }
}

impl Drop for ProcessTree {
    fn drop(&mut self) {
        let _ = self.kill_remaining_group();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(unix)]
fn startup_launcher(command: &Command) -> io::Result<(Command, UnixStream)> {
    let (reader, writer) = UnixStream::pair()?;
    let reader: OwnedFd = reader.into();
    let mut launcher = Command::new("sh");
    launcher
        .args([
            "-c",
            "IFS= read -r release && test \"$release\" = run || exit 125; exec \"$@\" </dev/null",
            "native-owner-startup",
        ])
        .arg(command.get_program())
        .args(command.get_args())
        .stdin(Stdio::from(reader))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = command.get_current_dir() {
        launcher.current_dir(directory);
    }
    for (key, value) in command.get_envs() {
        match value {
            Some(value) => launcher.env(key, value),
            None => launcher.env_remove(key),
        };
    }
    Ok((launcher, writer))
}

#[cfg(unix)]
fn spawn_parent_death_watchdog(process_group: u32) -> io::Result<(UnixStream, Child)> {
    let (reader, mut writer) = UnixStream::pair()?;
    let acknowledgement: OwnedFd = reader.try_clone()?.into();
    let reader: OwnedFd = reader.into();
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("printf armed; IFS= read -r _ || true; exec /bin/kill -KILL -- \"-$1\"")
        .arg("native-owner-watchdog")
        .arg(process_group.to_string())
        .stdin(Stdio::from(reader))
        .stdout(Stdio::from(acknowledgement))
        .stderr(Stdio::null());
    configure_process_group(&mut command);
    let mut watchdog = command.spawn()?;
    let mut acknowledgement = [0; 5];
    let acknowledged = writer
        .set_read_timeout(Some(Duration::from_secs(30)))
        .and_then(|()| writer.read_exact(&mut acknowledgement));
    if let Err(error) = acknowledged {
        drop(writer);
        let _ = wait_for_watchdog(&mut watchdog);
        return Err(error);
    }
    if &acknowledgement != b"armed" {
        drop(writer);
        let _ = wait_for_watchdog(&mut watchdog);
        return Err(io::Error::other(
            "parent-death watchdog did not acknowledge supervision",
        ));
    }
    Ok((writer, watchdog))
}

#[cfg(unix)]
fn wait_for_watchdog(watchdog: &mut Child) -> io::Result<()> {
    let deadline = Instant::now() + WATCHDOG_JOIN_GRACE;
    while Instant::now() < deadline {
        if watchdog.try_wait()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    let _ = watchdog.kill();
    watchdog.wait()?;
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "native owner parent-death watchdog did not exit after EOF",
    ))
}

#[cfg(windows)]
fn create_kill_on_close_job(child: &Child) -> io::Result<OwnedHandle> {
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    // SAFETY: every handle and pointer is either returned by Windows, borrowed
    // from the live Child, or points to an initialized structure for the call.
    unsafe {
        let raw_job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if raw_job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = OwnedHandle::from_raw_handle(raw_job);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            raw_job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&limits).cast(),
            std::mem::size_of_val(&limits) as u32,
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        if AssignProcessToJobObject(raw_job, child.as_raw_handle()) == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }
}

#[cfg(windows)]
fn resume_contained_child(child: &Child) -> io::Result<()> {
    use windows_sys::Win32::System::Threading::ResumeThread;
    // SAFETY: the thread handle is borrowed from the live suspended Child.
    let previous_suspend_count =
        unsafe { ResumeThread(child.main_thread_handle().as_raw_handle()) };
    if previous_suspend_count == u32::MAX {
        return Err(io::Error::last_os_error());
    }
    if previous_suspend_count != 1 {
        return Err(io::Error::other(format!(
            "native owner process had unexpected suspend count {previous_suspend_count}"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn configure_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED};

    // Suspend before any user code runs so the process can be assigned to its
    // kill-on-close job atomically with respect to descendant creation.
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_SUSPENDED);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_tree(process_id: u32, force: bool) {
    let signal = if force { "-KILL" } else { "-TERM" };
    let _ = Command::new("kill")
        .arg(signal)
        .arg(format!("-{process_id}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(windows)]
fn terminate_process_tree(process_id: u32, _force: bool) {
    let _ = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(any(unix, windows)))]
fn terminate_process_tree(_process_id: u32, _force: bool) {}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("mech-build lives at <workspace>/src/build")
        .to_path_buf()
}
