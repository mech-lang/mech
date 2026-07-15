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
        if !width.is_finite() || width <= 0.0 {
            return Err(scene_error("SceneSchema", "scene.width must be positive"));
        }
        if !height.is_finite() || height <= 0.0 {
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
    const KIND: &'static str;
    const REQUIRED: &'static [&'static str];
    fn from_record(record: &MechRecord) -> MResult<Self>;
}

impl FromRecord for CircleElement {
    const KIND: &'static str = "circle";
    const REQUIRED: &'static [&'static str] = &[
        "id", "x", "y", "radius", "fill", "stroke", "stroke-width", "opacity",
    ];
    fn from_record(record: &MechRecord) -> MResult<Self> {
        reject_unknown_fields(record, Self::REQUIRED, Self::KIND)?;
        let id = required_string(record, "id", "circle.id")?;
        validate_id(Self::KIND, &id)?;
        let radius = required_number(record, "radius", &format!("circle `{id}` radius"))?;
        let stroke_width = required_number(
            record,
            "stroke-width",
            &format!("circle `{id}` stroke-width"),
        )?;
        let opacity = required_number(record, "opacity", &format!("circle `{id}` opacity"))?;
        if !radius.is_finite() || radius < 0.0 {
            return Err(scene_error(
                "SceneSchema",
                format!("circle `{id}` radius must be finite and non-negative"),
            ));
        }
        validate_stroke_width(&id, stroke_width)?;
        validate_opacity(&id, opacity)?;
        Ok(Self {
            id: id.clone(),
            x: finite_number(required_number(record, "x", &format!("circle `{id}` x"))?, &format!("circle `{id}` x"))?,
            y: finite_number(required_number(record, "y", &format!("circle `{id}` y"))?, &format!("circle `{id}` y"))?,
            radius,
            fill: required_string(record, "fill", &format!("circle `{id}` fill"))?,
            stroke: required_string(record, "stroke", &format!("circle `{id}` stroke"))?,
            stroke_width,
            opacity,
        })
    }
}

impl FromRecord for LineElement {
    const KIND: &'static str = "line";
    const REQUIRED: &'static [&'static str] = &[
        "id", "x1", "y1", "x2", "y2", "stroke", "stroke-width", "line-cap", "opacity",
        "rotation", "origin-x", "origin-y",
    ];
    fn from_record(record: &MechRecord) -> MResult<Self> {
        reject_unknown_fields(record, Self::REQUIRED, Self::KIND)?;
        let id = required_string(record, "id", "line.id")?;
        validate_id(Self::KIND, &id)?;
        let stroke_width =
            required_number(record, "stroke-width", &format!("line `{id}` stroke-width"))?;
        let opacity = required_number(record, "opacity", &format!("line `{id}` opacity"))?;
        validate_stroke_width(&id, stroke_width)?;
        validate_opacity(&id, opacity)?;
        let line_cap = required_string(record, "line-cap", &format!("line `{id}` line-cap"))?;
        validate_line_cap(&id, &line_cap)?;
        Ok(Self {
            id: id.clone(),
            x1: finite_number(required_number(record, "x1", &format!("line `{id}` x1"))?, &format!("line `{id}` x1"))?,
            y1: finite_number(required_number(record, "y1", &format!("line `{id}` y1"))?, &format!("line `{id}` y1"))?,
            x2: finite_number(required_number(record, "x2", &format!("line `{id}` x2"))?, &format!("line `{id}` x2"))?,
            y2: finite_number(required_number(record, "y2", &format!("line `{id}` y2"))?, &format!("line `{id}` y2"))?,
            stroke: required_string(record, "stroke", &format!("line `{id}` stroke"))?,
            stroke_width,
            line_cap,
            opacity,
            rotation: finite_number(required_number(record, "rotation", &format!("line `{id}` rotation"))?, &format!("line `{id}` rotation"))?,
            origin_x: finite_number(required_number(record, "origin-x", &format!("line `{id}` origin-x"))?, &format!("line `{id}` origin-x"))?,
            origin_y: finite_number(required_number(record, "origin-y", &format!("line `{id}` origin-y"))?, &format!("line `{id}` origin-y"))?,
        })
    }
}

fn validate_stroke_width(id: &str, value: f64) -> MResult<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(scene_error(
            "SceneSchema",
            format!("element `{id}` stroke-width must be finite and non-negative"),
        ));
    }
    Ok(())
}
fn validate_opacity(id: &str, value: f64) -> MResult<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(scene_error(
            "SceneSchema",
            format!("element `{id}` opacity must be finite and between 0 and 1"),
        ));
    }
    Ok(())
}
fn validate_id(kind: &str, id: &str) -> MResult<()> {
    if id.is_empty() {
        return Err(scene_error(
            "SceneSchema",
            format!("{kind} id must be non-empty"),
        ));
    }
    Ok(())
}
fn validate_line_cap(id: &str, value: &str) -> MResult<()> {
    if !matches!(value, "butt" | "round" | "square") {
        return Err(scene_error(
            "SceneSchema",
            format!("line `{id}` line-cap must be butt, round, or square"),
        ));
    }
    Ok(())
}
fn finite_number(value: f64, label: &str) -> MResult<f64> {
    if !value.is_finite() {
        return Err(scene_error(
            "SceneSchema",
            format!("field `{label}` must be finite"),
        ));
    }
    Ok(value)
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

fn table_rows<T: FromRecord>(table: &MechTable) -> MResult<Vec<T>> {
    for required in T::REQUIRED {
        if !table.col_names.values().any(|name| name == required) {
            return Err(scene_error(
                "SceneSchema",
                format!("{} table missing required column `{required}`", T::KIND),
            ));
        }
    }
    for (col_id, name) in &table.col_names {
        if !T::REQUIRED.contains(&name.as_str()) {
            return Err(scene_error(
                "SceneSchema",
                format!("{} table unknown column `{name}`", T::KIND),
            ));
        }
        let Some((_, matrix)) = table.data.get(col_id) else {
            return Err(scene_error(
                "SceneSchema",
                format!("{} table column `{name}` has no data", T::KIND),
            ));
        };
        if matrix.rows() != table.rows {
            return Err(scene_error(
                "SceneSchema",
                format!(
                    "{} table column `{name}` length mismatch: expected {}, got {}",
                    T::KIND,
                    table.rows,
                    matrix.rows()
                ),
            ));
        }
    }
    let mut out = Vec::with_capacity(table.rows);
    for row in 1..=table.rows {
        let record = table.get_record(row).ok_or_else(|| {
            scene_error(
                "SceneSchema",
                format!("{} table row {row} could not be materialized", T::KIND),
            )
        })?;
        out.push(T::from_record(&record).map_err(|err| {
            scene_error(
                "SceneSchema",
                format!("{} table row {row}: {err:?}", T::KIND),
            )
        })?);
    }
    Ok(out)
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
    let value = required_value(record, field, label)?;
    let value = value.as_f64().map_err(|_| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be numeric, got {value:?}"),
        )
    })?;
    finite_number(*value.borrow(), label)
}
fn reject_unknown_fields(record: &MechRecord, allowed: &[&str], kind: &str) -> MResult<()> {
    for (_, name) in &record.field_names {
        if !allowed.contains(&name.as_str()) {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} has unknown field `{name}`"),
            ));
        }
    }
    Ok(())
}
