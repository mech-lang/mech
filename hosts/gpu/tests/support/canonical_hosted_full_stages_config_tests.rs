use super::*;
use serde_json::{Value, json};

fn stage(name: &str, path: &str) -> Value {
    json!({"name":name, "configuration":{"path":path,"sha256":"a".repeat(64)}})
}
fn wire(stages: Vec<Value>) -> Value {
    json!({"schema":"mech.canonical-hosted-staged-input-config.v1", "stages":stages})
}
fn check(value: &Value) -> Result<Config, String> {
    parse(
        "/actual/stages.json".into(),
        serde_json::to_vec(value).unwrap(),
    )
}

#[test]
fn staged_config_is_explicit_exclusive_strict_and_at_most_three() {
    assert!(exclusive(None, None).is_ok());
    assert!(exclusive(Some(OsStr::new("/single")), None).is_ok());
    assert!(exclusive(None, Some(OsStr::new("/stages"))).is_ok());
    assert!(exclusive(Some(OsStr::new("/single")), Some(OsStr::new("/stages"))).is_err());
    for count in 1..=3 {
        let value = wire(
            (0..count)
                .map(|i| stage(&format!("stage-{i}"), &format!("/config-{i}")))
                .collect(),
        );
        assert_eq!(check(&value).unwrap().stages.len(), count);
    }
    for count in [0, 4] {
        assert!(
            check(&wire(
                (0..count)
                    .map(|i| stage(&format!("s{i}"), &format!("/c{i}")))
                    .collect()
            ))
            .is_err()
        );
    }
    let valid = wire(vec![stage("medium", "/medium")]);
    for field in ["extra", "deadline_seconds", "samples"] {
        let mut bad = valid.clone();
        bad[field] = json!(1);
        assert!(check(&bad).is_err());
    }
    let mut bad = valid.clone();
    bad["schema"] = json!("unknown");
    assert!(check(&bad).is_err());
    let encoded = serde_json::to_string(&valid).unwrap();
    let duplicate = format!("{},\"stages\":[]}}", &encoded[..encoded.len() - 1]);
    assert!(parse("/stages".into(), duplicate.into_bytes()).is_err());
    assert!(parse("relative".into(), encoded.clone().into_bytes()).is_err());
    assert!(parse("/stages".into(), vec![b' '; 65537]).is_err());
}

#[test]
fn stage_names_paths_pins_and_resolved_aliases_are_bounded_and_unique() {
    for name in ["", "..", "a/b", "a b", "Medium", "é"] {
        assert!(check(&wire(vec![stage(name, "/config")])).is_err());
    }
    assert!(check(&wire(vec![stage(&"a".repeat(33), "/config")])).is_err());
    assert!(check(&wire(vec![stage("a", "/one"), stage("a", "/two")])).is_err());
    assert!(check(&wire(vec![stage("a", "/one"), stage("b", "/one")])).is_err());
    for path in ["relative", ""] {
        assert!(check(&wire(vec![stage("a", path)])).is_err());
    }
    let mut bad = wire(vec![stage("a", "/one")]);
    bad["stages"][0]["configuration"]["sha256"] = json!("A".repeat(64));
    assert!(check(&bad).is_err());
    bad = wire(vec![stage("a", "/one")]);
    bad["stages"][0]["unknown"] = json!(true);
    assert!(check(&bad).is_err());
    assert!(distinct_resolved_paths(vec!["/resolved/a".into(), "/resolved/a".into()]).is_err());
    assert!(distinct_resolved_paths(vec!["/resolved/a".into(), "/resolved/b".into()]).is_ok());
}
