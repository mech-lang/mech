use super::sequence::SequenceStorage;
use super::{
    CanonicalKeyValue, EnumValue, MapEntryValue, MapValue, MatrixValue, RecordValue, ReifiedKind,
    ReifiedType, ReifiedTypeDraft, SchemaDataKind, SetValue, SnapshotPath, SnapshotPathSegment,
    SnapshotValueError, TableValue, ValueData, ValueDataDraft, ValueDraft,
};
use crate::{
    FloatWidth, IntegerWidth, NamedKindPathResolver, Schema, SchemaBody, SchemaId, SchemaKey,
    SchemaTable, ShapeInstance,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

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
    }
}

/// Rebuilds one tuple or record layer from already validated canonical child
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
        _ => return None,
    };
    Some(finalized_value(
        template.schema,
        template.schema_key,
        template.shape.clone(),
        data,
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
        expected.elements().len(),
        candidates,
    )
}

/// Constructs a canonical `set<f64>` snapshot from schema metadata already
/// validated by resident activation.
pub fn build_f64_set_snapshot(
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
    expected_cardinality: usize,
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
    if elements.len() != expected_cardinality {
        return None;
    }
    Some(finalized_value(
        schema,
        schema_key,
        shape,
        ValueData::Set(SetValue {
            elements: elements.into_boxed_slice(),
        }),
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
    Ok(finalized_value(draft.schema, entry.key(), shape, data))
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
    exact!(SchemaBody::Index, ValueDataDraft::Index(value) => ValueData::Index(value));
    exact!(SchemaBody::Atom(_), ValueDataDraft::Atom => ValueData::Atom);

    match (schema, draft) {
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
            let expected = crate::schema::evaluate_dimension(rows, shape.parameter_values())?;
            let values = order_table_columns(columns, values, path)?;
            let mut finalized_columns = Vec::with_capacity(values.len());
            for (column_index, (column, drafts)) in columns.iter().zip(values).enumerate() {
                ensure_cardinality(path, expected, drafts.len())?;
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
            let expected =
                crate::schema::evaluate_dimension(cardinality, shape.parameter_values())?;
            ensure_cardinality(path, expected, values.len())?;
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
            let expected =
                crate::schema::evaluate_dimension(cardinality, shape.parameter_values())?;
            ensure_cardinality(path, expected, entries.len())?;
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
