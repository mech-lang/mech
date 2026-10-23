use super::*;
use std::fs;

fn fixture(directory: &Path) -> (Inputs, Value) {
    let policy = super::super::super::launch();
    let input = process::freeze_bytes(&directory.join("input"), b"input", 5).unwrap();
    let oracle = process::freeze_bytes(&directory.join("oracle"), b"oracle", 6).unwrap();
    let launch = process::freeze_bytes(
        &directory.join("launch"),
        &serde_json::to_vec(&policy).unwrap(),
        65536,
    )
    .unwrap();
    (
        Inputs {
            input,
            oracle,
            launch,
        },
        policy,
    )
}

#[test]
fn production_driver_keeps_two_trials_old_labels_and_unmodified_launch() {
    let temp = tempfile::tempdir().unwrap();
    let (base, value) = fixture(temp.path());
    let set = RunSet::Production;
    assert_eq!(set.trials(), 2);
    assert_eq!(set.modes(), [TrialPolicy::ProductionSecured]);
    assert_eq!(
        set.result_schema(),
        "mech.canonical-hosted-full-input-result.v1"
    );
    assert_eq!(
        set.stage_schema(),
        "mech.canonical-hosted-staged-input-result.v1"
    );
    assert_eq!(set.modes()[0].label("qualify-0"), "qualify-0");
    let launches = set.launches(&base, &value, temp.path()).unwrap();
    assert_eq!(launches.len(), 1);
    assert_eq!(launches[0].1.launch.path, base.launch.path);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
    base.check().unwrap();
    for round in 0..20 {
        assert_eq!(set.sample_slot(round, 0), 0);
    }
}

#[test]
fn owned_derived_files_enforce_cap_create_new_and_immutable_hash_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("derived");
    assert!(process::freeze_bytes(&target, b"abcd", 3).is_err());
    assert!(!target.exists());
    let frozen = process::freeze_bytes(&target, b"abcd", 4).unwrap();
    assert_eq!(frozen.bytes, 4);
    assert!(fs::metadata(&target).unwrap().permissions().readonly());
    frozen.check().unwrap();
    let alias = frozen.clone();
    assert_eq!(alias.path, frozen.path);
    assert_eq!(alias.sha256, frozen.sha256);
    assert!(process::freeze_bytes(&target, b"replace", 7).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"abcd");
    alias.check().unwrap();
}

#[cfg(feature = "canonical-native-benchmark-policy")]
fn complete() -> Vec<Value> {
    (0..2)
        .flat_map(|variant| {
            RunSet::BenchmarkComparison.modes().iter().map(move |mode| {
                json!({"variant":variant,"policy":mode.name(),"route":mode.route(),"ok":true,
                    "observed":{"report":{"native_work":100,"native_comparison_work":10,
                        "native_visits":20,"native_transport_bytes":30,"host_result_carrier_bytes":40}}})
    })
        })
        .collect()
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn comparison_requires_all_four_unique_correct_variant_policy_outcomes() {
    let set = RunSet::BenchmarkComparison;
    let rows = complete();
    assert_eq!(set.trials(), 4);
    assert!(set.qualifies(&rows));
    assert!(!set.qualifies(&[]));
    for index in 0..4 {
        let mut missing = rows.clone();
        missing.remove(index);
        assert!(!set.qualifies(&missing));
        for key in ["variant", "policy", "route", "ok"] {
            let mut wrong = rows.clone();
            wrong[index][key] = Value::Null;
            assert!(!set.qualifies(&wrong), "{index}/{key}");
        }
    }
    let mut duplicates = rows.clone();
    duplicates[3] = rows[0].clone();
    assert!(!set.qualifies(&duplicates));
    let mut failed = rows.clone();
    failed[3]["ok"] = json!(false);
    assert!(!set.qualifies(&failed));
    let mut production = rows.clone();
    production[0]["route"] = json!(TrialPolicy::ProductionSecured.route());
    assert!(!set.qualifies(&production));
    for index in 0..4 {
        for field in [
            "native_work",
            "native_comparison_work",
            "native_visits",
            "native_transport_bytes",
            "host_result_carrier_bytes",
        ] {
            for wrong in [Value::Null, json!(999)] {
                let mut changed = rows.clone();
                changed[index]["observed"]["report"][field] = wrong;
                assert!(!set.qualifies(&changed), "{index}/{field}");
            }
        }
    }
    for round in 0..20 {
        let order = (0..2)
            .map(|offset| set.sample_slot(round, offset))
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            if round % 2 == 0 {
                vec![0, 1]
            } else {
                vec![1, 0]
            }
        );
    }
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn review_regression_comparison_rejects_pairwise_agreement_without_four_way_accounting() {
    let set = RunSet::BenchmarkComparison;
    for pair in [[0, 1], [2, 3]] {
        for field in [
            "native_work",
            "native_comparison_work",
            "native_visits",
            "native_transport_bytes",
            "host_result_carrier_bytes",
        ] {
            let mut rows = complete();
            for index in pair {
                rows[index]["observed"]["report"][field] = json!(999);
            }
            assert!(!set.qualifies(&rows), "pair {pair:?}/{field}");
        }
    }
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn comparison_launches_change_only_the_explicit_mode_and_share_frozen_payloads() {
    let temp = tempfile::tempdir().unwrap();
    let (base, value) = fixture(temp.path());
    let set = RunSet::BenchmarkComparison;
    let launches = set.launches(&base, &value, temp.path()).unwrap();
    assert_eq!(launches.len(), 2);
    for (mode, inputs) in &launches {
        let mut derived: Value =
            serde_json::from_slice(&fs::read(&inputs.launch.path).unwrap()).unwrap();
        assert_eq!(
            derived.as_object_mut().unwrap().remove("benchmark_policy"),
            Some(json!(mode.name()))
        );
        assert_eq!(derived, value);
        assert_eq!(inputs.input.path, base.input.path);
        assert_eq!(inputs.oracle.path, base.oracle.path);
        assert_eq!(inputs.input.sha256, base.input.sha256);
        assert_eq!(inputs.oracle.sha256, base.oracle.sha256);
        assert_ne!(inputs.launch.path, base.launch.path);
        assert!(
            fs::metadata(&inputs.launch.path)
                .unwrap()
                .permissions()
                .readonly()
        );
        inputs.check().unwrap();
    }
    assert_ne!(launches[0].1.launch.sha256, launches[1].1.launch.sha256);
    assert!(set.launches(&base, &value, temp.path()).is_err());
    base.check().unwrap();
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn preselected_or_unknown_policy_is_never_a_scale_policy_backdoor() {
    for set in [RunSet::Production, RunSet::BenchmarkComparison] {
        for mode in ["secured", "trusted", "", "invalid"] {
            let temp = tempfile::tempdir().unwrap();
            let (base, mut value) = fixture(temp.path());
            value["benchmark_policy"] = json!(mode);
            assert!(set.launches(&base, &value, temp.path()).is_err());
            assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
        }
    }
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn comparison_refuses_mismatched_binary_or_object_provenance_before_opening_executables() {
    let row = json!({"variant":0,"binary":"/unopened/executable","rust_profile":"release",
        "program_revision":"revision","binary_sha256":"a","source_sha256":"b",
        "bytecode_sha256":"c","object_sha256":"d","identity_sha256":"e"});
    for key in [
        "binary_sha256",
        "source_sha256",
        "bytecode_sha256",
        "object_sha256",
        "identity_sha256",
    ] {
        let mut second = row.clone();
        second["variant"] = json!(1);
        second[key] = json!("different");
        let error = super::super::binaries(&[row.clone(), second], RunSet::BenchmarkComparison)
            .err()
            .unwrap();
        assert!(error.contains(&format!("SAME original/decoded {key}")));
    }
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn comparison_report_validation_distinguishes_policy_and_route() {
    let mut report = json!({"schema":"mech.canonical-hosted-native-execution.v1",
        "route":"canonical-artifact-program-object-benchmark-cli","policy":"secured",
        "program_revision":"actual","launch_sha256":"launch","native_aot":true,
        "hosted_phases":"admit-activate-capture-resume","source_parser":false,"jit":false,
        "resident_body_evaluation":false,"shutdown":"completed","requirements":2,"observations":1,
        "effects":1,"input_acquisition_byte_cap":200,"native_work":100,"native_comparison_work":1,
        "native_visits":10,"native_transport_bytes":10,"host_result_carrier_bytes":1});
    for mode in RunSet::BenchmarkComparison.modes() {
        report["policy"] = json!(mode.name());
        assert!(super::super::report_issues(&report, "actual", "launch", 200, *mode).is_empty());
        for key in [
            "policy",
            "route",
            "program_revision",
            "launch_sha256",
            "shutdown",
        ] {
            let mut bad = report.clone();
            bad[key] = json!("wrong");
            assert!(!super::super::report_issues(&bad, "actual", "launch", 200, *mode).is_empty());
        }
    }
}
