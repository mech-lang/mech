//! Pure report and qualification ordering tests; no child process is launched.
use super::*;

fn may_time(rows: &[Value]) -> bool {
    super::may_time(rows, RunSet::Production)
}
fn report_issues(report: &Value, revision: &str, launch: &str, cap: u64) -> Vec<String> {
    super::report_issues(
        report,
        revision,
        launch,
        cap,
        TrialPolicy::ProductionSecured,
    )
}

#[test]
fn full_timing_requires_both_original_and_decoded_correctness_before_samples() {
    assert!(!may_time(&[]));
    assert!(!may_time(&[json!({"variant":0,"ok":true})]));
    assert!(!may_time(&[
        json!({"variant":0,"ok":true}),
        json!({"variant":1,"ok":false})
    ]));
    assert!(!may_time(&[
        json!({"variant":0,"ok":true}),
        json!({"variant":0,"ok":true})
    ]));
    assert!(may_time(&[
        json!({"variant":0,"ok":true}),
        json!({"variant":1,"ok":true})
    ]));
}
#[test]
fn full_success_report_requires_actual_route_revision_launch_and_shutdown() {
    let mut report = json!({"schema":"mech.canonical-hosted-native-execution.v1",
        "route":"canonical-artifact-program-object-secured-cli", "policy":"secured",
        "program_revision":"actual", "launch_sha256":"launch", "native_aot":true,
        "hosted_phases":"admit-activate-capture-resume", "source_parser":false, "jit":false,
        "resident_body_evaluation":false, "shutdown":"completed", "requirements":2,
        "observations":1, "effects":1, "input_acquisition_byte_cap":200,
        "native_work":100, "native_comparison_work":1, "native_visits":10,
        "native_transport_bytes":10, "host_result_carrier_bytes":1});
    assert!(report_issues(&report, "actual", "launch", 200).is_empty());
    for key in [
        "route",
        "program_revision",
        "launch_sha256",
        "shutdown",
        "native_aot",
        "source_parser",
        "jit",
        "resident_body_evaluation",
    ] {
        let old = report[key].clone();
        report[key] = Value::Null;
        assert!(!report_issues(&report, "actual", "launch", 200).is_empty());
        report[key] = old;
    }
}
