use super::sequence::SequenceStorage;
use super::{
    CanonicalKeyValue, DynamicValue, EnumDraft, EnumValue, MapEntryDraft, MapValue, MatrixValue,
    NamedValueDraft, OptionDraft, RecordValue, ReifiedKind, ReifiedType, ReifiedTypeDraft,
    SchemaDataKind, SetValue, SnapshotPath, SnapshotPathSegment, SnapshotValueError,
    TableColumnDraft, TableValue, ValueData, ValueDataDraft, ValueDraft,
};
use crate::{
    FloatWidth, IntegerWidth, NamedKindPathResolver, Schema, SchemaBody, SchemaId, SchemaKey,
    SchemaTable, ShapeInstance,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String, sync::Arc, vec::Vec};

pub struct SnapshotValidationContext<'a> {
    schemas: &'a SchemaTable,
    named_kinds: Option<&'a dyn NamedKindPathResolver>,
}

impl<'a> SnapshotValidationContext<'a> {
    pub const fn new(schemas: &'a SchemaTable) -> Self {
        Self {
            schemas,
            named_kinds: None,
        }
    }

    pub const fn with_named_kinds(
        schemas: &'a SchemaTable,
        named_kinds: &'a dyn NamedKindPathResolver,
    ) -> Self {
        Self {
            schemas,
            named_kinds: Some(named_kinds),
        }
    }

    pub const fn schemas(&self) -> &'a SchemaTable {
        self.schemas
    }

    pub const fn named_kinds(&self) -> Option<&'a dyn NamedKindPathResolver> {
        self.named_kinds
    }
}

#[derive(Clone)]
pub struct Value {
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    data: ValueData,
    resident_token: u64,
    schemas: Option<Arc<SchemaTable>>,
}

impl core::fmt::Debug for Value {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Value")
            .field("schema", &self.schema)
            .field("schema_key", &self.schema_key)
            .field("shape", &self.shape)
            .field("data", &self.data)
            .finish()
    }
}

impl Value {
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }

    pub const fn schema_key(&self) -> SchemaKey {
        self.schema_key
    }

    pub const fn shape(&self) -> &ShapeInstance {
        &self.shape
    }

    pub const fn data(&self) -> &ValueData {
        &self.data
    }

    /// Returns the immutable schema table that validates this detached value.
    pub fn schemas(&self) -> Option<Arc<SchemaTable>> {
        self.schemas.clone()
    }

    /// Revalidates this immutable payload against an equivalent schema in a
    /// different schema table and returns a value bound to that table.
    pub fn rebind(
        &self,
        schema: SchemaId,
        shape: &ShapeInstance,
        schemas: &SchemaTable,
    ) -> Result<Self, SnapshotValueError> {
        let source_schemas =
            self.schemas
                .as_deref()
                .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                    schema: self.schema,
                })?;
        let source_schema = self.validate_against(source_schemas)?;
        let target_entry = schemas
            .entry(schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
        let target_schema = target_entry.schema();
        let exact_definition = self.schema_key == target_entry.key()
            && source_schema.canonical_bytes() == target_schema.canonical_bytes();
        let equivalent_at_shape =
            crate::cell_binding::close_schema_body(source_schema.body(), &self.shape)
                .and_then(|source| {
                    crate::cell_binding::close_schema_body(target_schema.body(), shape)
                        .map(|target| source == target)
                })
                .unwrap_or(false);
        let target_accepts_source_extent = dynamic_extent_rebind_compatible(
            source_schema.body(),
            &self.shape,
            target_schema.body(),
            shape,
        );
        if !exact_definition && !equivalent_at_shape && !target_accepts_source_extent {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: self.schema_key,
            });
        }
        let data = canonical_data_to_rebound_draft(
            source_schema.body(),
            &self.data,
            &SnapshotPath::root(),
            schemas,
        )?;
        let data = adapt_dynamic_bytecode_placeholders(
            source_schema.body(),
            target_schema.body(),
            data,
            &SnapshotPath::root(),
        )?;
        ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(schemas))
    }

    /// Returns schema-directed draft data suitable for embedding this value in
    /// a newly derived aggregate schema. Nominal identity and option/enum
    /// structure remain governed by the value's originating schema.
    pub fn canonical_data_draft(&self) -> Result<ValueDataDraft, SnapshotValueError> {
        let schemas = self
            .schemas
            .as_deref()
            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                schema: self.schema,
            })?;
        let schema = schemas
            .get(self.schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                schema: self.schema,
            })?;
        canonical_data_to_draft(schema.body(), &self.data, &SnapshotPath::root())
    }

    /// Compact deterministic token computed when the finalized value is
    /// constructed. Resident receipts use it without consulting schemas or
    /// re-encoding immutable payloads during a turn.
    #[doc(hidden)]
    pub const fn resident_token(&self) -> u64 {
        self.resident_token
    }

    pub fn validate_against<'a>(
        &self,
        schemas: &'a SchemaTable,
    ) -> Result<&'a Schema, SnapshotValueError> {
        let entry = schemas.entry(self.schema);
        if entry.map(|entry| entry.key()) != Some(self.schema_key) {
            return Err(SnapshotValueError::SnapshotSchemaTableMismatch {
                schema: self.schema,
                expected: self.schema_key,
                actual: entry.map(|entry| entry.key()),
            });
        }
        Ok(entry.expect("matching entry exists").schema())
    }

    fn rebuild(
        &self,
        data: ValueDataDraft,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        self.validate_against(context.schemas())?;
        ValueDraft {
            schema: self.schema,
            shape_values: self.shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(context)
    }

    pub fn rebuild_enum(
        &self,
        ordinal: u32,
        payload: Option<ValueData>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Enum { variants, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Enum,
            ));
        };
        let variant = variants.get(ordinal as usize).ok_or_else(|| {
            SnapshotValueError::EnumOrdinalOutOfRangeV1 {
                path: SnapshotPath::root(),
                ordinal,
                variants: variants.len() as u32,
            }
        })?;
        let payload = match (variant.payload.as_ref(), payload) {
            (Some(schema), Some(payload)) => Some(Box::new(canonical_data_to_draft(
                schema,
                &payload,
                &SnapshotPath::root().child(SnapshotPathSegment::EnumPayload(ordinal)),
            )?)),
            (None, None) => None,
            _ => {
                return Err(SnapshotValueError::EnumPayloadMismatchV1 {
                    path: SnapshotPath::root(),
                });
            }
        };
        self.rebuild(
            ValueDataDraft::Enum(EnumDraft { ordinal, payload }),
            context,
        )
    }

    pub fn set_element_drafts(
        &self,
        schemas: &SchemaTable,
    ) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        };
        let ValueData::Set(set) = self.data() else {
            unreachable!("validated set schema has set data")
        };
        set.elements()
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value.data(),
                    &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice)
    }

    /// Converts canonical set element data back into drafts using this set's
    /// declared element schema without imposing this set's container
    /// cardinality on a derived result.
    pub fn set_element_data_drafts(
        &self,
        schemas: &SchemaTable,
        elements: &[ValueData],
    ) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        };
        elements
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value,
                    &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice)
    }

    pub fn rebuild_option(
        &self,
        payload: Option<ValueData>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Option(element) = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Option,
            ));
        };
        let payload = payload
            .as_ref()
            .map(|payload| {
                canonical_data_to_draft(
                    element,
                    payload,
                    &SnapshotPath::root().child(SnapshotPathSegment::OptionValue),
                )
                .map(Box::new)
            })
            .transpose()?;
        self.rebuild(
            ValueDataDraft::Option(OptionDraft {
                present: payload.is_some(),
                value: payload,
            }),
            context,
        )
    }

    pub fn rebuild_tuple(
        &self,
        children: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Tuple(elements) = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Tuple,
            ));
        };
        ensure_arity(&SnapshotPath::root(), elements.len(), children.len())?;
        let children = elements
            .iter()
            .zip(children.iter())
            .enumerate()
            .map(|(index, (schema, child))| {
                canonical_data_to_draft(
                    schema,
                    child,
                    &SnapshotPath::root().child(SnapshotPathSegment::TupleElement(index as u32)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.rebuild(ValueDataDraft::Tuple(children.into_boxed_slice()), context)
    }

    pub fn rebuild_record(
        &self,
        children: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Record(fields) = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Record,
            ));
        };
        ensure_arity(&SnapshotPath::root(), fields.len(), children.len())?;
        let children = fields
            .iter()
            .zip(children.iter())
            .enumerate()
            .map(|(index, (field, child))| {
                Ok(NamedValueDraft {
                    name: field.name.clone(),
                    value: canonical_data_to_draft(
                        &field.schema,
                        child,
                        &SnapshotPath::root().child(SnapshotPathSegment::RecordField(index as u32)),
                    )?,
                })
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        self.rebuild(ValueDataDraft::Record(children.into_boxed_slice()), context)
    }

    pub fn rebuild_matrix(
        &self,
        elements: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Matrix { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Matrix,
            ));
        };
        let elements = elements
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value,
                    &SnapshotPath::root().child(SnapshotPathSegment::MatrixElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.rebuild(ValueDataDraft::Matrix(elements.into_boxed_slice()), context)
    }

    pub fn rebuild_table(
        &self,
        columns: Box<[Box<[ValueData]>]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Table {
            columns: expected, ..
        } = schema.body()
        else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Table,
            ));
        };
        ensure_arity(&SnapshotPath::root(), expected.len(), columns.len())?;
        let columns = expected
            .iter()
            .zip(columns.iter())
            .enumerate()
            .map(|(column_index, (field, values))| {
                let values = values
                    .iter()
                    .enumerate()
                    .map(|(row_index, value)| {
                        canonical_data_to_draft(
                            &field.schema,
                            value,
                            &SnapshotPath::root()
                                .child(SnapshotPathSegment::TableColumn(column_index as u32))
                                .child(SnapshotPathSegment::TableRow(row_index as u64)),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(TableColumnDraft {
                    name: field.name.clone(),
                    values: values.into_boxed_slice(),
                })
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        self.rebuild(ValueDataDraft::Table(columns.into_boxed_slice()), context)
    }

    pub fn rebuild_set(
        &self,
        elements: Box<[ValueData]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Set { element, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        };
        let elements = elements
            .iter()
            .enumerate()
            .map(|(index, value)| {
                canonical_data_to_draft(
                    element,
                    value,
                    &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.rebuild(ValueDataDraft::Set(elements.into_boxed_slice()), context)
    }

    pub fn rebuild_set_drafts(
        &self,
        elements: Box<[ValueDataDraft]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        if !matches!(schema.body(), SchemaBody::Set { .. }) {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Set,
            ));
        }
        self.rebuild(ValueDataDraft::Set(elements), context)
    }

    pub fn rebuild_map(
        &self,
        entries: Box<[(ValueData, ValueData)]>,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let schema = self.validate_against(context.schemas())?;
        let SchemaBody::Map { key, value, .. } = schema.body() else {
            return Err(rebuild_kind_mismatch(
                schema.body(),
                super::ValueDataKind::Map,
            ));
        };
        let entries = entries
            .iter()
            .enumerate()
            .map(|(index, (entry_key, entry_value))| {
                Ok(MapEntryDraft {
                    items: vec![
                        canonical_data_to_draft(
                            key,
                            entry_key,
                            &SnapshotPath::root().child(SnapshotPathSegment::MapKey(index as u64)),
                        )?,
                        canonical_data_to_draft(
                            value,
                            entry_value,
                            &SnapshotPath::root()
                                .child(SnapshotPathSegment::MapValue(index as u64)),
                        )?,
                    ]
                    .into_boxed_slice(),
                })
            })
            .collect::<Result<Vec<_>, SnapshotValueError>>()?;
        self.rebuild(ValueDataDraft::Map(entries.into_boxed_slice()), context)
    }
}

fn dynamic_extent_rebind_compatible(
    source: &SchemaBody,
    source_shape: &ShapeInstance,
    target: &SchemaBody,
    target_shape: &ShapeInstance,
) -> bool {
    let Ok(source) = crate::cell_binding::close_schema_body(source, source_shape) else {
        return false;
    };
    let Ok(target) = crate::cell_binding::close_schema_body(target, target_shape) else {
        return false;
    };
    closed_schema_rebind_compatible(&source, &target)
}

fn closed_schema_rebind_compatible(source: &SchemaBody, target: &SchemaBody) -> bool {
    if source == target {
        return true;
    }
    let dynamic_target =
        |target: &crate::CardinalitySpec| matches!(target, crate::CardinalitySpec::Dynamic { .. });
    match (source, target) {
        (SchemaBody::ReifiedType, SchemaBody::Dynamic) => true,
        (SchemaBody::Option(source), SchemaBody::Option(target)) => {
            closed_schema_rebind_compatible(source, target)
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target)) => {
            source.len() == target.len()
                && source
                    .iter()
                    .zip(target)
                    .all(|(source, target)| closed_schema_rebind_compatible(source, target))
        }
        (SchemaBody::Record(source), SchemaBody::Record(target)) => {
            fields_rebind_compatible(source, target)
        }
        (
            SchemaBody::Matrix {
                element: source_element,
                dimensions: source_dimensions,
            },
            SchemaBody::Matrix {
                element: target_element,
                dimensions: target_dimensions,
            },
        ) => {
            source_dimensions == target_dimensions
                && closed_schema_rebind_compatible(source_element, target_element)
        }
        (
            SchemaBody::Table {
                columns: source_columns,
                rows: source_rows,
            },
            SchemaBody::Table {
                columns: target_columns,
                rows: target_rows,
            },
        ) => {
            (source_rows == target_rows || dynamic_target(target_rows))
                && fields_rebind_compatible(source_columns, target_columns)
        }
        (
            SchemaBody::Set {
                element: source_element,
                cardinality: source_cardinality,
            },
            SchemaBody::Set {
                element: target_element,
                cardinality: target_cardinality,
            },
        ) => {
            (source_cardinality == target_cardinality || dynamic_target(target_cardinality))
                && closed_schema_rebind_compatible(source_element, target_element)
        }
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                cardinality: source_cardinality,
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                cardinality: target_cardinality,
            },
        ) => {
            (source_cardinality == target_cardinality || dynamic_target(target_cardinality))
                && closed_schema_rebind_compatible(source_key, target_key)
                && closed_schema_rebind_compatible(source_value, target_value)
        }
        _ => false,
    }
}

fn adapt_dynamic_bytecode_placeholders(
    source: &SchemaBody,
    target: &SchemaBody,
    draft: ValueDataDraft,
    path: &SnapshotPath,
) -> Result<ValueDataDraft, SnapshotValueError> {
    if source == target {
        return Ok(draft);
    }
    let actual = draft.kind();
    match (source, target, draft) {
        (SchemaBody::ReifiedType, SchemaBody::Dynamic, ValueDataDraft::Type(_)) => {
            Ok(ValueDataDraft::Dynamic(None))
        }
        (SchemaBody::Option(source), SchemaBody::Option(target), ValueDataDraft::Option(draft)) => {
            Ok(ValueDataDraft::Option(OptionDraft {
                present: draft.present,
                value: draft
                    .value
                    .map(|value| {
                        adapt_dynamic_bytecode_placeholders(
                            source,
                            target,
                            *value,
                            &path.child(SnapshotPathSegment::OptionValue),
                        )
                        .map(Box::new)
                    })
                    .transpose()?,
            }))
        }
        (SchemaBody::Tuple(source), SchemaBody::Tuple(target), ValueDataDraft::Tuple(values))
            if source.len() == target.len() && source.len() == values.len() =>
        {
            Ok(ValueDataDraft::Tuple(
                source
                    .iter()
                    .zip(target)
                    .zip(values.into_vec())
                    .enumerate()
                    .map(|(index, ((source, target), value))| {
                        adapt_dynamic_bytecode_placeholders(
                            source,
                            target,
                            value,
                            &path.child(SnapshotPathSegment::TupleElement(index as u32)),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            ))
        }
        (
            SchemaBody::Record(source),
            SchemaBody::Record(target),
            ValueDataDraft::Record(values),
        ) if source.len() == target.len() && source.len() == values.len() => {
            Ok(ValueDataDraft::Record(
                source
                    .iter()
                    .zip(target)
                    .zip(values.into_vec())
                    .enumerate()
                    .map(|(index, ((source, target), value))| {
                        Ok(NamedValueDraft {
                            name: value.name,
                            value: adapt_dynamic_bytecode_placeholders(
                                &source.schema,
                                &target.schema,
                                value.value,
                                &path.child(SnapshotPathSegment::RecordField(index as u32)),
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, SnapshotValueError>>()?
                    .into_boxed_slice(),
            ))
        }
        (
            SchemaBody::Table {
                columns: source, ..
            },
            SchemaBody::Table {
                columns: target, ..
            },
            ValueDataDraft::Table(columns),
        ) if source.len() == target.len() && source.len() == columns.len() => {
            Ok(ValueDataDraft::Table(
                source
                    .iter()
                    .zip(target)
                    .zip(columns.into_vec())
                    .enumerate()
                    .map(|(column_index, ((source, target), column))| {
                        Ok(TableColumnDraft {
                            name: column.name,
                            values: column
                                .values
                                .into_vec()
                                .into_iter()
                                .enumerate()
                                .map(|(row_index, value)| {
                                    adapt_dynamic_bytecode_placeholders(
                                        &source.schema,
                                        &target.schema,
                                        value,
                                        &path
                                            .child(SnapshotPathSegment::TableColumn(
                                                column_index as u32,
                                            ))
                                            .child(SnapshotPathSegment::TableRow(row_index as u64)),
                                    )
                                })
                                .collect::<Result<Vec<_>, _>>()?
                                .into_boxed_slice(),
                        })
                    })
                    .collect::<Result<Vec<_>, SnapshotValueError>>()?
                    .into_boxed_slice(),
            ))
        }
        (
            SchemaBody::Matrix {
                element: source, ..
            },
            SchemaBody::Matrix {
                element: target, ..
            },
            ValueDataDraft::Matrix(values),
        ) => Ok(ValueDataDraft::Matrix(
            values
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    adapt_dynamic_bytecode_placeholders(
                        source,
                        target,
                        value,
                        &path.child(SnapshotPathSegment::MatrixElement(index as u64)),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        )),
        (
            SchemaBody::Set {
                element: source, ..
            },
            SchemaBody::Set {
                element: target, ..
            },
            ValueDataDraft::Set(values),
        ) => Ok(ValueDataDraft::Set(
            values
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    adapt_dynamic_bytecode_placeholders(
                        source,
                        target,
                        value,
                        &path.child(SnapshotPathSegment::SetElement(index as u64)),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        )),
        (
            SchemaBody::Map {
                key: source_key,
                value: source_value,
                ..
            },
            SchemaBody::Map {
                key: target_key,
                value: target_value,
                ..
            },
            ValueDataDraft::Map(entries),
        ) => Ok(ValueDataDraft::Map(
            entries
                .into_vec()
                .into_iter()
                .enumerate()
                .map(|(index, entry)| {
                    ensure_arity(path, 2, entry.items.len())?;
                    let mut items = entry.items.into_vec().into_iter();
                    let key = items.next().expect("validated map entry key exists");
                    let value = items.next().expect("validated map entry value exists");
                    Ok(MapEntryDraft {
                        items: vec![
                            adapt_dynamic_bytecode_placeholders(
                                source_key,
                                target_key,
                                key,
                                &path.child(SnapshotPathSegment::MapKey(index as u64)),
                            )?,
                            adapt_dynamic_bytecode_placeholders(
                                source_value,
                                target_value,
                                value,
                                &path.child(SnapshotPathSegment::MapValue(index as u64)),
                            )?,
                        ]
                        .into_boxed_slice(),
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?
                .into_boxed_slice(),
        )),
        _ => Err(data_mismatch_kind(target, actual, path)),
    }
}

fn fields_rebind_compatible(source: &[crate::SchemaField], target: &[crate::SchemaField]) -> bool {
    source.len() == target.len()
        && source.iter().zip(target).all(|(source, target)| {
            source.name == target.name
                && closed_schema_rebind_compatible(&source.schema, &target.schema)
        })
}

fn rebuild_kind_mismatch(schema: &SchemaBody, actual: super::ValueDataKind) -> SnapshotValueError {
    SnapshotValueError::SnapshotDataSchemaMismatch {
        path: SnapshotPath::root(),
        expected: schema_kind(schema),
        actual,
    }
}

fn canonical_data_to_draft(
    schema: &SchemaBody,
    data: &ValueData,
    path: &SnapshotPath,
) -> Result<ValueDataDraft, SnapshotValueError> {
    canonical_data_to_draft_with_target(schema, data, path, None)
}

/// Materializes a canonical draft for schema-directed data that has already
/// been validated as part of a finalized snapshot value.
pub fn canonical_snapshot_data_draft(
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<ValueDataDraft, SnapshotValueError> {
    canonical_data_to_draft(schema, data, &SnapshotPath::root())
}

fn canonical_data_to_rebound_draft(
    schema: &SchemaBody,
    data: &ValueData,
    path: &SnapshotPath,
    target_schemas: &SchemaTable,
) -> Result<ValueDataDraft, SnapshotValueError> {
    canonical_data_to_draft_with_target(schema, data, path, Some(target_schemas))
}

fn canonical_data_to_draft_with_target(
    schema: &SchemaBody,
    data: &ValueData,
    path: &SnapshotPath,
    target_schemas: Option<&SchemaTable>,
) -> Result<ValueDataDraft, SnapshotValueError> {
    let draft = match (schema, data) {
        (SchemaBody::Dynamic, ValueData::Dynamic(value)) => {
            let value = value
                .value()
                .map(|value| -> Result<Box<ValueDraft>, SnapshotValueError> {
                    let rebound;
                    let value = if let Some(target_schemas) = target_schemas {
                        let schema = target_schemas.find_by_key(value.schema_key()).ok_or(
                            SnapshotValueError::SnapshotSchemaTableMismatch {
                                schema: value.schema(),
                                expected: value.schema_key(),
                                actual: target_schemas
                                    .entry(value.schema())
                                    .map(|entry| entry.key()),
                            },
                        )?;
                        rebound = value.rebind(schema, value.shape(), target_schemas)?;
                        &rebound
                    } else {
                        value
                    };
                    Ok(Box::new(ValueDraft {
                        schema: value.schema(),
                        shape_values: value.shape().parameter_values().to_vec().into_boxed_slice(),
                        data: value.canonical_data_draft()?,
                    }))
                })
                .transpose()?;
            ValueDataDraft::Dynamic(value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueData::U8(value)) => {
            ValueDataDraft::U8(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueData::U16(value)) => {
            ValueDataDraft::U16(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueData::U32(value)) => {
            ValueDataDraft::U32(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueData::U64(value)) => {
            ValueDataDraft::U64(*value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueData::U128(value)) => {
            ValueDataDraft::U128(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W8), ValueData::I8(value)) => {
            ValueDataDraft::I8(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W16), ValueData::I16(value)) => {
            ValueDataDraft::I16(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W32), ValueData::I32(value)) => {
            ValueDataDraft::I32(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W64), ValueData::I64(value)) => {
            ValueDataDraft::I64(*value)
        }
        (SchemaBody::SignedInteger(IntegerWidth::W128), ValueData::I128(value)) => {
            ValueDataDraft::I128(*value)
        }
        (SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(value)) => {
            ValueDataDraft::F32(*value)
        }
        (SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(value)) => {
            ValueDataDraft::F64(*value)
        }
        (SchemaBody::Complex(FloatWidth::W32), ValueData::Complex32(value)) => {
            ValueDataDraft::Complex32(*value)
        }
        (SchemaBody::Complex(FloatWidth::W64), ValueData::Complex64(value)) => {
            ValueDataDraft::Complex64(*value)
        }
        (SchemaBody::Rational64, ValueData::Rational64(value)) => ValueDataDraft::Rational64 {
            numerator: value.numerator(),
            denominator: value.denominator(),
        },
        (SchemaBody::Bool, ValueData::Bool(value)) => ValueDataDraft::Bool(*value),
        (SchemaBody::String, ValueData::String(value)) => {
            ValueDataDraft::String(String::from(value.as_ref()))
        }
        (SchemaBody::Id, ValueData::Id(value)) => ValueDataDraft::Id(*value),
        (SchemaBody::Index, ValueData::Index(value)) => ValueDataDraft::Index(*value),
        (SchemaBody::Atom(_), ValueData::Atom) => ValueDataDraft::Atom,
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            let variant = variants.get(value.ordinal() as usize).ok_or_else(|| {
                SnapshotValueError::EnumOrdinalOutOfRangeV1 {
                    path: path.clone(),
                    ordinal: value.ordinal(),
                    variants: variants.len() as u32,
                }
            })?;
            let payload = match (variant.payload.as_ref(), value.payload()) {
                (Some(schema), Some(payload)) => {
                    Some(Box::new(canonical_data_to_draft_with_target(
                        schema,
                        payload,
                        &path.child(SnapshotPathSegment::EnumPayload(value.ordinal())),
                        target_schemas,
                    )?))
                }
                (None, None) => None,
                _ => {
                    return Err(SnapshotValueError::EnumPayloadMismatchV1 { path: path.clone() });
                }
            };
            ValueDataDraft::Enum(EnumDraft {
                ordinal: value.ordinal(),
                payload,
            })
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => {
            let value = value
                .as_deref()
                .map(|value| {
                    canonical_data_to_draft_with_target(
                        element,
                        value,
                        &path.child(SnapshotPathSegment::OptionValue),
                        target_schemas,
                    )
                    .map(Box::new)
                })
                .transpose()?;
            ValueDataDraft::Option(OptionDraft {
                present: value.is_some(),
                value,
            })
        }
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => {
            ensure_arity(path, elements.len(), values.len())?;
            let values = elements
                .iter()
                .zip(values.iter())
                .enumerate()
                .map(|(index, (schema, value))| {
                    canonical_data_to_draft_with_target(
                        schema,
                        value,
                        &path.child(SnapshotPathSegment::TupleElement(index as u32)),
                        target_schemas,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ValueDataDraft::Tuple(values.into_boxed_slice())
        }
        (SchemaBody::Record(fields), ValueData::Record(value)) => {
            ensure_arity(path, fields.len(), value.fields().len())?;
            let values = fields
                .iter()
                .zip(value.fields().iter())
                .enumerate()
                .map(|(index, (field, value))| {
                    Ok(NamedValueDraft {
                        name: field.name.clone(),
                        value: canonical_data_to_draft_with_target(
                            &field.schema,
                            value,
                            &path.child(SnapshotPathSegment::RecordField(index as u32)),
                            target_schemas,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?;
            ValueDataDraft::Record(values.into_boxed_slice())
        }
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(value)) => {
            let values = value
                .elements
                .to_values()
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    canonical_data_to_draft_with_target(
                        element,
                        value,
                        &path.child(SnapshotPathSegment::MatrixElement(index as u64)),
                        target_schemas,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ValueDataDraft::Matrix(values.into_boxed_slice())
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(value)) => {
            ensure_arity(path, columns.len(), value.columns.len())?;
            let columns = columns
                .iter()
                .zip(value.columns.iter())
                .enumerate()
                .map(|(column_index, (column, values))| {
                    let values = values
                        .to_values()
                        .iter()
                        .enumerate()
                        .map(|(row_index, value)| {
                            canonical_data_to_draft_with_target(
                                &column.schema,
                                value,
                                &path
                                    .child(SnapshotPathSegment::TableColumn(column_index as u32))
                                    .child(SnapshotPathSegment::TableRow(row_index as u64)),
                                target_schemas,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(TableColumnDraft {
                        name: column.name.clone(),
                        values: values.into_boxed_slice(),
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?;
            ValueDataDraft::Table(columns.into_boxed_slice())
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(value)) => {
            let values = value
                .elements()
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    canonical_data_to_draft_with_target(
                        element,
                        value.data(),
                        &path.child(SnapshotPathSegment::SetElement(index as u64)),
                        target_schemas,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ValueDataDraft::Set(values.into_boxed_slice())
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            let entries = map
                .entries()
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    Ok(MapEntryDraft {
                        items: vec![
                            canonical_data_to_draft_with_target(
                                key,
                                entry.key().data(),
                                &path.child(SnapshotPathSegment::MapKey(index as u64)),
                                target_schemas,
                            )?,
                            canonical_data_to_draft_with_target(
                                value,
                                entry.value(),
                                &path.child(SnapshotPathSegment::MapValue(index as u64)),
                                target_schemas,
                            )?,
                        ]
                        .into_boxed_slice(),
                    })
                })
                .collect::<Result<Vec<_>, SnapshotValueError>>()?;
            ValueDataDraft::Map(entries.into_boxed_slice())
        }
        (SchemaBody::ReifiedType, ValueData::Type(value)) => ValueDataDraft::Type(match value {
            ReifiedType::Kind(value) => {
                ReifiedTypeDraft::CanonicalKind(value.canonical_bytes().to_vec().into_boxed_slice())
            }
            ReifiedType::Schema(value) => ReifiedTypeDraft::Schema(*value),
        }),
        _ => return Err(data_mismatch_kind(schema, data.kind(), path)),
    };
    Ok(draft)
}

const RESIDENT_TOKEN_SEED: u64 = 0x6d65_6368_2d76_616c;

#[inline(always)]
fn token_word(hash: u64, word: u64) -> u64 {
    (hash.rotate_left(17) ^ word).wrapping_mul(0xd6e8_feb8_6659_fd93)
}

fn token_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    hash = token_word(hash, bytes.len() as u64);
    for byte in bytes {
        hash = token_word(hash, u64::from(*byte));
    }
    hash
}

fn token_sequence(mut hash: u64, sequence: &SequenceStorage) -> u64 {
    macro_rules! words {
        ($tag:literal, $values:expr, $convert:expr) => {{
            let values = $values;
            hash = token_word(hash, $tag);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, $convert(value));
            }
        }};
    }
    match sequence {
        SequenceStorage::U8(values) => words!(1, values, u64::from),
        SequenceStorage::U16(values) => words!(2, values, u64::from),
        SequenceStorage::U32(values) => words!(3, values, u64::from),
        SequenceStorage::U64(values) => words!(4, values, core::convert::identity),
        SequenceStorage::U128(values) => {
            hash = token_word(hash, 5);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, value as u64);
                hash = token_word(hash, (value >> 64) as u64);
            }
        }
        SequenceStorage::I8(values) => words!(6, values, |value: i8| value as u8 as u64),
        SequenceStorage::I16(values) => words!(7, values, |value: i16| value as u16 as u64),
        SequenceStorage::I32(values) => words!(8, values, |value: i32| value as u32 as u64),
        SequenceStorage::I64(values) => words!(9, values, |value: i64| value as u64),
        SequenceStorage::I128(values) => {
            hash = token_word(hash, 10);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                let value = value as u128;
                hash = token_word(hash, value as u64);
                hash = token_word(hash, (value >> 64) as u64);
            }
        }
        SequenceStorage::F32(values) => words!(11, values, |value: super::F32Bits| {
            u64::from(value.bits())
        }),
        SequenceStorage::F64(values) => words!(12, values, |value: super::F64Bits| value.bits()),
        SequenceStorage::Complex32(values) => {
            hash = token_word(hash, 13);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, u64::from(value.real().bits()));
                hash = token_word(hash, u64::from(value.imaginary().bits()));
            }
        }
        SequenceStorage::Complex64(values) => {
            hash = token_word(hash, 14);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter().copied() {
                hash = token_word(hash, value.real().bits());
                hash = token_word(hash, value.imaginary().bits());
            }
        }
        SequenceStorage::Rational64(values) => {
            hash = token_word(hash, 15);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter() {
                hash = token_word(hash, value.numerator() as u64);
                hash = token_word(hash, value.denominator());
            }
        }
        SequenceStorage::Bool(values) => words!(16, values, u64::from),
        SequenceStorage::String(values) => {
            hash = token_word(hash, 17);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter() {
                hash = token_bytes(hash, value.as_bytes());
            }
        }
        SequenceStorage::Id(values) => words!(18, values, core::convert::identity),
        SequenceStorage::Index(values) => words!(19, values, core::convert::identity),
        SequenceStorage::Unit(count) => {
            hash = token_word(hash, 20);
            hash = token_word(hash, *count);
        }
        SequenceStorage::Values(values) => {
            hash = token_word(hash, 21);
            hash = token_word(hash, values.len() as u64);
            for value in values.iter() {
                hash = token_data(hash, value);
            }
        }
    }
    hash
}

fn token_data(mut hash: u64, data: &ValueData) -> u64 {
    macro_rules! scalar {
        ($tag:literal, $word:expr) => {{
            hash = token_word(hash, $tag);
            hash = token_word(hash, $word);
        }};
    }
    match data {
        ValueData::Dynamic(value) => {
            hash = token_word(hash, 31);
            hash = token_bytes(hash, &value.canonical);
        }
        ValueData::U8(value) => scalar!(1, u64::from(*value)),
        ValueData::U16(value) => scalar!(2, u64::from(*value)),
        ValueData::U32(value) => scalar!(3, u64::from(*value)),
        ValueData::U64(value) => scalar!(4, *value),
        ValueData::U128(value) => {
            scalar!(5, *value as u64);
            hash = token_word(hash, (*value >> 64) as u64);
        }
        ValueData::I8(value) => scalar!(6, *value as u8 as u64),
        ValueData::I16(value) => scalar!(7, *value as u16 as u64),
        ValueData::I32(value) => scalar!(8, *value as u32 as u64),
        ValueData::I64(value) => scalar!(9, *value as u64),
        ValueData::I128(value) => {
            let value = *value as u128;
            scalar!(10, value as u64);
            hash = token_word(hash, (value >> 64) as u64);
        }
        ValueData::F32(value) => scalar!(11, u64::from(value.bits())),
        ValueData::F64(value) => scalar!(12, value.bits()),
        ValueData::Complex32(value) => {
            scalar!(13, u64::from(value.real().bits()));
            hash = token_word(hash, u64::from(value.imaginary().bits()));
        }
        ValueData::Complex64(value) => {
            scalar!(14, value.real().bits());
            hash = token_word(hash, value.imaginary().bits());
        }
        ValueData::Rational64(value) => {
            scalar!(15, value.numerator() as u64);
            hash = token_word(hash, value.denominator());
        }
        ValueData::Bool(value) => scalar!(16, u64::from(*value)),
        ValueData::String(value) => {
            hash = token_word(hash, 17);
            hash = token_bytes(hash, value.as_bytes());
        }
        ValueData::Id(value) => scalar!(18, *value),
        ValueData::Index(value) => scalar!(19, *value),
        ValueData::Atom => hash = token_word(hash, 20),
        ValueData::Enum(value) => {
            scalar!(21, u64::from(value.ordinal()));
            match value.payload() {
                Some(payload) => {
                    hash = token_word(hash, 1);
                    hash = token_data(hash, payload);
                }
                None => hash = token_word(hash, 0),
            }
        }
        ValueData::Option(value) => {
            hash = token_word(hash, 22);
            match value.as_deref() {
                Some(payload) => {
                    hash = token_word(hash, 1);
                    hash = token_data(hash, payload);
                }
                None => hash = token_word(hash, 0),
            }
        }
        ValueData::Tuple(values) => {
            scalar!(23, values.len() as u64);
            for value in values.iter() {
                hash = token_data(hash, value);
            }
        }
        ValueData::Record(value) => {
            scalar!(24, value.fields().len() as u64);
            for field in value.fields() {
                hash = token_data(hash, field);
            }
        }
        ValueData::Matrix(value) => {
            hash = token_word(hash, 25);
            hash = token_sequence(hash, &value.elements);
        }
        ValueData::Table(value) => {
            scalar!(26, value.columns.len() as u64);
            for column in value.columns.iter() {
                hash = token_sequence(hash, column);
            }
        }
        ValueData::Set(value) => {
            scalar!(27, value.elements().len() as u64);
            for element in value.elements() {
                hash = token_data(hash, element.data());
            }
        }
        ValueData::Map(value) => {
            scalar!(28, value.entries().len() as u64);
            for entry in value.entries() {
                hash = token_data(hash, entry.key().data());
                hash = token_data(hash, entry.value());
            }
        }
        ValueData::Type(ReifiedType::Kind(value)) => {
            hash = token_word(hash, 29);
            hash = token_bytes(hash, value.canonical_bytes());
        }
        ValueData::Type(ReifiedType::Schema(value)) => {
            hash = token_word(hash, 30);
            hash = token_bytes(hash, value.as_bytes());
        }
    }
    hash
}

fn finalized_value(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    data: ValueData,
    schemas: Option<Arc<SchemaTable>>,
) -> Value {
    let mut resident_token = token_bytes(RESIDENT_TOKEN_SEED, schema_key.as_bytes());
    resident_token = token_word(resident_token, shape.parameter_values().len() as u64);
    for value in shape.parameter_values() {
        resident_token = token_word(resident_token, *value);
    }
    resident_token = token_data(resident_token, &data);
    Value {
        schema,
        schema_key,
        shape,
        data,
        resident_token,
        schemas,
    }
}

fn dynamic_canonical(value: Option<&Value>, schema: Option<&SchemaBody>) -> Box<[u8]> {
    let Some(value) = value else {
        return Vec::from([0]).into_boxed_slice();
    };
    let schema = schema.expect("materialized dynamic values carry their concrete schema");
    let shape = value.shape().canonical_bytes();
    let payload = super::encoding::canonical_material(schema, value.data());
    let mut bytes = Vec::with_capacity(
        1 + value.schema_key().as_bytes().len() + 8 + shape.len() + 8 + payload.len(),
    );
    bytes.push(1);
    bytes.extend_from_slice(value.schema_key().as_bytes());
    bytes.extend_from_slice(&(shape.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&shape);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes.into_boxed_slice()
}

/// Wraps canonical resident data in a self-describing dynamic snapshot cell.
/// The binder has already validated the schema identity, shape, and physical
/// representation before this constructor is used.
#[doc(hidden)]
pub fn wrap_resident_dynamic_data(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: Arc<SchemaTable>,
    body: &SchemaBody,
    data: ValueData,
) -> ValueData {
    debug_assert_eq!(
        schemas.entry(schema).map(|entry| entry.key()),
        Some(schema_key),
        "resident dynamic values retain their authoritative schema arena"
    );
    let value = finalized_value(schema, schema_key, shape, data, Some(schemas));
    let canonical = dynamic_canonical(Some(&value), Some(body));
    ValueData::Dynamic(DynamicValue {
        value: Some(Box::new(value)),
        canonical,
    })
}

/// Rebuilds one tuple, record, or table layer from already validated canonical child
/// payloads. The template retains the authoritative schema, shape, and record
/// field ordering; callers may only replace children with the same canonical
/// representation kinds.
pub fn rebuild_composite_snapshot(template: &Value, children: Box<[ValueData]>) -> Option<Value> {
    let data = match template.data() {
        ValueData::Tuple(expected)
            if expected.len() == children.len()
                && expected
                    .iter()
                    .zip(children.iter())
                    .all(|(expected, child)| expected.kind() == child.kind()) =>
        {
            ValueData::Tuple(children)
        }
        ValueData::Record(expected)
            if expected.fields().len() == children.len()
                && expected
                    .fields()
                    .iter()
                    .zip(children.iter())
                    .all(|(expected, child)| expected.kind() == child.kind()) =>
        {
            ValueData::Record(RecordValue { fields: children })
        }
        ValueData::Table(expected) => {
            let column_lengths = expected
                .columns
                .iter()
                .map(SequenceStorage::len)
                .collect::<Option<Vec<_>>>()?;
            let expected_children = column_lengths
                .iter()
                .try_fold(0usize, |total, length| total.checked_add(*length))?;
            if expected_children != children.len() {
                return None;
            }
            let mut children = children.into_vec().into_iter();
            let columns = expected
                .columns
                .iter()
                .zip(column_lengths)
                .map(|(column, length)| {
                    column.rebuild_with_values(children.by_ref().take(length).collect())
                })
                .collect::<Option<Vec<_>>>()?
                .into_boxed_slice();
            if children.next().is_some() {
                return None;
            }
            ValueData::Table(TableValue { columns })
        }
        _ => return None,
    };
    Some(finalized_value(
        template.schema,
        template.schema_key,
        template.shape.clone(),
        data,
        template.schemas.clone(),
    ))
}

/// Rebuilds a canonical `set<f64>` snapshot from candidate values while
/// preserving the output template's authoritative schema and shape.
/// Duplicate candidates use the same normalized key equality as ordinary
/// snapshot finalization.
pub fn rebuild_f64_set_snapshot(template: &Value, candidates: &[f64]) -> Option<Value> {
    let ValueData::Set(expected) = template.data() else {
        return None;
    };
    if expected
        .elements()
        .iter()
        .any(|element| !matches!(element.data(), ValueData::F64(_)))
    {
        return None;
    }

    build_f64_set_snapshot(
        template.schema,
        template.schema_key,
        template.shape.clone(),
        template.schemas.as_deref()?,
        Some(expected.elements().len()),
        Some(expected.elements().len()),
        candidates,
    )
}

/// Constructs a canonical `set<f64>` snapshot for an exact or dynamic
/// cardinality contract from metadata already validated by resident
/// activation.
pub fn build_f64_set_snapshot(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: &SchemaTable,
    exact_cardinality: Option<usize>,
    maximum_cardinality: Option<usize>,
    candidates: &[f64],
) -> Option<Value> {
    let element_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let mut elements = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().copied().enumerate() {
        let data = super::relations::normalized_key_data(
            &element_schema,
            ValueData::F64(super::F64Bits::from_f64(candidate)),
        )
        .ok()?;
        let duplicate = elements.iter().any(|existing: &CanonicalKeyValue| {
            super::relations::compare_key_data(&element_schema, existing.data(), &data)
                .is_ok_and(|order| order == core::cmp::Ordering::Equal)
        });
        if !duplicate {
            super::relations::insert_set_key(
                &element_schema,
                &mut elements,
                data,
                &SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64)),
            )
            .ok()?;
        }
    }
    if exact_cardinality.is_some_and(|expected| elements.len() != expected)
        || maximum_cardinality.is_some_and(|maximum| elements.len() > maximum)
    {
        return None;
    }
    Some(finalized_value(
        schema,
        schema_key,
        shape,
        ValueData::Set(SetValue {
            elements: elements.into_boxed_slice(),
        }),
        Some(Arc::new(schemas.clone())),
    ))
}

/// Tests membership in a canonical `set<f64>` snapshot with set-key float
/// normalization (`-0.0` and NaN payloads included).
pub fn f64_set_snapshot_contains(value: &Value, candidate: f64) -> Option<bool> {
    let ValueData::Set(set) = value.data() else {
        return None;
    };
    let element_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let candidate = super::relations::normalized_key_data(
        &element_schema,
        ValueData::F64(super::F64Bits::from_f64(candidate)),
    )
    .ok()?;
    set.elements()
        .iter()
        .map(|element| {
            super::relations::compare_key_data(&element_schema, element.data(), &candidate)
        })
        .collect::<Result<Vec<_>, _>>()
        .ok()
        .map(|orders| {
            orders
                .into_iter()
                .any(|order| order == core::cmp::Ordering::Equal)
        })
}

/// Constructs the canonical output of removing one `f64` element from a
/// resident set snapshot. Key comparison uses the same normalization as set
/// construction, including signed zero and NaN payloads.
pub fn build_f64_set_snapshot_after_remove(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    schemas: &SchemaTable,
    exact_cardinality: Option<usize>,
    maximum_cardinality: Option<usize>,
    source: &Value,
    candidate: f64,
) -> Option<Value> {
    let ValueData::Set(set) = source.data() else {
        return None;
    };
    let element_schema = SchemaBody::FloatingPoint(FloatWidth::W64);
    let candidate = super::relations::normalized_key_data(
        &element_schema,
        ValueData::F64(super::F64Bits::from_f64(candidate)),
    )
    .ok()?;
    let values = set
        .elements()
        .iter()
        .filter_map(|element| {
            let equal =
                super::relations::compare_key_data(&element_schema, element.data(), &candidate)
                    .ok()
                    == Some(core::cmp::Ordering::Equal);
            if equal {
                None
            } else {
                match element.data() {
                    ValueData::F64(value) => Some(value.to_f64()),
                    _ => None,
                }
            }
        })
        .collect::<Vec<_>>();
    if values.len()
        + usize::from(f64_set_snapshot_contains(
            source,
            candidate_f64(&candidate),
        )?)
        != set.elements().len()
    {
        return None;
    }
    build_f64_set_snapshot(
        schema,
        schema_key,
        shape,
        schemas,
        exact_cardinality,
        maximum_cardinality,
        &values,
    )
}

fn candidate_f64(candidate: &ValueData) -> f64 {
    match candidate {
        ValueData::F64(value) => value.to_f64(),
        _ => unreachable!("f64 candidate normalization preserves its data kind"),
    }
}

pub(super) fn finalize_value(
    draft: ValueDraft,
    context: &SnapshotValidationContext<'_>,
) -> Result<Value, SnapshotValueError> {
    let entry =
        context
            .schemas
            .entry(draft.schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema {
                schema: draft.schema,
            })?;
    let shape = entry.schema().instantiate_shape(draft.shape_values)?;
    let path = SnapshotPath::root();
    let data = finalize_data(entry.schema().body(), draft.data, &shape, context, &path)?;
    Ok(finalized_value(
        draft.schema,
        entry.key(),
        shape,
        data,
        Some(Arc::new(context.schemas.clone())),
    ))
}

pub(super) fn finalize_data(
    schema: &SchemaBody,
    draft: ValueDataDraft,
    shape: &ShapeInstance,
    context: &SnapshotValidationContext<'_>,
    path: &SnapshotPath,
) -> Result<ValueData, SnapshotValueError> {
    let actual_kind = draft.kind();
    macro_rules! exact {
        ($schema:pat, $draft:pat => $value:expr) => {
            if matches!(schema, $schema) {
                if let $draft = draft {
                    return Ok($value);
                }
                return Err(data_mismatch_kind(schema, actual_kind, path));
            }
        };
    }

    exact!(SchemaBody::Bool, ValueDataDraft::Bool(value) => ValueData::Bool(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueDataDraft::U8(value) => ValueData::U8(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueDataDraft::U16(value) => ValueData::U16(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueDataDraft::U32(value) => ValueData::U32(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueDataDraft::U64(value) => ValueData::U64(value));
    exact!(SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueDataDraft::U128(value) => ValueData::U128(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W8), ValueDataDraft::I8(value) => ValueData::I8(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W16), ValueDataDraft::I16(value) => ValueData::I16(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W32), ValueDataDraft::I32(value) => ValueData::I32(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W64), ValueDataDraft::I64(value) => ValueData::I64(value));
    exact!(SchemaBody::SignedInteger(IntegerWidth::W128), ValueDataDraft::I128(value) => ValueData::I128(value));
    exact!(SchemaBody::FloatingPoint(FloatWidth::W32), ValueDataDraft::F32(value) => ValueData::F32(value));
    exact!(SchemaBody::FloatingPoint(FloatWidth::W64), ValueDataDraft::F64(value) => ValueData::F64(value));
    exact!(SchemaBody::Complex(FloatWidth::W32), ValueDataDraft::Complex32(value) => ValueData::Complex32(value));
    exact!(SchemaBody::Complex(FloatWidth::W64), ValueDataDraft::Complex64(value) => ValueData::Complex64(value));
    exact!(SchemaBody::String, ValueDataDraft::String(value) => ValueData::String(value.into_boxed_str()));
    exact!(SchemaBody::Id, ValueDataDraft::Id(value) => ValueData::Id(value));
    if matches!(schema, SchemaBody::Index) {
        if let ValueDataDraft::Index(value) = draft {
            if value == 0 {
                return Err(SnapshotValueError::InvalidIndexV1 {
                    path: path.clone(),
                    value,
                });
            }
            return Ok(ValueData::Index(value));
        }
        return Err(data_mismatch_kind(schema, actual_kind, path));
    }
    exact!(SchemaBody::Atom(_), ValueDataDraft::Atom => ValueData::Atom);

    match (schema, draft) {
        (SchemaBody::Dynamic, ValueDataDraft::Dynamic(draft)) => {
            let value = draft
                .map(|draft| finalize_value(*draft, context).map(Box::new))
                .transpose()?;
            let concrete = value
                .as_deref()
                .map(|value| value.validate_against(context.schemas))
                .transpose()?;
            let canonical = dynamic_canonical(value.as_deref(), concrete.map(Schema::body));
            Ok(ValueData::Dynamic(DynamicValue { value, canonical }))
        }
        (
            SchemaBody::Rational64,
            ValueDataDraft::Rational64 {
                numerator,
                denominator,
            },
        ) => Ok(ValueData::Rational64(super::Rational64Value::new(
            numerator,
            denominator,
        )?)),
        (SchemaBody::Enum { variants, .. }, ValueDataDraft::Enum(draft)) => {
            let variant = variants.get(draft.ordinal as usize).ok_or(
                SnapshotValueError::EnumOrdinalOutOfRangeV1 {
                    path: path.clone(),
                    ordinal: draft.ordinal,
                    variants: variants.len() as u32,
                },
            )?;
            let payload_path = path.child(SnapshotPathSegment::EnumPayload(draft.ordinal));
            let payload = match (&variant.payload, draft.payload) {
                (None, None) => None,
                (Some(schema), Some(payload)) => Some(Box::new(finalize_data(
                    schema,
                    *payload,
                    shape,
                    context,
                    &payload_path,
                )?)),
                _ => {
                    return Err(SnapshotValueError::EnumPayloadMismatchV1 { path: path.clone() });
                }
            };
            Ok(ValueData::Enum(EnumValue {
                ordinal: draft.ordinal,
                payload,
            }))
        }
        (SchemaBody::Option(element), ValueDataDraft::Option(draft)) => {
            let value = match (draft.present, draft.value) {
                (false, None) => None,
                (true, Some(value)) => Some(Box::new(finalize_data(
                    element,
                    *value,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::OptionValue),
                )?)),
                (present, value) => {
                    return Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
                        path: path.clone(),
                        expected: u64::from(present),
                        actual: u64::from(value.is_some()),
                    });
                }
            };
            Ok(ValueData::Option(value))
        }
        (SchemaBody::Tuple(elements), ValueDataDraft::Tuple(values)) => {
            ensure_arity(path, elements.len(), values.len())?;
            let mut finalized = Vec::with_capacity(values.len());
            for (index, (schema, draft)) in elements.iter().zip(values.into_vec()).enumerate() {
                finalized.push(finalize_data(
                    schema,
                    draft,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::TupleElement(index as u32)),
                )?);
            }
            Ok(ValueData::Tuple(finalized.into_boxed_slice()))
        }
        (SchemaBody::Record(fields), ValueDataDraft::Record(values)) => {
            let values = order_named_values(fields, values, path)?;
            let mut finalized = Vec::with_capacity(values.len());
            for (index, (field, draft)) in fields.iter().zip(values).enumerate() {
                finalized.push(finalize_data(
                    &field.schema,
                    draft,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::RecordField(index as u32)),
                )?);
            }
            Ok(ValueData::Record(RecordValue {
                fields: finalized.into_boxed_slice(),
            }))
        }
        (
            SchemaBody::Matrix {
                element,
                dimensions,
            },
            ValueDataDraft::Matrix(values),
        ) => {
            let expected = resolved_product(dimensions, shape)?;
            ensure_cardinality(path, expected, values.len())?;
            let mut finalized = Vec::with_capacity(values.len());
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                finalized.push(finalize_data(
                    element,
                    draft,
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::MatrixElement(index as u64)),
                )?);
            }
            Ok(ValueData::Matrix(MatrixValue {
                elements: SequenceStorage::from_values(element, finalized),
            }))
        }
        (SchemaBody::Table { columns, rows }, ValueDataDraft::Table(values)) => {
            let values = order_table_columns(columns, values, path)?;
            let actual_rows = values.first().map_or(0, |values| values.len());
            ensure_collection_cardinality(path, rows, shape, actual_rows)?;
            let mut finalized_columns = Vec::with_capacity(values.len());
            for (column_index, (column, drafts)) in columns.iter().zip(values).enumerate() {
                if drafts.len() != actual_rows {
                    return Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
                        path: path.clone(),
                        expected: actual_rows as u64,
                        actual: drafts.len() as u64,
                    });
                }
                let mut finalized = Vec::with_capacity(drafts.len());
                for (row, draft) in drafts.into_vec().into_iter().enumerate() {
                    let column_path = path
                        .child(SnapshotPathSegment::TableColumn(column_index as u32))
                        .child(SnapshotPathSegment::TableRow(row as u64));
                    finalized.push(finalize_data(
                        &column.schema,
                        draft,
                        shape,
                        context,
                        &column_path,
                    )?);
                }
                finalized_columns.push(SequenceStorage::from_values(&column.schema, finalized));
            }
            Ok(ValueData::Table(TableValue {
                columns: finalized_columns.into_boxed_slice(),
            }))
        }
        (
            SchemaBody::Set {
                element,
                cardinality,
            },
            ValueDataDraft::Set(values),
        ) => {
            ensure_collection_cardinality(path, cardinality, shape, values.len())?;
            let mut finalized = Vec::with_capacity(values.len());
            for (index, draft) in values.into_vec().into_iter().enumerate() {
                let element_path = path.child(SnapshotPathSegment::SetElement(index as u64));
                let data = finalize_data(element, draft, shape, context, &element_path)?;
                super::relations::insert_set_key(element, &mut finalized, data, &element_path)?;
            }
            Ok(ValueData::Set(SetValue {
                elements: finalized.into_boxed_slice(),
            }))
        }
        (
            SchemaBody::Map {
                key,
                value,
                cardinality,
            },
            ValueDataDraft::Map(entries),
        ) => {
            ensure_collection_cardinality(path, cardinality, shape, entries.len())?;
            let mut finalized = Vec::with_capacity(entries.len());
            for (index, entry) in entries.into_vec().into_iter().enumerate() {
                if entry.items.len() != 2 {
                    return Err(SnapshotValueError::MapEntryArityMismatchV1 {
                        path: path.clone(),
                        actual: entry.items.len() as u64,
                    });
                }
                let mut items = entry.items.into_vec().into_iter();
                let key_data = finalize_data(
                    key,
                    items.next().expect("validated map key exists"),
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::MapKey(index as u64)),
                )?;
                let value_data = finalize_data(
                    value,
                    items.next().expect("validated map value exists"),
                    shape,
                    context,
                    &path.child(SnapshotPathSegment::MapValue(index as u64)),
                )?;
                super::relations::insert_map_entry(
                    key,
                    &mut finalized,
                    key_data,
                    value_data,
                    &path.child(SnapshotPathSegment::MapKey(index as u64)),
                )?;
            }
            Ok(ValueData::Map(MapValue {
                entries: finalized.into_boxed_slice(),
            }))
        }
        (SchemaBody::ReifiedType, ValueDataDraft::Type(draft)) => {
            let reified = match draft {
                ReifiedTypeDraft::Schema(key) => ReifiedType::Schema(key),
                ReifiedTypeDraft::CanonicalKind(bytes) => {
                    ReifiedType::Kind(ReifiedKind::from_canonical_bytes(bytes)?)
                }
                ReifiedTypeDraft::Kind {
                    kind,
                    dimension_parameters,
                } => ReifiedType::Kind(ReifiedKind::from_closed_kind_with_optional_resolver(
                    &kind,
                    &dimension_parameters,
                    context.named_kinds,
                )?),
            };
            Ok(ValueData::Type(reified))
        }
        (_, draft) => Err(data_mismatch(schema, &draft, path)),
    }
}

fn ensure_arity(
    path: &SnapshotPath,
    expected: usize,
    actual: usize,
) -> Result<(), SnapshotValueError> {
    if expected == actual {
        return Ok(());
    }
    Err(SnapshotValueError::AggregateArityMismatchV1 {
        path: path.clone(),
        expected: expected as u64,
        actual: actual as u64,
    })
}

fn ensure_cardinality(
    path: &SnapshotPath,
    expected: u64,
    actual: usize,
) -> Result<(), SnapshotValueError> {
    let actual = actual as u64;
    if expected == actual {
        return Ok(());
    }
    Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
        path: path.clone(),
        expected,
        actual,
    })
}

fn ensure_collection_cardinality(
    path: &SnapshotPath,
    cardinality: &crate::CardinalitySpec,
    shape: &ShapeInstance,
    actual: usize,
) -> Result<(), SnapshotValueError> {
    match cardinality {
        crate::CardinalitySpec::Exact(value) => ensure_cardinality(
            path,
            crate::schema::evaluate_dimension(value, shape.parameter_values())?,
            actual,
        ),
        crate::CardinalitySpec::Dynamic { upper_bound: None } => Ok(()),
        crate::CardinalitySpec::Dynamic {
            upper_bound: Some(value),
        } => {
            let upper = crate::schema::evaluate_dimension(value, shape.parameter_values())?;
            if actual as u64 <= upper {
                Ok(())
            } else {
                Err(SnapshotValueError::PayloadCardinalityMismatchV1 {
                    path: path.clone(),
                    expected: upper,
                    actual: actual as u64,
                })
            }
        }
    }
}

fn resolved_product(
    dimensions: &[crate::DimensionExpr],
    shape: &ShapeInstance,
) -> Result<u64, SnapshotValueError> {
    let mut total = 1_u64;
    for dimension in dimensions {
        let extent = crate::schema::evaluate_dimension(dimension, shape.parameter_values())?;
        total = total
            .checked_mul(extent)
            .ok_or(crate::SemanticModelError::DimensionOverflowV1)?;
    }
    Ok(total)
}

fn order_named_values(
    fields: &[crate::SchemaField],
    values: Box<[super::NamedValueDraft]>,
    path: &SnapshotPath,
) -> Result<Vec<ValueDataDraft>, SnapshotValueError> {
    if fields.len() != values.len() {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    let mut pending = values.into_vec().into_iter().map(Some).collect::<Vec<_>>();
    let mut ordered = Vec::with_capacity(fields.len());
    for field in fields {
        let mut matched = None;
        for (index, value) in pending.iter().enumerate() {
            if value.as_ref().is_some_and(|value| value.name == field.name) {
                if matched.is_some() {
                    return Err(SnapshotValueError::AggregateFieldMismatchV1 {
                        path: path.clone(),
                    });
                }
                matched = Some(index);
            }
        }
        let Some(index) = matched else {
            return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
        };
        ordered.push(pending[index].take().expect("matched record field").value);
    }
    if pending.iter().any(Option::is_some) {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    Ok(ordered)
}

fn order_table_columns(
    columns: &[crate::SchemaField],
    values: Box<[super::TableColumnDraft]>,
    path: &SnapshotPath,
) -> Result<Vec<Box<[ValueDataDraft]>>, SnapshotValueError> {
    if columns.len() != values.len() {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    let mut pending = values.into_vec().into_iter().map(Some).collect::<Vec<_>>();
    let mut ordered = Vec::with_capacity(columns.len());
    for column in columns {
        let mut matched = None;
        for (index, value) in pending.iter().enumerate() {
            if value
                .as_ref()
                .is_some_and(|value| value.name == column.name)
            {
                if matched.is_some() {
                    return Err(SnapshotValueError::AggregateFieldMismatchV1 {
                        path: path.clone(),
                    });
                }
                matched = Some(index);
            }
        }
        let Some(index) = matched else {
            return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
        };
        ordered.push(pending[index].take().expect("matched table column").values);
    }
    if pending.iter().any(Option::is_some) {
        return Err(SnapshotValueError::AggregateFieldMismatchV1 { path: path.clone() });
    }
    Ok(ordered)
}

fn data_mismatch(
    schema: &SchemaBody,
    draft: &ValueDataDraft,
    path: &SnapshotPath,
) -> SnapshotValueError {
    SnapshotValueError::SnapshotDataSchemaMismatch {
        path: path.clone(),
        expected: schema_kind(schema),
        actual: draft.kind(),
    }
}

fn data_mismatch_kind(
    schema: &SchemaBody,
    actual: super::ValueDataKind,
    path: &SnapshotPath,
) -> SnapshotValueError {
    SnapshotValueError::SnapshotDataSchemaMismatch {
        path: path.clone(),
        expected: schema_kind(schema),
        actual,
    }
}

pub(super) const fn schema_kind(schema: &SchemaBody) -> SchemaDataKind {
    match schema {
        SchemaBody::Dynamic => SchemaDataKind::Dynamic,
        SchemaBody::Bool => SchemaDataKind::Bool,
        SchemaBody::UnsignedInteger(_) => SchemaDataKind::UnsignedInteger,
        SchemaBody::SignedInteger(_) => SchemaDataKind::SignedInteger,
        SchemaBody::FloatingPoint(_) => SchemaDataKind::FloatingPoint,
        SchemaBody::Complex(_) => SchemaDataKind::Complex,
        SchemaBody::Rational64 => SchemaDataKind::Rational64,
        SchemaBody::String => SchemaDataKind::String,
        SchemaBody::Id => SchemaDataKind::Id,
        SchemaBody::Index => SchemaDataKind::Index,
        SchemaBody::Atom(_) => SchemaDataKind::Atom,
        SchemaBody::Enum { .. } => SchemaDataKind::Enum,
        SchemaBody::Option(_) => SchemaDataKind::Option,
        SchemaBody::Tuple(_) => SchemaDataKind::Tuple,
        SchemaBody::Record(_) => SchemaDataKind::Record,
        SchemaBody::Matrix { .. } => SchemaDataKind::Matrix,
        SchemaBody::Table { .. } => SchemaDataKind::Table,
        SchemaBody::Set { .. } => SchemaDataKind::Set,
        SchemaBody::Map { .. } => SchemaDataKind::Map,
        SchemaBody::ReifiedType => SchemaDataKind::ReifiedType,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{F64Bits, TableColumnDraft};
    use crate::{SchemaDraft, SchemaField, SchemaTableBuilder};

    #[test]
    fn index_snapshots_are_one_based() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Index,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let context = SnapshotValidationContext::new(&schemas);

        let draft = |value| ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Index(value),
        };
        assert!(matches!(
            draft(0).finalize(&context),
            Err(SnapshotValueError::InvalidIndexV1 { value: 0, .. })
        ));
        assert!(draft(1).finalize(&context).is_ok());
        assert!(draft(u64::MAX).finalize(&context).is_ok());
    }

    #[test]
    fn composite_rebuild_preserves_table_columns_and_storage() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Table {
                columns: vec![
                    SchemaField {
                        name: "id".to_owned(),
                        schema: SchemaBody::String,
                    },
                    SchemaField {
                        name: "x".to_owned(),
                        schema: SchemaBody::FloatingPoint(FloatWidth::W64),
                    },
                ]
                .into_boxed_slice(),
                rows: crate::CardinalitySpec::Exact(crate::DimensionExpr::Constant(2)),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let template = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Table(
                vec![
                    TableColumnDraft {
                        name: "id".to_owned(),
                        values: vec![
                            ValueDataDraft::String("a".to_owned()),
                            ValueDataDraft::String("b".to_owned()),
                        ]
                        .into_boxed_slice(),
                    },
                    TableColumnDraft {
                        name: "x".to_owned(),
                        values: vec![
                            ValueDataDraft::F64(F64Bits::from_f64(1.0)),
                            ValueDataDraft::F64(F64Bits::from_f64(2.0)),
                        ]
                        .into_boxed_slice(),
                    },
                ]
                .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();

        let rebuilt = rebuild_composite_snapshot(
            &template,
            vec![
                ValueData::String("c".into()),
                ValueData::String("d".into()),
                ValueData::F64(F64Bits::from_f64(3.0)),
                ValueData::F64(F64Bits::from_f64(4.0)),
            ]
            .into_boxed_slice(),
        )
        .unwrap();
        let ValueData::Table(table) = rebuilt.data() else {
            panic!("rebuilt composite must remain a table");
        };
        let super::super::SequenceView::String(ids) = table.column(0).unwrap() else {
            panic!("string table column changed representation");
        };
        assert_eq!(
            ids.iter().map(|id| id.as_ref()).collect::<Vec<_>>(),
            ["c", "d"]
        );
        let super::super::SequenceView::F64(values) = table.column(1).unwrap() else {
            panic!("f64 table column changed representation");
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            [3.0, 4.0]
        );
    }
}
