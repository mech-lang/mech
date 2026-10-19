//! Explicit public-command qualification; no direct generator substitute or timing.
#![cfg(all(
    feature = "cranelift_build",
    any(feature = "full_source", feature = "full_compiler"),
    any(target_os = "linux", target_os = "macos"),
    any(target_arch = "x86_64", target_arch = "aarch64"),
))]
#[path = "support/canonical_native_build.rs"]
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{
    bounded, case_directory, cli, complete, hash, invoke, launch, project, success, validate_report,
};

#[test]
#[ignore = "explicit serial public CLI builds and real secured executions; no timings"]
fn public_source_and_bytecode_builds_run_unchanged_knucleotide_without_fallback() {
    let temporary = case_directory("source-bytecode-");
    let root = temporary.path();
    let source = root.join("knucleotide.mec");
    fs::write(
        &source,
        include_str!("../hosts/gpu/tests/fixtures/native/knucleotide-idiomatic.mec"),
    )
    .unwrap();
    let bytecode = root.join("knucleotide.mecb");
    success(
        &bounded(
            &mut cli(root, &source, &bytecode, false),
            root,
            "bytecode",
            120,
        ),
        "bytecode",
    );
    let input = root.join("input.fasta");
    fs::write(&input, b">ONE\nAAAA\n>THREE\nGGTA\n>FOUR\nTTTT\n").unwrap();
    let expected: &[u8] = b"G 50.000\nA 25.000\nT 25.000\n\nGG 33.333\nGT 33.333\nTA 33.333\n\n1\tGGT\n1\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n";
    let mut identities = Vec::new();
    for (variant, input_program) in [&source, &bytecode].into_iter().enumerate() {
        let output = root.join(format!("native-{variant}"));
        success(
            &bounded(
                &mut cli(root, input_program, &output, true),
                root,
                &format!("build-{variant}"),
                1800,
            ),
            "canonical native build",
        );
        let project = project(&output);
        let manifest: Value =
            serde_json::from_slice(&fs::read(project.join("linked-manifest.json")).unwrap())
                .unwrap();
        for (relative, pin) in manifest["pins"].as_object().unwrap() {
            assert_eq!(
                hash(&fs::read(project.join(relative)).unwrap()),
                pin.as_str().unwrap()
            );
        }
        let provenance: Value =
            serde_json::from_slice(&fs::read(project.join("application.mec")).unwrap()).unwrap();
        assert_eq!(provenance["provenance_only"], true);
        assert_eq!(
            provenance["source_pairing"],
            if variant == 0 {
                "retained-canonical-compilation"
            } else {
                "source-unpaired-bytecode"
            }
        );
        if variant == 1 {
            assert_eq!(provenance["roots"], json!([]));
            assert_eq!(provenance["dependencies"], json!([]));
        }
        identities.push(fs::read(project.join("object-identity.bin")).unwrap());
        let launch_bytes = serde_json::to_vec(&launch()).unwrap();
        let policy = root.join(format!("launch-{variant}.json"));
        fs::write(&policy, &launch_bytes).unwrap();
        let report = root.join(format!("report-{variant}.json"));
        let run = bounded(
            &mut invoke(&output, &policy, &report, &input),
            root,
            &format!("run-{variant}"),
            120,
        );
        success(&run, "secured native execution");
        assert_eq!(run.stdout, expected);
        assert!(run.stderr.is_empty());
        let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
        validate_report(&report, &manifest, &launch_bytes);
        let mut denied = launch();
        denied["grants"] = json!([{"target":"cli/stdout","operations":["write"],"paths":["text"]}]);
        let denied_path = root.join(format!("denied-{variant}.json"));
        fs::write(&denied_path, serde_json::to_vec(&denied).unwrap()).unwrap();
        let report = root.join(format!("denied-report-{variant}.json"));
        let mut command = invoke(&output, &denied_path, &report, &input);
        // A legacy environment variable is not authority in this secured shell.
        command.env("MECH_NATIVE_SECURITY", "trusted");
        let run = bounded(&mut command, root, &format!("denied-{variant}"), 120);
        assert!(!run.status.success());
        assert!(run.stdout.is_empty());
        assert!(!run.stderr.is_empty());
        assert!(!report.exists());
    }
    assert_eq!(identities[0], identities[1]);
    complete(root);
}

#[test]
#[ignore = "explicit public CLI build and unused-requirement authority check; no timings"]
fn public_native_build_retains_unused_observations_without_printing_root_values() {
    let temporary = case_directory("unused-observation-");
    let root = temporary.path();
    let source = root.join("unused.mec");
    fs::write(
        &source,
        "+> @unused := cli/stdin\nignored := @unused/text\n1u64\n",
    )
    .unwrap();
    let output = root.join("unused");
    success(
        &bounded(
            &mut cli(root, &source, &output, true),
            root,
            "unused-build",
            1800,
        ),
        "unused build",
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(project(&output).join("linked-manifest.json")).unwrap())
            .unwrap();
    let input = root.join("empty");
    fs::write(&input, b"").unwrap();
    let policy = root.join("denied-launch.json");
    let mut launch = launch();
    launch["grants"] = json!([]);
    fs::write(&policy, serde_json::to_vec(&launch).unwrap()).unwrap();
    let report = root.join("denied.json");
    let denied = bounded(
        &mut invoke(&output, &policy, &report, &input),
        root,
        "unused-denied",
        120,
    );
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    assert!(!report.exists());
    launch["grants"] = json!([{"target":"cli/stdin","operations":["read"],"paths":["text"]}]);
    let launch_bytes = serde_json::to_vec(&launch).unwrap();
    let policy = root.join("accepted-launch.json");
    fs::write(&policy, &launch_bytes).unwrap();
    let report = root.join("accepted.json");
    let accepted = bounded(
        &mut invoke(&output, &policy, &report, &input),
        root,
        "unused-accepted",
        120,
    );
    success(&accepted, "unused authority");
    assert!(accepted.stdout.is_empty() && accepted.stderr.is_empty());
    let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    validate_report(&report, &manifest, &launch_bytes);
    assert_eq!(report["requirements"], 1);
    assert_eq!(report["observations"], 1);
    assert_eq!(report["effects"], 0);
    complete(root);
}

#[test]
#[ignore = "explicit public CLI build of a zero-resource hosted program; no timings"]
fn public_closed_native_program_uses_same_shell_with_zero_grants_and_zero_ports() {
    let temporary = case_directory("zero-resources-");
    let root = temporary.path();
    let source = root.join("closed.mec");
    fs::write(&source, "answer := 7u64\nanswer + 5u64\n").unwrap();
    let output = root.join("closed");
    success(
        &bounded(
            &mut cli(root, &source, &output, true),
            root,
            "closed-build",
            1800,
        ),
        "closed build",
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(project(&output).join("linked-manifest.json")).unwrap())
            .unwrap();
    let input = root.join("empty");
    fs::write(&input, b"").unwrap();
    let mut launch = launch();
    launch["grants"] = json!([]);
    launch["max_ports"] = json!(0);
    launch["input_text_bytes"] = json!(0);
    launch["output_text_bytes"] = json!(0);
    let launch_bytes = serde_json::to_vec(&launch).unwrap();
    let policy = root.join("launch.json");
    fs::write(&policy, &launch_bytes).unwrap();
    let report = root.join("closed-report.json");
    let accepted = bounded(
        &mut invoke(&output, &policy, &report, &input),
        root,
        "closed-run",
        120,
    );
    success(&accepted, "closed native execution");
    assert!(accepted.stdout.is_empty() && accepted.stderr.is_empty());
    let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    validate_report(&report, &manifest, &launch_bytes);
    assert_eq!(report["requirements"], 0);
    assert_eq!(report["observations"], 0);
    assert_eq!(report["effects"], 0);
    assert!(report["native_work"].as_u64().unwrap() > 0);
    complete(root);
}
