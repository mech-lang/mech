//! Same checked Object and launcher; an explicit experiment, never a CLI default.
use super::*;
use mech_gpu::native_artifact::generate_linked_benchmark_program_application;

#[path = "canonical_hosted_benchmark_build.rs"]
mod build;
#[path = "canonical_hosted_benchmark_cases.rs"]
mod cases;
#[path = "canonical_hosted_benchmark_driver.rs"]
mod driver;
#[path = "canonical_hosted_benchmark_production.rs"]
mod production;

#[test]
fn benchmark_generator_preserves_the_original_object_and_isolates_policy_controls() {
    let (variants, catalog) = artifacts(FULL);
    let temporary = tempfile::tempdir().unwrap();
    let mut benchmark_pins = None;
    for (index, artifact) in variants.iter().enumerate() {
        let secured = generate_linked_hosted_program_application(
            &workspace(),
            &temporary.path().join(format!("secured-{index}")),
            "canonical-policy-controls",
            artifact,
            &catalog,
            FULL.as_bytes(),
        )
        .unwrap();
        let benchmark = generate_linked_benchmark_program_application(
            &workspace(),
            &temporary.path().join(format!("benchmark-{index}")),
            "canonical-policy-controls",
            artifact,
            &catalog,
            FULL.as_bytes(),
        )
        .unwrap();
        for relative in [
            "application.mec",
            "program.mecb",
            "program.o",
            "object-identity.bin",
        ] {
            assert_eq!(
                fs::read(secured.root().join(relative)).unwrap(),
                fs::read(benchmark.root().join(relative)).unwrap()
            );
        }
        for project in [&secured, &benchmark] {
            for (relative, expected) in project.pins() {
                assert_eq!(&pin(&project.root().join(relative)), expected);
            }
            assert_eq!(project.metadata()["executed"], false);
            assert_eq!(project.metadata()["timing_samples"], 0);
            let main = fs::read_to_string(project.root().join("src/main.rs")).unwrap();
            assert!(!main.contains("@BENCHMARK_") && !main.contains("@HOSTED_"));
            for same_boundary in [
                "with_canonical_native_resources_phased",
                "linked.prepare_hosted",
                "session.resume_hosted_into",
                "runtime.shutdown()",
            ] {
                assert!(main.contains(same_boundary));
            }
        }
        let normal = fs::read_to_string(secured.root().join("src/main.rs")).unwrap();
        assert!(!normal.contains("NativeBenchmark") && !normal.contains("benchmark_policy"));
        assert!(normal.contains("#[serde(deny_unknown_fields)]"));
        let experiment = fs::read_to_string(benchmark.root().join("src/main.rs")).unwrap();
        assert!(experiment.contains("benchmark_policy: String,"));
        assert!(experiment.contains("NativeBenchmarkPolicy::parse(&launch.benchmark_policy)"));
        assert!(experiment.contains(".capability_kernel(NativeBenchmarkCapabilityKernel::new("));
        assert!(!experiment.contains("MECH_NATIVE_SECURITY"));
        assert_eq!(
            secured.metadata()["route"],
            "canonical-artifact-program-object-secured-cli"
        );
        assert_eq!(
            benchmark.metadata()["route"],
            "canonical-artifact-program-object-benchmark-cli"
        );
        let cargo = fs::read_to_string(benchmark.manifest_path()).unwrap();
        assert!(cargo.contains("features = [\"canonical-native-benchmark-policy-runtime\"]"));
        assert!(
            !fs::read_to_string(secured.manifest_path())
                .unwrap()
                .contains("benchmark-policy")
        );
        if let Some(pins) = &benchmark_pins {
            assert_eq!(benchmark.pins(), pins);
        } else {
            benchmark_pins = Some(benchmark.pins().clone());
        }
    }
    let (inputs, catalog) = artifacts("unbound<u64>\n");
    for (index, artifact) in inputs.iter().enumerate() {
        let directory = temporary.path().join(format!("unbound-{index}"));
        assert!(
            generate_linked_benchmark_program_application(
                &workspace(),
                &directory,
                "canonical-policy-controls",
                artifact,
                &catalog,
                b"unpaired provenance",
            )
            .is_err()
        );
        assert!(!directory.exists());
    }
}

#[test]
#[ignore = "explicit clean-source policy qualification; optional full-input phase"]
fn complete_original_and_decoded_knucleotide_same_binary_policy_controls() {
    let profile =
        profile::Profile::parse(std::env::var_os("MECH_CANONICAL_HOSTED_PROFILE").as_deref())
            .unwrap();
    let commit =
        std::env::var("MECH_CANONICAL_POLICY_SOURCE_COMMIT").expect("exact source commit required");
    let evidence = PathBuf::from(
        std::env::var_os("MECH_CANONICAL_POLICY_EVIDENCE_DIR")
            .expect("fresh absolute policy evidence directory required"),
    );
    assert!(evidence.is_absolute() && !evidence.exists());
    fs::create_dir_all(&evidence).unwrap();
    clean(&evidence, &commit, "before");
    assert_no_ambient_cargo_config();
    let (variants, catalog) = artifacts(FULL);
    let binary_name = "canonical-policy-knucleotide";
    let projects = variants
        .iter()
        .enumerate()
        .map(|(index, artifact)| {
            generate_linked_benchmark_program_application(
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
    let target = evidence.join("cargo-target");
    let mut results = Vec::new();
    for (index, project) in projects.iter().enumerate() {
        let directory = evidence.join(format!("variant-{index}"));
        fs::create_dir(&directory).unwrap();
        for (relative, expected) in project.pins() {
            assert_eq!(&pin(&project.root().join(relative)), expected);
        }
        let build::Built {
            executable: built_binary,
            units,
            compiler_profiles,
        } = build::project(project, binary_name, &target, &directory, profile);
        assert!(units.iter().any(|unit| {
            unit["name"] == "mech-gpu"
                && unit["features"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("canonical-native-benchmark-policy-runtime"))
        }));
        let binary = directory.join("native-executable");
        fs::copy(&built_binary, &binary).unwrap();
        let binary_pin = pin(&binary);
        let generated_template_tests = template_tests::qualify(
            &project.manifest_path(),
            binary_name,
            &target,
            &directory,
            profile.release(),
        );
        assert_eq!(pin(&built_binary), binary_pin);
        let cases = cases::qualify(&binary, &directory, &binary_pin);
        let pin_rejections = if index == 0 {
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
        assert_eq!(pin(&built_binary), binary_pin);
        assert_eq!(pin(&binary), binary_pin);
        for (relative, expected) in project.pins() {
            assert_eq!(&pin(&project.root().join(relative)), expected);
        }
        let revision = variants[index]
            .revision()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        results.push(json!({"variant":index, "binary":binary, "binary_sha256":binary_pin,
            "rust_profile":profile.name(),"program_revision":revision,
            "runtime_units":units, "compiler_profiles":compiler_profiles, "cases":cases,
            "generated_template_tests":generated_template_tests, "pin_rejections":pin_rejections,
            "source_sha256":hash(FULL.as_bytes()), "bytecode_sha256":pin(&project.root().join("program.mecb")),
            "object_sha256":pin(&project.root().join("program.o")), "identity_sha256":pin(&project.root().join("object-identity.bin"))}));
    }
    assert_eq!(results[0]["binary_sha256"], results[1]["binary_sha256"]);
    let production = production::qualify(&evidence, &target, &variants[0], &catalog, profile);
    let driver_control = driver::qualify(&evidence, &results, profile);
    // Persist completed small controls before optional large work can fail.
    let mut receipt = json!({
        "schema":"mech.canonical-native-benchmark-policy-qualification.v1", "source_commit":commit,
        "results":results, "production_control":production, "rust_profile":profile.name(), "object_optimization":"none", "whole_program_optimized":false,
        "driver_protocol_control":driver_control,"full_input":null,"timing_samples":0,
        "full_phase_status":"not-completed",
        "scope":"complete original/decoded canonical artifacts; each identical executable runs both explicit policies through the same checked Object, real CLI provider and phased native lifecycle; trusted bypasses resource/operation matching, not all safety overhead"
    });
    let result_path = evidence.join("benchmark-policy-result.json");
    fs::write(&result_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let full = full::run_comparison(profile, &evidence, &results);
    receipt["timing_samples"] = json!(full["timing_samples"].as_u64().unwrap_or(0));
    receipt["full_phase_status"] = json!(if full.is_null() {
        "not-configured"
    } else {
        "qualified"
    });
    receipt["full_input"] = full;
    fs::write(result_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    clean(&evidence, &commit, "after");
}
