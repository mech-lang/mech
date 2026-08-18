use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn resident_repl_starts_without_arguments_and_with_the_compatibility_flag() {
    assert_repl_session(&[]);
    assert_repl_session(&["--repl"]);
}

#[test]
fn resident_repl_multiplies_dynamic_f64_matrices() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mech"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the Mech CLI");

    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(b"[1 2 3] ** [4 5 6]'\n:quit\n")
        .expect("multiply matrices and exit REPL");

    let output = child.wait_with_output().expect("wait for Mech CLI");
    assert!(output.status.success(), "stderr: {:?}", output.stderr);

    let stdout = String::from_utf8(output.stdout).expect("UTF-8 REPL output");
    assert!(
        stdout.contains("32"),
        "missing matrix product from REPL output: {stdout}"
    );
    assert!(
        !stdout.contains("ResidentRouteFailure"),
        "matrix multiplication failed resident routing: {stdout}"
    );
}

fn assert_repl_session(arguments: &[&str]) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mech"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the Mech CLI");

    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(b"1 + 1\n[1 1 2]\n:whos\n:plan\n:quit\n")
        .expect("exercise and exit REPL");

    let output = child.wait_with_output().expect("wait for Mech CLI");
    assert!(output.status.success(), "stderr: {:?}", output.stderr);

    let stdout = String::from_utf8(output.stdout).expect("UTF-8 REPL output");
    assert!(
        stdout.contains("www.mech-lang.org"),
        "missing REPL banner: {stdout}"
    );
    assert!(
        stdout.contains("Okay cya!"),
        "missing REPL farewell: {stdout}"
    );
    assert!(stdout.contains("f64"), "missing scalar type: {stdout}");
    assert!(
        stdout.contains("\x1b[38;5;218m"),
        "value kinds lost the REPL's pink ANSI style: {stdout:?}"
    );
    assert!(
        stdout.contains("[f64]:1,3"),
        "missing matrix type: {stdout}"
    );
    assert!(
        stdout.contains("resident plan:"),
        "missing resident plan: {stdout}"
    );
}
