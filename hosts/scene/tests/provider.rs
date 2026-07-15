use std::collections::BTreeMap;

use mech_core::{MechRecord, MechTuple, Ref, Value};
use mech_host_scene::*;
use mech_runtime::{
    ConfigValue, RuntimeCapabilityOperation, RuntimeHostFactory, RuntimeResourceProvider,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

fn f(value: f64) -> Value {
    Value::F64(Ref::new(value))
}
fn s(value: &str) -> Value {
    Value::String(Ref::new(value.to_string()))
}
fn record(fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Ref::new(MechRecord::new(fields)))
}
fn tuple(values: Vec<Value>) -> Value {
    Value::Tuple(Ref::new(MechTuple::from_vec(values)))
}

fn settings(renderer: &str) -> ConfigValue {
    let mut map = BTreeMap::new();
    map.insert(
        "selector".to_string(),
        ConfigValue::String("#scene".to_string()),
    );
    map.insert(
        "renderer".to_string(),
        ConfigValue::String(renderer.to_string()),
    );
    ConfigValue::Map(map)
}
fn empty_scene() -> Value {
    record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ])
}
fn circle(id: &str) -> Value {
    record(vec![
        ("id", s(id)),
        ("x", f(1.0)),
        ("y", f(2.0)),
        ("radius", f(3.0)),
        ("fill", s("red")),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(1.0)),
    ])
}
fn line(id: &str) -> Value {
    record(vec![
        ("id", s(id)),
        ("x1", f(0.0)),
        ("y1", f(0.0)),
        ("x2", f(1.0)),
        ("y2", f(1.0)),
        ("stroke", s("red")),
        ("stroke-width", f(1.0)),
        ("line-cap", s("round")),
        ("opacity", f(1.0)),
        ("rotation", f(45.0)),
        ("origin-x", f(0.0)),
        ("origin-y", f(0.0)),
    ])
}

#[test]
fn valid_empty_scene() {
    assert!(SceneSnapshot::from_value(&empty_scene()).is_ok());
}

#[test]
fn valid_circle_scene() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![circle("c1")])),
        ("lines", tuple(vec![])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().circles.len(), 1);
}

#[test]
fn valid_line_scene() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![line("l1")])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().lines.len(), 1);
}

#[test]
fn invalid_dimensions() {
    let scene = record(vec![
        ("width", f(0.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn duplicate_ids() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![circle("x")])),
        ("lines", tuple(vec![line("x")])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn missing_required_columns() {
    assert!(SceneSnapshot::from_value(&record(vec![("width", f(1.0))])).is_err());
}

#[test]
fn invalid_opacity() {
    let bad = record(vec![
        ("id", s("bad")),
        ("x", f(1.0)),
        ("y", f(2.0)),
        ("radius", f(3.0)),
        ("fill", s("red")),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(2.0)),
    ]);
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![bad])),
        ("lines", tuple(vec![])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn unknown_renderer() {
    assert!(scene_settings_from_config(&settings("webgl")).is_err());
}

#[test]
fn assignment_rejected() {
    let provider = SceneResourceProvider::new("view", RecordingSceneBackend::new());
    let err = provider
        .preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: "scene://view/frame".to_string(),
            path: "replace".to_string(),
            context_name: "view".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap_err();
    assert!(format!("{err:?}").contains("send"));
}

#[test]
fn unknown_send_path_rejected() {
    let provider = SceneResourceProvider::new("view", RecordingSceneBackend::new());
    assert!(
        provider
            .preflight_write(RuntimeResourceWritePreflightRequest {
                base_uri: "scene://view/frame".to_string(),
                path: "append".to_string(),
                context_name: "view".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                intent: RuntimeResourceWriteIntent::Send
            })
            .is_err()
    );
}

#[test]
fn latest_scene_replaces_older_scene() {
    let backend = RecordingSceneBackend::new();
    let mut provider = SceneResourceProvider::new("view", backend.clone());
    provider
        .write(RuntimeResourceWriteRequest {
            base_uri: "scene://view/frame".to_string(),
            path: "replace".to_string(),
            context_name: "view".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Send,
            value: empty_scene(),
        })
        .unwrap();
    let newer = record(vec![
        ("width", f(200.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ]);
    provider
        .write(RuntimeResourceWriteRequest {
            base_uri: "scene://view/frame".to_string(),
            path: "replace".to_string(),
            context_name: "view".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Send,
            value: newer,
        })
        .unwrap();
    assert_eq!(backend.latest().unwrap().width, 200.0);
}

#[cfg(feature = "native")]
#[test]
fn native_recording_backend_retains_latest_complete_scene() {
    let backend = RecordingSceneBackend::new();
    let factory = SceneHostFactory::with_backend(backend.clone()).unwrap();
    let installation = factory.instantiate("view", &settings("svg")).unwrap();
    assert_eq!(installation.resource_providers.len(), 1);
}
