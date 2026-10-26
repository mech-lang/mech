use super::*;

pub(super) fn qualify(binary: &Path, directory: &Path, binary_pin: &str) -> Value {
    let mut cases = Vec::new();
    let mut baseline: Option<Value> = None;
    for mode in ["secured", "trusted"] {
        for label in [
            "ordinary",
            "missing-read-grant",
            "missing-write-grant",
            "no-grants",
            "work-denial",
            "memory-denial",
            "input-cap-denial",
            "output-cap-denial",
            "invalid-utf8",
            "missing-policy",
            "invalid-policy",
        ] {
            let directory = directory.join(format!("{mode}-{label}"));
            fs::create_dir(&directory).unwrap();
            let mut launch = launch();
            launch["benchmark_policy"] = json!(mode);
            let input: &[u8] = if label == "invalid-utf8" {
                b"\xff"
            } else {
                GGTA_INPUT
            };
            match label {
                "missing-read-grant" => {
                    launch["grants"].as_array_mut().unwrap().remove(0);
                }
                "missing-write-grant" => {
                    launch["grants"].as_array_mut().unwrap().remove(1);
                }
                "no-grants" => launch["grants"] = json!([]),
                "work-denial" => launch["work"] = json!(1),
                "memory-denial" => launch["memory_bytes"] = json!(0),
                "input-cap-denial" => launch["input_text_bytes"] = json!(input.len() - 1),
                "output-cap-denial" => launch["output_text_bytes"] = json!(GGTA_OUTPUT.len() - 1),
                "missing-policy" => {
                    launch.as_object_mut().unwrap().remove("benchmark_policy");
                }
                "invalid-policy" => launch["benchmark_policy"] = json!("Trusted"),
                _ => (),
            }
            let accepted = label == "ordinary"
                || (mode == "trusted"
                    && matches!(label, "missing-read-grant" | "missing-write-grant"));
            let launch_path = directory.join("launch.json");
            let input_path = directory.join("input");
            let report_path = directory.join("report.json");
            let launch_bytes = serde_json::to_vec_pretty(&launch).unwrap();
            fs::write(&launch_path, &launch_bytes).unwrap();
            fs::write(&input_path, input).unwrap();
            let mut command = Command::new(binary);
            command.arg(&launch_path).arg(&report_path).arg("--");
            // The legacy environment must neither enable nor disable this
            // explicit JSON choice, including malformed/omitted choices.
            command.env(
                "MECH_NATIVE_SECURITY",
                if mode == "secured" {
                    "trusted"
                } else {
                    "secured"
                },
            );
            let output = with_input(&mut command, &directory, "run", &input_path);
            assert_eq!(
                output.status.success(),
                accepted,
                "{mode}/{label}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report = if accepted {
                assert_eq!(output.stdout, GGTA_OUTPUT.as_bytes());
                assert!(output.stderr.is_empty());
                let report: Value =
                    serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
                assert_eq!(
                    report["route"],
                    "canonical-artifact-program-object-benchmark-cli"
                );
                assert_eq!(report["policy"], mode);
                for (field, expected) in [
                    ("native_aot", true),
                    ("source_parser", false),
                    ("jit", false),
                    ("resident_body_evaluation", false),
                ] {
                    assert_eq!(report[field], expected);
                }
                assert_eq!(report["hosted_phases"], "admit-activate-capture-resume");
                assert_eq!(report["shutdown"], "completed");
                assert_eq!(report["requirements"], 2);
                assert_eq!(report["observations"], 1);
                assert_eq!(report["effects"], 1);
                if label == "ordinary" {
                    if let Some(baseline) = &baseline {
                        for field in [
                            "program_revision",
                            "native_work",
                            "native_comparison_work",
                            "native_visits",
                            "native_transport_bytes",
                            "host_result_carrier_bytes",
                        ] {
                            assert_eq!(
                                report[field], baseline[field],
                                "policy changed native accounting: {field}"
                            );
                        }
                    } else {
                        baseline = Some(report.clone());
                    }
                }
                report
            } else {
                assert!(
                    output.stdout.is_empty(),
                    "{mode}/{label}: denied output escaped"
                );
                assert!(!output.stderr.is_empty());
                assert!(!report_path.exists());
                Value::Null
            };
            assert_eq!(pin(binary), binary_pin);
            cases.push(json!({"policy":mode, "label":label, "accepted":accepted, "exit_code":output.status.code(),
                "launch_sha256":hash(&launch_bytes), "input_sha256":hash(input), "report":report,
                "stdout_sha256":hash(&output.stdout), "stderr_sha256":hash(&output.stderr), "binary_sha256":binary_pin}));
        }
        let ordinary = directory.join(format!("{mode}-ordinary"));
        let broken = closed_stdout(
            binary,
            &ordinary.join("launch.json"),
            &ordinary.join("input"),
            &ordinary,
        );
        assert!(!broken.status.success());
        assert!(
            String::from_utf8_lossy(&broken.stderr).contains("CommittedWithFailures"),
            "{mode}: {}",
            String::from_utf8_lossy(&broken.stderr)
        );
        assert_eq!(pin(binary), binary_pin);
        cases.push(json!({"policy":mode, "label":"closed-stdout", "accepted":false,
            "postcommit_delivery_failure":true, "exit_code":broken.status.code(), "stderr_sha256":hash(&broken.stderr), "binary_sha256":binary_pin}));
    }
    json!(cases)
}
