//! Explicit clean-source qualification; full-input timing is separately opt-in.
use super::{EMPTY_OUTPUT, FULL, GGTA_INPUT, GGTA_OUTPUT, artifacts};
use mech_build::process::ContainedChild;
use mech_build::{CargoInvocation, parse_cargo_build_messages};
use mech_gpu::native_artifact::generate_linked_hosted_program_application;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};
#[path = "linked_process.rs"]
mod process;
use process::{assert_no_ambient_cargo_config, bounded, clean, closure, hash, pin, workspace};
#[cfg(feature = "canonical-native-benchmark-policy")]
#[path = "canonical_hosted_benchmark.rs"]
mod benchmark;
#[path = "canonical_hosted_launcher_tests.rs"]
mod launcher_tests;
#[path = "canonical_hosted_profile.rs"]
mod profile;
#[path = "canonical_program_pins.rs"]
mod provenance;

fn cargo_target(evidence: &Path) -> PathBuf {
    let target = std::env::var_os("MECH_CANONICAL_HOSTED_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| evidence.join("cargo-target"));
    assert!(target.is_absolute(), "qualification cache must be absolute");
    target
}

fn launch() -> Value {
    json!({
        "grants": [
            {"target":"cli/stdin","operations":["read"],"paths":["text"]},
            {"target":"cli/stdout","operations":["write"],"paths":["text"]}
        ],
        "max_ports":16,
        "input_text_bytes":1048576,
        "output_text_bytes":1048576,
        "memory_bytes":268435456,
        "work":1000000000,
        "transport_bytes":16777216,
        "native_stack_bytes":1048576,
        "library": {
            "output_elements":1000000, "output_bytes":134217728,
            "temporary_bytes":134217728, "cloned_bytes":134217728,
            "retained_nodes":1000000, "compute_work":134217728,
            "comparison_work":1000000
        }
    })
}
fn with_input(command: &mut Command, directory: &Path, label: &str, input: &Path) -> Output {
    let stdout = directory.join(format!("{label}.stdout"));
    let stderr = directory.join(format!("{label}.stderr"));
    command
        .stdin(Stdio::from(File::open(input).unwrap()))
        .stdout(Stdio::from(File::create(&stdout).unwrap()))
        .stderr(Stdio::from(File::create(&stderr).unwrap()));
    let mut child = ContainedChild::spawn(command).unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.child_mut().try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.terminate_tree().unwrap();
            let reap = Instant::now() + Duration::from_secs(5);
            while child.child_mut().try_wait().unwrap().is_none() && Instant::now() < reap {
                thread::sleep(Duration::from_millis(20));
            }
            panic!("hosted native case {label} exceeded deadline");
        }
        thread::sleep(Duration::from_millis(20));
    };
    child.terminate_tree().unwrap();
    Output {
        status,
        stdout: fs::read(stdout).unwrap(),
        stderr: fs::read(stderr).unwrap(),
    }
}

fn closed_stdout(binary: &Path, launch: &Path, input: &Path, directory: &Path) -> Output {
    let stderr = directory.join("closed-stdout.stderr");
    let mut command = Command::new(binary);
    command
        .arg(launch)
        .arg("-")
        .arg("--")
        .stdin(Stdio::from(File::open(input).unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::from(File::create(&stderr).unwrap()));
    let mut child = ContainedChild::spawn(&mut command).unwrap();
    drop(child.child_mut().stdout.take().expect("owned stdout pipe"));
    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.child_mut().try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.terminate_tree().unwrap();
            panic!("closed stdout native delivery exceeded deadline");
        }
        thread::sleep(Duration::from_millis(20));
    };
    child.terminate_tree().unwrap();
    Output {
        status,
        stdout: Vec::new(),
        stderr: fs::read(stderr).unwrap(),
    }
}

#[test]
#[ignore = "explicit clean-source hosted qualification; optional explicit full-input phase"]
fn complete_original_and_decoded_knucleotide_linked_processes_are_secured_and_pinned() {
    for variable in [
        "MECH_CANONICAL_HOSTED_FULL_INPUT_CONFIG",
        "MECH_CANONICAL_HOSTED_STAGED_INPUT_CONFIG",
    ] {
        assert!(
            std::env::var_os(variable).is_none(),
            "{variable} now belongs to the pinned benchmark runner; first qualify current binaries here, then run tools/canonical-hosted-runner in mech-lang/benchmarks"
        );
    }
    let profile =
        profile::Profile::parse(std::env::var_os("MECH_CANONICAL_HOSTED_PROFILE").as_deref())
            .unwrap();
    let commit = std::env::var("MECH_CANONICAL_HOSTED_SOURCE_COMMIT")
        .expect("exact immutable source commit required");
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_HOSTED_EVIDENCE_DIR")
            .expect("fresh absolute evidence directory required"),
    );
    assert!(evidence.is_absolute() && !evidence.exists());
    fs::create_dir_all(&evidence).unwrap();
    clean(&evidence, &commit, "before");
    assert_no_ambient_cargo_config();
    let (variants, catalog) = artifacts(FULL);
    let binary_name = "canonical-hosted-knucleotide";
    let projects = variants
        .iter()
        .enumerate()
        .map(|(index, artifact)| {
            generate_linked_hosted_program_application(
                &workspace(),
                &evidence.join(format!("project-{index}")),
                binary_name,
                artifact,
                &catalog,
                FULL.as_bytes(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(projects[0].pins(), projects[1].pins());
    assert_eq!(
        projects[0].metadata()["typed_ir_sha256"],
        projects[1].metadata()["typed_ir_sha256"]
    );
    let mut all_pins = BTreeMap::new();
    let mut results = Vec::new();
    for (variant, project) in projects.iter().enumerate() {
        let directory = evidence.join(format!("variant-{variant}"));
        fs::create_dir(&directory).unwrap();
        assert_eq!(
            fs::read(project.root().join("application.mec")).unwrap(),
            FULL.as_bytes()
        );
        let embedded = mech_engine::decode_program_artifact_bytecode_v1(
            &fs::read(project.root().join("program.mecb")).unwrap(),
        )
        .unwrap();
        assert_eq!(embedded.revision(), variants[variant].revision());
        assert_eq!(embedded.requirements(), variants[variant].requirements());
        for (relative, expected) in project.pins() {
            let path = project.root().join(relative);
            assert_eq!(&pin(&path), expected);
            all_pins.insert(path, expected.clone());
        }
        fs::copy(
            workspace().join("Cargo.lock"),
            project.root().join("Cargo.lock"),
        )
        .unwrap();
        let isolated = project.root().ancestors().last().unwrap();
        let metadata_output = bounded(
            &mut CargoInvocation::resolve_lockfile(project.manifest_path(), true)
                .with_current_dir(isolated)
                .command(),
            &directory,
            "metadata",
            120,
        );
        assert!(
            metadata_output.status.success(),
            "{}",
            String::from_utf8_lossy(&metadata_output.stderr)
        );
        let metadata: Value = serde_json::from_slice(&metadata_output.stdout).unwrap();
        let target = cargo_target(&evidence);
        let mut command = profile
            .invocation(&project.manifest_path(), binary_name, &target)
            .with_current_dir(isolated)
            .command();
        command
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("CARGO_BUILD_JOBS", "1");
        let built = bounded(&mut command, &directory, "build", 15 * 60);
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
        assert_eq!(messages.build_finished, Some(true));
        let units = closure(&metadata, &built.stdout);
        for unit in &units {
            if unit["name"] == "mech-runtime" {
                for feature in unit["features"].as_array().unwrap() {
                    assert!(
                        ![
                            "source",
                            "semantic-compiler",
                            "compiler",
                            "full_compiler",
                            "resident-routing-source"
                        ]
                        .contains(&feature.as_str().unwrap()),
                        "native shell contains source/runtime body frontend feature {feature}"
                    );
                }
            }
        }
        assert!(units.iter().any(|unit| unit["name"] == "mech-runtime"));
        assert!(units.iter().any(|unit| unit["name"] == "mech-terminal"));
        let built_binary = messages.select_binary(binary_name).unwrap().executable;
        let compiler_profiles = profile.audit(&built.stdout, &built_binary);
        let binary = directory.join("native-executable");
        fs::copy(&built_binary, &binary).unwrap();
        let binary_pin = pin(&binary);
        assert_eq!(binary_pin, pin(&built_binary));
        let compiled_launcher_tests = launcher_tests::qualify(
            &project.manifest_path(),
            &target,
            &directory,
            profile.release(),
        );
        assert_eq!(binary_pin, pin(&built_binary));
        let revision = variants[variant]
            .revision()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mut cases = Vec::new();
        for (label, input, expected) in [
            ("ordinary", GGTA_INPUT, Some(GGTA_OUTPUT)),
            ("exact-input-cap", GGTA_INPUT, Some(GGTA_OUTPUT)),
            ("unused-arguments-exact-cap", GGTA_INPUT, Some(GGTA_OUTPUT)),
            ("unused-arguments-over-cap", GGTA_INPUT, None),
            ("argument-header-memory-denial", GGTA_INPUT, None),
            (
                "empty",
                b">THREE\n>FOUR\nA\n".as_slice(),
                Some(EMPTY_OUTPUT),
            ),
            ("missing-read-grant", GGTA_INPUT, None),
            ("missing-write-grant", GGTA_INPUT, None),
            ("work-denial", GGTA_INPUT, None),
            ("input-cap-denial", GGTA_INPUT, None),
            ("overflow-input-cap", GGTA_INPUT, None),
            ("invalid-utf8", b"\xff".as_slice(), None),
        ] {
            let mut policy = launch();
            match label {
                "missing-read-grant" => {
                    policy["grants"].as_array_mut().unwrap().remove(0);
                }
                "missing-write-grant" => {
                    policy["grants"].as_array_mut().unwrap().remove(1);
                }
                "work-denial" => policy["work"] = json!(1),
                "input-cap-denial" => policy["input_text_bytes"] = json!(3),
                "exact-input-cap" => policy["input_text_bytes"] = json!(input.len()),
                "unused-arguments-exact-cap" | "unused-arguments-over-cap" => {
                    policy["input_text_bytes"] = json!(input.len());
                }
                "argument-header-memory-denial" => policy["memory_bytes"] = json!(0),
                "overflow-input-cap" => policy["input_text_bytes"] = json!(u64::MAX),
                _ => (),
            }
            let launch_path = directory.join(format!("{label}.launch.json"));
            let input_path = directory.join(format!("{label}.input"));
            let report_path = directory.join(format!("{label}.report.json"));
            let policy_bytes = serde_json::to_vec_pretty(&policy).unwrap();
            fs::write(&launch_path, &policy_bytes).unwrap();
            fs::write(&input_path, input).unwrap();
            let mut command = Command::new(&binary);
            command
                .arg(&launch_path)
                // Request a fresh report even for denied cases, so their
                // no-report assertion verifies failure publication ordering.
                .arg(&report_path)
                .arg("--");
            // FULL reads stdin, not @args: even UNUSED payload arguments must
            // pass aggregate acquisition and same-account snapshot admission.
            let payload_arguments = match label {
                "unused-arguments-exact-cap" | "unused-arguments-over-cap" => {
                    let first = input.len() / 2;
                    let extra = usize::from(label == "unused-arguments-over-cap");
                    vec!["x".repeat(first), "y".repeat(input.len() - first + extra)]
                }
                "argument-header-memory-denial" => vec![String::new(), String::new()],
                _ => Vec::new(),
            };
            command.args(&payload_arguments);
            // Legacy source-native policy cannot grant authority to this route.
            command.env("MECH_NATIVE_SECURITY", "trusted");
            let output = with_input(&mut command, &directory, label, &input_path);
            assert_eq!(
                output.status.success(),
                expected.is_some(),
                "{label}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if let Some(expected) = expected {
                assert_eq!(output.stdout, expected.as_bytes());
                assert!(
                    output.stderr.is_empty(),
                    "{label}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let report: Value =
                    serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
                assert_eq!(
                    report["route"],
                    "canonical-artifact-program-object-secured-cli"
                );
                assert_eq!(report["policy"], "secured");
                assert_eq!(report["native_aot"], true);
                assert_eq!(report["hosted_phases"], "admit-activate-capture-resume");
                assert_eq!(report["source_parser"], false);
                assert_eq!(report["jit"], false);
                assert_eq!(report["resident_body_evaluation"], false);
                assert_eq!(report["shutdown"], "completed");
                assert_eq!(
                    (
                        report["requirements"].as_u64(),
                        report["observations"].as_u64(),
                        report["effects"].as_u64()
                    ),
                    (Some(2), Some(1), Some(1))
                );
                assert_eq!(report["program_revision"], revision);
            } else {
                assert!(
                    output.stdout.is_empty(),
                    "{label}: denied program delivered output"
                );
                assert!(!output.stderr.is_empty());
                assert!(!report_path.exists());
                if label == "work-denial" {
                    assert!(
                        String::from_utf8_lossy(&output.stderr)
                            .contains("NativeProgramExecutionError")
                    );
                }
                if label == "unused-arguments-over-cap" {
                    assert!(String::from_utf8_lossy(&output.stderr).contains(
                        "payload arguments exceed their finite aggregate acquisition cap"
                    ));
                }
                if label == "argument-header-memory-denial" {
                    assert!(
                        String::from_utf8_lossy(&output.stderr)
                            .contains("payload argument snapshot memory: BudgetExceeded")
                    );
                }
            }
            assert_eq!(
                pin(&binary),
                binary_pin,
                "runtime invocation rewrote executable"
            );
            cases.push(json!({
                "label":label, "accepted":expected.is_some(),
                "input_sha256":hash(input), "launch_sha256":hash(&policy_bytes),
                "stdout_sha256":hash(&output.stdout), "stderr_sha256":hash(&output.stderr),
                "payload_arguments":payload_arguments,
                "binary_sha256":binary_pin,
            }));
        }
        let broken = closed_stdout(
            &binary,
            &directory.join("ordinary.launch.json"),
            &directory.join("ordinary.input"),
            &directory,
        );
        assert!(!broken.status.success());
        assert!(
            String::from_utf8_lossy(&broken.stderr).contains("CommittedWithFailures"),
            "real write/flush error lost committed outcome: {}",
            String::from_utf8_lossy(&broken.stderr)
        );
        assert_eq!(pin(&binary), binary_pin);
        cases.push(json!({"label":"closed-stdout", "accepted":false,
            "postcommit_delivery_failure":true, "stderr_sha256":hash(&broken.stderr),
            "binary_sha256":binary_pin}));
        let pin_rejections = if variant == 0 {
            provenance::qualify(
                project,
                binary_name,
                &built_binary,
                &target,
                &directory,
                profile.release(),
            )
        } else {
            Value::Null
        };
        assert_eq!(pin(&binary), binary_pin);
        results.push(json!({
            "variant":variant, "binary":binary, "binary_sha256":binary_pin,
            "program_revision":revision,
            "runtime_units":units, "cases":cases, "pin_rejections":pin_rejections,
            "compiled_launcher_tests":compiled_launcher_tests,
            "rust_profile":profile.name(), "compiler_profiles":compiler_profiles,
            "source_sha256":hash(FULL.as_bytes()),
            "bytecode_sha256":pin(&project.root().join("program.mecb")),
            "object_sha256":pin(&project.root().join("program.o")),
            "identity_sha256":pin(&project.root().join("object-identity.bin")),
        }));
    }
    for (path, expected) in &all_pins {
        assert_eq!(&pin(path), expected);
    }
    fs::write(
        evidence.join("small-launch.json"),
        serde_json::to_vec_pretty(&launch()).unwrap(),
    )
    .unwrap();
    fs::write(
        evidence.join("release-qualifier-results.json"),
        serde_json::to_vec_pretty(&results).unwrap(),
    )
    .unwrap();
    let full_result = Value::Null; // Full-input orchestration is maintained in benchmarks.
    for (path, expected) in all_pins {
        assert_eq!(pin(&path), expected);
    }
    fs::write(evidence.join("hosted-linked-result.json"), serde_json::to_vec_pretty(&json!({
        "schema":"mech.canonical-hosted-linked-qualification.v1",
        "source_commit":commit, "results":results, "timing_samples":full_result["timing_samples"].as_u64().unwrap_or(0),
        "full_input":full_result,
        "rust_profile":profile.name(), "object_optimization":"speed", "whole_program_optimized":false,
        "scope":"actual full original/decoded canonical artifacts; same checked phased Object route with activation before capture and same-session resume, real CLI provider/capability system, no parser/JIT/resident body",
    })).unwrap()).unwrap();
    clean(&evidence, &commit, "after");
}
