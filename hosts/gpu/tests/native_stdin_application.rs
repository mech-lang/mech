//! One linked activated-stdin executable; correctness qualification, not timings.
#![cfg(all(feature = "native-build", feature = "native-io"))]

use mech_gpu::{
    native_build,
    sequence::{NativeIoGrant, NativeResourceIntent, NativeResourceRequirement, compile},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

const SOURCE: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\nfirst := @in/text\nsecond := @in/text\n@out/text <- \"first:\\n\" + first + \"\\nsecond:\\n\" + second + \"\\n\"\n";
const INPUT: &[u8] = "e\u{301} 👩🏽‍💻 🇺🇸\r\nsecond line\nlast".as_bytes();
const INVALID_UTF8: &[u8] = &[b'A', b'\n', 0xf0, 0x28, 0x8c, 0x28];
const EXECUTION_ENVIRONMENT: [(&str, &str); 7] = [
    ("MECH_SEQUENCE_PARTITIONS", "1"),
    ("MECH_ARRAY_GRAIN", "16"),
    ("MECH_SEQUENCE_NARROW", "1"),
    ("MECH_SEQUENCE_STD_MAP", "1"),
    ("MECH_SEQUENCE_ASYNC", "0"),
    ("MECH_SEQUENCE_BUFFER_REUSE", "0"),
    ("RAYON_NUM_THREADS", "2"),
];

fn bytes_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest(path: &Path) -> String {
    bytes_digest(&fs::read(path).unwrap())
}

fn git(workspace: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "git metadata: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn source_commit(workspace: &Path) -> String {
    assert!(
        git(
            workspace,
            &["status", "--porcelain", "--untracked-files=no"]
        )
        .is_empty(),
        "qualify an immutable source commit, with no tracked changes"
    );
    let untracked = git(workspace, &["ls-files", "--others", "--exclude-standard"]);
    assert!(
        untracked
            .lines()
            .all(|path| path == "hosts/gpu/tests/native_stdin_application.rs"),
        "only this externally reviewed qualification test may be untracked: {untracked}"
    );
    git(workspace, &["rev-parse", "HEAD"])
}

fn expected(input: &[u8]) -> Vec<u8> {
    [b"first:\n".as_slice(), input, b"\nsecond:\n", input, b"\n"].concat()
}

fn run_with_stdin(command: &mut Command, input: &[u8]) -> (Output, Option<std::io::Error>, bool) {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let write_error = stdin.write_all(input).err();
    // The provider consumes the complete stream. EOF must precede waiting;
    // reading the same source port again must reuse the first snapshot.
    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut timed_out = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            timed_out = true;
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    (child.wait_with_output().unwrap(), write_error, timed_out)
}

#[test]
#[ignore = "links one isolated generated Cargo application; run explicitly for activated stdin qualification"]
fn one_linked_stdin_executable_preserves_unicode_snapshots_and_launch_policy() {
    let retained = std::env::var_os("MECH_NATIVE_STDIN_QUALIFICATION_DIRECTORY");
    let root = retained.clone().map(PathBuf::from).unwrap_or_else(|| {
        std::env::temp_dir().join(format!(
            "mech-native-stdin-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    });
    fs::create_dir(&root).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let commit = source_commit(&workspace);
    let workspace_lock = digest(&workspace.join("Cargo.lock"));
    let source = root.join("application.mec");
    fs::write(&source, SOURCE).unwrap();

    let program = compile(SOURCE, 4096).unwrap();
    let raw_read_nodes = program.nodes.iter().filter(|node| node.op == 81).count();
    let raw_send_nodes = program.nodes.iter().filter(|node| node.op == 83).count();
    assert!(
        program
            .nodes
            .iter()
            .all(|node| { !matches!(node.op, 1 | 8 | 9 | 10 | 12 | 32 | 41 | 45 | 52) })
    );
    let requirements = program.resource_requirements().unwrap();
    let expected_requirements = vec![
        NativeResourceRequirement {
            base_uri: "cli://cli/stdin".into(),
            context_name: "stdin".into(),
            path: "text".into(),
            intent: NativeResourceIntent::Read,
        },
        NativeResourceRequirement {
            base_uri: "cli://cli/stdout".into(),
            context_name: "stdout".into(),
            path: "text".into(),
            intent: NativeResourceIntent::Send,
        },
    ];
    assert_eq!(requirements, expected_requirements);

    let name = "native_stdin_qualification";
    let application = root.join("generated");
    let kind = native_build::emit_application(&source, &application, &workspace, name, true, false)
        .unwrap();
    assert_eq!(kind, native_build::ApplicationKind::Sequence);
    assert_eq!(
        fs::read(application.join("source.mec")).unwrap(),
        SOURCE.as_bytes()
    );
    let emitted_requirements: Vec<NativeResourceRequirement> =
        serde_json::from_slice(&fs::read(application.join("resource-requirements.json")).unwrap())
            .unwrap();
    assert_eq!(emitted_requirements, expected_requirements);
    assert!(
        fs::metadata(application.join("mech-sequence.o"))
            .unwrap()
            .len()
            > 0
    );

    let executable = root.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    native_build::build_application(kind, &application, &executable, name, false, true).unwrap();
    let binary_sha = digest(&executable);
    let artifacts = [
        "source.mec",
        "mech-sequence.o",
        "constants.json",
        "resource-requirements.json",
        "src/main.rs",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
    ]
    .map(|path| (path.to_owned(), digest(&application.join(path))))
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let grants = vec![
        NativeIoGrant {
            target: "cli/stdin".into(),
            operations: vec!["read".into()],
            paths: vec!["text".into()],
        },
        NativeIoGrant {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["text".into()],
        },
    ];
    let grant_file = root.join("grants.json");
    fs::write(&grant_file, serde_json::to_vec_pretty(&grants).unwrap()).unwrap();
    let grant_sha = digest(&grant_file);
    let mut attempts = Vec::new();
    for (label, policy, input, success) in [
        ("trusted-unicode", "trusted", INPUT, true),
        ("secure-unicode", "secure", INPUT, true),
        ("trusted-empty", "trusted", b"".as_slice(), true),
        ("secure-empty", "secure", b"".as_slice(), true),
        ("default-denied", "default", INPUT, false),
        ("invalid-policy", "invalid", INPUT, false),
        ("trusted-invalid-utf8", "trusted", INVALID_UTF8, false),
        ("secure-invalid-utf8", "secure", INVALID_UTF8, false),
    ] {
        assert_eq!(digest(&executable), binary_sha);
        let input_path = root.join(format!("{label}.stdin"));
        let expected_path = root.join(format!("{label}.expected.stdout"));
        fs::write(&input_path, input).unwrap();
        let oracle = if success { expected(input) } else { Vec::new() };
        fs::write(&expected_path, &oracle).unwrap();
        let mut command = Command::new(&executable);
        command
            .env_remove("MECH_NATIVE_SECURITY")
            .env_remove("MECH_NATIVE_GRANTS")
            .envs(EXECUTION_ENVIRONMENT);
        if policy != "default" {
            command.env("MECH_NATIVE_SECURITY", policy);
        }
        if policy == "secure" {
            command.env("MECH_NATIVE_GRANTS", &grant_file);
        }
        let (output, write_error, timed_out) = run_with_stdin(&mut command, input);
        let stdout = root.join(format!("{label}.stdout"));
        let stderr = root.join(format!("{label}.stderr"));
        fs::write(&stdout, &output.stdout).unwrap();
        fs::write(&stderr, &output.stderr).unwrap();
        attempts.push(serde_json::json!({
            "label": label, "policy": policy, "arguments": [],
            "exit_code": output.status.code(), "expected_success": success,
            "timed_out": timed_out,
            "stdin_pipe_write_error": write_error.as_ref().map(ToString::to_string),
            "stdin_sha256": digest(&input_path),
            "expected_stdout_sha256": digest(&expected_path),
            "stdout_sha256": digest(&stdout), "stderr_sha256": digest(&stderr),
            "stdout_validation": "exact-complete-bytes",
            "stderr_validation": if success { "empty" } else { "nonempty" },
            "binary_sha256_after": digest(&executable),
        }));
        fs::write(
            root.join("attempts.json"),
            serde_json::to_vec_pretty(&attempts).unwrap(),
        )
        .unwrap();
        assert!(
            !timed_out,
            "{label}: child exceeded its correctness timeout"
        );
        if let Some(error) = write_error {
            // Policy rejection can close stdin before the parent writes.
            assert!(
                !success && error.kind() == ErrorKind::BrokenPipe,
                "{label}: {error}"
            );
        }
        assert_eq!(output.status.success(), success, "{label}: {output:?}");
        assert_eq!(output.stdout, oracle, "{label}");
        assert_eq!(output.stderr.is_empty(), success, "{label}: {output:?}");
        assert_eq!(digest(&executable), binary_sha, "{label}: binary changed");
        assert_eq!(digest(&grant_file), grant_sha, "{label}: grants changed");
    }
    for (path, sha) in &artifacts {
        assert_eq!(
            digest(&application.join(path)),
            *sha,
            "artifact changed: {path}"
        );
    }
    assert_eq!(source_commit(&workspace), commit);
    assert_eq!(digest(&workspace.join("Cargo.lock")), workspace_lock);
    let test_source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/native_stdin_application.rs"
    ));
    let receipt = serde_json::json!({
        "schema": "native-stdin-application-qualification-v1",
        "application_kind": "experimental-sequence-cranelift",
        "source_commit": commit, "tracked_worktree_clean": true,
        "workspace_cargo_lock_sha256": workspace_lock,
        "qualification_test_sha256": bytes_digest(test_source.as_bytes()),
        "source_sha256": digest(&source), "generated_artifacts_sha256": artifacts,
        "grants_sha256": grant_sha, "binary_sha256": binary_sha,
        "emission_count": 1, "build_count": 1,
        "source_stdin_read_occurrences": 2,
        "raw_stdin_read_nodes": raw_read_nodes, "raw_send_nodes": raw_send_nodes,
        "raw_node_counts_are_diagnostic_only": true,
        "activated_legacy_io_absent": true,
        "same_executable_for_all_attempts": true,
        "execution_environment": EXECUTION_ENVIRONMENT.into_iter().collect::<BTreeMap<_, _>>(),
        "attempts": attempts, "benchmark_measurements": 0,
        "ordinary_program_artifact_to_cranelift_qualified": false,
    });
    fs::write(
        root.join("qualification.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    if retained.is_some() {
        println!("native stdin qualification: {}", root.display());
    } else {
        fs::remove_dir_all(root).unwrap();
    }
}
