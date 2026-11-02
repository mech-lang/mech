//! Linked, generated-resident CLI execution. This does not qualify Cranelift lowering.
//!
//! Run the named qualification test explicitly from a clean committed checkout.
//! Set MECH_CANONICAL_CLI_SOURCE_COMMIT to its HEAD and
//! MECH_CANONICAL_CLI_EVIDENCE_DIR to a new absolute directory outside it.
//! Three binaries share only that evidence directory's dedicated Cargo target.
#![cfg(all(feature = "standard-hosts", feature = "process-supervision"))]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use mech_build::{
    CargoInvocation, NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile,
    NativeBuildRequest, NativeDependencySource, NativeEmit, NativeRuntimeConfig,
    parse_cargo_build_messages, process::ContainedChild, selected_planning_host_factory,
    standard_native_host_catalog,
};
use mech_core::ParsedProgram;
use mech_runtime::{
    ConfigValue, HostInstanceConfig, ModuleBuildOptions, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeConfig, SourceRequest, resolver::InMemorySourceResolver,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const NUMBER_SOURCE: &str = "+> number\n+> @args := cli/args\n+> @out := cli/stdout\nn := number/parse-u64(@args/1)\n@out/text <- number/format(n, 0u64)\n";
const ARGUMENT_SOURCE: &str = "+> @args := cli/args\n+> @out := cli/stdout\n@out/text <- @args/1\n@out/text <- \"|\"\n@out/text <- @args/2\n";
const STDIN_SOURCE: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\na := @in/text\nb := @in/text\n@out/text <- a\n@out/text <- \"|\"\n@out/text <- b\n";

const PROCESS_DEADLINE: Duration = Duration::from_secs(15);
const BUILD_DEADLINE: Duration = Duration::from_secs(20 * 60);

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn file_digest(path: &Path) -> String {
    digest(&fs::read(path).unwrap())
}

struct Captured {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// File-backed streams avoid pipe deadlocks; the owned group/job also contains
/// Cargo descendants. Every wait and timeout reap has a bounded deadline.
fn bounded(
    mut command: Command,
    root: &Path,
    label: &str,
    stdin: &[u8],
    deadline: Duration,
) -> Captured {
    let input = root.join(format!("{label}.stdin"));
    let output = root.join(format!("{label}.stdout"));
    let errors = root.join(format!("{label}.stderr"));
    fs::write(&input, stdin).unwrap();
    command
        .stdin(Stdio::from(File::open(input).unwrap()))
        .stdout(Stdio::from(File::create(&output).unwrap()))
        .stderr(Stdio::from(File::create(&errors).unwrap()));
    let mut owned = ContainedChild::spawn(&mut command).unwrap();
    let end = Instant::now() + deadline;
    let status = loop {
        if let Some(status) = owned.child_mut().try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= end {
            owned.terminate_tree().unwrap();
            let reap_end = Instant::now() + Duration::from_secs(5);
            while owned.child_mut().try_wait().unwrap().is_none() && Instant::now() < reap_end {
                thread::sleep(Duration::from_millis(20));
            }
            panic!(
                "{label} exceeded its deadline; diagnostics: {}",
                errors.display()
            );
        }
        thread::sleep(Duration::from_millis(20));
    };
    owned.terminate_tree().unwrap();
    Captured {
        status,
        stdout: fs::read(output).unwrap(),
        stderr: fs::read(errors).unwrap(),
    }
}

fn assert_clean_commit(workspace: &Path, evidence: &Path, expected: &str, label: &str) {
    let mut head = Command::new("git");
    head.args(["rev-parse", "HEAD"]).current_dir(workspace);
    let head = bounded(
        head,
        evidence,
        &format!("{label}-head"),
        &[],
        PROCESS_DEADLINE,
    );
    assert!(head.status.success());
    assert_eq!(std::str::from_utf8(&head.stdout).unwrap().trim(), expected);
    let mut status = Command::new("git");
    status
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(workspace);
    let status = bounded(
        status,
        evidence,
        &format!("{label}-status"),
        &[],
        PROCESS_DEADLINE,
    );
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "qualification requires a clean immutable checkout"
    );
}

fn configuration() -> NativeRuntimeConfig {
    NativeRuntimeConfig {
        runtime: RuntimeConfig::new("canonical-cli-qualification"),
        actor_bootstrap: None,
        hosts: vec![HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(BTreeMap::new()),
        }],
        run_grants: [
            ("args", "read", vec!["1", "2"]),
            ("stdin", "read", vec!["text"]),
            ("stdout", "write", vec!["text"]),
        ]
        .into_iter()
        .map(|(context, operation, paths)| RunResourceGrantConfig {
            target: format!("cli/{context}"),
            operations: vec![operation.into()],
            paths: paths.into_iter().map(str::to_owned).collect(),
        })
        .collect(),
    }
}

struct Application {
    executable: PathBuf,
    root: PathBuf,
    pins: BTreeMap<PathBuf, String>,
    receipt: Value,
    cases: Vec<Value>,
}

impl Application {
    fn assert_pins(&self) {
        for (path, expected) in &self.pins {
            assert_eq!(
                &file_digest(path),
                expected,
                "changed qualification input {}",
                path.display()
            );
        }
    }

    fn launch(
        &mut self,
        name: &str,
        arguments: &[OsString],
        stdin: &[u8],
        expected: Result<&[u8], &str>,
        runtime_info: bool,
    ) {
        self.assert_pins();
        let oracle = expected.as_ref().copied().unwrap_or(&[]);
        fs::write(self.root.join(format!("{name}.expected.stdout")), oracle).unwrap();
        let mut command = Command::new(&self.executable);
        command.args(arguments);
        let output = bounded(command, &self.root, name, stdin, PROCESS_DEADLINE);
        match expected {
            Ok(expected) => {
                assert!(
                    output.status.success(),
                    "{name}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(output.stderr.is_empty(), "{name}: unexpected stderr");
                if runtime_info {
                    let remainder = output
                        .stdout
                        .strip_prefix(expected)
                        .expect("exact application output prefix");
                    let info = remainder
                        .strip_prefix(b"MECH_RUNTIME_INFO ")
                        .expect("launcher information follows application output");
                    let info: Value = serde_json::from_slice(info).unwrap();
                    assert_eq!(info["route"], "resident-external");
                    assert_eq!(info["program_revision"], self.receipt["artifact_revision"]);
                    assert_eq!(info["resident_rejected_turns"], 0);
                    assert!(info["resident_accepted_turns"].as_u64().unwrap() >= 1);
                } else {
                    assert_eq!(output.stdout, expected, "{name}");
                }
            }
            Err(expected_diagnostic) => {
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{name}: launcher must report a program/provider error"
                );
                assert!(
                    output.stdout.is_empty(),
                    "{name}: failed turn published output"
                );
                assert!(!output.stderr.is_empty());
                let diagnostic = String::from_utf8_lossy(&output.stderr);
                assert!(
                    diagnostic.contains(expected_diagnostic),
                    "unexpected program error: {diagnostic}"
                );
                assert!(!diagnostic.contains("panicked"), "{diagnostic}");
                assert!(
                    !diagnostic.contains("usage: generated-app"),
                    "payload was rejected by launcher"
                );
            }
        }
        self.assert_pins();
        self.cases.push(json!({
            "case": name,
            "arguments_os_encoded_hex": arguments.iter().map(|arg| arg.as_encoded_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>()).collect::<Vec<_>>(),
            "stdin_sha256": digest(stdin), "exit_code": output.status.code(),
            "expected_stdout_sha256": digest(oracle), "expected_diagnostic_contains": expected.err(),
            "stdout_sha256": digest(&output.stdout), "stderr_sha256": digest(&output.stderr),
            "stdout_validation": if runtime_info { "exact-application-prefix-and-resident-runtime-info" } else { "exact-complete-bytes" },
            "success_expected": expected.is_ok(), "binary_unchanged": true,
        }));
    }

    fn finish(mut self) -> Value {
        self.assert_pins();
        self.receipt["cases"] = json!(self.cases);
        self.receipt
    }
}

fn build_application_worker(
    workspace: &Path,
    evidence: &Path,
    nested_target: &Path,
    name: &str,
    source: &str,
) -> Application {
    let root = evidence.join(name);
    let source_path = root.join("application.mec");
    fs::write(&source_path, source).unwrap();
    let config = configuration();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(InMemorySourceResolver::new().with_string("application.mec", source))
        .host_factory(selected_planning_host_factory("cli").unwrap())
        .unwrap()
        .host_instance(config.hosts[0].clone());
    for grant in &config.run_grants {
        compiler = compiler.run_resource_grant(grant.clone());
    }
    let product = compiler
        .build_compiler()
        .unwrap()
        .compile_canonical_roots(
            &[SourceRequest::new("application.mec")],
            ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]),
        )
        .unwrap();
    let parsed = ParsedProgram::from_bytes(product.bytecode()).unwrap();
    assert!(!parsed.artifact.is_empty());
    assert!(parsed.instructions.is_empty());
    assert_eq!(parsed.header.instruction_count, 0);
    assert_eq!(parsed.header.register_count, 0);
    let request = NativeBuildRequest {
        bytecode: product.bytecode().to_vec(),
        instruction_type_bindings: None,
        instruction_type_binding_requirements: None,
        runtime_config: Some(config),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: name.into(),
        output: root.join("unused"),
        emit: NativeEmit::CargoProject,
        keep_project: false,
        offline: true,
    };
    let builder = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::source_native_plan_catalog(),
        host_catalog: standard_native_host_catalog().unwrap(),
        dependency_source: NativeDependencySource::Workspace {
            root: workspace.into(),
        },
    });
    let plan = builder.plan(&request).unwrap();
    assert!(plan.live, "driverless observation delivery remains live");
    assert!(
        plan.runtime_functions.is_empty(),
        "canonical artifact, not legacy instruction factories"
    );
    let project = builder
        .generate_at(&request, &plan, root.join("project"))
        .unwrap();
    let mut command = CargoInvocation::build(
        project.manifest_path(),
        name,
        None,
        nested_target,
        false,
        true,
    )
    .unwrap()
    .command();
    command
        .current_dir(project.root.ancestors().last().unwrap())
        .env("CARGO_BUILD_JOBS", "2")
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0");
    // The parent qualification process owns this entire worker group, including
    // synchronous lockfile metadata and Cargo's descendants.
    let built = command.output().unwrap();
    fs::write(root.join("build.stdout"), &built.stdout).unwrap();
    fs::write(root.join("build.stderr"), &built.stderr).unwrap();
    assert!(
        built.status.success(),
        "generated build: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
    assert_eq!(messages.build_finished, Some(true));
    let executable = messages.select_binary(name).unwrap().executable;
    let mut pins = BTreeMap::from([
        (source_path, digest(source.as_bytes())),
        (executable.clone(), file_digest(&executable)),
    ]);
    for relative in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "build-plan.json",
        "program.mecb",
        "src/main.rs",
        "src/catalog.rs",
        "src/runtime.rs",
    ] {
        let path = project.root.join(relative);
        pins.insert(path.clone(), file_digest(&path));
    }
    let receipt = json!({
        "name": name, "source_sha256": digest(source.as_bytes()),
        "canonical_artifact": true, "legacy_instructions": 0, "build_count": 1,
        "artifact_revision": product.artifact().revision().as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "bytecode_sha256": digest(product.bytecode()), "plan_sha256": plan.plan_sha256,
        "workspace_fingerprint": plan.workspace_fingerprint,
        "binary": executable, "binary_sha256": file_digest(&executable),
        "generated_and_source_pins": pins,
    });
    Application {
        executable,
        root,
        pins,
        receipt,
        cases: Vec::new(),
    }
}

// A worker keeps the maintained synchronous generation API inside the same
// bounded group/job as Cargo. It is never an application execution route.
#[test]
#[ignore = "internal bounded build worker for linked_canonical_cli_arguments_and_stdin_finish_without_drivers"]
fn canonical_cli_build_worker() {
    let name = std::env::var("MECH_CANONICAL_CLI_WORKER_NAME").unwrap();
    let evidence = PathBuf::from(std::env::var_os("MECH_CANONICAL_CLI_EVIDENCE_DIR").unwrap());
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let source = fs::read_to_string(evidence.join(&name).join("application.mec")).unwrap();
    let application = build_application_worker(
        workspace,
        &evidence,
        &evidence.join("cargo-target"),
        &name,
        &source,
    );
    fs::write(application.root.join("application-build.json"), serde_json::to_vec_pretty(&json!({
        "executable": application.executable, "pins": application.pins, "receipt": application.receipt,
    })).unwrap()).unwrap();
}

fn build_application(evidence: &Path, name: &str, source: &str) -> Application {
    let root = evidence.join(name);
    fs::create_dir(&root).unwrap();
    fs::write(root.join("application.mec"), source).unwrap();
    let mut worker = Command::new(std::env::current_exe().unwrap());
    worker
        .args([
            "--ignored",
            "--exact",
            "canonical_cli_build_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MECH_CANONICAL_CLI_WORKER_NAME", name);
    let built = bounded(worker, &root, "build-worker", &[], BUILD_DEADLINE);
    assert!(
        built.status.success(),
        "build worker failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let build_evidence = root.join("application-build.json");
    let mut built: Value = serde_json::from_slice(&fs::read(&build_evidence).unwrap()).unwrap();
    let executable = serde_json::from_value(built["executable"].take()).unwrap();
    let mut pins: BTreeMap<PathBuf, String> = serde_json::from_value(built["pins"].take()).unwrap();
    pins.insert(build_evidence.clone(), file_digest(&build_evidence));
    let mut receipt = built["receipt"].take();
    assert_eq!(receipt["source_sha256"], digest(source.as_bytes()));
    receipt["build_evidence"] =
        json!({"path": build_evidence, "sha256": file_digest(&build_evidence)});
    Application {
        executable,
        root,
        pins,
        receipt,
        cases: Vec::new(),
    }
}

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(|value| OsString::from(*value)).collect()
}

#[test]
#[ignore = "explicit linked generated-resident qualification; requires a clean pinned checkout and separate nested Cargo cache"]
fn linked_canonical_cli_arguments_and_stdin_finish_without_drivers() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let commit =
        std::env::var("MECH_CANONICAL_CLI_SOURCE_COMMIT").expect("pin the immutable source commit");
    assert_eq!(commit.len(), 40);
    assert!(commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_CLI_EVIDENCE_DIR")
            .expect("set a new retained evidence directory"),
    );
    assert!(evidence.is_absolute());
    assert!(
        !evidence.starts_with(&workspace),
        "retain generated evidence outside the immutable checkout"
    );
    fs::create_dir(&evidence).unwrap();
    assert_clean_commit(&workspace, &evidence, &commit, "before");

    let mut number = build_application(&evidence, "canonical_cli_number", NUMBER_SOURCE);
    number.launch("default", &args(&["19"]), &[], Ok(b"19"), false);
    number.launch("once", &args(&["--once", "19"]), &[], Ok(b"19"), false);
    number.launch(
        "explicit-result-display",
        &args(&["--print-result", "19"]),
        &[],
        Ok(b"1919\n"),
        false,
    );
    number.launch(
        "once-explicit-result-display",
        &args(&["--once", "--print-result", "19"]),
        &[],
        Ok(b"1919\n"),
        false,
    );
    number.launch(
        "launcher-controls",
        &args(&["--once", "--max-live-turns", "1", "--runtime-info", "19"]),
        &[],
        Ok(b"19"),
        true,
    );
    number.launch(
        "invalid-number",
        &args(&["not-a-number"]),
        &[],
        // Resident kernels expose their fixed error category rather than a
        // source function name. The launcher must still reject atomically.
        Err("error: InvalidInput"),
        false,
    );

    let mut echo = build_application(&evidence, "canonical_cli_arguments", ARGUMENT_SOURCE);
    echo.launch(
        "payload-stops-controls",
        &args(&["19", "--once"]),
        &[],
        Ok(b"19|--once"),
        false,
    );
    echo.launch(
        "once-payload-tail",
        &args(&["--once", "19", "--max-live-turns"]),
        &[],
        Ok(b"19|--max-live-turns"),
        false,
    );
    echo.launch(
        "explicit-boundary",
        &args(&["--", "--once", "--runtime-info"]),
        &[],
        Ok(b"--once|--runtime-info"),
        false,
    );
    echo.launch(
        "empty-argument",
        &args(&["", "tail"]),
        &[],
        Ok(b"|tail"),
        false,
    );
    echo.launch(
        "unicode",
        &args(&["e\u{301} 👋", "\r\n"]),
        &[],
        Ok("e\u{301} 👋|\r\n".as_bytes()),
        false,
    );
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        echo.launch(
            "invalid-os-argument",
            &[OsString::from_vec(vec![0xff]), "tail".into()],
            &[],
            Err("argument 1 is not valid Unicode"),
            false,
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        echo.launch(
            "invalid-os-argument",
            &[OsString::from_wide(&[0xd800]), "tail".into()],
            &[],
            Err("argument 1 is not valid Unicode"),
            false,
        );
    }

    echo.launch(
        "print-control-after-payload",
        &args(&["19", "--print-result"]),
        &[],
        Ok(b"19|--print-result"),
        false,
    );
    echo.launch(
        "print-control-after-boundary",
        &args(&["--", "--print-result", "tail"]),
        &[],
        Ok(b"--print-result|tail"),
        false,
    );

    let mut input = build_application(&evidence, "canonical_cli_stdin", STDIN_SOURCE);
    for (label, payload) in [
        ("unicode", "héllo e\u{301} 👋\n\nlast\r\n".as_bytes()),
        ("empty", b"".as_slice()),
    ] {
        let expected = [payload, b"|", payload].concat();
        for once in [false, true] {
            input.launch(
                &format!("{label}-{}", if once { "once" } else { "default" }),
                &args(if once { &["--once"] } else { &[] }),
                payload,
                Ok(&expected),
                false,
            );
        }
    }
    for once in [false, true] {
        input.launch(
            &format!("invalid-utf8-{}", if once { "once" } else { "default" }),
            &args(if once { &["--once"] } else { &[] }),
            &[0xff],
            Err("UTF-8"),
            false,
        );
    }
    assert_clean_commit(&workspace, &evidence, &commit, "after");
    let receipt = json!({
        "schema": "mech.generated-resident-cli-qualification.v1", "source_commit": commit,
        "source_checkout_clean_before_and_after": true, "execution_route": "resident-external",
        "cranelift_lowering_qualified": false, "benchmark_measurements": 0,
        "builds": [number.finish(), echo.finish(), input.finish()],
    });
    fs::write(
        evidence.join("qualification.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
