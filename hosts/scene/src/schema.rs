use std::collections::HashSet;

use mech_core::{MResult, MechRecord, MechTable, Value, hash_str};

use crate::scene_error;

#[derive(Clone, Debug, PartialEq)]
pub struct CircleElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub opacity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineElement {
    pub id: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stroke: String,
    pub stroke_width: f64,
    pub line_cap: String,
    pub opacity: f64,
    pub rotation: f64,
    pub origin_x: f64,
    pub origin_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshot {
    pub width: f64,
    pub height: f64,
    pub background: String,
    pub circles: Vec<CircleElement>,
    pub lines: Vec<LineElement>,
}

impl SceneSnapshot {
    pub fn from_value(value: &Value) -> MResult<Self> {
        if let Value::MutableReference(value) = value {
            return Self::from_value(&value.borrow());
        }
        let Value::Record(record) = value else {
            return Err(scene_error("SceneSchema", "scene must be a record"));
        };
        let record = record.borrow();
        let allowed = ["width", "height", "background", "circles", "lines"];
        for (_, name) in &record.field_names {
            if !allowed.contains(&name.as_str()) {
                return Err(scene_error(
                    "SceneSchema",
                    format!("unknown scene field `{name}`"),
                ));
            }
        }
        let width = required_number(&record, "width", "scene.width")?;
        let height = required_number(&record, "height", "scene.height")?;
        if width <= 0.0 {
            return Err(scene_error("SceneSchema", "scene.width must be positive"));
        }
        if height <= 0.0 {
            return Err(scene_error("SceneSchema", "scene.height must be positive"));
        }
        let background = required_string(&record, "background", "scene.background")?;
        let circles = record_value(&record, "circles")
            .map(elements_from_value::<CircleElement>)
            .transpose()?
            .unwrap_or_default();
        let lines = record_value(&record, "lines")
            .map(elements_from_value::<LineElement>)
            .transpose()?
            .unwrap_or_default();
        let mut ids = HashSet::new();
        for id in circles
            .iter()
            .map(|c| c.id.as_str())
            .chain(lines.iter().map(|l| l.id.as_str()))
        {
            if !ids.insert(id.to_string()) {
                return Err(scene_error(
                    "SceneSchema",
                    format!("duplicate scene element id `{id}`"),
                ));
            }
        }
        Ok(Self {
            width,
            height,
            background,
            circles,
            lines,
        })
    }
}

trait FromRecord: Sized {
    fn from_record(record: &MechRecord) -> MResult<Self>;
}

impl FromRecord for CircleElement {
    fn from_record(record: &MechRecord) -> MResult<Self> {
        let id = required_string(record, "id", "circle.id")?;
        let radius = required_number(record, "radius", &format!("circle `{id}` radius"))?;
        let stroke_width = required_number(
            record,
            "stroke-width",
            &format!("circle `{id}` stroke-width"),
        )?;
        let opacity = required_number(record, "opacity", &format!("circle `{id}` opacity"))?;
        if radius < 0.0 {
            return Err(scene_error(
                "SceneSchema",
                format!("circle `{id}` radius must be non-negative"),
            ));
        }
        validate_stroke_width(&id, stroke_width)?;
        validate_opacity(&id, opacity)?;
        Ok(Self {
            id: id.clone(),
            x: required_number(record, "x", &format!("circle `{id}` x"))?,
            y: required_number(record, "y", &format!("circle `{id}` y"))?,
            radius,
            fill: required_string(record, "fill", &format!("circle `{id}` fill"))?,
            stroke: required_string(record, "stroke", &format!("circle `{id}` stroke"))?,
            stroke_width,
            opacity,
        })
    }
}

impl FromRecord for LineElement {
    fn from_record(record: &MechRecord) -> MResult<Self> {
        let id = required_string(record, "id", "line.id")?;
        let stroke_width =
            required_number(record, "stroke-width", &format!("line `{id}` stroke-width"))?;
        let opacity = required_number(record, "opacity", &format!("line `{id}` opacity"))?;
        validate_stroke_width(&id, stroke_width)?;
        validate_opacity(&id, opacity)?;
        Ok(Self {
            id: id.clone(),
            x1: required_number(record, "x1", &format!("line `{id}` x1"))?,
            y1: required_number(record, "y1", &format!("line `{id}` y1"))?,
            x2: required_number(record, "x2", &format!("line `{id}` x2"))?,
            y2: required_number(record, "y2", &format!("line `{id}` y2"))?,
            stroke: required_string(record, "stroke", &format!("line `{id}` stroke"))?,
            stroke_width,
            line_cap: required_string(record, "line-cap", &format!("line `{id}` line-cap"))?,
            opacity,
            rotation: required_number(record, "rotation", &format!("line `{id}` rotation"))?,
            origin_x: required_number(record, "origin-x", &format!("line `{id}` origin-x"))?,
            origin_y: required_number(record, "origin-y", &format!("line `{id}` origin-y"))?,
        })
    }
}

fn validate_stroke_width(id: &str, value: f64) -> MResult<()> {
    if value < 0.0 {
        return Err(scene_error(
            "SceneSchema",
            format!("element `{id}` stroke-width must be non-negative"),
        ));
    }
    Ok(())
}
fn validate_opacity(id: &str, value: f64) -> MResult<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(scene_error(
            "SceneSchema",
            format!("element `{id}` opacity must be between 0 and 1"),
        ));
    }
    Ok(())
}

fn elements_from_value<T: FromRecord>(value: &Value) -> MResult<Vec<T>> {
    match value {
        Value::MutableReference(value) => elements_from_value::<T>(&value.borrow()),
        Value::Tuple(tuple) => tuple
            .borrow()
            .elements
            .iter()
            .map(|value| record_element::<T>(value.as_ref()))
            .collect(),
        Value::Table(table) => table_rows::<T>(&table.borrow()),
        Value::Empty => Ok(Vec::new()),
        other => Err(scene_error(
            "SceneSchema",
            format!("scene elements must be a tuple or table, got {other:?}"),
        )),
    }
}

fn record_element<T: FromRecord>(value: &Value) -> MResult<T> {
    if let Value::MutableReference(value) = value {
        return record_element::<T>(&value.borrow());
    }
    let Value::Record(record) = value else {
        return Err(scene_error("SceneSchema", "scene element must be a record"));
    };
    T::from_record(&record.borrow())
}

fn table_rows<T: FromRecord>(_table: &MechTable) -> MResult<Vec<T>> {
    Err(scene_error(
        "SceneSchema",
        "table-backed scene elements are not yet materialized by this build; use tuple records",
    ))
}

fn record_value<'a>(record: &'a MechRecord, field: &str) -> Option<&'a Value> {
    record.get(&hash_str(field))
}
fn required_value<'a>(record: &'a MechRecord, field: &str, label: &str) -> MResult<&'a Value> {
    record_value(record, field)
        .ok_or_else(|| scene_error("SceneSchema", format!("missing required field `{label}`")))
}
fn required_string(record: &MechRecord, field: &str, label: &str) -> MResult<String> {
    match required_value(record, field, label)? {
        Value::String(value) => Ok(value.borrow().clone()),
        Value::MutableReference(value) => match &*value.borrow() {
            Value::String(value) => Ok(value.borrow().clone()),
            _ => Err(scene_error(
                "SceneSchema",
                format!("field `{label}` must be a string"),
            )),
        },
        _ => Err(scene_error(
            "SceneSchema",
            format!("field `{label}` must be a string"),
        )),
    }
}
fn required_number(record: &MechRecord, field: &str, label: &str) -> MResult<f64> {
    match required_value(record, field, label)? {
        Value::F64(value) => Ok(*value.borrow()),
        Value::MutableReference(value) => match &*value.borrow() {
            Value::F64(value) => Ok(*value.borrow()),
            other => Err(scene_error(
                "SceneSchema",
                format!("field `{label}` must be numeric, got {other:?}"),
            )),
        },
        other => Err(scene_error(
            "SceneSchema",
            format!("field `{label}` must be numeric, got {other:?}"),
        )),
    }
}
