//! Explicit linked qualification of canonical artifact Cranelift object code.
//!
//! Run from a clean committed checkout with MECH_CANONICAL_MATCH_SOURCE_COMMIT
//! and a fresh external MECH_CANONICAL_MATCH_EVIDENCE_DIR. This records no timings.
#![cfg(feature = "canonical-native")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

#[path = "support/canonical_runtime_closure.rs"]
mod runtime_closure;

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

use mech_build::{CargoInvocation, parse_cargo_build_messages, process::ContainedChild};
use mech_core::{
    ReactiveInstanceId, ValueData, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchArtifactPlan, NativeScalarKind, NativeScalarValue,
    lower_scalar_match_artifact,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{
    JitMatchArtifact, NativeExecutionError, NativeExecutionLimits, NativeMatchExecutionError,
    generate_linked_match_application,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MIXED: &str = "selected := (enabled<bool>? | true, (gate<bool>? | true => left<u64> / divisor<u64> == 1u64 | false => false) => left<u64> + step<u64> | true => left<u64> | false => (left<u64>? | 0u64 => 1u64 | found => found - 1u64))\nflag := !enabled<bool>\nselected\n";
const ORDERED: &str = "unused := left<u64> / divisor<u64>\nselected := (enabled<bool>? | true => left<u64> + 1u64 | false => left<u64>)\nselected\n";
const PROCESS_DEADLINE: Duration = Duration::from_secs(15);
const WORKER_DEADLINE: Duration = Duration::from_secs(20 * 60);

#[path = "support/canonical_cast_cases.rs"]
mod match_cast_cases;
#[path = "support/canonical_match_linked_cases.rs"]
mod wider_cases;

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

fn compile(source: &str, interactive: bool) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4c494e4b), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    // Public interactive compilation retains real named Bool/U64 outputs;
    // no artifact output, graph, operation or semantic witness is fabricated.
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&document, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend
            .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
    }
    .unwrap();
    let result = product
        .document_outputs()
        .iter()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], result)
}

fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, NativeScalarValue)],
) -> Vec<NativeScalarValue> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            named.iter().find(|(key, _)| *key == name).unwrap().1
        })
        .collect()
}
fn arguments(values: &[NativeScalarValue]) -> Vec<String> {
    values
        .iter()
        .map(|value| match value {
            NativeScalarValue::Bool(value) => value.to_string(),
            NativeScalarValue::U64(value) => value.to_string(),
            NativeScalarValue::I64(value) => value.to_string(),
            NativeScalarValue::F64(value) => format!("f64:{:016x}", value.bits()),
        })
        .collect()
}
fn stdout(values: &[NativeScalarValue]) -> String {
    arguments(values)
        .into_iter()
        .map(|value| value + "\n")
        .collect()
}

fn resident(
    artifact: &ProgramArtifact,
    arguments: &[NativeScalarValue],
) -> Result<Vec<NativeScalarValue>, String> {
    let mut instance = activate(
        ReactiveInstanceId::new(0x4c494e4b, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let values = artifact
        .inputs()
        .iter()
        .zip(arguments)
        .map(|(input, value)| {
            ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data: match value {
                    NativeScalarValue::Bool(value) => ValueDataDraft::Bool(*value),
                    NativeScalarValue::U64(value) => ValueDataDraft::U64(*value),
                    NativeScalarValue::I64(value) => ValueDataDraft::I64(*value),
                    NativeScalarValue::F64(value) => ValueDataDraft::F64(*value),
                },
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        })
        .collect::<Vec<_>>();
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|value| value.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &values[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|prepared| prepared.publish())
        .map_err(|error| format!("{error:?}"))?;
    Ok((0..artifact.outputs().len())
        .map(
            |index| match instance.copied_output(index).unwrap().data() {
                ValueData::Bool(value) => NativeScalarValue::Bool(*value),
                ValueData::U64(value) => NativeScalarValue::U64(*value),
                ValueData::I64(value) => NativeScalarValue::I64(*value),
                ValueData::F64(value) => NativeScalarValue::F64(*value),
                value => panic!("unexpected scalar {value:?}"),
            },
        )
        .collect())
}

fn jit_success(
    jit: &JitMatchArtifact,
    arguments: &[NativeScalarValue],
) -> (Vec<NativeScalarValue>, u64) {
    let mut output = vec![NativeScalarValue::U64(0xfeed); jit.descriptor().outputs().len()];
    let report = jit
        .evaluate_into(
            arguments,
            &mut output,
            NativeExecutionLimits::default(),
            None,
        )
        .unwrap();
    (output, report.work)
}
fn jit_error(
    jit: &JitMatchArtifact,
    arguments: &[NativeScalarValue],
    limits: NativeExecutionLimits,
) -> NativeMatchExecutionError {
    let mut output = vec![NativeScalarValue::U64(0xfeed); jit.descriptor().outputs().len()];
    let before = output.clone();
    let error = jit
        .evaluate_into(arguments, &mut output, limits, None)
        .unwrap_err();
    assert_eq!(output, before, "failed evaluation published an output");
    error
}
fn success_case(name: &str, arguments: Vec<String>, output: &[NativeScalarValue]) -> Value {
    json!({"name": name, "arguments": arguments, "stdout": stdout(output), "diagnostic": null})
}
fn failure_case(name: &str, arguments: Vec<String>, diagnostic: &str) -> Value {
    json!({"name": name, "arguments": arguments, "stdout": "", "diagnostic": diagnostic})
}
fn exact_failure_case(
    name: &str,
    arguments: Vec<String>,
    error: &NativeMatchExecutionError,
) -> Value {
    let mut case = failure_case(name, arguments, &error.to_string());
    case["stderr"] = json!(format!("{error}\n"));
    case
}
fn invalid_unicode_argument() -> Option<OsString> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Some(OsString::from_vec(vec![0xff, b'X']))
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        Some(OsString::from_wide(&[0xd800]))
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}
fn with_limit(limit: &str, arguments: &[NativeScalarValue]) -> Vec<String> {
    std::iter::once(limit.to_owned())
        .chain(self::arguments(arguments))
        .collect()
}

fn reference_cases(name: &str, artifacts: &[ProgramArtifact; 2], result: usize) -> Vec<Value> {
    if wider_cases::contains(name) {
        return wider_cases::reference_cases(name, artifacts);
    }
    let plans = artifacts.each_ref().map(|artifact| {
        lower_scalar_match_artifact(artifact, &mech_stdlib::source_catalog()).unwrap()
    });
    let jits = plans
        .each_ref()
        .map(|plan| JitMatchArtifact::compile(plan).unwrap());
    let mut cases = Vec::new();
    if name == "canonical-match-mixed" {
        assert!(
            plans[0]
                .outputs()
                .iter()
                .any(|output| output.ty().kind() == NativeScalarKind::Bool)
        );
        assert!(
            plans[0]
                .outputs()
                .iter()
                .any(|output| output.ty().kind() == NativeScalarKind::U64)
        );
        for (label, left, step, divisor, enabled, gate, answer) in [
            (
                "above-f64-exact-range",
                (1u64 << 53) + 1,
                1,
                0,
                true,
                false,
                (1u64 << 53) + 1,
            ),
            ("u64-max-lazy-guard", u64::MAX, 1, 0, true, false, u64::MAX),
            ("selected-guard-and-body", 7, 2, 7, true, true, 9),
            ("nested-zero-lazy", 0, 1, 0, false, true, 1),
            ("nested-bind", 9, 1, 0, false, true, 8),
        ] {
            let args = inputs(
                &artifacts[0],
                &[
                    ("left", NativeScalarValue::U64(left)),
                    ("step", NativeScalarValue::U64(step)),
                    ("divisor", NativeScalarValue::U64(divisor)),
                    ("enabled", NativeScalarValue::Bool(enabled)),
                    ("gate", NativeScalarValue::Bool(gate)),
                ],
            );
            let expected = resident(&artifacts[0], &args).unwrap();
            assert_eq!(expected[result], NativeScalarValue::U64(answer));
            for index in 0..2 {
                assert_eq!(resident(&artifacts[index], &args).unwrap(), expected);
                assert_eq!(jit_success(&jits[index], &args).0, expected);
            }
            cases.push(success_case(label, arguments(&args), &expected));
            if label != "above-f64-exact-range" {
                continue;
            }
            cases.push(success_case(
                "explicit-boundary",
                std::iter::once("--".into())
                    .chain(arguments(&args))
                    .collect(),
                &expected,
            ));
            let work = jit_success(&jits[0], &args).1;
            for (label, option, limits) in [
                (
                    "completion-denial",
                    format!("--work={}", work - 1),
                    NativeExecutionLimits {
                        max_work: work - 1,
                        ..NativeExecutionLimits::default()
                    },
                ),
                (
                    "setup-denial",
                    "--work=0".into(),
                    NativeExecutionLimits {
                        max_work: 0,
                        ..NativeExecutionLimits::default()
                    },
                ),
                (
                    "memory-denial",
                    "--memory=0".into(),
                    NativeExecutionLimits {
                        max_transport_bytes: 0,
                        ..NativeExecutionLimits::default()
                    },
                ),
            ] {
                let error = jit_error(&jits[0], &args, limits);
                assert_eq!(jit_error(&jits[1], &args, limits), error);
                cases.push(exact_failure_case(
                    label,
                    with_limit(&option, &args),
                    &error,
                ));
            }
            let budget = mech_core::ManagedMemoryBudget::new(0);
            let mut outputs = vec![NativeScalarValue::U64(77); plans[0].outputs().len()];
            let before = outputs.clone();
            let error = jits[0]
                .evaluate_into(
                    &args,
                    &mut outputs,
                    NativeExecutionLimits::default(),
                    Some(&budget),
                )
                .unwrap_err();
            assert!(matches!(
                error,
                NativeMatchExecutionError::Base(NativeExecutionError::Memory { .. })
            ));
            assert_eq!(outputs, before);
            assert_eq!(budget.used_bytes(), 0);
            cases.push(exact_failure_case(
                "shared-budget-denial",
                with_limit("--budget=0", &args),
                &error,
            ));
            cases.push(failure_case(
                "stack-denial",
                with_limit("--stack=0", &args),
                "StackLimit",
            ));
            let numeric = plans[0]
                .inputs()
                .iter()
                .position(|input| input.declaration().ty().kind() == NativeScalarKind::U64)
                .unwrap();
            let boolean = plans[0]
                .inputs()
                .iter()
                .position(|input| input.declaration().ty().kind() == NativeScalarKind::Bool)
                .unwrap();
            for (label, text) in [
                ("u64-overflow-input", "18446744073709551616"),
                ("signed-u64-input", "-1"),
                ("whitespace-u64-input", " 1"),
            ] {
                let mut invalid = arguments(&args);
                invalid[numeric] = text.into();
                invalid.insert(0, "--".into());
                cases.push(failure_case(label, invalid, "invalid U64 argument"));
            }
            if invalid_unicode_argument().is_some() {
                let mut invalid = failure_case(
                    "invalid-unicode-input",
                    arguments(&args),
                    "argument is not Unicode",
                );
                invalid["invalid_os_argument"] = json!(numeric);
                cases.push(invalid);
            }
            let mut invalid = arguments(&args);
            invalid[boolean] = "TRUE".into();
            cases.push(failure_case(
                "invalid-bool-input",
                invalid,
                "invalid Bool argument",
            ));
            cases.push(failure_case("missing-input", vec![], "typed inputs"));
            let mut extra = arguments(&args);
            extra.push("tail".into());
            cases.push(failure_case("extra-input", extra, "typed inputs"));
            cases.push(failure_case(
                "unknown-option",
                vec!["--unknown".into()],
                "unknown option",
            ));
            cases.push(failure_case(
                "invalid-limit",
                vec!["--work=invalid".into()],
                "limit",
            ));
            cases.push(success_case(
                "recovery-after-denials",
                arguments(&args),
                &expected,
            ));
        }
        let bad = inputs(
            &artifacts[0],
            &[
                ("left", NativeScalarValue::U64(8)),
                ("step", NativeScalarValue::U64(1)),
                ("divisor", NativeScalarValue::U64(0)),
                ("enabled", NativeScalarValue::Bool(true)),
                ("gate", NativeScalarValue::Bool(true)),
            ],
        );
        for index in 0..2 {
            assert!(
                resident(&artifacts[index], &bad)
                    .unwrap_err()
                    .contains("Arithmetic")
            );
        }
        let error = jit_error(&jits[0], &bad, NativeExecutionLimits::default());
        assert!(
            matches!(error,NativeMatchExecutionError::Arithmetic { origin } if origin.block().is_some() && origin.local().is_some())
        );
        assert_eq!(
            jit_error(&jits[1], &bad, NativeExecutionLimits::default()),
            error
        );
        cases.push(exact_failure_case(
            "selected-nested-guard-error",
            arguments(&bad),
            &error,
        ));
    } else {
        let args = |left, divisor| {
            inputs(
                &artifacts[0],
                &[
                    ("left", NativeScalarValue::U64(left)),
                    ("divisor", NativeScalarValue::U64(divisor)),
                    ("enabled", NativeScalarValue::Bool(true)),
                ],
            )
        };
        let good = args(3, 1);
        let first_bad = args(u64::MAX, 0);
        let later_bad = args(u64::MAX, 1);
        let expected = resident(&artifacts[0], &good).unwrap();
        assert_eq!(expected[result], NativeScalarValue::U64(4));
        for index in 0..2 {
            assert_eq!(resident(&artifacts[index], &good).unwrap(), expected);
            assert_eq!(jit_success(&jits[index], &good).0, expected);
            assert!(
                resident(&artifacts[index], &first_bad)
                    .unwrap_err()
                    .contains("Arithmetic")
            );
            assert!(
                resident(&artifacts[index], &later_bad)
                    .unwrap_err()
                    .contains("Arithmetic")
            );
        }
        cases.push(success_case("success", arguments(&good), &expected));
        // Find the finite budget admitting exactly the first failing checked
        // operation. No later branch/node is admitted before its arithmetic error.
        let mut low = 0;
        let mut high = NativeExecutionLimits::default().max_work;
        while low < high {
            let mid = low + (high - low) / 2;
            let limits = NativeExecutionLimits {
                max_work: mid,
                ..NativeExecutionLimits::default()
            };
            match jit_error(&jits[0], &first_bad, limits) {
                NativeMatchExecutionError::Arithmetic { .. } => high = mid,
                NativeMatchExecutionError::Work { .. } => low = mid + 1,
                other => panic!("unexpected error {other:?}"),
            }
        }
        let limits = NativeExecutionLimits {
            max_work: low,
            ..NativeExecutionLimits::default()
        };
        let first = jit_error(&jits[0], &first_bad, limits);
        assert!(
            matches!(first,NativeMatchExecutionError::Arithmetic {origin} if origin.block().is_none())
        );
        let later = jit_error(&jits[0], &later_bad, limits);
        assert!(matches!(later, NativeMatchExecutionError::Work { .. }));
        let overflow = jit_error(&jits[0], &later_bad, NativeExecutionLimits::default());
        assert!(
            matches!(overflow,NativeMatchExecutionError::Arithmetic {origin} if origin.block().is_some())
        );
        for index in 0..2 {
            assert_eq!(jit_error(&jits[index], &first_bad, limits), first);
            assert_eq!(jit_error(&jits[index], &later_bad, limits), later);
            assert_eq!(
                jit_error(&jits[index], &later_bad, NativeExecutionLimits::default()),
                overflow
            );
        }
        cases.push(exact_failure_case(
            "unused-error-before-later-denial",
            with_limit(&format!("--work={low}"), &first_bad),
            &first,
        ));
        cases.push(exact_failure_case(
            "later-node-denial-before-overflow",
            with_limit(&format!("--work={low}"), &later_bad),
            &later,
        ));
        cases.push(exact_failure_case(
            "later-arithmetic-is-atomic",
            arguments(&later_bad),
            &overflow,
        ));
        cases.push(success_case("recovery", arguments(&good), &expected));
    }
    cases
}

/// Only called in the contained worker. Subprocesses inherit its owned group;
/// they do not open another process group that could escape its deadline.
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

fn compiled_feature_closure(metadata: &Value, build_stdout: &[u8]) -> Vec<Value> {
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
        let message: Value = serde_json::from_slice(line).unwrap();
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
        runtime_closure::package(name);
        runtime_closure::features(name, message["features"].as_array().unwrap());
        if seen.insert((
            id.to_owned(),
            message["target"].to_string(),
            message["features"].to_string(),
        )) {
            result.push(json!({"id":id, "name":name, "version":package["version"], "features":message["features"], "target_name":message["target"]["name"], "target_kind":message["target"]["kind"]}));
        }
    }
    assert!(result.iter().any(|unit| unit["id"] == root));
    runtime_closure::required(&seen_packages);
    result.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    result
}

fn review_ir(plan: &NativeMatchArtifactPlan) -> Value {
    json!({
        "revision":format!("{:?}",plan.program_revision()),"entry":plan.entry(),
        "inputs":plan.inputs().iter().map(|input|json!({"id":input.declaration().input().get(),"name":input.declaration().name(),"value":input.value().get(),"kind":format!("{:?}",input.declaration().ty().kind())})).collect::<Vec<_>>(),
        "constants":plan.constants().iter().map(|value|json!({"id":value.value().get(),"value":format!("{:?}",value.scalar())})).collect::<Vec<_>>(),
        "blocks":plan.blocks().iter().map(|block|json!({"origin":format!("{:?}",block.origin()),"action":match block.action() {NativeMatchAction::Primitive{op,operands,output,..}=>json!({"op":format!("{op:?}"),"operands":operands.iter().map(|v|v.get()).collect::<Vec<_>>(),"output":output.get()}),NativeMatchAction::Copy{source,output}=>json!({"copy":source.get(),"output":output.get()}),NativeMatchAction::None=>Value::Null},"terminator":format!("{:?}",block.terminator())})).collect::<Vec<_>>(),
        "outputs":plan.outputs().iter().map(|output|json!({"id":output.output().get(),"name":output.name(),"value":output.value().get(),"kind":format!("{:?}",output.ty().kind()),"interactive_binding":format!("{:?}",output.interactive_binding())})).collect::<Vec<_>>(),
    })
}

#[test]
#[ignore = "internal worker; the outer qualification owns this complete process tree"]
fn canonical_match_link_build_worker() {
    let name = std::env::var("MECH_CANONICAL_MATCH_WORKER_NAME").unwrap();
    assert!(
        matches!(
            name.as_str(),
            "canonical-match-mixed" | "canonical-match-ordered"
        ) || wider_cases::contains(&name)
    );
    let evidence = PathBuf::from(std::env::var_os("MECH_CANONICAL_MATCH_EVIDENCE_DIR").unwrap());
    let root = evidence.join(&name);
    let source = fs::read_to_string(root.join("application.mec")).unwrap();
    // Only the mixed fixture requests real interactive named outputs. The
    // ordered fixture uses ordinary compilation, so its first checked node is
    // genuinely unused by outputs and must nevertheless retain its error.
    let interactive = name == "canonical-match-mixed";
    let (artifacts, result) = compile(&source, interactive);
    let catalog = mech_stdlib::source_catalog();
    let plan = lower_scalar_match_artifact(&artifacts[0], &catalog).unwrap();
    let decoded_plan = lower_scalar_match_artifact(&artifacts[1], &catalog).unwrap();
    assert_eq!(review_ir(&plan), review_ir(&decoded_plan));
    fs::write(
        root.join("typed-ir-review.json"),
        serde_json::to_vec_pretty(&review_ir(&plan)).unwrap(),
    )
    .unwrap();
    let mut cases = reference_cases(&name, &artifacts, result);
    let generated = generate_linked_match_application(
        &workspace(),
        &root.join("project"),
        &name,
        &artifacts[0],
        &catalog,
        source.as_bytes(),
    )
    .unwrap();
    let decoded = generate_linked_match_application(
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
        generated.metadata()["program_revision"],
        decoded.metadata()["program_revision"]
    );
    assert_eq!(
        generated.metadata()["typed_ir_sha256"],
        decoded.metadata()["typed_ir_sha256"]
    );
    let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifacts[0]).unwrap();
    assert_eq!(
        fs::read(generated.root().join("program.mecb")).unwrap(),
        bytecode
    );
    assert_eq!(
        fs::read(generated.root().join("application.mec")).unwrap(),
        source.as_bytes()
    );
    for project in [&generated, &decoded] {
        for (relative, expected) in project.pins() {
            assert_eq!(&pin(&project.root().join(relative)), expected);
        }
    }
    // Use the immutable workspace lock as a resolution seed, retaining the
    // actual resolved lock and complete feature graph before the locked build.
    fs::copy(
        workspace().join("Cargo.lock"),
        generated.root().join("Cargo.lock"),
    )
    .unwrap();
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    if let Some(home) = cargo_home {
        assert!(
            !home.join("config").exists() && !home.join("config.toml").exists(),
            "ambient Cargo configuration is unsupported for this pinned qualification"
        );
    }
    let isolated = generated.root().ancestors().last().unwrap();
    let locked = worker_command(
        CargoInvocation::resolve_lockfile(generated.manifest_path(), true)
            .with_current_dir(isolated)
            .command(),
        &root,
        "metadata",
    );
    assert!(
        locked.status.success(),
        "{}",
        String::from_utf8_lossy(&locked.stderr)
    );
    let metadata: Value = serde_json::from_slice(&locked.stdout).unwrap();
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
        .env("CARGO_BUILD_JOBS", "1");
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
    for (label, project) in [("original", &generated), ("decoded", &decoded)] {
        for (relative, expected) in project.pins() {
            let path = project.root().join(relative);
            assert_eq!(&pin(&path), expected);
            pins.insert(path, expected.clone());
        }
        let path = project.root().join("linked-manifest.json");
        pins.insert(path.clone(), pin(&path));
        fs::write(
            root.join(format!("{label}-generation.json")),
            serde_json::to_vec_pretty(project.metadata()).unwrap(),
        )
        .unwrap();
    }
    for path in [
        root.join("application.mec"),
        root.join("typed-ir-review.json"),
        generated.root().join("Cargo.lock"),
        workspace().join("Cargo.lock"),
        executable.clone(),
        root.join("metadata.stdout"),
        root.join("original-generation.json"),
        root.join("decoded-generation.json"),
    ] {
        pins.insert(path.clone(), pin(&path));
    }
    // Every failure oracle is complete stderr, including the exact checked
    // NodeId/admission phase. Object stack metadata belongs to that object,
    // rather than assuming its compiled frame equals the JIT frame.
    for case in &mut cases {
        if case["diagnostic"].is_null() || case["stderr"].is_string() {
            continue;
        }
        let error = match case["name"].as_str().unwrap() {
            "stack-denial" => NativeMatchExecutionError::Base(NativeExecutionError::StackLimit {
                required: generated.metadata()["object"]["stack_bytes"]
                    .as_u64()
                    .unwrap(),
                limit: 0,
            })
            .to_string(),
            "u64-overflow-input" | "signed-u64-input" | "whitespace-u64-input" => {
                "invalid U64 argument".into()
            }
            "invalid-bool-input" => "invalid Bool argument".into(),
            "invalid-unicode-input" => "argument is not Unicode".into(),
            "missing-input" | "extra-input" => {
                format!("expected {} typed inputs", plan.inputs().len())
            }
            "unknown-option" => "unknown option".into(),
            "invalid-limit" => "invalid limit argument".into(),
            other => panic!("failure oracle missing for {other}"),
        };
        case["stderr"] = json!(format!("{error}\n"));
    }
    let receipt = json!({"name":name,"source_compile_mode":if interactive {"interactive"} else {"ordinary"},"source_sha256":digest(source.as_bytes()),"program_revision":generated.metadata()["program_revision"],"bytecode_sha256":digest(&bytecode),"typed_ir_sha256":generated.metadata()["typed_ir_sha256"],"typed_ir_review_sha256":pin(&root.join("typed-ir-review.json")),"original_and_decoded_pair_equal":true,"generator":generated.metadata(),"generated_runtime_feature_closure":closure,"binary":executable,"binary_sha256":pin(&executable),"build_count":1,"object_emissions":2,"reference":"ordinary canonical resident and JIT original+decoded; all outputs exact; finite native denials compared with JIT, not resident cost policy","benchmark_measurements":0});
    fs::write(
        root.join("application-build.json"),
        serde_json::to_vec_pretty(
            &json!({"executable":executable,"pins":pins,"receipt":receipt,"cases":cases}),
        )
        .unwrap(),
    )
    .unwrap();
}

fn qualify(evidence: &Path, name: &str, source: &str) -> Value {
    let root = evidence.join(name);
    fs::create_dir(&root).unwrap();
    fs::write(root.join("application.mec"), source).unwrap();
    let mut worker = Command::new(std::env::current_exe().unwrap());
    worker
        .args([
            "--ignored",
            "--exact",
            "canonical_match_link_build_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MECH_CANONICAL_MATCH_WORKER_NAME", name);
    let built = bounded(worker, &root, "build-worker", WORKER_DEADLINE);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let evidence_path = root.join("application-build.json");
    let mut data: Value = serde_json::from_slice(&fs::read(&evidence_path).unwrap()).unwrap();
    let executable: PathBuf = serde_json::from_value(data["executable"].take()).unwrap();
    let mut pins: BTreeMap<PathBuf, String> = serde_json::from_value(data["pins"].take()).unwrap();
    pins.insert(evidence_path.clone(), pin(&evidence_path));
    let check_pins = || {
        for (path, expected) in &pins {
            assert_eq!(&pin(path), expected, "changed {}", path.display());
        }
    };
    check_pins();
    let mut records = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let label = case["name"].as_str().unwrap();
        let mut args = case["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| OsString::from(value.as_str().unwrap()))
            .collect::<Vec<_>>();
        if let Some(index) = case["invalid_os_argument"].as_u64() {
            args[index as usize] = invalid_unicode_argument().unwrap();
        }
        let expected = case["stdout"].as_str().unwrap().as_bytes();
        fs::write(root.join(format!("{label}.expected.stdout")), expected).unwrap();
        let mut command = Command::new(&executable);
        command.args(&args);
        let actual = bounded(command, &root, label, PROCESS_DEADLINE);
        assert_eq!(
            actual.stdout,
            expected,
            "{name}/{label}: {}",
            String::from_utf8_lossy(&actual.stderr)
        );
        if let Some(diagnostic) = case["diagnostic"].as_str() {
            assert_eq!(actual.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&actual.stderr);
            assert!(stderr.contains(diagnostic), "{name}/{label}: {stderr}");
            let oracle = case["stderr"]
                .as_str()
                .expect("complete failure stderr oracle");
            fs::write(root.join(format!("{label}.expected.stderr")), oracle).unwrap();
            assert_eq!(
                actual.stderr,
                oracle.as_bytes(),
                "{name}/{label}: exact failure origin/phase"
            );
            assert!(!stderr.contains("panicked"), "{stderr}");
        } else {
            assert!(
                actual.status.success(),
                "{}",
                String::from_utf8_lossy(&actual.stderr)
            );
            assert!(actual.stderr.is_empty());
        }
        check_pins();
        records.push(json!({"name":label,"arguments_os_encoded_hex":args.iter().map(|arg|arg.as_encoded_bytes().iter().map(|b|format!("{b:02x}")).collect::<String>()).collect::<Vec<_>>(),"exit_code":actual.status.code(),"stdout_sha256":digest(&actual.stdout),"stderr_sha256":digest(&actual.stderr),"expected_stdout_sha256":digest(expected),"expected_diagnostic":case["diagnostic"],"expected_stderr_sha256":case["stderr"].as_str().map(|value|digest(value.as_bytes())),"exact_complete_stderr":true,"binary_unchanged":true,"exact_complete_stdout":true}));
    }
    let mut receipt = data["receipt"].take();
    assert_eq!(receipt["source_sha256"], digest(source.as_bytes()));
    receipt["cases"] = json!(records);
    receipt["pins"] = json!(pins);
    receipt["linked"] = json!(true);
    receipt["executed"] = json!(true);
    receipt
}

#[test]
#[ignore = "explicit linked canonical Cranelift qualification; requires clean pinned checkout and fresh external evidence"]
fn linked_canonical_match_applications_are_lazy_bounded_and_atomic() {
    let workspace = workspace().canonicalize().unwrap();
    let commit = std::env::var("MECH_CANONICAL_MATCH_SOURCE_COMMIT").expect("pin immutable HEAD");
    assert_eq!(commit.len(), 40);
    assert!(commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_MATCH_EVIDENCE_DIR")
            .expect("new external evidence directory"),
    );
    assert!(evidence.is_absolute());
    fs::create_dir(&evidence).unwrap();
    let evidence = evidence.canonicalize().unwrap();
    assert!(!evidence.starts_with(&workspace));
    clean(&workspace, &evidence, &commit, "before");
    let lock = pin(&workspace.join("Cargo.lock"));
    let mut applications = vec![
        qualify(&evidence, "canonical-match-mixed", MIXED),
        qualify(&evidence, "canonical-match-ordered", ORDERED),
    ];
    for (name, source) in wider_cases::fixtures() {
        applications.push(qualify(&evidence, &name, &source));
    }
    assert_eq!(pin(&workspace.join("Cargo.lock")), lock);
    clean(&workspace, &evidence, &commit, "after");
    fs::write(evidence.join("qualification.json"),serde_json::to_vec_pretty(&json!({"schema":"mech.canonical-artifact-linked-match-qualification.v1","source_commit":commit,"source_checkout_clean_before_and_after":true,"route":"canonical-artifact-match-cranelift-object","linked":true,"executed":true,"benchmark_measurements":0,"applications":applications})).unwrap()).unwrap();
}
