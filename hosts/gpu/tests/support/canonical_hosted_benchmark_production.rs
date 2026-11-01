use super::*;
use mech_core::FunctionCatalog;
use mech_engine::ProgramArtifact;

pub(super) fn qualify(
    evidence: &Path,
    target: &Path,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    profile: profile::Profile,
) -> Value {
    let directory = evidence.join("production-control");
    fs::create_dir(&directory).unwrap();
    let name = "canonical-policy-production";
    let project = generate_linked_hosted_program_application(
        &workspace(),
        &directory.join("project"),
        name,
        artifact,
        catalog,
        FULL.as_bytes(),
    )
    .unwrap();
    let build::Built {
        executable,
        units,
        compiler_profiles,
    } = build::project(&project, name, target, &directory, profile);
    assert!(
        units
            .iter()
            .filter(|unit| unit["name"] == "mech-gpu")
            .all(|unit| !unit["features"]
                .as_array()
                .unwrap()
                .contains(&json!("canonical-native-benchmark-policy-runtime")))
    );
    let binary = directory.join("native-executable");
    fs::copy(&executable, &binary).unwrap();
    let binary_pin = pin(&binary);
    let input = directory.join("input");
    fs::write(&input, GGTA_INPUT).unwrap();
    let mut cases = Vec::new();
    for label in ["ordinary", "reject-policy-field", "reject-legacy-trusted"] {
        let mut launch = launch();
        if label == "reject-policy-field" {
            launch["benchmark_policy"] = json!("trusted");
        }
        if label == "reject-legacy-trusted" {
            launch["grants"].as_array_mut().unwrap().remove(0);
        }
        let launch_path = directory.join(format!("{label}.launch.json"));
        let report_path = directory.join(format!("{label}.report.json"));
        let launch_bytes = serde_json::to_vec_pretty(&launch).unwrap();
        fs::write(&launch_path, &launch_bytes).unwrap();
        let mut command = Command::new(&binary);
        command
            .arg(&launch_path)
            .arg(&report_path)
            .arg("--")
            .env("MECH_NATIVE_SECURITY", "trusted");
        let output = with_input(&mut command, &directory, label, &input);
        assert_eq!(
            output.status.success(),
            label == "ordinary",
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if label == "ordinary" {
            assert_eq!(output.stdout, GGTA_OUTPUT.as_bytes());
            assert!(output.stderr.is_empty());
            let report: Value = serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
            assert_eq!(report["policy"], "secured");
            assert_eq!(
                report["route"],
                "canonical-artifact-program-object-secured-cli"
            );
            assert_eq!(report["shutdown"], "completed");
        } else {
            assert!(output.stdout.is_empty());
            assert!(!report_path.exists());
            if label == "reject-policy-field" {
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("unknown field `benchmark_policy`")
                );
            } else {
                assert!(!output.stderr.is_empty());
            }
        }
        assert_eq!(pin(&binary), binary_pin);
        cases.push(json!({"label":label,"exit_code":output.status.code(),"accepted":output.status.success(),
            "launch_sha256":hash(&launch_bytes),"stdout_sha256":hash(&output.stdout),"stderr_sha256":hash(&output.stderr)}));
    }
    for (relative, expected) in project.pins() {
        assert_eq!(&pin(&project.root().join(relative)), expected);
    }
    json!({"binary":binary,"binary_sha256":binary_pin,"cases":cases,"runtime_units":units,"compiler_profiles":compiler_profiles})
}
