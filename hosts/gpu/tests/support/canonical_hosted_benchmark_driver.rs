//! Tiny protocol witness through the SAME full-size driver, not a benchmark.
use super::*;

pub(super) fn qualify(evidence: &Path, results: &[Value], profile: profile::Profile) -> Value {
    if !profile.release() {
        return Value::Null;
    }
    let directory = evidence.join("driver-protocol-control");
    fs::create_dir(&directory).unwrap();
    let supplied = directory.join("supplied");
    fs::create_dir(&supplied).unwrap();
    let spec = |name: &str, bytes: &[u8]| {
        let path = supplied.join(name);
        fs::write(&path, bytes).unwrap();
        json!({"path":path,"sha256":hash(bytes)})
    };
    let configuration = json!({
        "schema":"mech.canonical-hosted-full-input-config.v1",
        "input":spec("input", GGTA_INPUT),"oracle":spec("oracle", GGTA_OUTPUT.as_bytes()),
        "launch":spec("launch.json", &serde_json::to_vec_pretty(&launch()).unwrap()),
        "deadline_seconds":120,"samples":0,
        "external_label":"small driver-protocol control; NOT full-size or timing"
    });
    let config = supplied.join("configuration.json");
    fs::write(&config, serde_json::to_vec_pretty(&configuration).unwrap()).unwrap();
    let result =
        full::comparison_control(profile, &directory, results, config.as_os_str()).unwrap();
    assert_eq!(result["qualified"], true);
    assert_eq!(result["expected_qualification_trials"], 4);
    assert_eq!(result["qualification"].as_array().unwrap().len(), 4);
    assert_eq!(result["timing_samples"], 0);
    assert!(result["samples"].as_array().unwrap().is_empty());
    result
}
