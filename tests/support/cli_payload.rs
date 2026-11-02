use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

const COUNT: &str = "describe(count<u64>) => <string>\n  | 0u64 => \"argc-0\"\n  | 1u64 => \"argc-1\"\n  | 2u64 => \"argc-2\".\n+> @args := cli/args\n+> @out := cli/stdout\n@out/line <- describe(@args/count)\n0u64\n";
const ECHO: &str = "+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- \"[\"\n@out/text <- @args/1\n@out/text <- \"]\\n\"\n0u64\n";
const STRICT: &str = "accept-one(count<u64>) => <string>\n  | 1u64 => \"strict-accepted\".\n+> @args := cli/args\n+> @out := cli/stdout\n@out/line <- \"must-not-write-payload\"\n@out/line <- accept-one(@args/count)\n0u64\n";

#[derive(Clone, Copy)]
enum Form {
    BarePath,
    RunPath,
    Inline,
}

const FORMS: &[Form] = &[Form::BarePath, Form::RunPath, Form::Inline];

struct Fixture {
    root: PathBuf,
    path: PathBuf,
    source: String,
}

impl Fixture {
    fn new(source: &str) -> Self {
        let root = super::temp_root("payload");
        let path = root.join("payload.mec");
        std::fs::write(&path, source).unwrap();
        Self {
            root,
            path,
            source: source.into(),
        }
    }

    fn invoke(
        &self,
        form: Form,
        payload: &[OsString],
        grant_args: bool,
        separator: bool,
    ) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
        command.current_dir(&self.root);
        if !matches!(form, Form::BarePath) {
            command.arg("run");
        }
        command.args([
            "--no-config",
            "--deny-default-capabilities",
            "--capabilities",
            ":cli/stdout",
        ]);
        if grant_args {
            command.args(["--capabilities", ":cli/args"]);
        }
        if matches!(form, Form::Inline) {
            command.arg(&self.source);
        } else {
            command.arg("--allow-read").arg(&self.path).arg(&self.path);
        }
        if separator {
            command.arg("--");
        }
        command
            .args(payload)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if child.try_wait().unwrap().is_some() {
                return child.wait_with_output().unwrap();
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();
                panic!("source CLI payload command exceeded30s: {output:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn successful(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn source_launchers_exclude_executable_controls_and_source_from_an_empty_payload() {
    let fixture = Fixture::new(COUNT);
    for &form in FORMS {
        for separator in [false, true] {
            let stdout = successful(fixture.invoke(form, &[], true, separator));
            assert_eq!(
                stdout.lines().filter(|line| *line == "argc-0").count(),
                1,
                "{stdout}"
            );
        }
    }
}

#[test]
fn source_payload_flags_are_literal_values_and_do_not_select_launcher_options() {
    let fixture = Fixture::new(COUNT);
    for &form in FORMS {
        let payload = [
            OsString::from("--runtime-info"),
            OsString::from("--backend"),
        ];
        let stdout = successful(fixture.invoke(form, &payload, true, true));
        assert_eq!(
            stdout.lines().filter(|line| *line == "argc-2").count(),
            1,
            "{stdout}"
        );
        assert!(
            !stdout.contains("resident_accepted_turns"),
            "payload enabled runtime diagnostics: {stdout}"
        );
    }
    let echo = Fixture::new(ECHO);
    for &form in FORMS {
        for payload in ["", "héllo", "--", "--cap-root"] {
            let stdout = successful(echo.invoke(form, &[OsString::from(payload)], true, true));
            assert!(
                stdout.lines().any(|line| line == format!("[{payload}]")),
                "{stdout}"
            );
        }
    }
}

#[test]
fn payload_capability_spelling_cannot_authorize_its_own_count_read() {
    let fixture = Fixture::new(COUNT);
    let payload = [
        OsString::from("--capabilities"),
        OsString::from(":cli/args"),
    ];
    for &form in FORMS {
        let output = fixture.invoke(form, &payload, false, true);
        assert!(!output.status.success());
        let combined = super::combined_output(&output);
        assert!(combined.contains("CapabilityDenied"), "{combined}");
        assert!(
            !String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.starts_with("argc-")),
            "{combined}"
        );
    }
}

#[test]
fn real_source_launcher_strict_arity_refuses_without_delivering_earlier_sends() {
    let fixture = Fixture::new(STRICT);
    for &form in FORMS {
        for payload in [
            Vec::new(),
            vec![OsString::from("one"), OsString::from("extra")],
        ] {
            let output = fixture.invoke(form, &payload, true, true);
            assert!(!output.status.success());
            let combined = super::combined_output(&output);
            assert!(combined.contains("InvalidInput"), "{combined}");
            assert!(
                !String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line == "must-not-write-payload"),
                "{combined}"
            );
        }
        let stdout = successful(fixture.invoke(form, &[OsString::from("one")], true, true));
        assert_eq!(
            stdout
                .lines()
                .filter(|line| *line == "strict-accepted")
                .count(),
            1,
            "{stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn source_count_keeps_non_unicode_payload_until_a_requested_index_read() {
    use std::os::unix::ffi::OsStringExt;
    let payload = [OsString::from_vec(vec![0xff, 0x80])];
    let count = Fixture::new(COUNT);
    let echo = Fixture::new(ECHO);
    for &form in FORMS {
        let stdout = successful(count.invoke(form, &payload, true, true));
        assert_eq!(
            stdout.lines().filter(|line| *line == "argc-1").count(),
            1,
            "{stdout}"
        );
        let output = echo.invoke(form, &payload, true, true);
        assert!(!output.status.success());
        let combined = super::combined_output(&output);
        assert!(combined.contains("Unicode"), "{combined}");
        assert!(
            !String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line == "["),
            "{combined}"
        );
    }
}
