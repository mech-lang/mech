//! Schema-directed HTML for detached canonical values. Source text only enters
//! escaped text nodes; aggregate structure comes from validated value schemas.

use mech_core::snapshot::SequenceView;
use mech_core::{SchemaBody, ShapeInstance, Value, ValueData};

use super::{escape_html, format_data};

pub(super) fn format_value(value: &Value) -> String {
    let schemas = value.schemas();
    let schema = schemas
        .as_ref()
        .and_then(|schemas| schemas.get(value.schema()));
    render(
        value.data(),
        schema.map(|schema| schema.body()),
        value.shape(),
    )
}

fn scalar(data: &ValueData) -> String {
    let mut remaining = usize::MAX;
    format!(
        "<span class='mech-value'>{}</span>",
        escape_html(&format_data(data, &[], &mut remaining))
    )
}

fn render(data: &ValueData, schema: Option<&SchemaBody>, shape: &ShapeInstance) -> String {
    match (data, schema) {
        (ValueData::Dynamic(value), _) => value
            .value()
            .map(format_value)
            .unwrap_or_else(|| scalar(data)),
        (ValueData::Record(value), Some(SchemaBody::Record(fields))) => {
            let mut html = String::from("<table class='mech-record'><tbody>");
            for (field, value) in fields.iter().zip(value.fields()) {
                html.push_str("<tr><th scope='row'>");
                html.push_str(&escape_html(&field.name));
                html.push_str("</th><td>");
                html.push_str(&render(value, Some(&field.schema), shape));
                html.push_str("</td></tr>");
            }
            html.push_str("</tbody></table>");
            html
        }
        (
            ValueData::Matrix(value),
            Some(SchemaBody::Matrix {
                element,
                dimensions,
            }),
        ) => {
            let extents = dimensions
                .iter()
                .map(|dimension| shape.resolve_dimension(dimension))
                .collect::<Result<Vec<_>, _>>();
            let Ok(extents) = extents else {
                return scalar(data);
            };
            let columns = extents.get(1).copied().unwrap_or(1) as usize;
            let values = value.elements();
            let mut html = String::from("<table class='mech-matrix'><tbody>");
            if columns > 0 {
                for index in 0..values.len() {
                    if index % columns == 0 {
                        html.push_str("<tr>");
                    }
                    html.push_str("<td>");
                    html.push_str(&render_element(values, index, element, shape));
                    html.push_str("</td>");
                    if index % columns + 1 == columns || index + 1 == values.len() {
                        html.push_str("</tr>");
                    }
                }
            }
            html.push_str("</tbody></table>");
            html
        }
        (ValueData::Table(value), Some(SchemaBody::Table { columns, .. })) => {
            let mut html = String::from("<table class='mech-table'><thead><tr>");
            for column in columns {
                html.push_str("<th scope='col'>");
                html.push_str(&escape_html(&column.name));
                html.push_str("</th>");
            }
            html.push_str("</tr></thead><tbody>");
            let rows = value.column(0).map_or(0, SequenceView::len);
            for row in 0..rows {
                html.push_str("<tr>");
                for (index, column) in columns.iter().enumerate() {
                    html.push_str("<td>");
                    if let Some(values) = value.column(index) {
                        html.push_str(&render_element(values, row, &column.schema, shape));
                    }
                    html.push_str("</td>");
                }
                html.push_str("</tr>");
            }
            html.push_str("</tbody></table>");
            html
        }
        (ValueData::Option(Some(value)), Some(SchemaBody::Option(inner))) => {
            format!(
                "<div class='mech-option'><span>some(</span>{}<span>)</span></div>",
                render(value, Some(inner), shape)
            )
        }
        (ValueData::Tuple(values), Some(SchemaBody::Tuple(elements))) => {
            let items = values
                .iter()
                .zip(elements)
                .map(|(value, element)| render(value, Some(element), shape))
                .collect::<Vec<_>>();
            format!(
                "<div class='mech-tuple'><span>(</span>{}<span>)</span></div>",
                items.join("<span>, </span>")
            )
        }
        _ => scalar(data),
    }
}

// Borrow aggregate elements and copy only a single packed scalar at a time.
// This avoids expanding a complete matrix/table column to owned ValueData.
fn render_element(
    values: SequenceView<'_>,
    index: usize,
    schema: &SchemaBody,
    shape: &ShapeInstance,
) -> String {
    macro_rules! element {
        ($values:expr, $variant:ident) => {
            $values
                .get(index)
                .map(|value| render(&ValueData::$variant(value.clone()), Some(schema), shape))
                .unwrap_or_default()
        };
    }
    match values {
        SequenceView::U8(values) => element!(values, U8),
        SequenceView::U16(values) => element!(values, U16),
        SequenceView::U32(values) => element!(values, U32),
        SequenceView::U64(values) => element!(values, U64),
        SequenceView::U128(values) => element!(values, U128),
        SequenceView::I8(values) => element!(values, I8),
        SequenceView::I16(values) => element!(values, I16),
        SequenceView::I32(values) => element!(values, I32),
        SequenceView::I64(values) => element!(values, I64),
        SequenceView::I128(values) => element!(values, I128),
        SequenceView::F32(values) => element!(values, F32),
        SequenceView::F64(values) => element!(values, F64),
        SequenceView::Complex32(values) => element!(values, Complex32),
        SequenceView::Complex64(values) => element!(values, Complex64),
        SequenceView::Rational64(values) => element!(values, Rational64),
        SequenceView::Bool(values) => element!(values, Bool),
        SequenceView::String(values) => element!(values, String),
        SequenceView::Id(values) => element!(values, Id),
        SequenceView::Index(values) => element!(values, Index),
        SequenceView::Unit(_) => render(&ValueData::Atom, Some(schema), shape),
        SequenceView::Values(values) => values
            .get(index)
            .map(|value| render(value, Some(schema), shape))
            .unwrap_or_default(),
    }
}
