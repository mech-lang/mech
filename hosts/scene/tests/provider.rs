use std::collections::BTreeMap;

use mech_core::{
    EffectContract, EffectDeliveryPolicy, ExternalInteraction, IdempotencyRequirement, LegacyValue,
    MechError, MechErrorKind, MechRecord, MechTable, MechTuple, Ref, ToMatrix,
};
#[cfg(feature = "native")]
use mech_runtime::RuntimeHostFactory;
use mech_runtime::{
    ConfigValue, PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeResourceProvider,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};
use mech_scene::*;

fn deliver_write(
    provider: &mut dyn RuntimeResourceProvider,
    request: RuntimeResourceWriteRequest,
) -> mech_core::MResult<()> {
    match provider.prepare_write(request)? {
        PreparedRuntimeEffect::AfterCommit(mut effect) => effect.deliver(),
        effect => panic!("expected scene after-commit effect, got {effect:?}"),
    }
}

fn f(value: f64) -> LegacyValue {
    LegacyValue::F64(Ref::new(value))
}
fn s(value: &str) -> LegacyValue {
    LegacyValue::String(Ref::new(value.to_string()))
}
fn b(value: bool) -> LegacyValue {
    LegacyValue::Bool(Ref::new(value))
}
fn record(fields: Vec<(&str, LegacyValue)>) -> LegacyValue {
    LegacyValue::Record(Ref::new(MechRecord::new(fields)))
}
fn tuple(values: Vec<LegacyValue>) -> LegacyValue {
    LegacyValue::Tuple(Ref::new(MechTuple::from_vec(values)))
}
fn table(records: Vec<LegacyValue>) -> LegacyValue {
    let records: Vec<MechRecord> = records
        .into_iter()
        .map(|value| match value {
            LegacyValue::Record(record) => record.borrow().clone(),
            other => panic!("expected record, got {other:?}"),
        })
        .collect();
    LegacyValue::Table(Ref::new(MechTable::from_records(records).unwrap()))
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
fn empty_scene() -> LegacyValue {
    record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ])
}
fn points(rows: usize, columns: usize, values: Vec<f64>) -> LegacyValue {
    LegacyValue::MatrixF64(ToMatrix::to_matrix(values, rows, columns))
}
fn points_write(value: LegacyValue) -> RuntimeResourceWriteRequest {
    RuntimeResourceWriteRequest {
        base_uri: "scene://view/frame".to_string(),
        path: "points".to_string(),
        context_name: "view".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value,
    }
}
fn circle(id: &str) -> LegacyValue {
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
fn line(id: &str) -> LegacyValue {
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
fn text(id: &str) -> LegacyValue {
    record(vec![
        ("id", s(id)),
        ("x", f(10.0)),
        ("y", f(20.0)),
        ("fill", s("white")),
        ("font-size", f(12.0)),
        ("font-family", s("sans-serif")),
        ("font-weight", s("600")),
        ("text-anchor", s("start")),
        ("opacity", f(1.0)),
        ("value", s("Scene label")),
    ])
}
fn point_set(id: &str) -> LegacyValue {
    record(vec![
        ("id", s(id)),
        ("positions", points(2, 2, vec![10.0, 20.0, 30.0, 40.0])),
        ("radius", f(3.0)),
        ("first-radius", f(6.0)),
        ("fills", tuple(vec![s("gold"), s("blue")])),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(1.0)),
    ])
}
fn line_strip(id: &str) -> LegacyValue {
    record(vec![
        ("id", s(id)),
        (
            "positions",
            points(3, 2, vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0]),
        ),
        ("stroke", s("gray")),
        ("stroke-width", f(0.75)),
        ("line-cap", s("round")),
        ("line-join", s("round")),
        ("opacity", f(0.5)),
        ("closed", b(true)),
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
fn valid_text_scene() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("texts", tuple(vec![text("title")])),
    ]);
    let scene = SceneSnapshot::from_value(&scene).unwrap();
    assert_eq!(scene.texts[0].value, "Scene label");
}

#[test]
fn point_set_expands_matrix_rows_into_stable_circles() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("point-sets", point_set("body")),
    ]);
    let scene = SceneSnapshot::from_value(&scene).unwrap();
    assert_eq!(scene.circles.len(), 2);
    assert_eq!(scene.circles[0].id, "body-0");
    assert_eq!(scene.circles[0].radius, 6.0);
    assert_eq!(scene.circles[0].fill, "gold");
    assert_eq!((scene.circles[1].x, scene.circles[1].y), (20.0, 40.0));
}

#[test]
fn line_strip_keeps_matrix_rows_as_one_closed_path() {
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("line-strips", tuple(vec![line_strip("orbit-earth")])),
    ]);
    let scene = SceneSnapshot::from_value(&scene).unwrap();
    assert_eq!(scene.line_strips.len(), 1);
    assert_eq!(scene.line_strips[0].id, "orbit-earth");
    assert_eq!(
        scene.line_strips[0].positions,
        vec![[10.0, 40.0], [20.0, 50.0], [30.0, 60.0]]
    );
    assert!(scene.line_strips[0].closed);
}

#[test]
fn point_set_rejects_a_palette_with_the_wrong_length() {
    let bad = record(vec![
        ("id", s("body")),
        ("positions", points(2, 2, vec![10.0, 20.0, 30.0, 40.0])),
        ("radius", f(3.0)),
        ("first-radius", f(6.0)),
        ("fills", tuple(vec![s("gold")])),
        ("stroke", s("none")),
        ("stroke-width", f(0.0)),
        ("opacity", f(1.0)),
    ]);
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("point-sets", bad),
    ]);
    assert!(SceneSnapshot::from_value(&scene).is_err());
}

#[test]
fn point_set_rejects_nonfinite_radii() {
    for (radius, first_radius) in [(f64::NAN, 6.0), (3.0, f64::INFINITY)] {
        let bad = record(vec![
            ("id", s("body")),
            ("positions", points(2, 2, vec![10.0, 20.0, 30.0, 40.0])),
            ("radius", f(radius)),
            ("first-radius", f(first_radius)),
            ("fills", tuple(vec![s("gold"), s("blue")])),
            ("stroke", s("none")),
            ("stroke-width", f(0.0)),
            ("opacity", f(1.0)),
        ]);
        let scene = record(vec![
            ("width", f(100.0)),
            ("height", f(50.0)),
            ("background", s("#000")),
            ("point-sets", bad),
        ]);
        assert!(SceneSnapshot::from_value(&scene).is_err());
    }
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
        LegacyValue::Table(table) => table.borrow().empty_table(0),
        _ => unreachable!(),
    };
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", LegacyValue::Table(Ref::new(base))),
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
    assert_eq!(
        SceneSnapshot::from_value(&scene).unwrap().circles[0].id,
        "c1"
    );
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
        LegacyValue::Table(table) => table.borrow().clone(),
        _ => unreachable!(),
    };
    table.rows = 2;
    let scene = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", LegacyValue::Table(Ref::new(table))),
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
fn output_renderer_does_not_require_a_selector() {
    let mut map = BTreeMap::new();
    map.insert(
        "renderer".to_string(),
        ConfigValue::String("output".to_string()),
    );

    let settings = scene_settings_from_config(&ConfigValue::Map(map)).unwrap();

    assert_eq!(settings.renderer, SceneRendererKind::Output);
    assert!(settings.selector.is_empty());
}

#[test]
fn dom_renderer_still_requires_a_selector() {
    let mut map = BTreeMap::new();
    map.insert(
        "renderer".to_string(),
        ConfigValue::String("svg".to_string()),
    );

    let error = scene_settings_from_config(&ConfigValue::Map(map)).unwrap_err();

    assert!(format!("{error:?}").contains("scene selector is required"));
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
fn scene_send_contract_is_at_most_once_without_idempotency() {
    let provider = SceneResourceProvider::new("view", RecordingSceneBackend::new());
    let contract = provider
        .semantic_write_contract(RuntimeResourceWriteIntent::Send)
        .unwrap();
    assert!(matches!(
        &contract.interaction,
        ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        })
    ));
    assert!(
        provider
            .semantic_write_contract(RuntimeResourceWriteIntent::Assign)
            .is_none()
    );
    assert!(!provider.supports_resident_idempotency(RuntimeResourceWriteIntent::Send));
}

#[test]
fn points_reject_wrong_kinds_shapes_and_nonfinite_coordinates() {
    let provider = SceneResourceProvider::new("view", RecordingSceneBackend::new());
    assert!(provider.plan_write(points_write(f(1.0))).is_err());
    assert!(
        provider
            .plan_write(points_write(points(2, 1, vec![0.0; 2])))
            .is_err()
    );
    assert!(
        provider
            .plan_write(points_write(points(2, 3, vec![0.0; 6])))
            .is_err()
    );
    assert!(
        provider
            .plan_write(points_write(points(
                2,
                2,
                vec![1.0, 2.0, 3.0, f64::INFINITY],
            )))
            .is_err()
    );
}

#[test]
fn points_use_column_major_screen_coordinates_ids_palette_and_radii() {
    let mut settings = SceneHostSettings::new("#scene", SceneRendererKind::Svg);
    settings.width = 100;
    settings.height = 80;
    settings.point_radius = 2;
    settings.first_point_radius = 7;
    let scene =
        scene_snapshot_from_points(&points(2, 2, vec![60.0, 30.0, 10.0, 80.0]), &settings).unwrap();
    assert_eq!(scene.circles.len(), 2);
    assert_eq!(scene.circles[0].id, "body-0");
    assert_eq!(scene.circles[0].x, 60.0);
    assert_eq!(scene.circles[0].y, 10.0);
    assert_eq!(scene.circles[0].radius, 7.0);
    assert_eq!(scene.circles[0].fill, "#ffd166");
    assert_eq!(scene.circles[1].id, "body-1");
    assert_eq!(scene.circles[1].x, 30.0);
    assert_eq!(scene.circles[1].y, 80.0);
    assert_eq!(scene.circles[1].radius, 2.0);
    assert_eq!(scene.circles[1].fill, "#b8b8b8");
    assert!(scene.lines.is_empty());
}

#[test]
fn recording_scene_backend_receives_an_accepted_points_frame() {
    let backend = RecordingSceneBackend::new();
    let mut provider = SceneResourceProvider::new("view", backend.clone());
    deliver_write(&mut provider, points_write(points(1, 2, vec![1.0, 2.0]))).unwrap();
    assert_eq!(backend.latest().unwrap().circles[0].id, "body-0");
}

#[cfg(feature = "browser")]
#[test]
fn browser_scene_registry_receives_an_accepted_points_frame() {
    let registry = BrowserSceneRegistry::new();
    let settings = SceneHostSettings::new("#scene", SceneRendererKind::Svg);
    registry.register("view", settings.clone()).unwrap();
    let backend = BrowserSceneBackend::new("view", registry.clone());
    let mut provider = SceneResourceProvider::new_with_settings("view", backend, settings);
    deliver_write(&mut provider, points_write(points(1, 2, vec![1.0, 2.0]))).unwrap();
    assert_eq!(registry.latest("view").unwrap().circles[0].id, "body-0");
}

#[test]
fn latest_scene_replaces_older_scene() {
    let backend = RecordingSceneBackend::new();
    let mut provider = SceneResourceProvider::new("view", backend.clone());
    deliver_write(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "scene://view/frame".to_string(),
            path: "replace".to_string(),
            context_name: "view".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Send,
            value: empty_scene(),
        },
    )
    .unwrap();
    let newer = record(vec![
        ("width", f(200.0)),
        ("height", f(50.0)),
        ("background", s("#000")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ]);
    deliver_write(
        &mut provider,
        RuntimeResourceWriteRequest {
            base_uri: "scene://view/frame".to_string(),
            path: "replace".to_string(),
            context_name: "view".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Send,
            value: newer,
        },
    )
    .unwrap();
    assert_eq!(backend.latest().unwrap().width, 200.0);
}

#[test]
fn scene_prepare_write_does_not_render_before_delivery() {
    let backend = RecordingSceneBackend::new();
    let provider = SceneResourceProvider::new("view", backend.clone());
    let effect = provider
        .prepare_write(RuntimeResourceWriteRequest {
            base_uri: "scene://view/frame".to_string(),
            path: "replace".to_string(),
            context_name: "view".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Send,
            value: empty_scene(),
        })
        .unwrap();

    assert_eq!(backend.generation(), 0);
    match effect {
        PreparedRuntimeEffect::AfterCommit(mut effect) => {
            effect.deliver().unwrap();
        }
        effect => panic!("expected scene after-commit effect, got {effect:?}"),
    }
    assert_eq!(backend.generation(), 1);
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
    deliver_write(
        main.as_mut(),
        RuntimeResourceWriteRequest {
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
        },
    )
    .unwrap();
    deliver_write(
        hud.as_mut(),
        RuntimeResourceWriteRequest {
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
        },
    )
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

    deliver_write(&mut provider, write(empty_scene())).unwrap();
    assert_eq!(backend.generation(), 1);
    deliver_write(&mut provider, write(empty_scene())).unwrap();
    assert_eq!(backend.generation(), 1);

    let changed = record(vec![
        ("width", f(100.0)),
        ("height", f(50.0)),
        ("background", s("#111")),
        ("circles", tuple(vec![])),
        ("lines", tuple(vec![])),
    ]);
    deliver_write(&mut provider, write(changed)).unwrap();
    assert_eq!(backend.generation(), 2);

    let other_backend = RecordingSceneBackend::new();
    let mut other_provider = SceneResourceProvider::new("other", other_backend.clone());
    deliver_write(
        &mut other_provider,
        RuntimeResourceWriteRequest {
            base_uri: "scene://other/frame".to_string(),
            path: "replace".to_string(),
            context_name: "main".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            intent: RuntimeResourceWriteIntent::Send,
            value: empty_scene(),
        },
    )
    .unwrap();
    assert_eq!(other_backend.generation(), 1);
}

#[derive(Clone, Debug, Default)]
struct FailableSceneBackend {
    inner: RecordingSceneBackend,
    fail_next: std::sync::Arc<std::sync::Mutex<bool>>,
}
impl FailableSceneBackend {
    fn generation(&self) -> u64 {
        self.inner.generation()
    }
    fn fail_next(&self) {
        *self.fail_next.lock().unwrap() = true;
    }
}
impl SceneBackend for FailableSceneBackend {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> mech_core::MResult<()> {
        let mut fail = self.fail_next.lock().unwrap();
        if *fail {
            *fail = false;
            return Err(MechError::new(TestSceneError, None));
        }
        self.inner.replace_scene(scene)
    }
}

#[derive(Debug, Clone)]
struct TestSceneError;
impl MechErrorKind for TestSceneError {
    fn name(&self) -> &str {
        "SceneBackendRejected"
    }
    fn message(&self) -> String {
        "backend rejected scene".to_string()
    }
}

#[test]
fn scene_provider_failed_replace_does_not_advance_dedup_state() {
    let backend = FailableSceneBackend::default();
    let mut provider = SceneResourceProvider::new("main", backend.clone());
    let write = |value| RuntimeResourceWriteRequest {
        base_uri: "scene://main/frame".to_string(),
        path: "replace".to_string(),
        context_name: "main".to_string(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
        value,
    };
    backend.fail_next();
    assert!(deliver_write(&mut provider, write(empty_scene())).is_err());
    assert_eq!(backend.generation(), 0);
    deliver_write(&mut provider, write(empty_scene())).unwrap();
    assert_eq!(backend.generation(), 1);
    deliver_write(&mut provider, write(empty_scene())).unwrap();
    assert_eq!(backend.generation(), 1);
}
