use super::{SequenceView, SnapshotValueError, Value, ValueData};
use crate::{SchemaBody, SchemaTable};

impl Value {
    /// Largest canonical `Index` payload in this finalized value, including
    /// nested aggregates and self-describing Dynamic values. Payloads remain
    /// `u64`; inspection does not convert through the current process's usize.
    pub fn maximum_index_constant(
        &self,
        schemas: &SchemaTable,
    ) -> Result<Option<u64>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let mut maximum = None;
        collect_maximum_index(schema.body(), self.data(), &mut maximum)?;
        Ok(maximum)
    }
}

fn record_maximum(maximum: &mut Option<u64>, value: u64) {
    *maximum = Some(maximum.map_or(value, |current| current.max(value)));
}

fn collect_sequence_maximum(
    schema: &SchemaBody,
    values: SequenceView<'_>,
    maximum: &mut Option<u64>,
) -> Result<(), SnapshotValueError> {
    match values {
        SequenceView::Index(values) => {
            if let Some(value) = values.iter().copied().max() {
                record_maximum(maximum, value);
            }
        }
        SequenceView::Values(values) => {
            for value in values {
                collect_maximum_index(schema, value, maximum)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_maximum_index(
    schema: &SchemaBody,
    data: &ValueData,
    maximum: &mut Option<u64>,
) -> Result<(), SnapshotValueError> {
    // The schema and data pair is already certified by Value finalization.
    // Use that schema to interpret packed sequences and enum payloads rather
    // than reconstructing legacy runtime values or runtime type tables.
    match schema {
        SchemaBody::Index => {
            if let ValueData::Index(value) = data {
                record_maximum(maximum, *value);
            }
        }
        SchemaBody::Dynamic => {
            if let ValueData::Dynamic(value) = data {
                if let Some(value) = value.value() {
                    let schemas =
                        value
                            .schemas()
                            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                                schema: value.schema(),
                            })?;
                    if let Some(value) = value.maximum_index_constant(&schemas)? {
                        record_maximum(maximum, value);
                    }
                }
            }
        }
        SchemaBody::Enum { variants, .. } => {
            if let ValueData::Enum(value) = data {
                if let (Some(schema), Some(payload)) = (
                    variants[value.ordinal() as usize].payload.as_ref(),
                    value.payload(),
                ) {
                    collect_maximum_index(schema, payload, maximum)?;
                }
            }
        }
        SchemaBody::Option(schema) => {
            if let ValueData::Option(Some(value)) = data {
                collect_maximum_index(schema, value, maximum)?;
            }
        }
        SchemaBody::Tuple(schemas) => {
            if let ValueData::Tuple(values) = data {
                for (schema, value) in schemas.iter().zip(values) {
                    collect_maximum_index(schema, value, maximum)?;
                }
            }
        }
        SchemaBody::Record(fields) => {
            if let ValueData::Record(value) = data {
                for (field, value) in fields.iter().zip(value.fields()) {
                    collect_maximum_index(&field.schema, value, maximum)?;
                }
            }
        }
        SchemaBody::Matrix { element, .. } => {
            if let ValueData::Matrix(value) = data {
                collect_sequence_maximum(element, value.elements(), maximum)?;
            }
        }
        SchemaBody::Table { columns, .. } => {
            if let ValueData::Table(value) = data {
                for (index, column) in columns.iter().enumerate() {
                    if let Some(values) = value.column(index) {
                        collect_sequence_maximum(&column.schema, values, maximum)?;
                    }
                }
            }
        }
        SchemaBody::Set { element, .. } => {
            if let ValueData::Set(value) = data {
                for value in value.elements() {
                    collect_maximum_index(element, value.data(), maximum)?;
                }
            }
        }
        SchemaBody::Map { key, value, .. } => {
            if let ValueData::Map(entries) = data {
                for entry in entries.entries() {
                    collect_maximum_index(key, entry.key().data(), maximum)?;
                    collect_maximum_index(value, entry.value(), maximum)?;
                }
            }
        }
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => {}
    }
    Ok(())
}
