use std::collections::{BTreeMap, HashSet};

use mech_core::snapshot::SequenceView;
use mech_core::{MResult, SchemaBody, ShapeInstance, Value, ValueData};

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
    pub fn from_value(value: &Value) -> MResult<Self> {
        let value = CanonicalNode::from_value(value)?;
        let record = value
            .as_record()
            .ok_or_else(|| scene_error("SceneSchema", "scene must be a record"))?;
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
        for name in record.fields.keys() {
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
    fn from_record(record: &CanonicalRecord) -> MResult<Self>;
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
    fn from_record(record: &CanonicalRecord) -> MResult<Self> {
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
    fn from_record(record: &CanonicalRecord) -> MResult<Self> {
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
    fn from_record(record: &CanonicalRecord) -> MResult<Self> {
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

fn elements_from_value<T: FromRecord>(value: &CanonicalNode) -> MResult<Vec<T>> {
    let Some(value) = value.resolved() else {
        return Ok(Vec::new());
    };
    if let Some(tuple) = value.as_tuple() {
        return tuple.iter().map(record_element::<T>).collect();
    }
    if value.is_table() {
        return table_rows::<T>(&value);
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "scene elements must be a tuple or table, got {:?}",
            resolved_for_diagnostic(&value)
        ),
    ))
}

fn point_set_circles(value: &CanonicalNode) -> MResult<Vec<CircleElement>> {
    let Some(value) = value.resolved() else {
        return Ok(Vec::new());
    };
    if let Some(tuple) = value.as_tuple() {
        let mut circles = Vec::new();
        for value in &tuple {
            circles.extend(point_set_from_record_value(value)?);
        }
        return Ok(circles);
    }
    if value.is_table() {
        let records = table_records(&value, "point-set", POINT_SET_FIELDS)?;
        let mut circles = Vec::new();
        for (row, record) in records.iter().enumerate() {
            circles.extend(point_set_from_record(record).map_err(|error| {
                scene_error(
                    "SceneSchema",
                    format!("point-set table row {}: {error:?}", row + 1),
                )
            })?);
        }
        return Ok(circles);
    }
    if value.as_record().is_some() {
        return point_set_from_record_value(&value);
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "scene point-sets must be a record, tuple, or table, got {:?}",
            resolved_for_diagnostic(&value)
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

fn point_set_from_record_value(value: &CanonicalNode) -> MResult<Vec<CircleElement>> {
    let record = value
        .as_record()
        .ok_or_else(|| scene_error("SceneSchema", "scene point-set must be a record"))?;
    point_set_from_record(&record)
}

fn point_set_from_record(record: &CanonicalRecord) -> MResult<Vec<CircleElement>> {
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

fn line_strips_from_value(value: &CanonicalNode) -> MResult<Vec<LineStripElement>> {
    let Some(value) = value.resolved() else {
        return Ok(Vec::new());
    };
    if let Some(tuple) = value.as_tuple() {
        return tuple.iter().map(line_strip_from_record_value).collect();
    }
    if value.is_table() {
        return table_records(&value, "line-strip", LINE_STRIP_FIELDS)?
            .iter()
            .enumerate()
            .map(|(row, record)| {
                line_strip_from_record(record).map_err(|error| {
                    scene_error(
                        "SceneSchema",
                        format!("line-strip table row {}: {error:?}", row + 1),
                    )
                })
            })
            .collect();
    }
    if value.as_record().is_some() {
        return Ok(vec![line_strip_from_record_value(&value)?]);
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "scene line-strips must be a record, tuple, or table, got {:?}",
            resolved_for_diagnostic(&value)
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

fn line_strip_from_record_value(value: &CanonicalNode) -> MResult<LineStripElement> {
    let record = value
        .as_record()
        .ok_or_else(|| scene_error("SceneSchema", "scene line-strip must be a record"))?;
    line_strip_from_record(&record)
}

fn line_strip_from_record(record: &CanonicalRecord) -> MResult<LineStripElement> {
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

fn matrix_f64_values(value: &CanonicalNode, label: &str) -> MResult<F64MatrixValues> {
    if let Some((rows, columns, row_major)) = value.f64_matrix() {
        let mut values = Vec::with_capacity(row_major.len());
        for column in 0..columns {
            for row in 0..rows {
                values.push(row_major[row * columns + column]);
            }
        }
        return Ok(F64MatrixValues {
            rows,
            columns,
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

fn record_element<T: FromRecord>(value: &CanonicalNode) -> MResult<T> {
    let record = value
        .as_record()
        .ok_or_else(|| scene_error("SceneSchema", "scene element must be a record"))?;
    T::from_record(&record)
}

fn table_rows<T: FromRecord>(table: &CanonicalNode) -> MResult<Vec<T>> {
    let (ValueData::Table(table_data), SchemaBody::Table { columns, .. }) =
        (&table.data, &table.schema)
    else {
        return Err(scene_error("SceneSchema", "scene elements must be a table"));
    };
    for required in T::REQUIRED {
        if !columns.iter().any(|column| column.name == *required) {
            return Err(scene_error(
                "SceneSchema",
                format!("{} table missing required column `{required}`", T::KIND),
            ));
        }
    }
    let rows = table_data.column(0).map(sequence_len).unwrap_or_default();
    for (column_index, column) in columns.iter().enumerate() {
        if !T::REQUIRED.contains(&column.name.as_str()) {
            return Err(scene_error(
                "SceneSchema",
                format!("{} table unknown column `{}`", T::KIND, column.name),
            ));
        }
        let Some(values) = table_data.column(column_index) else {
            return Err(scene_error(
                "SceneSchema",
                format!("{} table column `{}` has no data", T::KIND, column.name),
            ));
        };
        if sequence_len(values) != rows {
            return Err(scene_error(
                "SceneSchema",
                format!(
                    "{} table column `{name}` length mismatch: expected {}, got {}",
                    T::KIND,
                    rows,
                    sequence_len(values),
                    name = column.name,
                ),
            ));
        }
    }
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let display_row = row + 1;
        let mut fields = BTreeMap::new();
        for (column_index, column) in columns.iter().enumerate() {
            let data = table_data
                .column(column_index)
                .and_then(|values| sequence_value(values, row))
                .ok_or_else(|| {
                    scene_error(
                        "SceneSchema",
                        format!(
                            "{} table row {display_row} could not be materialized",
                            T::KIND
                        ),
                    )
                })?;
            fields.insert(
                column.name.clone(),
                CanonicalNode {
                    data,
                    schema: column.schema.clone(),
                    shape: table.shape.clone(),
                },
            );
        }
        let record = CanonicalRecord { fields };
        out.push(T::from_record(&record).map_err(|err| {
            scene_error(
                "SceneSchema",
                format!("{} table row {display_row}: {err:?}", T::KIND),
            )
        })?);
    }
    Ok(out)
}

fn table_records(
    table: &CanonicalNode,
    kind: &str,
    required_fields: &[&str],
) -> MResult<Vec<CanonicalRecord>> {
    let (ValueData::Table(table_data), SchemaBody::Table { columns, .. }) =
        (&table.data, &table.schema)
    else {
        return Err(scene_error("SceneSchema", "scene elements must be a table"));
    };
    for required in required_fields {
        if !columns.iter().any(|column| column.name == *required) {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} table missing required column `{required}`"),
            ));
        }
    }
    let rows = table_data.column(0).map(sequence_len).unwrap_or_default();
    for (column_index, column) in columns.iter().enumerate() {
        if !required_fields.contains(&column.name.as_str()) {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} table unknown column `{}`", column.name),
            ));
        }
        let Some(values) = table_data.column(column_index) else {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} table column `{}` has no data", column.name),
            ));
        };
        if sequence_len(values) != rows {
            return Err(scene_error(
                "SceneSchema",
                format!(
                    "{kind} table column `{name}` length mismatch: expected {rows}, got {found}",
                    name = column.name,
                    found = sequence_len(values),
                ),
            ));
        }
    }
    let mut records = Vec::with_capacity(rows);
    for row in 0..rows {
        let display_row = row + 1;
        let mut fields = BTreeMap::new();
        for (column_index, column) in columns.iter().enumerate() {
            let data = table_data
                .column(column_index)
                .and_then(|values| sequence_value(values, row))
                .ok_or_else(|| {
                    scene_error(
                        "SceneSchema",
                        format!("{kind} table row {display_row} could not be materialized"),
                    )
                })?;
            fields.insert(
                column.name.clone(),
                CanonicalNode {
                    data,
                    schema: column.schema.clone(),
                    shape: table.shape.clone(),
                },
            );
        }
        records.push(CanonicalRecord { fields });
    }
    Ok(records)
}

fn record_value<'a>(record: &'a CanonicalRecord, field: &str) -> Option<&'a CanonicalNode> {
    record.fields.get(field)
}
fn required_value<'a>(
    record: &'a CanonicalRecord,
    field: &str,
    label: &str,
) -> MResult<&'a CanonicalNode> {
    record_value(record, field)
        .ok_or_else(|| scene_error("SceneSchema", format!("missing required field `{label}`")))
}
fn required_string(record: &CanonicalRecord, field: &str, label: &str) -> MResult<String> {
    required_value(record, field, label)?
        .string()
        .ok_or_else(|| scene_error("SceneSchema", format!("field `{label}` must be a string")))
}
fn required_paint(record: &CanonicalRecord, field: &str, label: &str) -> MResult<String> {
    let value = required_value(record, field, label)?;
    if let Some(value) = value.string() {
        return Ok(value);
    }
    if let Some(value) = value.f64()
        && value.is_finite()
        && value.fract() == 0.0
        && (0.0..=16_777_215.0).contains(&value)
    {
        return Ok(format!("#{:06x}", value as u32));
    }
    Err(scene_error(
        "SceneSchema",
        format!(
            "field `{label}` must be a paint string or 24-bit numeric RGB color, got {:?}",
            resolved_for_diagnostic(value)
        ),
    ))
}
fn required_font_weight(record: &CanonicalRecord, field: &str, label: &str) -> MResult<String> {
    let value = required_value(record, field, label)?;
    if let Some(value) = value.string() {
        return Ok(value);
    }
    if let Some(value) = value.f64()
        && value.is_finite()
        && value.fract() == 0.0
        && (1.0..=1000.0).contains(&value)
    {
        return Ok(format!("{value:.0}"));
    }
    Err(scene_error(
        "SceneSchema",
        format!("field `{label}` must be a string or a numeric value from 1 through 1000"),
    ))
}
fn required_strings(record: &CanonicalRecord, field: &str, label: &str) -> MResult<Vec<String>> {
    strings_from_value(required_value(record, field, label)?, label)
}
fn strings_from_value(value: &CanonicalNode, label: &str) -> MResult<Vec<String>> {
    let tuple = value.as_tuple().ok_or_else(|| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be a tuple of strings"),
        )
    })?;
    tuple
        .iter()
        .map(|value| {
            value.string().ok_or_else(|| {
                scene_error(
                    "SceneSchema",
                    format!("field `{label}` must contain only strings"),
                )
            })
        })
        .collect()
}
fn required_number(record: &CanonicalRecord, field: &str, label: &str) -> MResult<f64> {
    let value = required_value(record, field, label)?;
    let value = value.f64().ok_or_else(|| {
        scene_error(
            "SceneSchema",
            format!("field `{label}` must be numeric, got {value:?}"),
        )
    })?;
    finite_number(value, label)
}
fn required_bool(record: &CanonicalRecord, field: &str, label: &str) -> MResult<bool> {
    required_value(record, field, label)?
        .bool()
        .ok_or_else(|| scene_error("SceneSchema", format!("field `{label}` must be a bool")))
}
fn reject_unknown_fields(record: &CanonicalRecord, allowed: &[&str], kind: &str) -> MResult<()> {
    for name in record.fields.keys() {
        if !allowed.contains(&name.as_str()) {
            return Err(scene_error(
                "SceneSchema",
                format!("{kind} has unknown field `{name}`"),
            ));
        }
    }
    Ok(())
}

fn resolved_for_diagnostic(value: &CanonicalNode) -> CanonicalNode {
    value.resolved().unwrap_or_else(|| value.clone())
}

#[derive(Clone, Debug)]
struct CanonicalNode {
    data: ValueData,
    schema: SchemaBody,
    shape: ShapeInstance,
}

#[derive(Clone, Debug)]
struct CanonicalRecord {
    fields: BTreeMap<String, CanonicalNode>,
}

impl CanonicalNode {
    fn from_value(value: &Value) -> MResult<Self> {
        let schemas = value.schemas().ok_or_else(|| {
            scene_error("SceneSchema", "canonical scene value has no schema context")
        })?;
        let schema = schemas
            .entry(value.schema())
            .ok_or_else(|| scene_error("SceneSchema", "canonical scene schema is absent"))?;
        Ok(Self {
            data: value.data().clone(),
            schema: schema.schema().body().clone(),
            shape: value.shape().clone(),
        })
    }

    fn resolved(&self) -> Option<Self> {
        match (&self.data, &self.schema) {
            (ValueData::Dynamic(value), SchemaBody::Dynamic) => {
                Self::from_value(value.value()?).ok()?.resolved()
            }
            (ValueData::Option(None), SchemaBody::Option(_)) => None,
            (ValueData::Option(Some(value)), SchemaBody::Option(schema)) => Some(Self {
                data: (**value).clone(),
                schema: (**schema).clone(),
                shape: self.shape.clone(),
            }),
            _ => Some(self.clone()),
        }
    }

    fn as_tuple(&self) -> Option<Vec<Self>> {
        let value = self.resolved()?;
        let (ValueData::Tuple(values), SchemaBody::Tuple(schemas)) = (&value.data, &value.schema)
        else {
            return None;
        };
        Some(
            values
                .iter()
                .zip(schemas.iter())
                .map(|(data, schema)| Self {
                    data: data.clone(),
                    schema: schema.clone(),
                    shape: value.shape.clone(),
                })
                .collect(),
        )
    }

    fn as_record(&self) -> Option<CanonicalRecord> {
        let value = self.resolved()?;
        let (ValueData::Record(record), SchemaBody::Record(fields)) = (&value.data, &value.schema)
        else {
            return None;
        };
        Some(CanonicalRecord {
            fields: fields
                .iter()
                .zip(record.fields())
                .map(|(field, data)| {
                    (
                        field.name.clone(),
                        Self {
                            data: data.clone(),
                            schema: field.schema.clone(),
                            shape: value.shape.clone(),
                        },
                    )
                })
                .collect(),
        })
    }

    fn is_table(&self) -> bool {
        matches!(self.data, ValueData::Table(_)) && matches!(self.schema, SchemaBody::Table { .. })
    }

    fn string(&self) -> Option<String> {
        let value = self.resolved()?;
        match value.data {
            ValueData::String(value) => Some(value.into()),
            _ => None,
        }
    }

    fn f64(&self) -> Option<f64> {
        let value = self.resolved()?;
        match value.data {
            ValueData::U8(value) => Some(f64::from(value)),
            ValueData::U16(value) => Some(f64::from(value)),
            ValueData::U32(value) => Some(f64::from(value)),
            ValueData::U64(value) => Some(value as f64),
            ValueData::U128(value) => Some(value as f64),
            ValueData::I8(value) => Some(f64::from(value)),
            ValueData::I16(value) => Some(f64::from(value)),
            ValueData::I32(value) => Some(f64::from(value)),
            ValueData::I64(value) => Some(value as f64),
            ValueData::I128(value) => Some(value as f64),
            ValueData::F32(value) => Some(f64::from(value.to_f32())),
            ValueData::F64(value) => Some(value.to_f64()),
            _ => None,
        }
    }

    fn bool(&self) -> Option<bool> {
        let value = self.resolved()?;
        match value.data {
            ValueData::Bool(value) => Some(value),
            _ => None,
        }
    }

    fn f64_matrix(&self) -> Option<(usize, usize, Vec<f64>)> {
        let value = self.resolved()?;
        let (ValueData::Matrix(matrix), SchemaBody::Matrix { dimensions, .. }) =
            (&value.data, &value.schema)
        else {
            return None;
        };
        let [rows, columns] = dimensions.as_ref() else {
            return None;
        };
        let rows = usize::try_from(value.shape.resolve_dimension(rows).ok()?).ok()?;
        let columns = usize::try_from(value.shape.resolve_dimension(columns).ok()?).ok()?;
        let values = match matrix.elements() {
            SequenceView::F64(values) => values.iter().map(|value| value.to_f64()).collect(),
            SequenceView::Values(values) => values
                .iter()
                .map(|value| match value {
                    ValueData::F64(value) => Some(value.to_f64()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        Some((rows, columns, values))
    }
}

fn sequence_len(values: SequenceView<'_>) -> usize {
    macro_rules! len {
        ($values:expr) => {
            $values.len()
        };
    }
    match values {
        SequenceView::U8(values) => len!(values),
        SequenceView::U16(values) => len!(values),
        SequenceView::U32(values) => len!(values),
        SequenceView::U64(values) => len!(values),
        SequenceView::U128(values) => len!(values),
        SequenceView::I8(values) => len!(values),
        SequenceView::I16(values) => len!(values),
        SequenceView::I32(values) => len!(values),
        SequenceView::I64(values) => len!(values),
        SequenceView::I128(values) => len!(values),
        SequenceView::F32(values) => len!(values),
        SequenceView::F64(values) => len!(values),
        SequenceView::Complex32(values) => len!(values),
        SequenceView::Complex64(values) => len!(values),
        SequenceView::Rational64(values) => len!(values),
        SequenceView::Bool(values) => len!(values),
        SequenceView::String(values) => len!(values),
        SequenceView::Id(values) => len!(values),
        SequenceView::Index(values) => len!(values),
        SequenceView::Unit(count) => usize::try_from(count).unwrap_or(usize::MAX),
        SequenceView::Values(values) => len!(values),
    }
}

fn sequence_value(values: SequenceView<'_>, index: usize) -> Option<ValueData> {
    macro_rules! value {
        ($values:expr, $variant:ident) => {
            $values.get(index).cloned().map(ValueData::$variant)
        };
    }
    match values {
        SequenceView::U8(values) => value!(values, U8),
        SequenceView::U16(values) => value!(values, U16),
        SequenceView::U32(values) => value!(values, U32),
        SequenceView::U64(values) => value!(values, U64),
        SequenceView::U128(values) => value!(values, U128),
        SequenceView::I8(values) => value!(values, I8),
        SequenceView::I16(values) => value!(values, I16),
        SequenceView::I32(values) => value!(values, I32),
        SequenceView::I64(values) => value!(values, I64),
        SequenceView::I128(values) => value!(values, I128),
        SequenceView::F32(values) => value!(values, F32),
        SequenceView::F64(values) => value!(values, F64),
        SequenceView::Complex32(values) => value!(values, Complex32),
        SequenceView::Complex64(values) => value!(values, Complex64),
        SequenceView::Rational64(values) => value!(values, Rational64),
        SequenceView::Bool(values) => value!(values, Bool),
        SequenceView::String(values) => value!(values, String),
        SequenceView::Id(values) => value!(values, Id),
        SequenceView::Index(values) => value!(values, Index),
        SequenceView::Unit(count) => {
            (index < usize::try_from(count).ok()?).then_some(ValueData::Atom)
        }
        SequenceView::Values(values) => values.get(index).cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        CardinalitySpec, DimensionExpr, IntegerWidth, SchemaField, ValueCell, ValueDataDraft,
    };

    #[test]
    fn dynamic_table_values_resolve_to_their_concrete_scene_types() {
        let table = ValueCell::table_from_cell_columns(
            vec![(
                SchemaField {
                    name: "paint".into(),
                    schema: SchemaBody::Dynamic,
                },
                vec![
                    ValueCell::from_exact("none".to_owned()).unwrap(),
                    ValueCell::from_schema_data(
                        SchemaBody::SignedInteger(IntegerWidth::W64),
                        ValueDataDraft::I64(16_711_773),
                    )
                    .unwrap(),
                ]
                .into_boxed_slice(),
            )]
            .into_boxed_slice(),
            CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        )
        .unwrap();
        let value = CanonicalNode::from_value(&table.snapshot().unwrap()).unwrap();
        let (ValueData::Table(table), SchemaBody::Table { columns, .. }) =
            (&value.data, &value.schema)
        else {
            panic!("table value")
        };
        let values = table.column(0).unwrap();
        let first = CanonicalNode {
            data: sequence_value(values.clone(), 0).unwrap(),
            schema: columns[0].schema.clone(),
            shape: value.shape.clone(),
        };
        let second = CanonicalNode {
            data: sequence_value(values, 1).unwrap(),
            schema: columns[0].schema.clone(),
            shape: value.shape,
        };

        assert_eq!(first.string().as_deref(), Some("none"));
        assert_eq!(second.f64(), Some(16_711_773.0));
    }
}
