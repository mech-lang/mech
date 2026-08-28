use std::collections::HashSet;

use mech_core::{LegacyValue, MResult, MechRecord, MechTable, hash_str};
use mech_runtime::{
    host_arg_f64, host_arg_matrix_f64, host_arg_matrix_value_matrix, host_arg_optional,
    host_arg_record, host_arg_resolved, host_arg_string, host_arg_table, host_arg_tuple,
};

use crate::scene_error;

#[cfg_attr(feature = "rich-output", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg_attr(feature = "rich-output", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg_attr(feature = "rich-output", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LineStripElement {
    pub id: String,
    pub positions: Vec<[f64; 2]>,
    pub stroke: String,
    pub stroke_width: f64,
    pub line_cap: String,
    pub line_join: String,
    pub opacity: f64,
    pub closed: bool,
}

#[cfg_attr(feature = "rich-output", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TextElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fill: String,
    pub font_size: f64,
    pub font_family: String,
    pub font_weight: String,
    pub text_anchor: String,
    pub opacity: f64,
    pub value: String,
}

#[cfg_attr(feature = "rich-output", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshot {
    pub width: f64,
    pub height: f64,
    pub background: String,
    pub circles: Vec<CircleElement>,
    pub lines: Vec<LineElement>,
    #[cfg_attr(feature = "rich-output", serde(default))]
    pub line_strips: Vec<LineStripElement>,
    pub texts: Vec<TextElement>,
}

impl SceneSnapshot {
    pub fn from_value(value: &LegacyValue) -> MResult<Self> {
        let record = host_record(value, "scene must be a record")?;
        let allowed = [
            "width",
            "height",
            "background",
            "circles",
            "lines",
            "line-strips",
            "texts",
            "point-sets",
        ];
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
        let background = required_paint(&record, "background", "scene.background")?;
        let mut circles = record_value(&record, "circles")
            .map(elements_from_value::<CircleElement>)
            .transpose()?
            .unwrap_or_default();
        if let Some(point_sets) = record_value(&record, "point-sets") {
            circles.extend(point_set_circles(point_sets)?);
        }
        let lines = record_value(&record, "lines")
            .map(elements_from_value::<LineElement>)
            .transpose()?
            .unwrap_or_default();
        let line_strips = record_value(&record, "line-strips")
            .map(line_strips_from_value)
            .transpose()?
            .unwrap_or_default();
        let texts = record_value(&record, "texts")
            .map(elements_from_value::<TextElement>)
            .transpose()?
            .unwrap_or_default();
        let mut ids = HashSet::new();
        for id in circles
            .iter()
            .map(|c| c.id.as_str())
            .chain(lines.iter().map(|l| l.id.as_str()))
            .chain(line_strips.iter().map(|strip| strip.id.as_str()))
            .chain(texts.iter().map(|text| text.id.as_str()))
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
            line_strips,
            texts,
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
        "id",
        "x",
        "y",
        "radius",
        "fill",
        "stroke",
        "stroke-width",
        "opacity",
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
        validate_radius(radius, &format!("circle `{id}` radius"))?;
        validate_stroke_width(&id, stroke_width)?;
        validate_opacity(&id, opacity)?;
        Ok(Self {
            id: id.clone(),
            x: finite_number(
                required_number(record, "x", &format!("circle `{id}` x"))?,
                &format!("circle `{id}` x"),
            )?,
            y: finite_number(
                required_number(record, "y", &format!("circle `{id}` y"))?,
                &format!("circle `{id}` y"),
            )?,
            radius,
            fill: required_paint(record, "fill", &format!("circle `{id}` fill"))?,
            stroke: required_paint(record, "stroke", &format!("circle `{id}` stroke"))?,
            stroke_width,
            opacity,
        })
    }
}

impl FromRecord for LineElement {
    const KIND: &'static str = "line";
    const REQUIRED: &'static [&'static str] = &[
        "id",
        "x1",
        "y1",
        "x2",
        "y2",
        "stroke",
        "stroke-width",
        "line-cap",
        "opacity",
        "rotation",
        "origin-x",
        "origin-y",
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
            x1: finite_number(
                required_number(record, "x1", &format!("line `{id}` x1"))?,
                &format!("line `{id}` x1"),
            )?,
            y1: finite_number(
                required_number(record, "y1", &format!("line `{id}` y1"))?,
                &format!("line `{id}` y1"),
            )?,
            x2: finite_number(
                required_number(record, "x2", &format!("line `{id}` x2"))?,
                &format!("line `{id}` x2"),
            )?,
            y2: finite_number(
                required_number(record, "y2", &format!("line `{id}` y2"))?,
                &format!("line `{id}` y2"),
            )?,
            stroke: required_paint(record, "stroke", &format!("line `{id}` stroke"))?,
            stroke_width,
            line_cap,
            opacity,
            rotation: finite_number(
                required_number(record, "rotation", &format!("line `{id}` rotation"))?,
                &format!("line `{id}` rotation"),
            )?,
            origin_x: finite_number(
                required_number(record, "origin-x", &format!("line `{id}` origin-x"))?,
                &format!("line `{id}` origin-x"),
            )?,
            origin_y: finite_number(
                required_number(record, "origin-y", &format!("line `{id}` origin-y"))?,
                &format!("line `{id}` origin-y"),
            )?,
        })
    }
}

impl FromRecord for TextElement {
    const KIND: &'static str = "text";
    const REQUIRED: &'static [&'static str] = &[
        "id",
        "x",
        "y",
        "fill",
        "font-size",
        "font-family",
        "font-weight",
        "text-anchor",
        "opacity",
        "value",
    ];
    fn from_record(record: &MechRecord) -> MResult<Self> {
        reject_unknown_fields(record, Self::REQUIRED, Self::KIND)?;
        let id = required_string(record, "id", "text.id")?;
        validate_id(Self::KIND, &id)?;
        let font_size = required_number(record, "font-size", &format!("text `{id}` font-size"))?;
        if !font_size.is_finite() || font_size <= 0.0 {
            return Err(scene_error(
                "SceneSchema",
                format!("text `{id}` font-size must be finite and positive"),
            ));
        }
        let text_anchor =
            required_string(record, "text-anchor", &format!("text `{id}` text-anchor"))?;
        validate_text_anchor(&id, &text_anchor)?;
        let opacity = required_number(record, "opacity", &format!("text `{id}` opacity"))?;
        validate_opacity(&id, opacity)?;
        Ok(Self {
            id: id.clone(),
            x: finite_number(
                required_number(record, "x", &format!("text `{id}` x"))?,
                &format!("text `{id}` x"),
            )?,
            y: finite_number(
                required_number(record, "y", &format!("text `{id}` y"))?,
                &format!("text `{id}` y"),
            )?,
            fill: required_paint(record, "fill", &format!("text `{id}` fill"))?,
            font_size,
            font_family: required_string(
                record,
                "font-family",
                &format!("text `{id}` font-family"),
            )?,
            font_weight: required_font_weight(
                record,
                "font-weight",
                &format!("text `{id}` font-weight"),
            )?,
            text_anchor,
            opacity,
            value: required_string(record, "value", &format!("text `{id}` value"))?,
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
fn validate_line_join(id: &str, value: &str) -> MResult<()> {
    if !matches!(value, "miter" | "round" | "bevel") {
        return Err(scene_error(
            "SceneSchema",
            format!("line-strip `{id}` line-join must be miter, round, or bevel"),
        ));
    }
    Ok(())
}
fn validate_text_anchor(id: &str, value: &str) -> MResult<()> {
    if !matches!(value, "start" | "middle" | "end") {
        return Err(scene_error(
            "SceneSchema",
            format!("text `{id}` text-anchor must be start, middle, or end"),
        ));
    }
    Ok(())
}
fn validate_radius(value: f64, label: &str) -> MResult<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(scene_error(
            "SceneSchema",
            format!("{label} must be finite and non-negative"),
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

fn elements_from_value<T: FromRecord>(value: &LegacyValue) -> MResult<Vec<T>> {
    if host_arg_optional(SCENE_SCHEMA, one_arg(value), 0)
        .map_err(|_| scene_error("SceneSchema", "scene elements could not be resolved"))?
        .is_none()
    {
        return Ok(Vec::new());
    }
    if let Ok(tuple) = host_arg_tuple(SCENE_SCHEMA, one_arg(value), 0) {
        return tuple
            .elements
            .iter()
            .map(|value| record_element::<T>(value.as_ref()))
            .collect();
    }
    if let Ok(table) = host_arg_table(SCENE_SCHEMA, one_arg(value), 0) {
        return table_rows::<T>(&table);
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "scene elements must be a tuple or table, got {:?}",
            resolved_for_diagnostic(value)
        ),
    ))
}

fn point_set_circles(value: &LegacyValue) -> MResult<Vec<CircleElement>> {
    if host_arg_optional(SCENE_SCHEMA, one_arg(value), 0)
        .map_err(|_| scene_error("SceneSchema", "scene point-sets could not be resolved"))?
        .is_none()
    {
        return Ok(Vec::new());
    }
    if let Ok(tuple) = host_arg_tuple(SCENE_SCHEMA, one_arg(value), 0) {
        let mut circles = Vec::new();
        for value in &tuple.elements {
            circles.extend(point_set_from_record_value(value.as_ref())?);
        }
        return Ok(circles);
    }
    if let Ok(table) = host_arg_table(SCENE_SCHEMA, one_arg(value), 0) {
        let records = table_records(&table, "point-set", POINT_SET_FIELDS)?;
        let mut circles = Vec::new();
        for (row, record) in records.iter().enumerate() {
            circles.extend(point_set_from_record(record).map_err(|err| {
                scene_error(
                    "SceneSchema",
                    format!("point-set table row {}: {err:?}", row + 1),
                )
            })?);
        }
        return Ok(circles);
    }
    if host_arg_record(SCENE_SCHEMA, one_arg(value), 0).is_ok() {
        return point_set_from_record_value(value);
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "scene point-sets must be a record, tuple, or table, got {:?}",
            resolved_for_diagnostic(value)
        ),
    ))
}

const POINT_SET_FIELDS: &[&str] = &[
    "id",
    "positions",
    "radius",
    "first-radius",
    "fills",
    "stroke",
    "stroke-width",
    "opacity",
];

fn point_set_from_record_value(value: &LegacyValue) -> MResult<Vec<CircleElement>> {
    let record = host_record(value, "scene point-set must be a record")?;
    point_set_from_record(&record)
}

fn point_set_from_record(record: &MechRecord) -> MResult<Vec<CircleElement>> {
    reject_unknown_fields(record, POINT_SET_FIELDS, "point-set")?;
    let id = required_string(record, "id", "point-set.id")?;
    validate_id("point-set", &id)?;
    let positions = required_value(record, "positions", &format!("point-set `{id}` positions"))?;
    let positions = matrix_f64_values(positions, &format!("point-set `{id}` positions"))?;
    if positions.rows == 0 || positions.columns != 2 {
        return Err(scene_error(
            "SceneSchema",
            format!("point-set `{id}` positions must be a non-empty f64 matrix with two columns"),
        ));
    }
    let radius = required_number(record, "radius", &format!("point-set `{id}` radius"))?;
    let first_radius = required_number(
        record,
        "first-radius",
        &format!("point-set `{id}` first-radius"),
    )?;
    validate_radius(radius, &format!("point-set `{id}` radius"))?;
    validate_radius(first_radius, &format!("point-set `{id}` first-radius"))?;
    let fills = required_strings(record, "fills", &format!("point-set `{id}` fills"))?;
    if fills.len() != positions.rows {
        return Err(scene_error(
            "SceneSchema",
            format!(
                "point-set `{id}` fills length mismatch: expected {}, got {}",
                positions.rows,
                fills.len()
            ),
        ));
    }
    let stroke = required_paint(record, "stroke", &format!("point-set `{id}` stroke"))?;
    let stroke_width = required_number(
        record,
        "stroke-width",
        &format!("point-set `{id}` stroke-width"),
    )?;
    validate_stroke_width(&id, stroke_width)?;
    let opacity = required_number(record, "opacity", &format!("point-set `{id}` opacity"))?;
    validate_opacity(&id, opacity)?;

    let mut circles = Vec::with_capacity(positions.rows);
    for row in 0..positions.rows {
        let x = positions.values[row];
        let y = positions.values[positions.rows + row];
        if !x.is_finite() || !y.is_finite() {
            return Err(scene_error(
                "SceneSchema",
                format!("point-set `{id}` row {row} contains a nonfinite coordinate"),
            ));
        }
        circles.push(CircleElement {
            id: format!("{id}-{row}"),
            x,
            y,
            radius: if row == 0 { first_radius } else { radius },
            fill: fills[row].clone(),
            stroke: stroke.clone(),
            stroke_width,
            opacity,
        });
    }
    Ok(circles)
}

fn line_strips_from_value(value: &LegacyValue) -> MResult<Vec<LineStripElement>> {
    if host_arg_optional(SCENE_SCHEMA, one_arg(value), 0)
        .map_err(|_| scene_error("SceneSchema", "scene line-strips could not be resolved"))?
        .is_none()
    {
        return Ok(Vec::new());
    }
    if let Ok(tuple) = host_arg_tuple(SCENE_SCHEMA, one_arg(value), 0) {
        return tuple
            .elements
            .iter()
            .map(|value| line_strip_from_record_value(value.as_ref()))
            .collect();
    }
    if let Ok(table) = host_arg_table(SCENE_SCHEMA, one_arg(value), 0) {
        return table_records(&table, "line-strip", LINE_STRIP_FIELDS)?
            .iter()
            .enumerate()
            .map(|(row, record)| {
                line_strip_from_record(record).map_err(|err| {
                    scene_error(
                        "SceneSchema",
                        format!("line-strip table row {}: {err:?}", row + 1),
                    )
                })
            })
            .collect();
    }
    if host_arg_record(SCENE_SCHEMA, one_arg(value), 0).is_ok() {
        return Ok(vec![line_strip_from_record_value(value)?]);
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "scene line-strips must be a record, tuple, or table, got {:?}",
            resolved_for_diagnostic(value)
        ),
    ))
}

const LINE_STRIP_FIELDS: &[&str] = &[
    "id",
    "positions",
    "stroke",
    "stroke-width",
    "line-cap",
    "line-join",
    "opacity",
    "closed",
];

fn line_strip_from_record_value(value: &LegacyValue) -> MResult<LineStripElement> {
    let record = host_record(value, "scene line-strip must be a record")?;
    line_strip_from_record(&record)
}

fn line_strip_from_record(record: &MechRecord) -> MResult<LineStripElement> {
    reject_unknown_fields(record, LINE_STRIP_FIELDS, "line-strip")?;
    let id = required_string(record, "id", "line-strip.id")?;
    validate_id("line-strip", &id)?;
    let positions = required_value(record, "positions", &format!("line-strip `{id}` positions"))?;
    let positions = matrix_f64_values(positions, &format!("line-strip `{id}` positions"))?;
    if positions.rows < 2 || positions.columns != 2 {
        return Err(scene_error(
            "SceneSchema",
            format!(
                "line-strip `{id}` positions must be an f64 matrix with at least two rows and exactly two columns"
            ),
        ));
    }
    let positions = (0..positions.rows)
        .map(|row| {
            let x = positions.values[row];
            let y = positions.values[positions.rows + row];
            if !x.is_finite() || !y.is_finite() {
                return Err(scene_error(
                    "SceneSchema",
                    format!("line-strip `{id}` row {row} contains a nonfinite coordinate"),
                ));
            }
            Ok([x, y])
        })
        .collect::<MResult<Vec<_>>>()?;
    let stroke_width = required_number(
        record,
        "stroke-width",
        &format!("line-strip `{id}` stroke-width"),
    )?;
    validate_stroke_width(&id, stroke_width)?;
    let opacity = required_number(record, "opacity", &format!("line-strip `{id}` opacity"))?;
    validate_opacity(&id, opacity)?;
    let line_cap = required_string(record, "line-cap", &format!("line-strip `{id}` line-cap"))?;
    validate_line_cap(&id, &line_cap)?;
    let line_join = required_string(record, "line-join", &format!("line-strip `{id}` line-join"))?;
    validate_line_join(&id, &line_join)?;
    Ok(LineStripElement {
        id,
        positions,
        stroke: required_paint(record, "stroke", "line-strip.stroke")?,
        stroke_width,
        line_cap,
        line_join,
        opacity,
        closed: required_bool(record, "closed", "line-strip.closed")?,
    })
}

struct F64MatrixValues {
    rows: usize,
    columns: usize,
    values: Vec<f64>,
}

fn matrix_f64_values(value: &LegacyValue, label: &str) -> MResult<F64MatrixValues> {
    if let Ok(matrix) = host_arg_matrix_f64(SCENE_SCHEMA, one_arg(value), 0) {
        return Ok(F64MatrixValues {
            rows: matrix.rows(),
            columns: matrix.cols(),
            values: matrix.as_vec(),
        });
    }
    if let Ok(matrix) = host_arg_matrix_value_matrix(SCENE_SCHEMA, one_arg(value), 0) {
        let values = matrix
            .as_vec()
            .into_iter()
            .map(|value| {
                host_arg_f64(SCENE_SCHEMA, one_arg(&value), 0).map_err(|_| {
                    scene_error(
                        "SceneSchema",
                        format!("field `{label}` must contain only f64 values"),
                    )
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        return Ok(F64MatrixValues {
            rows: matrix.rows(),
            columns: matrix.cols(),
            values,
        });
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "field `{label}` must be a dense f64 matrix, got {:?}",
            resolved_for_diagnostic(value)
        ),
    ))
}

fn record_element<T: FromRecord>(value: &LegacyValue) -> MResult<T> {
    T::from_record(&host_record(value, "scene element must be a record")?)
}

fn table_rows<T: FromRecord>(table: &MechTable) -> MResult<Vec<T>> {
    table_records(table, T::KIND, T::REQUIRED)?
        .iter()
        .enumerate()
        .map(|(row, record)| {
            T::from_record(record).map_err(|err| {
                scene_error(
                    "SceneSchema",
                    format!("{} table row {}: {err:?}", T::KIND, row + 1),
                )
            })
        })
        .collect()
}

fn table_records(
    table: &MechTable,
    kind: &str,
    required_fields: &[&str],
) -> MResult<Vec<MechRecord>> {
    for required in required_fields {
        if !table.col_names.values().any(|name| name == required) {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} table missing required column `{required}`"),
            ));
        }
    }
    for (col_id, name) in &table.col_names {
        if !required_fields.contains(&name.as_str()) {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} table unknown column `{name}`"),
            ));
        }
        let Some((_, matrix)) = table.data.get(col_id) else {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} table column `{name}` has no data"),
            ));
        };
        if matrix.rows() != table.rows {
            return Err(scene_error(
                "SceneSchema",
                format!(
                    "{kind} table column `{name}` length mismatch: expected {}, got {}",
                    table.rows,
                    matrix.rows()
                ),
            ));
        }
    }
    let mut records = Vec::with_capacity(table.rows);
    for row in 1..=table.rows {
        let record = table.get_record(row).ok_or_else(|| {
            scene_error(
                "SceneSchema",
                format!("{kind} table row {row} could not be materialized"),
            )
        })?;
        records.push(record);
    }
    Ok(records)
}

fn record_value<'a>(record: &'a MechRecord, field: &str) -> Option<&'a LegacyValue> {
    record.get(&hash_str(field))
}
fn required_value<'a>(
    record: &'a MechRecord,
    field: &str,
    label: &str,
) -> MResult<&'a LegacyValue> {
    record_value(record, field)
        .ok_or_else(|| scene_error("SceneSchema", format!("missing required field `{label}`")))
}
fn required_string(record: &MechRecord, field: &str, label: &str) -> MResult<String> {
    host_arg_string(
        SCENE_SCHEMA,
        one_arg(required_value(record, field, label)?),
        0,
    )
    .map_err(|_| scene_error("SceneSchema", format!("field `{label}` must be a string")))
}
fn required_paint(record: &MechRecord, field: &str, label: &str) -> MResult<String> {
    let resolved = host_arg_resolved(
        SCENE_SCHEMA,
        one_arg(required_value(record, field, label)?),
        0,
    )
    .map_err(|_| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be a paint string or f64 RGB color"),
        )
    })?;
    match resolved {
        LegacyValue::String(value) => Ok(value.borrow().clone()),
        LegacyValue::F64(value)
            if value.borrow().is_finite()
                && value.borrow().fract() == 0.0
                && (0.0..=16_777_215.0).contains(&*value.borrow()) =>
        {
            Ok(format!("#{:06x}", *value.borrow() as u32))
        }
        _ => Err(scene_error(
            "SceneSchema",
            format!("field `{label}` must be a paint string or 24-bit numeric RGB color"),
        )),
    }
}
fn required_font_weight(record: &MechRecord, field: &str, label: &str) -> MResult<String> {
    let resolved = host_arg_resolved(
        SCENE_SCHEMA,
        one_arg(required_value(record, field, label)?),
        0,
    )
    .map_err(|_| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be a string or f64"),
        )
    })?;
    match resolved {
        LegacyValue::String(value) => Ok(value.borrow().clone()),
        LegacyValue::F64(value)
            if value.borrow().is_finite()
                && value.borrow().fract() == 0.0
                && (1.0..=1000.0).contains(&*value.borrow()) =>
        {
            Ok(format!("{:.0}", *value.borrow()))
        }
        _ => Err(scene_error(
            "SceneSchema",
            format!("field `{label}` must be a string or a numeric value from 1 through 1000"),
        )),
    }
}
fn required_strings(record: &MechRecord, field: &str, label: &str) -> MResult<Vec<String>> {
    strings_from_value(required_value(record, field, label)?, label)
}
fn strings_from_value(value: &LegacyValue, label: &str) -> MResult<Vec<String>> {
    let tuple = host_arg_tuple(SCENE_SCHEMA, one_arg(value), 0).map_err(|_| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be a tuple of strings"),
        )
    })?;
    tuple
        .elements
        .iter()
        .map(|value| {
            host_arg_string(SCENE_SCHEMA, one_arg(value.as_ref()), 0).map_err(|_| {
                scene_error(
                    "SceneSchema",
                    format!("field `{label}` must contain only strings"),
                )
            })
        })
        .collect()
}
fn required_number(record: &MechRecord, field: &str, label: &str) -> MResult<f64> {
    let value = required_value(record, field, label)?;
    let value = host_arg_f64(SCENE_SCHEMA, one_arg(value), 0).map_err(|_| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be numeric, got {value:?}"),
        )
    })?;
    finite_number(value, label)
}
fn required_bool(record: &MechRecord, field: &str, label: &str) -> MResult<bool> {
    let resolved = host_arg_resolved(
        SCENE_SCHEMA,
        one_arg(required_value(record, field, label)?),
        0,
    )
    .map_err(|_| scene_error("SceneSchema", format!("field `{label}` must be a bool")))?;
    match resolved {
        LegacyValue::Bool(value) => Ok(*value.borrow()),
        _ => Err(scene_error(
            "SceneSchema",
            format!("field `{label}` must be a bool"),
        )),
    }
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

const SCENE_SCHEMA: &str = "scene schema";

fn one_arg(value: &LegacyValue) -> &[LegacyValue] {
    std::slice::from_ref(value)
}

fn host_record(value: &LegacyValue, message: &str) -> MResult<MechRecord> {
    host_arg_record(SCENE_SCHEMA, one_arg(value), 0)
        .map_err(|_| scene_error("SceneSchema", message))
}

fn resolved_for_diagnostic(value: &LegacyValue) -> LegacyValue {
    host_arg_resolved(SCENE_SCHEMA, one_arg(value), 0).unwrap_or_else(|_| value.clone())
}
