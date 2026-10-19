//! Opt-in, predeclared full-input workload and finite policy; no default runs.
use serde::Deserialize;
use serde_json::Value;
use std::{ffi::OsStr, fs, io::Read, path::PathBuf};

pub(super) const ENV: &str = "MECH_CANONICAL_HOSTED_FULL_INPUT_CONFIG";
pub(super) const SERIAL: &str = "MECH_CANONICAL_HOSTED_TIMING_SERIAL";
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileSpec {
    pub path: PathBuf,
    pub sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: String,
    input: FileSpec,
    oracle: FileSpec,
    launch: FileSpec,
    deadline_seconds: u64,
    samples: u64,
    external_label: Option<String>,
}
#[derive(Clone, Debug)]
pub(super) struct Config {
    pub path: PathBuf,
    pub input: FileSpec,
    pub oracle: FileSpec,
    pub launch: FileSpec,
    pub deadline_seconds: u64,
    pub samples: usize,
    pub external_label: Option<String>,
}
fn keys(value: &Value, allowed: &[&str]) -> Result<(), String> {
    let object = value.as_object().ok_or("expected JSON object")?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("unknown configuration or launch field".into());
    }
    Ok(())
}
fn integer(value: &Value) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| "expected unsigned integer".into())
}
fn positive(value: &Value) -> Result<u64, String> {
    let n = integer(value)?;
    if n == 0 || n.checked_add(1).is_none() {
        return Err("every scale policy cap must be positive and finite, not u64::MAX".into());
    }
    Ok(n)
}
pub(super) fn file(spec: FileSpec) -> Result<FileSpec, String> {
    if !spec.path.is_absolute()
        || spec.sha256.len() != 64
        || !spec
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "file path must be absolute and SHA256 must be 64 lowercase hex characters".into(),
        );
    }
    Ok(spec)
}
fn finish(path: PathBuf, wire: Wire) -> Result<Config, String> {
    if wire.schema != "mech.canonical-hosted-full-input-config.v1" {
        return Err("unknown full-input configuration schema".into());
    }
    let deadline_seconds = wire.deadline_seconds;
    let samples = wire.samples;
    if !(1..=3600).contains(&deadline_seconds) || samples > 20 {
        return Err("deadline_seconds must be 1..3600 and samples must be 0..20".into());
    }
    let external_label = match wire.external_label {
        None => None,
        Some(label) => {
            if label.len() > 128 {
                return Err("external_label exceeds 128 bytes".into());
            }
            Some(label.to_owned())
        }
    };
    Ok(Config {
        path,
        input: file(wire.input)?,
        oracle: file(wire.oracle)?,
        launch: file(wire.launch)?,
        deadline_seconds,
        samples: samples as usize,
        external_label,
    })
}
#[cfg(test)]
fn parse(path: PathBuf, value: &Value) -> Result<Config, String> {
    finish(
        path,
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?,
    )
}
pub(super) fn load(value: Option<&OsStr>) -> Result<Option<(Config, Vec<u8>)>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let path = PathBuf::from(value);
    if !path.is_absolute() || path.to_str().is_none() {
        return Err("full-input configuration path must be absolute UTF8".into());
    }
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 65536 {
        return Err("full-input configuration must be a regular file within 64 KiB".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("full-input configuration exceeds 64 KiB".into());
    }
    let wire = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok(Some((finish(path, wire)?, bytes)))
}
/// Keep small gate policy unchanged. Only an explicitly pinned separate scale
/// policy is accepted here, with the SAME grants/ports and finite every cap.
pub(super) fn policy(
    value: &Value,
    small: &Value,
    input_bytes: u64,
    oracle_bytes: u64,
) -> Result<(), String> {
    keys(
        value,
        &[
            "grants",
            "max_ports",
            "input_text_bytes",
            "output_text_bytes",
            "memory_bytes",
            "work",
            "transport_bytes",
            "native_stack_bytes",
            "library",
        ],
    )?;
    if value["grants"] != small["grants"] || value["max_ports"] != small["max_ports"] {
        return Err("full policy must keep exact existing CLI grants and max_ports".into());
    }
    for key in [
        "input_text_bytes",
        "output_text_bytes",
        "memory_bytes",
        "work",
        "transport_bytes",
        "native_stack_bytes",
    ] {
        positive(&value[key])?;
    }
    let library = &value["library"];
    keys(
        library,
        &[
            "output_elements",
            "output_bytes",
            "temporary_bytes",
            "cloned_bytes",
            "retained_nodes",
            "compute_work",
            "comparison_work",
        ],
    )?;
    for key in [
        "output_elements",
        "output_bytes",
        "temporary_bytes",
        "cloned_bytes",
        "retained_nodes",
        "compute_work",
        "comparison_work",
    ] {
        positive(&library[key])?;
    }
    if value["input_text_bytes"].as_u64().unwrap() < input_bytes
        || value["output_text_bytes"].as_u64().unwrap() < oracle_bytes
    {
        return Err("declared input/output acquisition caps do not cover actual bytes".into());
    }
    // The existing generated strict Launch and native runtime still perform
    // all final field/target/demand validation; sufficiency is NOT promised.
    Ok(())
}
pub(super) fn serial(samples: usize, attestation: Option<&OsStr>) -> Result<(), String> {
    if samples != 0 && attestation != Some(OsStr::new("1")) {
        return Err(format!(
            "timing requires explicit {SERIAL}=1 and operator-exclusive execution"
        ));
    }
    Ok(())
}

#[test]
fn full_input_configuration_is_opt_in_strict_and_bounded() {
    assert!(load(None).unwrap().is_none());
    let spec = serde_json::json!({"path":"/actual/input", "sha256":"a".repeat(64)});
    let value = serde_json::json!({"schema":"mech.canonical-hosted-full-input-config.v1",
        "input":spec, "oracle":spec, "launch":spec, "deadline_seconds":3600, "samples":0});
    assert_eq!(parse("/config".into(), &value).unwrap().samples, 0);
    for (key, bad) in [
        ("deadline_seconds", serde_json::json!(0)),
        ("deadline_seconds", serde_json::json!(3601)),
        ("samples", serde_json::json!(21)),
        ("samples", serde_json::json!(-1)),
        ("unknown", serde_json::json!(1)),
    ] {
        let mut bad_value = value.clone();
        bad_value[key] = bad;
        assert!(parse("/config".into(), &bad_value).is_err());
    }
    let mut bad = value.clone();
    bad["input"]["path"] = serde_json::json!("relative");
    assert!(parse("/config".into(), &bad).is_err());
    bad = value.clone();
    bad["input"]["sha256"] = serde_json::json!("A".repeat(64));
    assert!(parse("/config".into(), &bad).is_err());
    assert!(serial(0, None).is_ok());
    assert!(serial(1, None).is_err());
    assert!(serial(1, Some(OsStr::new("true"))).is_err());
    assert!(serial(20, Some(OsStr::new("1"))).is_ok());
    let encoded = serde_json::to_string(&value).unwrap();
    let duplicate = format!("{},\"samples\":0}}", &encoded[..encoded.len() - 1]);
    assert!(serde_json::from_slice::<Wire>(duplicate.as_bytes()).is_err());
}
#[test]
fn full_input_policy_is_separate_finite_and_preserves_real_grants() {
    let small = super::super::launch();
    let mut scale = small.clone();
    scale["input_text_bytes"] = serde_json::json!(300000000u64);
    scale["work"] = serde_json::json!(1000000000000u64);
    assert!(policy(&scale, &small, 254166745, 1024).is_ok());
    assert!(policy(&small, &small, 254166745, 1024).is_err());
    for key in [
        "input_text_bytes",
        "output_text_bytes",
        "memory_bytes",
        "work",
        "transport_bytes",
        "native_stack_bytes",
    ] {
        for bad in [0, u64::MAX] {
            let mut policy_value = scale.clone();
            policy_value[key] = serde_json::json!(bad);
            assert!(policy(&policy_value, &small, 10, 10).is_err());
        }
    }
    for key in [
        "output_elements",
        "output_bytes",
        "temporary_bytes",
        "cloned_bytes",
        "retained_nodes",
        "compute_work",
        "comparison_work",
    ] {
        for value in [
            Value::Null,
            serde_json::json!(0),
            serde_json::json!(u64::MAX),
        ] {
            let mut bad = scale.clone();
            bad["library"][key] = value;
            assert!(policy(&bad, &small, 10, 10).is_err());
        }
    }
    let mut bad = scale.clone();
    bad["grants"][0]["operations"] = serde_json::json!(["write"]);
    assert!(policy(&bad, &small, 10, 10).is_err());
    bad = scale.clone();
    bad["max_ports"] = serde_json::json!(32);
    assert!(policy(&bad, &small, 10, 10).is_err());
}
