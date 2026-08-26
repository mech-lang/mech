#![cfg(unix)]
#![cfg(any(feature = "distribution-standard", feature = "distribution-full"))]

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL: Duration = Duration::from_millis(20);
const TIMEOUT: Duration = Duration::from_secs(10);

fn temp_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("mech-repl-ctrlc-{}-{stamp}", std::process::id()));
    fs::create_dir(&path).expect("create test directory");
    path
}

fn output(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| format!("<read failed: {error}>"))
}

fn wait_for(child: &mut Child, stdout: &Path, description: &str, predicate: impl Fn(&str) -> bool) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let text = output(stdout);
        if predicate(&text) {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll REPL") {
            panic!("REPL exited with {status} before {description}:\n{text}");
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}:\n{text}"
        );
        thread::sleep(POLL);
    }
}

fn send_sigint(pid: u32) {
    let status = Command::new("kill")
        .args(["-INT", &pid.to_string()])
        .status()
        .expect("invoke the Unix kill command");
    assert!(status.success(), "failed to send SIGINT to PID {pid}");
}

#[test]
fn targetless_repl_exits_after_the_third_ctrl_c_without_stdin_input() {
    let directory = temp_dir();
    let stdout = directory.join("stdout");
    let stderr = directory.join("stderr");
    let mut child = Command::new(env!("CARGO_BIN_EXE_mech"))
        .stdin(Stdio::piped())
        .stdout(Stdio::from(File::create(&stdout).expect("create stdout")))
        .stderr(Stdio::from(File::create(&stderr).expect("create stderr")))
        .spawn()
        .expect("start targetless Mech REPL");
    let stdin = child.stdin.take().expect("retain REPL stdin");

    wait_for(&mut child, &stdout, "the REPL prompt", |text| {
        text.contains(">: ")
    });

    send_sigint(child.id());
    wait_for(&mut child, &stdout, "the first Ctrl-C warning", |text| {
        text.matches("to terminate this REPL session.").count() == 1
    });
    assert!(
        child.try_wait().unwrap().is_none(),
        "first Ctrl-C exited the REPL"
    );

    send_sigint(child.id());
    wait_for(&mut child, &stdout, "the second Ctrl-C warning", |text| {
        text.matches("to terminate this REPL session.").count() == 2
    });
    assert!(
        child.try_wait().unwrap().is_none(),
        "second Ctrl-C exited the REPL"
    );

    send_sigint(child.id());
    let status = child.wait().expect("wait for REPL exit");
    drop(stdin);

    let combined = format!("{}{}", output(&stdout), output(&stderr));
    assert!(status.success(), "REPL exit: {status}\n{combined}");
    assert_eq!(
        combined.matches("to terminate this REPL session.").count(),
        2,
        "{combined}"
    );
    assert_eq!(combined.matches(":ctrl+c").count(), 3, "{combined}");
    assert!(
        combined.contains("Okay cya!"),
        "missing farewell:\n{combined}"
    );
    fs::remove_dir_all(directory).expect("remove test directory");
}
