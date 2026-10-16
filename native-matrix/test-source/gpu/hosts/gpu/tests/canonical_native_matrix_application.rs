//! Explicit clean-checkout linked qualification; no benchmark measurements.
#![cfg(feature = "canonical-native")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]
use mech_build::{CargoInvocation, parse_cargo_build_messages, process::ContainedChild};
use mech_core::{
    ManagedMemoryBudget, ReactiveInstanceId, Value, ValueData, ValueDataDraft, ValueDraft,
    snapshot::{F64Bits, SequenceView, SnapshotValidationContext},
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact, decode_source_input_name,
    native_artifact::lower_matrix_artifact,
    resident::{ActivationFacts, CapturedValueInput, activate},
};
use mech_gpu::native_artifact::matrix::{
    JitMatrixArtifact, NativeMatrixExecutionError as Error, NativeMatrixExecutionLimits as Limits,
    NativeMatrixExecutionPhase as Phase, NativeMatrixValue, generate_matrix_linked_application,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File},
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};
const PROCESS_DEADLINE: Duration = Duration::from_secs(15);
const WORKER_DEADLINE: Duration = Duration::from_secs(20 * 60);
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn pin(path: &Path) -> String {
    digest(&fs::read(path).unwrap())
}
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

struct Captured {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// File-backed streams and an owned process group/job contain the complete
/// synchronous generation, lock resolution, metadata, build and link worker.
fn bounded(mut command: Command, root: &Path, name: &str, deadline: Duration) -> Captured {
    let stdout = root.join(format!("{name}.stdout"));
    let stderr = root.join(format!("{name}.stderr"));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(File::create(&stdout).unwrap()))
        .stderr(Stdio::from(File::create(&stderr).unwrap()));
    let mut child = ContainedChild::spawn(&mut command).unwrap();
    let end = Instant::now() + deadline;
    let status = loop {
        if let Some(status) = child.child_mut().try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= end {
            child.terminate_tree().unwrap();
            let reap = Instant::now() + Duration::from_secs(5);
            while child.child_mut().try_wait().unwrap().is_none() && Instant::now() < reap {
                thread::sleep(Duration::from_millis(20));
            }
            panic!("{name} exceeded deadline; diagnostics {}", stderr.display());
        }
        thread::sleep(Duration::from_millis(20));
    };
    child.terminate_tree().unwrap();
    Captured {
        status,
        stdout: fs::read(stdout).unwrap(),
        stderr: fs::read(stderr).unwrap(),
    }
}

fn clean(workspace: &Path, evidence: &Path, commit: &str, label: &str) {
    let mut command = Command::new("git");
    command.args(["rev-parse", "HEAD"]).current_dir(workspace);
    let head = bounded(
        command,
        evidence,
        &format!("{label}-head"),
        PROCESS_DEADLINE,
    );
    assert!(head.status.success());
    assert_eq!(String::from_utf8(head.stdout).unwrap().trim(), commit);
    let mut command = Command::new("git");
    command
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(workspace);
    let status = bounded(
        command,
        evidence,
        &format!("{label}-status"),
        PROCESS_DEADLINE,
    );
    assert!(status.status.success());
    assert!(
        status.stdout.is_empty(),
        "requires a clean immutable checkout"
    );
}

fn worker_command(mut command: Command, root: &Path, label: &str) -> Captured {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            File::create(root.join(format!("{label}.stdout"))).unwrap(),
        ))
        .stderr(Stdio::from(
            File::create(root.join(format!("{label}.stderr"))).unwrap(),
        ));
    let status = command.status().unwrap();
    Captured {
        status,
        stdout: fs::read(root.join(format!("{label}.stdout"))).unwrap(),
        stderr: fs::read(root.join(format!("{label}.stderr"))).unwrap(),
    }
}

fn compiled_feature_closure(metadata: &Json, build_stdout: &[u8]) -> Vec<Json> {
    let packages = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| (package["id"].as_str().unwrap(), package))
        .collect::<BTreeMap<_, _>>();
    let root = metadata["resolve"]["root"].as_str().unwrap();
    // Metadata can retain weak optional dependency edges that Cargo does not
    // build. Every actual compiler-artifact unit (including fresh units) is
    // authoritative for the successful build's selected packages/features.
    let mut seen = BTreeSet::new();
    let mut seen_packages = BTreeSet::new();
    let mut result = Vec::new();
    for line in build_stdout.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let message: Json = serde_json::from_slice(line).unwrap();
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let id = message["package_id"].as_str().unwrap();
        let package = packages[id];
        let name = package["name"].as_str().unwrap();
        seen_packages.insert(name.to_owned());
        if name.starts_with("mech-") {
            assert!(package["source"].is_null(), "registry Mech crate {name}");
            let manifest = Path::new(package["manifest_path"].as_str().unwrap())
                .canonicalize()
                .unwrap();
            assert!(
                manifest.starts_with(workspace().canonicalize().unwrap()),
                "Mech crate {name} is outside the pinned workspace"
            );
        }
        assert_ne!(name, "mech-syntax");
        assert!(
            !name.starts_with("cranelift-"),
            "runtime graph contains compiler crate {name}"
        );
        let forbidden: &[&str] = match name {
            "mech-gpu" => &[
                "canonical-native",
                "jit",
                "aot",
                "sequence-cli",
                "native-build",
                "fused-cli",
            ],
            "mech-engine" | "mech-stdlib" => &[
                "source",
                "semantic-compiler",
                "compiler",
                "full_compiler",
                "standard_compiler",
                "native-sequence",
            ],
            _ => &[],
        };
        for feature in message["features"].as_array().unwrap() {
            assert!(
                !forbidden.contains(&feature.as_str().unwrap()),
                "runtime-only graph enabled {name}/{feature}"
            );
        }
        if seen.insert((
            id.to_owned(),
            message["target"].to_string(),
            message["features"].to_string(),
        )) {
            result.push(json!({"id":id, "name":name, "version":package["version"], "features":message["features"], "target_name":message["target"]["name"], "target_kind":message["target"]["kind"]}));
        }
    }
    assert!(result.iter().any(|unit| unit["id"] == root));
    for required in ["mech-core", "mech-engine", "mech-gpu", "mech-stdlib"] {
        assert!(
            seen_packages.contains(required),
            "missing compiled {required}"
        );
    }
    result.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    result
}

fn artifacts(source: &str, interactive: bool) -> [ProgramArtifact; 2] {
    let parse = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d41544e4154), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parse.is_strictly_clean(),
        "{source}: {:?}",
        parse.diagnostics
    );
    let doc = DocumentSyntax::cast(parse.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&doc, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend.compile_document_with_catalog(&doc, mech_stdlib::source_catalog())
    }
    .unwrap();
    let artifact = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [artifact, decoded]
}
fn inputs(artifact: &ProgramArtifact, named: &[(&str, [u64; 2], Vec<u64>)]) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = decode_source_input_name(&input.name).unwrap();
            let (_, extents, bits) = named.iter().find(|(key, _, _)| *key == name).unwrap();
            let schema = artifact.schemas().get(input.schema).unwrap();
            let shape = mech_core::shape_for_resolved_extents(schema, extents).unwrap();
            ValueDraft {
                schema: input.schema,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data: ValueDataDraft::Matrix(
                    bits.iter()
                        .map(|bits| ValueDataDraft::F64(F64Bits::from_bits(*bits)))
                        .collect(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        })
        .collect()
}
fn reference(
    artifact: &ProgramArtifact,
    inputs: &[Value],
) -> Result<Vec<([u64; 2], Vec<u64>)>, String> {
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d41544e4154, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|declared| declared.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &inputs[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|p| p.publish())
        .map_err(|e| format!("{e:?}"))?;
    (0..artifact.outputs().len())
        .map(|index| {
            let value = instance.copied_output(index).unwrap();
            let owner = value.schemas().unwrap();
            let schema = value.validate_against(&owner).unwrap();
            let mech_core::SchemaBody::Matrix { dimensions, .. } = schema.body() else {
                panic!("matrix result required")
            };
            let extents = [
                value.shape().resolve_dimension(&dimensions[0]).unwrap(),
                value.shape().resolve_dimension(&dimensions[1]).unwrap(),
            ];
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("matrix payload required")
            };
            let bits = match matrix.elements() {
                SequenceView::F64(xs) => xs.iter().map(|v| v.bits()).collect(),
                SequenceView::Values(xs) => xs
                    .iter()
                    .map(|v| match v {
                        ValueData::F64(v) => v.bits(),
                        _ => panic!("F64 lane required"),
                    })
                    .collect(),
                other => {
                    assert_eq!(other.len(), 0);
                    Vec::new()
                }
            };
            Ok((extents, bits))
        })
        .collect()
}
fn actual(outputs: &[Option<NativeMatrixValue>]) -> Vec<([u64; 2], Vec<u64>)> {
    outputs
        .iter()
        .map(|output| {
            let output = output.as_ref().unwrap();
            let [rows, cols] = output.extents();
            let mut bits = Vec::new();
            for ordinal in 0..rows.checked_mul(cols).unwrap() {
                let row = ordinal / cols;
                let col = ordinal % cols;
                bits.push(output.column_major_values()[(row + col * rows) as usize].to_bits());
            }
            ([rows, cols], bits)
        })
        .collect()
}

const TRANSPOSE: &str = "transposed := input<[f64]>'\nalias := transposed\nalias\n";
const PRODUCT: &str = "left<[f64]> ** right<[f64]>\n";
const CHAIN: &str = "left<[f64]>' ** right<[f64]>\n";
fn formatted(values: &[([u64; 2], Vec<u64>)]) -> String {
    let mut output = String::new();
    use std::fmt::Write;
    for (shape, bits) in values {
        write!(&mut output, "{}x{}:", shape[0], shape[1]).unwrap();
        for (i, bit) in bits.iter().enumerate() {
            if i > 0 {
                output.push(',');
            }
            write!(&mut output, "{bit:016x}").unwrap();
        }
        output.push('\n');
    }
    output
}
fn arguments(artifact: &ProgramArtifact, named: &[(&str, [u64; 2], Vec<u64>)]) -> Vec<String> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = decode_source_input_name(&input.name).unwrap();
            let (_, shape, bits) = named.iter().find(|(key, _, _)| *key == name).unwrap();
            formatted(&[(*shape, bits.clone())])
                .trim_end_matches('\n')
                .to_owned()
        })
        .collect()
}
fn success(label: &str, args: Vec<String>, values: &[([u64; 2], Vec<u64>)]) -> Json {
    json!({"name":label,"arguments":args,"stdout":formatted(values),"stderr":"","exit":0})
}
fn failure(label: &str, args: Vec<String>, error: String) -> Json {
    json!({"name":label,"arguments":args,"stdout":"","stderr":format!("{error}\n"),"exit":1})
}
fn cases(name: &str, artifacts: &[ProgramArtifact; 2], object_stack: u64) -> Vec<Json> {
    let plans = artifacts
        .iter()
        .map(|a| lower_matrix_artifact(a, &mech_stdlib::source_catalog()).unwrap())
        .collect::<Vec<_>>();
    let jits = plans
        .iter()
        .map(|p| JitMatrixArtifact::compile(p).unwrap())
        .collect::<Vec<_>>();
    let one = 1.0f64.to_bits();
    let two = 2.0f64.to_bits();
    let datasets: Vec<(&str, Vec<(&str, [u64; 2], Vec<u64>)>)> = match name {
        "matrix-transpose" => vec![
            (
                "rectangle",
                vec![(
                    "input",
                    [2, 3],
                    vec![
                        0,
                        1,
                        1u64 << 63,
                        0x7ff0000000000000,
                        0xfff0000000000000,
                        0x7ff8000000001234,
                    ],
                )],
            ),
            ("zero-row", vec![("input", [0, 3], vec![])]),
            (
                "huge-zero-column",
                vec![("input", [1_000_000_000, 0], vec![])],
            ),
        ],
        "matrix-product" => vec![
            (
                "rectangle",
                vec![
                    ("left", [2, 3], vec![one, two, one, 0, two, one]),
                    ("right", [3, 2], vec![one, 0, 0, one, one, one]),
                ],
            ),
            (
                "empty-inner",
                vec![("left", [2, 0], vec![]), ("right", [0, 3], vec![])],
            ),
            (
                "nan-bits",
                vec![
                    ("left", [1, 2], vec![0x7ff8000000001234, 1]),
                    ("right", [2, 1], vec![one, one]),
                ],
            ),
        ],
        "matrix-chain" => vec![
            (
                "rectangle",
                vec![
                    ("left", [2, 3], vec![one, two, one, 0, two, one]),
                    ("right", [2, 2], vec![one, 0, 0, one]),
                ],
            ),
            (
                "empty-inner",
                vec![("left", [0, 2], vec![]), ("right", [0, 3], vec![])],
            ),
        ],
        _ => panic!("unexpected qualification application"),
    };
    let mut cases = Vec::new();
    for (label, named) in datasets {
        let args = inputs(&artifacts[0], &named);
        let expected = reference(&artifacts[0], &args).unwrap();
        let mut work = 0;
        for i in 0..2 {
            let other = inputs(&artifacts[i], &named);
            assert_eq!(reference(&artifacts[i], &other).unwrap(), expected);
            let mut out = vec![None; plans[i].outputs().len()];
            let budget = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let report = jits[i]
                .evaluate_into(&other, &mut out, Limits::default(), &budget)
                .unwrap();
            assert_eq!(actual(&out), expected);
            work = report.work;
            drop(out);
            assert_eq!(budget.used_bytes(), 0);
        }
        let argv = arguments(&artifacts[0], &named);
        cases.push(success(label, argv.clone(), &expected));
        if label == "rectangle" {
            let mut boundary = vec!["--".into()];
            boundary.extend(argv.clone());
            cases.push(success("explicit-boundary", boundary, &expected));
            for (label, prefix, limits, budget) in [
                (
                    "setup-denial",
                    "--work=0".to_owned(),
                    Limits {
                        max_work: 0,
                        ..Limits::default()
                    },
                    64 * 1024 * 1024,
                ),
                (
                    "completion-denial",
                    format!("--work={}", work - 1),
                    Limits {
                        max_work: work - 1,
                        ..Limits::default()
                    },
                    64 * 1024 * 1024,
                ),
                ("memory-denial", "--budget=0".into(), Limits::default(), 0),
            ] {
                let mut out = vec![None; plans[0].outputs().len()];
                let error = jits[0]
                    .evaluate_into(&args, &mut out, limits, &ManagedMemoryBudget::new(budget))
                    .unwrap_err();
                assert!(out.iter().all(Option::is_none));
                if label == "completion-denial" {
                    assert!(matches!(
                        error,
                        Error::Work {
                            phase: Phase::Completion,
                            ..
                        }
                    ));
                }
                let mut controlled = vec![prefix];
                controlled.extend(argv.clone());
                cases.push(failure(label, controlled, error.to_string()));
            }
            let mut stack = vec!["--stack=0".into()];
            stack.extend(argv.clone());
            cases.push(failure(
                "stack-denial",
                stack,
                Error::Stack {
                    required: object_stack,
                    limit: 0,
                }
                .to_string(),
            ));
            let mut invalid = argv.clone();
            invalid[0] = "1x1:not-hex".into();
            cases.push(failure(
                "invalid-word",
                invalid,
                "invalid F64 bit word".into(),
            ));
        }
    }
    if name != "matrix-transpose" {
        let named = vec![("left", [2, 3], vec![0; 6]), ("right", [4, 2], vec![0; 8])];
        let args = inputs(&artifacts[0], &named);
        assert!(reference(&artifacts[0], &args).is_err());
        let mut out = vec![None; plans[0].outputs().len()];
        let error = jits[0]
            .evaluate_into(
                &args,
                &mut out,
                Limits::default(),
                &ManagedMemoryBudget::new(64 * 1024 * 1024),
            )
            .unwrap_err();
        assert!(matches!(error, Error::Planning { .. }));
        cases.push(failure(
            "actual-shape-mismatch",
            arguments(&artifacts[0], &named),
            error.to_string(),
        ));
    }
    cases
}

#[test]
#[ignore = "internal worker; complete generation/build subtree is parent-owned"]
fn canonical_matrix_link_build_worker() {
    let name = std::env::var("MECH_CANONICAL_MATRIX_WORKER_NAME").unwrap();
    assert!(matches!(
        name.as_str(),
        "matrix-transpose" | "matrix-product" | "matrix-chain"
    ));
    let evidence = PathBuf::from(std::env::var_os("MECH_CANONICAL_MATRIX_EVIDENCE_DIR").unwrap());
    let root = evidence.join(&name);
    let source = fs::read_to_string(root.join("application.mec")).unwrap();
    let artifacts = artifacts(&source, true);
    let catalog = mech_stdlib::source_catalog();
    let generated = generate_matrix_linked_application(
        &workspace(),
        &root.join("project"),
        &name,
        &artifacts[0],
        &catalog,
        source.as_bytes(),
    )
    .unwrap();
    let decoded = generate_matrix_linked_application(
        &workspace(),
        &root.join("decoded-project"),
        &name,
        &artifacts[1],
        &catalog,
        source.as_bytes(),
    )
    .unwrap();
    assert_eq!(generated.pins(), decoded.pins());
    assert_eq!(
        generated.metadata()["typed_ir_sha256"],
        decoded.metadata()["typed_ir_sha256"]
    );
    let cases = cases(
        &name,
        &artifacts,
        generated.metadata()["object"]["stack_bytes"]
            .as_u64()
            .unwrap(),
    );
    for project in [&generated, &decoded] {
        for (relative, expected) in project.pins() {
            assert_eq!(&pin(&project.root().join(relative)), expected);
        }
    }
    let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifacts[0]).unwrap();
    assert_eq!(
        fs::read(generated.root().join("program.mecb")).unwrap(),
        bytecode
    );
    assert_eq!(
        fs::read(generated.root().join("application.mec")).unwrap(),
        source.as_bytes()
    );
    fs::copy(
        workspace().join("Cargo.lock"),
        generated.root().join("Cargo.lock"),
    )
    .unwrap();
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".cargo")));
    if let Some(home) = cargo_home {
        assert!(
            !home.join("config").exists() && !home.join("config.toml").exists(),
            "ambient Cargo config unsupported"
        );
    }
    let isolated = generated.root().ancestors().last().unwrap();
    let metadata = worker_command(
        CargoInvocation::resolve_lockfile(generated.manifest_path(), true)
            .with_current_dir(isolated)
            .command(),
        &root,
        "metadata",
    );
    assert!(
        metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let metadata: Json = serde_json::from_slice(&metadata.stdout).unwrap();
    let mut command = CargoInvocation::build(
        generated.manifest_path(),
        &name,
        None,
        evidence.join("cargo-target"),
        false,
        true,
    )
    .unwrap()
    .with_current_dir(isolated)
    .command();
    command
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_BUILD_JOBS", "2");
    let built = worker_command(command, &root, "build");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
    assert_eq!(messages.build_finished, Some(true));
    let closure = compiled_feature_closure(&metadata, &built.stdout);
    let executable = messages.select_binary(&name).unwrap().executable;
    let mut pins = BTreeMap::new();
    for project in [&generated, &decoded] {
        for (relative, expected) in project.pins() {
            pins.insert(project.root().join(relative), expected.clone());
        }
        let path = project.root().join("linked-manifest.json");
        pins.insert(path.clone(), pin(&path));
    }
    for path in [
        executable.clone(),
        generated.root().join("Cargo.lock"),
        workspace().join("Cargo.lock"),
        root.join("application.mec"),
        root.join("metadata.stdout"),
        root.join("build.stdout"),
    ] {
        pins.insert(path.clone(), pin(&path));
    }
    let record = json!({"executable":executable,"pins":pins,"cases":cases,"build":{"name":name,"source_sha256":digest(source.as_bytes()),"bytecode_sha256":digest(&bytecode),"source_revision":hex_revision(artifacts[0].revision().as_bytes()),"generated":generated.metadata(),"runtime_compiled_units":closure,"binary_sha256":pin(&executable),"build_count":1,"object_emissions":2,"reference":"actual canonical resident + JIT original and decoded; exact bitwise row-major outputs","benchmark_measurements":0}});
    fs::write(
        root.join("application-build.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
}
fn hex_revision(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn qualify(evidence: &Path, name: &str, source: &str) -> Json {
    let root = evidence.join(name);
    fs::create_dir(&root).unwrap();
    fs::write(root.join("application.mec"), source).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--ignored",
            "--exact",
            "canonical_matrix_link_build_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MECH_CANONICAL_MATRIX_WORKER_NAME", name);
    let built = bounded(command, &root, "build-worker", WORKER_DEADLINE);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let record_path = root.join("application-build.json");
    let mut record: Json = serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
    let executable: PathBuf = serde_json::from_value(record["executable"].take()).unwrap();
    let mut pins: BTreeMap<PathBuf, String> =
        serde_json::from_value(record["pins"].take()).unwrap();
    pins.insert(record_path.clone(), pin(&record_path));
    let check = || {
        for (path, expected) in &pins {
            assert_eq!(&pin(path), expected, "changed {}", path.display());
        }
    };
    check();
    let mut launches = Vec::new();
    for case in record["cases"].as_array().unwrap() {
        let label = case["name"].as_str().unwrap();
        let arguments = case["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| OsString::from(v.as_str().unwrap()))
            .collect::<Vec<_>>();
        let expected_out = case["stdout"].as_str().unwrap().as_bytes();
        let expected_err = case["stderr"].as_str().unwrap().as_bytes();
        fs::write(root.join(format!("{label}.expected.stdout")), expected_out).unwrap();
        fs::write(root.join(format!("{label}.expected.stderr")), expected_err).unwrap();
        let mut command = Command::new(&executable);
        command.args(&arguments);
        let actual = bounded(command, &root, label, PROCESS_DEADLINE);
        assert_eq!(
            actual.status.code(),
            Some(case["exit"].as_i64().unwrap() as i32),
            "{name}/{label}: {}",
            String::from_utf8_lossy(&actual.stderr)
        );
        assert_eq!(actual.stdout, expected_out, "{name}/{label}");
        assert_eq!(actual.stderr, expected_err, "{name}/{label}");
        check();
        launches.push(json!({"name":label,"arguments_os_encoded_hex":arguments.iter().map(|a|hex_revision(a.as_encoded_bytes())).collect::<Vec<_>>(),"exit":actual.status.code(),"stdout_sha256":digest(&actual.stdout),"stderr_sha256":digest(&actual.stderr),"expected_stdout_sha256":digest(expected_out),"expected_stderr_sha256":digest(expected_err),"exact_complete_outputs":true,"binary_unchanged":true}));
    }
    let mut build = record["build"].take();
    build["pins"] = json!(pins);
    build["cases"] = json!(launches);
    build["linked"] = json!(true);
    build["executed"] = json!(true);
    build
}
#[test]
#[ignore = "explicit actual linked matrix Object qualification; clean immutable checkout + fresh external evidence required"]
fn linked_canonical_matrix_applications_use_actual_managed_nativehost_leaves() {
    let source =
        std::env::var("MECH_CANONICAL_MATRIX_SOURCE_COMMIT").expect("pin immutable source HEAD");
    assert_eq!(source.len(), 40);
    assert!(source.bytes().all(|b| b.is_ascii_hexdigit()));
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_MATRIX_EVIDENCE_DIR").expect("fresh external evidence"),
    );
    assert!(evidence.is_absolute());
    fs::create_dir(&evidence).unwrap();
    let evidence = evidence.canonicalize().unwrap();
    assert!(!evidence.starts_with(workspace().canonicalize().unwrap()));
    clean(&workspace(), &evidence, &source, "before");
    let lock = pin(&workspace().join("Cargo.lock"));
    let applications = [
        qualify(&evidence, "matrix-transpose", TRANSPOSE),
        qualify(&evidence, "matrix-product", PRODUCT),
        qualify(&evidence, "matrix-chain", CHAIN),
    ];
    assert_eq!(pin(&workspace().join("Cargo.lock")), lock);
    clean(&workspace(), &evidence, &source, "after");
    fs::write(evidence.join("qualification.json"),serde_json::to_vec_pretty(&json!({"schema":"mech.canonical-native-matrix-linked-qualification.v1","source_commit":source,"clean_before_and_after":true,"route":"canonical-artifact-cranelift-object-with-selected-maintained-matrix-leaves","applications":applications,"linked":true,"executed":true,"benchmark_measurements":0})).unwrap()).unwrap();
}
