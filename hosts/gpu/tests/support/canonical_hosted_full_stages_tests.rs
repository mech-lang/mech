use super::*;

#[test]
fn stage_receipt_preserves_partial_attempts_successes_and_unknown_starts() {
    let mut receipt = json!({"stages":[
        {"name":"medium", "status":"passed", "result":{
            "qualification":[{"ok":true,"process_started":true},{"ok":true,"process_started":true}],
            "samples":[{"ok":true,"process_started":true},{"ok":true,"process_started":true}]}},
        {"name":"large", "status":"failed", "result":{
            "qualification":[{"ok":true,"process_started":true},{"ok":true,"process_started":true}],
            "samples":[{"ok":true,"process_started":true},{"ok":false}]}},
        {"name":"full", "status":"skipped"}]});
    refresh(&mut receipt);
    for (key, count) in [
        ("stages_attempted", 2),
        ("qualified_stages", 1),
        ("failed_stages", 1),
        ("skipped_stages", 1),
        ("qualification_attempts", 4),
        ("qualification_process_starts_confirmed", 4),
        ("timing_attempts", 4),
        ("timed_process_starts_confirmed", 3),
        ("timed_process_starts_unknown", 1),
        ("successful_timed_samples", 3),
        ("timing_samples", 0),
    ] {
        assert_eq!(receipt[key], count, "{key}");
    }
    assert_eq!(receipt["qualified"], false);
    assert!(receipt.get("timing_summary").is_none());
    // A pre-invocation refusal is an attempted row, NOT a confirmed start.
    receipt["stages"][1]["result"]["samples"][1]["process_started"] = json!(false);
    refresh(&mut receipt);
    assert_eq!(receipt["timing_attempts"], 4);
    assert_eq!(receipt["timed_process_starts_confirmed"], 3);
    assert_eq!(receipt["timed_process_starts_unknown"], 0);
}

#[test]
fn only_every_passed_stage_publishes_complete_sample_count() {
    let mut receipt = json!({"stages":[]});
    refresh(&mut receipt);
    assert_eq!(receipt["qualified"], false);
    receipt["stages"] = json!([
        {"status":"passed","result":{"qualification":[],"samples":[{"ok":true,"process_started":true}]}},
        {"status":"pending"}]);
    refresh(&mut receipt);
    assert_eq!(receipt["qualified"], false);
    assert_eq!(receipt["timing_samples"], 0);
    receipt["stages"][1] = json!({"status":"passed", "result":{"qualification":[],
        "samples":[{"ok":true,"process_started":true}]}});
    refresh(&mut receipt);
    assert_eq!(receipt["qualified"], true);
    assert_eq!(receipt["timing_samples"], 2);
}

#[test]
fn running_dispatch_and_preflight_failure_do_not_invent_process_starts() {
    let mut receipt = json!({"stages":[{"status":"running"},{"status":"pending"}]});
    refresh(&mut receipt);
    assert_eq!(receipt["stages_attempted"], 1);
    assert_eq!(receipt["running_stages"], 1);
    assert_eq!(receipt["qualified"], false);
    assert_eq!(receipt["qualification_attempts"], 0);
    assert_eq!(receipt["qualification_process_starts_confirmed"], 0);
    assert_eq!(receipt["timing_attempts"], 0);
    receipt["stages"][0] = json!({"status":"failed","result":{
        "preflight_error":"config pin refused", "qualification":[], "samples":[]}});
    receipt["stages"][1]["status"] = json!("skipped");
    refresh(&mut receipt);
    assert_eq!(receipt["stages_attempted"], 1);
    assert_eq!(receipt["running_stages"], 0);
    assert_eq!(receipt["failed_stages"], 1);
    assert_eq!(receipt["skipped_stages"], 1);
    assert_eq!(receipt["qualification_process_starts_confirmed"], 0);
    assert_eq!(receipt["timed_process_starts_unknown"], 0);
    assert_eq!(receipt["timing_samples"], 0);
}
