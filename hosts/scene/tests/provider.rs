use std::collections::BTreeMap;

use mech_core::{MechRecord, MechTable, MechTuple, Ref, Value};
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
fn table(records: Vec<Value>) -> Value {
    let records: Vec<MechRecord> = records
        .into_iter()
        .map(|value| match value {
            Value::Record(record) => record.borrow().clone(),
            other => panic!("expected record, got {other:?}"),
        })
        .collect();
    Value::Table(Ref::new(MechTable::from_records(records).unwrap()))
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
fn valid_empty_circle_table() {
    let base = match table(vec![circle("template")]) {
        Value::Table(table) => table.borrow().empty_table(0),
        _ => unreachable!(),
    };
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", Value::Table(Ref::new(base))),
        ("lines", tuple(vec![])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().circles.len(), 0);
}

#[test]
fn valid_single_circle_table() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", table(vec![circle("c1")])),
        ("lines", tuple(vec![])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().circles.len(), 1);
}

#[test]
fn valid_many_circle_table() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", table(vec![circle("c1"), circle("c2")])),
        ("lines", tuple(vec![])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().circles.len(), 2);
}

#[test]
fn valid_many_line_table() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", table(vec![line("l1"), line("l2")])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().lines.len(), 2);
}

#[test]
fn table_columns_may_be_reordered() {
    let circle = record(vec![
        ("opacity", f(1.0)),
        ("stroke-width", f(0.0)),
        ("stroke", s("none")),
        ("fill", s("red")),
        ("radius", f(3.0)),
        ("y", f(2.0)),
        ("x", f(1.0)),
        ("id", s("c1")),
    ]);
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", table(vec![circle])),
        ("lines", tuple(vec![])),
    ]);
    assert_eq!(SceneSnapshot::from_value(&scene).unwrap().circles[0].id, "c1");
}

#[test]
fn table_missing_column_is_rejected() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", table(vec![record(vec![("id", s("c1"))])])),
        ("lines", tuple(vec![])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn table_unknown_column_is_rejected() {
    let bad = record(vec![
        ("id", s("c1")),
        ("x", f(1.0)),
        ("y", f(2.0)),
        ("radius", f(3.0)),
        ("fill", s("red")),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(1.0)),
        ("extra", f(1.0)),
    ]);
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", table(vec![bad])),
        ("lines", tuple(vec![])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn table_column_length_mismatch_is_rejected() {
    let mut table = match table(vec![circle("c1")]) {
        Value::Table(table) => table.borrow().clone(),
        _ => unreachable!(),
    };
    table.rows = 2;
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", Value::Table(Ref::new(table))),
        ("lines", tuple(vec![])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn table_error_identifies_row_and_column() {
    let bad = record(vec![
        ("id", s("c1")),
        ("x", f(f64::NAN)),
        ("y", f(2.0)),
        ("radius", f(3.0)),
        ("fill", s("red")),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(1.0)),
    ]);
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", table(vec![bad])),
        ("lines", tuple(vec![])),
    ]);
    let err = format!("{:?}", SceneSnapshot::from_value(&scene).unwrap_err());
    assert!(err.contains("row 1"));
    assert!(err.contains("x"));
}

#[test]
fn tuple_unknown_field_is_rejected() {
    let bad = record(vec![
        ("id", s("c1")),
        ("x", f(1.0)),
        ("y", f(2.0)),
        ("radius", f(3.0)),
        ("fill", s("red")),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(1.0)),
        ("extra", f(1.0)),
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
fn empty_element_id_is_rejected() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![circle("")])),
        ("lines", tuple(vec![])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn non_finite_scene_number_is_rejected() {
    let scene = record(vec![
        ("width", f(f64::INFINITY)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn invalid_line_cap_is_rejected() {
    let bad = record(vec![
        ("id", s("l1")),
        ("x1", f(0.0)),
        ("y1", f(0.0)),
        ("x2", f(1.0)),
        ("y2", f(1.0)),
        ("stroke", s("red")),
        ("stroke-width", f(1.0)),
        ("line-cap", s("invalid")),
        ("opacity", f(1.0)),
        ("rotation", f(45.0)),
        ("origin-x", f(0.0)),
        ("origin-y", f(0.0)),
    ]);
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![bad])),
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
    let factory = NativeSceneHostFactory::new().unwrap();
    let installation = factory.instantiate("view", &settings("svg")).unwrap();
    assert_eq!(installation.resource_providers.len(), 1);
}

#[cfg(feature = "native")]
#[test]
fn native_scene_instances_are_isolated() {
    let factory = NativeSceneHostFactory::new().unwrap();
    let registry = factory.registry();
    let mut main = factory
        .instantiate("main", &settings("svg"))
        .unwrap()
        .resource_providers
        .remove(0);
    let mut hud = factory
        .instantiate("hud", &settings("svg"))
        .unwrap()
        .resource_providers
        .remove(0);
    main.write(RuntimeResourceWriteRequest {
        base_uri: "scene://main/frame".to_string(),
        path: "replace".to_string(),
        context_name: "view".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value: record(vec![
            ("width", f(100.0)),
            ("height", f(50.0)),
            ("background", s("#000")),
            ("circles", tuple(vec![])),
            ("lines", tuple(vec![])),
        ]),
    })
    .unwrap();
    hud.write(RuntimeResourceWriteRequest {
        base_uri: "scene://hud/frame".to_string(),
        path: "replace".to_string(),
        context_name: "view".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value: record(vec![
            ("width", f(200.0)),
            ("height", f(50.0)),
            ("background", s("#000")),
            ("circles", tuple(vec![])),
            ("lines", tuple(vec![])),
        ]),
    })
    .unwrap();
    assert_eq!(registry.latest("main").unwrap().width, 100.0);
    assert_eq!(registry.latest("hud").unwrap().width, 200.0);
}

#[test]
fn scene_provider_deduplicates_identical_replacements() {
    let backend = RecordingSceneBackend::new();
    let mut provider = SceneResourceProvider::new("main", backend.clone());
    let write = |value| RuntimeResourceWriteRequest {
        base_uri: "scene://main/frame".to_string(),
        path: "replace".to_string(),
        context_name: "main".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value,
    };

    provider.write(write(empty_scene())).unwrap();
    assert_eq!(backend.generation(), 1);
    provider.write(write(empty_scene())).unwrap();
    assert_eq!(backend.generation(), 1);

    let changed = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#111")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ]);
    provider.write(write(changed)).unwrap();
    assert_eq!(backend.generation(), 2);

    let other_backend = RecordingSceneBackend::new();
    let mut other_provider = SceneResourceProvider::new("other", other_backend.clone());
    other_provider.write(RuntimeResourceWriteRequest {
        base_uri: "scene://other/frame".to_string(),
        path: "replace".to_string(),
        context_name: "main".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value: empty_scene(),
    }).unwrap();
    assert_eq!(other_backend.generation(), 1);
}
