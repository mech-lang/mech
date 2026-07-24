#![cfg(all(unix, feature = "repl"))]

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL_INTERVAL: Duration = Duration::from_millis(20);

fn unique_temp_dir() -> PathBuf {
    let temp_root = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_nanos();

    for attempt in 0..100 {
        let path = temp_root.join(format!(
            "mech-repl-ctrlc-{}-{timestamp}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!(
                "failed to create temporary directory {}: {error}",
                path.display()
            ),
        }
    }

    panic!("failed to create a unique temporary directory")
}

fn captured_output(stdout_path: &Path, stderr_path: &Path) -> String {
    let stdout = fs::read_to_string(stdout_path)
        .unwrap_or_else(|error| format!("<failed to read stdout: {error}>"));
    let stderr = fs::read_to_string(stderr_path)
        .unwrap_or_else(|error| format!("<failed to read stderr: {error}>"));
    format!("stdout:\n{stdout}\nstderr:\n{stderr}")
}

fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn fail_with_output(
    child: &mut Child,
    stdout_path: &Path,
    stderr_path: &Path,
    message: impl std::fmt::Display,
) -> ! {
    kill_and_reap(child);
    panic!("{message}\n{}", captured_output(stdout_path, stderr_path),);
}

fn wait_for_stdout(
    child: &mut Child,
    stdout_path: &Path,
    stderr_path: &Path,
    timeout: Duration,
    description: &str,
    predicate: impl Fn(&str) -> bool,
) {
    let deadline = Instant::now() + timeout;

    loop {
        let stdout = match fs::read_to_string(stdout_path) {
            Ok(stdout) => stdout,
            Err(error) => fail_with_output(
                child,
                stdout_path,
                stderr_path,
                format!("failed to read stdout while waiting for {description}: {error}"),
            ),
        };
        if predicate(&stdout) {
            return;
        }

        match child.try_wait() {
            Ok(None) => {}
            Ok(Some(status)) => fail_with_output(
                child,
                stdout_path,
                stderr_path,
                format!("child exited with {status} while waiting for {description}"),
            ),
            Err(error) => fail_with_output(
                child,
                stdout_path,
                stderr_path,
                format!("failed to poll child while waiting for {description}: {error}"),
            ),
        }

        if Instant::now() >= deadline {
            fail_with_output(
                child,
                stdout_path,
                stderr_path,
                format!("timed out waiting for {description}"),
            );
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_exit(
    child: &mut Child,
    stdout_path: &Path,
    stderr_path: &Path,
    timeout: Duration,
) -> ExitStatus {
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) => {}
            Err(error) => fail_with_output(
                child,
                stdout_path,
                stderr_path,
                format!("failed to poll child while waiting for it to exit: {error}"),
            ),
        }

        if Instant::now() >= deadline {
            fail_with_output(
                child,
                stdout_path,
                stderr_path,
                "timed out waiting for child to exit",
            );
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn assert_still_running(child: &mut Child, stdout_path: &Path, stderr_path: &Path) {
    match child.try_wait() {
        Ok(None) => {}
        Ok(Some(status)) => fail_with_output(
            child,
            stdout_path,
            stderr_path,
            format!("child exited unexpectedly with {status}"),
        ),
        Err(error) => fail_with_output(
            child,
            stdout_path,
            stderr_path,
            format!("failed to poll child: {error}"),
        ),
    }
}

fn send_sigint(child: &mut Child, stdout_path: &Path, stderr_path: &Path) {
    let pid = child.id();
    // Safety: `pid` belongs to the live child process spawned by this test.
    let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };
    if result != 0 {
        fail_with_output(
            child,
            stdout_path,
            stderr_path,
            format!("failed to send SIGINT: {}", io::Error::last_os_error()),
        );
    }
}

#[test]
fn third_ctrl_c_exits_without_stdin_activity() {
    let temp_dir = unique_temp_dir();
    let stdout_path = temp_dir.join("stdout");
    let stderr_path = temp_dir.join("stderr");
    let stdout_file = File::create(&stdout_path).expect("failed to create stdout capture file");
    let stderr_file = File::create(&stderr_path).expect("failed to create stderr capture file");

    let mut child = Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("--repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .expect("failed to spawn mech REPL");
    let stdin: ChildStdin = child.stdin.take().expect("child stdin was not piped");

    wait_for_stdout(
        &mut child,
        &stdout_path,
        &stderr_path,
        Duration::from_secs(5),
        "the initial REPL prompt",
        |stdout| stdout.contains(">: "),
    );

    send_sigint(&mut child, &stdout_path, &stderr_path);
    wait_for_stdout(
        &mut child,
        &stdout_path,
        &stderr_path,
        Duration::from_secs(5),
        "the first Ctrl-C warning",
        |stdout| stdout.matches("to terminate this REPL session.").count() == 1,
    );
    assert_still_running(&mut child, &stdout_path, &stderr_path);

    send_sigint(&mut child, &stdout_path, &stderr_path);
    wait_for_stdout(
        &mut child,
        &stdout_path,
        &stderr_path,
        Duration::from_secs(5),
        "the second Ctrl-C warning",
        |stdout| stdout.matches("to terminate this REPL session.").count() == 2,
    );
    assert_still_running(&mut child, &stdout_path, &stderr_path);

    send_sigint(&mut child, &stdout_path, &stderr_path);
    let status = wait_for_exit(
        &mut child,
        &stdout_path,
        &stderr_path,
        Duration::from_secs(10),
    );
    drop(stdin);

    let stdout = fs::read_to_string(&stdout_path).expect("failed to read captured stdout");
    let stderr = fs::read_to_string(&stderr_path).expect("failed to read captured stderr");
    let combined_output = format!("stdout:\n{stdout}\nstderr:\n{stderr}");
    let warning_count = stdout.matches("to terminate this REPL session.").count();

    assert!(
        status.success(),
        "child did not exit successfully: {status}\n{combined_output}",
    );
    assert_eq!(
        status.code(),
        Some(0),
        "child did not exit with code 0\n{combined_output}",
    );
    assert_eq!(
        warning_count, 2,
        "expected exactly two Ctrl-C warnings\n{combined_output}",
    );

    #[cfg(not(feature = "mika"))]
    assert!(
        combined_output.contains("Okay cya!"),
        "expected the non-Mika farewell\n{combined_output}",
    );

    fs::remove_dir_all(temp_dir).expect("failed to remove temporary directory");
}
