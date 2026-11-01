//! Actual same-Object executable qualification, not a benchmark.
use super::{artifacts_mode, inputs, reference};
use mech_build::{CargoInvocation, parse_cargo_build_messages};
use mech_core::snapshot::F64Bits;
use mech_engine::native_artifact::{
    NativeScalarKind, NativeScalarValue, lower_scalar_iteration_artifact,
};
use mech_gpu::native_artifact::{
    JitIterationArtifact, NativeExecutionError, NativeExecutionLimits,
    NativeIterationExecutionError, generate_linked_iteration_application,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Cursor,
    path::PathBuf,
    process::Command,
};

#[path = "linked_process.rs"]
mod linked_process;
use linked_process::{
    assert_no_ambient_cargo_config, bounded, clean, closure, hash, pin, workspace,
};

const SOURCE: &str = "inc(x<u64>) => (next<u64>) := next := x + 1u64.\ndec(x<i64>) => (next<i64>) := next := x - 1i64.\nhalf(x<f64>) => (next<f64>) := next := x * 0.5.\nidentity(x<bool>) => (next<bool>) := next := x.\nunsigned := seed<u64> |25> inc\nsigned := start<i64> |3> dec\nfloating := real<f64> |2> half\nboolean := enabled<bool> |0> identity\n";

fn named(
    seed: u64,
    start: i64,
    real: F64Bits,
    enabled: bool,
) -> [(&'static str, NativeScalarValue); 4] {
    use NativeScalarValue::*;
    [
        ("seed", U64(seed)),
        ("start", I64(start)),
        ("real", F64(real)),
        ("enabled", Bool(enabled)),
    ]
}
fn arguments(values: &[NativeScalarValue]) -> Vec<String> {
    values
        .iter()
        .map(|value| match value {
            NativeScalarValue::Bool(v) => v.to_string(),
            NativeScalarValue::U64(v) => v.to_string(),
            NativeScalarValue::I64(v) => v.to_string(),
            NativeScalarValue::F64(v) => format!("f64:{:016x}", v.bits()),
        })
        .collect()
}

#[test]
fn four_typed_sibling_loops_have_independent_carriers_and_literal_outputs() {
    use NativeScalarValue::*;
    let (variants, _) = artifacts_mode(SOURCE, true);
    for artifact in variants {
        let args = inputs(&artifact, &named(5, -5, F64Bits::from_f64(8.0), true));
        // Public interactive compilation retains the final value and aliases
        // as well as the four named bindings. Preserve and check every output.
        let expected = vec![
            Bool(true),
            Bool(true),
            Bool(true),
            F64(F64Bits::from_f64(2.0)),
            I64(-8),
            U64(30),
        ];
        assert_eq!(reference(&artifact, &args).unwrap(), expected);
        let plan =
            lower_scalar_iteration_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.iterations().len(), 4);
        assert!(
            plan.iterations()
                .iter()
                .all(|loop_| loop_.parent().is_none())
        );
        assert_eq!(
            plan.iterations()
                .iter()
                .map(|loop_| loop_.carrier().get())
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        assert_eq!(
            plan.outputs()
                .iter()
                .map(|out| out.ty().kind())
                .collect::<Vec<_>>(),
            [
                NativeScalarKind::Bool,
                NativeScalarKind::Bool,
                NativeScalarKind::Bool,
                NativeScalarKind::F64,
                NativeScalarKind::I64,
                NativeScalarKind::U64,
            ]
        );
        let jit = JitIterationArtifact::compile(&plan).unwrap();
        let mut actual = vec![U64(99); plan.outputs().len()];
        jit.evaluate_into(&args, &mut actual, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
#[ignore = "explicit clean-source linked qualification; builds one runtime-only executable and records no timings"]
fn one_linked_iteration_executable_preserves_exact_typed_loops_and_denials() {
    let commit = std::env::var("MECH_CANONICAL_ITERATION_SOURCE_COMMIT")
        .expect("exact source commit required");
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_ITERATION_EVIDENCE_DIR")
            .expect("fresh absolute evidence directory required"),
    );
    assert!(evidence.is_absolute() && !evidence.exists());
    fs::create_dir_all(&evidence).unwrap();
    clean(&evidence, &commit, "before");
    fs::write(evidence.join("application.mec"), SOURCE).unwrap();
    let (variants, _) = artifacts_mode(SOURCE, true);
    let catalog = mech_stdlib::source_catalog();
    let generated = generate_linked_iteration_application(
        &workspace(),
        &evidence.join("project"),
        "canonical-iteration-linked",
        &variants[0],
        &catalog,
        SOURCE.as_bytes(),
    )
    .unwrap();
    let decoded = generate_linked_iteration_application(
        &workspace(),
        &evidence.join("decoded-project"),
        "canonical-iteration-linked",
        &variants[1],
        &catalog,
        SOURCE.as_bytes(),
    )
    .unwrap();
    assert_eq!(generated.pins(), decoded.pins());
    assert_eq!(
        generated.metadata()["typed_ir_sha256"],
        decoded.metadata()["typed_ir_sha256"]
    );
    let mut pins = BTreeMap::new();
    for project in [&generated, &decoded] {
        for (relative, expected) in project.pins() {
            let path = project.root().join(relative);
            assert_eq!(&pin(&path), expected);
            pins.insert(path, expected.clone());
        }
    }
    fs::copy(
        workspace().join("Cargo.lock"),
        generated.root().join("Cargo.lock"),
    )
    .unwrap();
    assert_no_ambient_cargo_config();
    let isolated = generated.root().ancestors().last().unwrap();
    let resolved = bounded(
        &mut CargoInvocation::resolve_lockfile(generated.manifest_path(), true)
            .with_current_dir(isolated)
            .command(),
        &evidence,
        "metadata",
        120,
    );
    assert!(
        resolved.status.success(),
        "{}",
        String::from_utf8_lossy(&resolved.stderr)
    );
    let metadata: Value = serde_json::from_slice(&resolved.stdout).unwrap();
    let mut command = CargoInvocation::build(
        generated.manifest_path(),
        "canonical-iteration-linked",
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
    let built = bounded(&mut command, &evidence, "build", 15 * 60);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let messages = parse_cargo_build_messages(Cursor::new(&built.stdout)).unwrap();
    assert_eq!(messages.build_finished, Some(true));
    let units = closure(&metadata, &built.stdout);
    let binary = messages
        .select_binary("canonical-iteration-linked")
        .unwrap()
        .executable;
    let binary_pin = pin(&binary);
    let plan = lower_scalar_iteration_artifact(&variants[0], &catalog).unwrap();
    let jit = JitIterationArtifact::compile(&plan).unwrap();
    let standard = named(5, -5, F64Bits::from_f64(8.0), true);
    let standard_args = inputs(&variants[0], &standard);
    let mut scratch = vec![NativeScalarValue::U64(99); plan.outputs().len()];
    let report = jit
        .evaluate_into(
            &standard_args,
            &mut scratch,
            NativeExecutionLimits::default(),
            None,
        )
        .unwrap();
    let mut cases = Vec::new();
    for (label, values, golden) in [
        (
            "ordinary",
            standard,
            "true\ntrue\ntrue\nf64:4000000000000000\n-8\n30\n",
        ),
        (
            "wide-and-negative-zero",
            named(9_007_199_254_740_993, -9, F64Bits::from_f64(-0.0), false),
            "false\nfalse\nfalse\nf64:8000000000000000\n-12\n9007199254741018\n",
        ),
        (
            "another-valid-input",
            named(0, 0, F64Bits::from_f64(4.0), true),
            "true\ntrue\ntrue\nf64:3ff0000000000000\n-3\n25\n",
        ),
    ] {
        for artifact in &variants {
            let args = inputs(artifact, &values);
            let reference = reference(artifact, &args).unwrap();
            let paired = lower_scalar_iteration_artifact(artifact, &catalog).unwrap();
            let entry = JitIterationArtifact::compile(&paired).unwrap();
            let mut actual = vec![NativeScalarValue::U64(99); paired.outputs().len()];
            entry
                .evaluate_into(&args, &mut actual, NativeExecutionLimits::default(), None)
                .unwrap();
            assert_eq!(actual, reference);
        }
        let argv = arguments(&inputs(&variants[0], &values));
        let result = bounded(Command::new(&binary).args(&argv), &evidence, label, 15);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(result.stdout, golden.as_bytes());
        assert!(result.stderr.is_empty());
        cases.push(json!({"name":label,"argv":argv,"stdout":golden,"exit":0}));
    }
    let argv = arguments(&standard_args);
    for (label, option) in [
        ("work-denial", "--work=0".to_owned()),
        ("completion-denial", format!("--work={}", report.work - 1)),
        ("transport-denial", "--memory=0".to_owned()),
        ("stack-denial", "--stack=0".to_owned()),
        ("shared-budget-denial", "--budget=0".to_owned()),
    ] {
        let before = scratch.clone();
        let expected = if label == "stack-denial" {
            NativeIterationExecutionError::Base(NativeExecutionError::StackLimit {
                required: generated.metadata()["object"]["stack_bytes"]
                    .as_u64()
                    .unwrap(),
                limit: 0,
            })
        } else {
            let mut limits = NativeExecutionLimits::default();
            let shared = if label == "shared-budget-denial" {
                Some(mech_core::ManagedMemoryBudget::new(0))
            } else {
                None
            };
            match label {
                "work-denial" => limits.max_work = 0,
                "completion-denial" => limits.max_work = report.work - 1,
                "transport-denial" => limits.max_transport_bytes = 0,
                "shared-budget-denial" => {}
                _ => unreachable!(),
            }
            jit.evaluate_into(&standard_args, &mut scratch, limits, shared.as_ref())
                .unwrap_err()
        }
        .to_string()
            + "\n";
        assert_eq!(scratch, before);
        let result = bounded(
            Command::new(&binary).arg(&option).args(&argv),
            &evidence,
            label,
            15,
        );
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, expected.as_bytes());
        cases.push(json!({"name":label,"option":option,"argv":argv,"stdout":"","stderr":String::from_utf8(result.stderr).unwrap(),"exit":result.status.code()}));
    }
    for (label, values) in [
        (
            "unsigned-overflow",
            named(u64::MAX - 24, -5, F64Bits::from_f64(8.0), true),
        ),
        (
            "signed-underflow",
            named(5, i64::MIN + 2, F64Bits::from_f64(8.0), true),
        ),
    ] {
        let args = inputs(&variants[0], &values);
        for artifact in &variants {
            assert!(reference(artifact, &inputs(artifact, &values)).is_err());
        }
        let expected = jit
            .evaluate_into(&args, &mut scratch, NativeExecutionLimits::default(), None)
            .unwrap_err()
            .to_string()
            + "\n";
        let argv = arguments(&args);
        let result = bounded(Command::new(&binary).args(&argv), &evidence, label, 15);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, expected.as_bytes());
        cases.push(json!({"name":label,"argv":argv,"stdout":"","stderr":expected,"exit":result.status.code()}));
    }
    for (label, extra) in [("missing-input", false), ("extra-input", true)] {
        let mut malformed = argv.clone();
        if extra {
            malformed.push("1".into());
        } else {
            malformed.pop();
        }
        let result = bounded(Command::new(&binary).args(&malformed), &evidence, label, 15);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, b"expected 4 typed inputs\n");
        cases.push(json!({"name":label,"argv":malformed,"stdout":"","stderr":"expected 4 typed inputs\n","exit":result.status.code()}));
    }
    for (label, input, invalid, diagnostic) in [
        ("invalid-bool", "enabled", "TRUE", "invalid Bool argument\n"),
        ("invalid-unsigned", "seed", "-1", "invalid U64 argument\n"),
        ("invalid-signed", "start", "+1", "invalid I64 argument\n"),
        (
            "invalid-floating",
            "real",
            "1.0",
            "invalid F64 bit argument\n",
        ),
    ] {
        let ordinal = variants[0]
            .inputs()
            .iter()
            .position(|declaration| {
                mech_engine::decode_source_input_name(&declaration.name).unwrap() == input
            })
            .unwrap();
        let mut malformed = argv.clone();
        malformed[ordinal] = invalid.to_owned();
        let result = bounded(Command::new(&binary).args(&malformed), &evidence, label, 15);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, diagnostic.as_bytes());
        cases.push(json!({"name":label,"argv":malformed,"stdout":"","stderr":diagnostic,"exit":result.status.code()}));
    }
    let exact = bounded(
        Command::new(&binary)
            .arg(format!("--work={}", report.work))
            .args(&argv),
        &evidence,
        "exact-work-and-recovery",
        15,
    );
    assert!(exact.status.success());
    assert_eq!(
        exact.stdout,
        b"true\ntrue\ntrue\nf64:4000000000000000\n-8\n30\n"
    );
    assert!(exact.stderr.is_empty());
    cases.push(json!({"name":"exact-work-and-recovery","work":report.work,"stdout":String::from_utf8(exact.stdout).unwrap(),"exit":0}));
    for (path, expected) in &pins {
        assert_eq!(&pin(path), expected);
    }
    assert_eq!(pin(&binary), binary_pin);
    clean(&evidence, &commit, "after");
    let receipt = json!({"schema":"mech.canonical-iteration-linked-qualification.v1","source_commit":commit,"source_sha256":hash(SOURCE.as_bytes()),"generator":generated.metadata(),"decoded_generation":decoded.metadata(),"generated_pins":pins,"workspace_lock_sha256":pin(&workspace().join("Cargo.lock")),"lock_sha256":pin(&generated.root().join("Cargo.lock")),"binary":binary,"binary_sha256":binary_pin,"compiled_units":units,"cases":cases,"build_count":1,"object_emissions":2,"same_binary":true,"linked":true,"executed":true,"benchmark_measurements":0,"scope":"fixed scalar typed-loop linked lifecycle only; no matrix/collection/FSM/resource/full-workload qualification"});
    fs::write(
        evidence.join("qualification.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
