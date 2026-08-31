//! Schema-first bytecode-v1 constant decoding.
//!
//! The wire format remains byte-for-byte compatible with bytecode v1.  This
//! decoder builds immutable canonical data directly; it never constructs the
//! retired universal runtime value or any of its aggregate wrappers.

use super::{
    ByteReader, ConstantCodecContext, ConstantEntry, RuntimeType, checked_usize, invalid, matrix,
    read_child_payload, validate_matrix_payload_feasibility, validate_table_payload_shape,
};
#[cfg(feature = "semantic-compiler")]
use super::{EncodedConstant, MatrixStorage};
#[cfg(any(feature = "semantic-compiler", feature = "matrix"))]
use crate::ValueData;
#[cfg(any(feature = "semantic-compiler", feature = "matrix"))]
use crate::snapshot::SequenceView;
use crate::snapshot::{
    Complex64Bits, F32Bits, F64Bits, MapEntryDraft, NamedValueDraft, OptionDraft, ReifiedTypeDraft,
    SnapshotValidationContext, TableColumnDraft,
};
#[cfg(feature = "matrix")]
use crate::{CanonicalMatrixElementBacking, Ref};
use crate::{
    CanonicalNominalPath, CardinalitySpec, DimensionExpr, FloatWidth, IntegerWidth, KindExpr,
    KindField, KindId, MResult, NamedKindPathResolver, SchemaBody, SchemaDraft, SchemaField,
    SchemaTableBuilder, Value, ValueCell, ValueDataDraft, ValueDraft,
};
#[cfg(feature = "semantic-compiler")]
use crate::{
    FunctionMatrixRepresentation, FunctionMatrixStoragePattern, FunctionValueRepresentation,
};

#[cfg(all(feature = "no_std", feature = "matrix"))]
use alloc::rc::Rc;
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec, vec::Vec};
#[cfg(all(not(feature = "no_std"), feature = "matrix"))]
use std::rc::Rc;
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec, vec::Vec};

#[derive(Clone)]
struct DecodedDraft {
    body: SchemaBody,
    data: ValueDataDraft,
    named_kinds: BTreeMap<KindId, CanonicalNominalPath>,
}

impl DecodedDraft {
    fn new(body: SchemaBody, data: ValueDataDraft) -> Self {
        Self {
            body,
            data,
            named_kinds: BTreeMap::new(),
        }
    }

    fn finalize(self) -> MResult<Value> {
        let schema = SchemaDraft {
            dimension_parameters: Vec::new().into_boxed_slice(),
            body: self.body,
        }
        .finalize()?;
        let mut schemas = SchemaTableBuilder::new();
        let handle = schemas.insert(schema)?;
        let build = schemas.finish()?;
        let schema = build.resolve(handle)?;
        let schemas = Arc::new(build.table);
        let context = BytecodeNamedKinds(self.named_kinds);
        let validation = SnapshotValidationContext::with_named_kinds(&schemas, &context);
        ValueDraft {
            schema,
            shape_values: Vec::new().into_boxed_slice(),
            data: self.data,
        }
        .finalize(&validation)
        .map_err(|error| {
            invalid::<()>(format!("canonical constant validation failed: {error:?}")).unwrap_err()
        })
    }
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn encode_value(
    value: &Value,
    representation: FunctionValueRepresentation,
) -> MResult<EncodedConstant> {
    let schema = ValueCell::from_snapshot(value.clone())?.closed_schema_body()?;
    encode_canonical_data(&schema, value.data(), Some(representation), 0, false)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn encode_exact_backing(
    value: &Value,
    representation: FunctionValueRepresentation,
) -> MResult<EncodedConstant> {
    let mut encoded = encode_value(value, representation)?;
    if let RuntimeType::Matrix { element, .. } = &encoded.runtime_type {
        encoded.alignment = matrix_element_alignment(element);
    }
    Ok(encoded)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn encode_composite_template(
    value: &Value,
    representation: FunctionValueRepresentation,
) -> MResult<EncodedConstant> {
    let schema = ValueCell::from_snapshot(value.clone())?.closed_schema_body()?;
    encode_canonical_data(&schema, value.data(), Some(representation), 0, true)
}

#[cfg(feature = "semantic-compiler")]
fn encode_canonical_data(
    schema: &SchemaBody,
    data: &ValueData,
    representation: Option<FunctionValueRepresentation>,
    depth: usize,
    dynamic_placeholder: bool,
) -> MResult<EncodedConstant> {
    if depth >= super::MAX_CONSTANT_NESTING {
        return Err(super::super::depth_exceeded(super::MAX_CONSTANT_NESTING));
    }
    let nested = |schema: &SchemaBody, data: &ValueData| {
        encode_canonical_data(schema, data, None, depth + 1, dynamic_placeholder)
    };
    let scalar = |runtime_type, alignment, bytes| {
        Ok(EncodedConstant {
            runtime_type,
            alignment,
            bytes,
        })
    };
    match (schema, data) {
        (SchemaBody::Dynamic, ValueData::Dynamic(_)) if dynamic_placeholder => {
            scalar(RuntimeType::Any, 1, Vec::new())
        }
        (SchemaBody::Bool, ValueData::Bool(value)) => {
            scalar(RuntimeType::Bool, 1, vec![u8::from(*value)])
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueData::U8(value)) => {
            scalar(RuntimeType::U8, 1, vec![*value])
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueData::U16(value)) => {
            scalar(RuntimeType::U16, 2, value.to_le_bytes().to_vec())
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueData::U32(value)) => {
            scalar(RuntimeType::U32, 4, value.to_le_bytes().to_vec())
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueData::U64(value)) => {
            scalar(RuntimeType::U64, 8, value.to_le_bytes().to_vec())
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueData::U128(value)) => {
            scalar(RuntimeType::U128, 16, value.to_le_bytes().to_vec())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W8), ValueData::I8(value)) => {
            scalar(RuntimeType::I8, 1, value.to_le_bytes().to_vec())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W16), ValueData::I16(value)) => {
            scalar(RuntimeType::I16, 2, value.to_le_bytes().to_vec())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W32), ValueData::I32(value)) => {
            scalar(RuntimeType::I32, 4, value.to_le_bytes().to_vec())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W64), ValueData::I64(value)) => {
            scalar(RuntimeType::I64, 8, value.to_le_bytes().to_vec())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W128), ValueData::I128(value)) => {
            scalar(RuntimeType::I128, 16, value.to_le_bytes().to_vec())
        }
        (SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(value)) => {
            scalar(RuntimeType::F32, 4, value.bits().to_le_bytes().to_vec())
        }
        (SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(value)) => {
            scalar(RuntimeType::F64, 8, value.bits().to_le_bytes().to_vec())
        }
        (SchemaBody::Complex(FloatWidth::W64), ValueData::Complex64(value)) => {
            let mut bytes = value.real().bits().to_le_bytes().to_vec();
            bytes.extend_from_slice(&value.imaginary().bits().to_le_bytes());
            scalar(RuntimeType::C64, 16, bytes)
        }
        (SchemaBody::Rational64, ValueData::Rational64(value)) => {
            let denominator = i64::try_from(value.denominator())
                .map_err(|_| invalid::<()>("R64 denominator exceeds i64").unwrap_err())?;
            let mut bytes = value.numerator().to_le_bytes().to_vec();
            bytes.extend_from_slice(&denominator.to_le_bytes());
            scalar(RuntimeType::R64, 16, bytes)
        }
        (SchemaBody::String, ValueData::String(value)) => {
            scalar(RuntimeType::String, 1, value.as_bytes().to_vec())
        }
        (SchemaBody::Id, ValueData::Id(value)) => {
            scalar(RuntimeType::Id, 8, value.to_le_bytes().to_vec())
        }
        (SchemaBody::Index, ValueData::Index(value)) => {
            scalar(RuntimeType::Index, 8, value.to_le_bytes().to_vec())
        }
        (SchemaBody::Tuple(schemas), ValueData::Tuple(values)) if schemas.is_empty() => {
            scalar(RuntimeType::Empty, 1, Vec::new())
        }
        (SchemaBody::Tuple(schemas), ValueData::Tuple(values)) => {
            let children = schemas
                .iter()
                .zip(values)
                .map(|(schema, value)| nested(schema, value))
                .collect::<MResult<Vec<_>>>()?;
            framed_composite(
                RuntimeType::Tuple(
                    children
                        .iter()
                        .map(|child| child.runtime_type.clone())
                        .collect(),
                ),
                children,
            )
        }
        (SchemaBody::Record(fields), ValueData::Record(value)) => {
            let children = fields
                .iter()
                .zip(value.fields())
                .map(|(field, value)| nested(&field.schema, value))
                .collect::<MResult<Vec<_>>>()?;
            let runtime_type = RuntimeType::Record(
                fields
                    .iter()
                    .zip(&children)
                    .map(|(field, child)| (field.name.clone(), child.runtime_type.clone()))
                    .collect(),
            );
            framed_composite(runtime_type, children)
        }
        (
            SchemaBody::Matrix {
                element,
                dimensions,
            },
            ValueData::Matrix(value),
        ) => {
            let [
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ] = dimensions.as_ref()
            else {
                return invalid("canonical matrix constant dimensions are unresolved");
            };
            let rows = u32::try_from(*rows)
                .map_err(|_| invalid::<()>("matrix row count exceeds u32").unwrap_err())?;
            let columns = u32::try_from(*columns)
                .map_err(|_| invalid::<()>("matrix column count exceeds u32").unwrap_err())?;
            let storage = matrix_storage(representation, rows, columns)?;
            let element_type = if (rows == 0 || columns == 0)
                && matches!(element.as_ref(), SchemaBody::Tuple(elements) if elements.is_empty())
            {
                // Bytecode v1 reserves `Matrix<Any, 0, 0>` for an empty
                // value-matrix whose canonical element schema is unit. The
                // canonical schema sidecar remains authoritative; source
                // absence and option absence are never encoded here.
                RuntimeType::Any
            } else {
                runtime_type_for_schema(element)?
            };
            let mut bytes = rows.to_le_bytes().to_vec();
            bytes.extend_from_slice(&columns.to_le_bytes());
            encode_matrix_sequence(value.elements(), &element_type, &mut bytes)?;
            scalar(
                RuntimeType::Matrix {
                    element: Box::new(element_type),
                    storage,
                    rows,
                    cols: columns,
                },
                4,
                bytes,
            )
        }
        (SchemaBody::Option(inner), ValueData::Option(value)) => {
            let (runtime_type, bytes) = match value.as_deref() {
                Some(value) => {
                    let child = nested(inner, value)?;
                    let mut bytes = vec![1];
                    append_framed(&mut bytes, &child)?;
                    (RuntimeType::Option(Box::new(child.runtime_type)), bytes)
                }
                None => (
                    RuntimeType::Option(Box::new(runtime_type_for_schema(inner)?)),
                    vec![0],
                ),
            };
            scalar(runtime_type, 4, bytes)
        }
        (
            SchemaBody::Set {
                element,
                cardinality,
            },
            ValueData::Set(value),
        ) => {
            let mut children = value
                .elements()
                .iter()
                .map(|value| nested(element, value.data()))
                .collect::<MResult<Vec<_>>>()?;
            children.sort_by(|left, right| left.bytes.cmp(&right.bytes));
            let element_type = children
                .first()
                .map(|child| child.runtime_type.clone())
                .unwrap_or(runtime_type_for_schema(element)?);
            let runtime_type = RuntimeType::Set {
                element: Box::new(element_type),
                max_len: cardinality_upper_bound(cardinality)?,
            };
            framed_composite(runtime_type, children)
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            let mut entries = map
                .entries()
                .iter()
                .map(|entry| {
                    Ok((
                        nested(key, entry.key().data())?,
                        nested(value, entry.value())?,
                    ))
                })
                .collect::<MResult<Vec<_>>>()?;
            entries.sort_by(|left, right| left.0.bytes.cmp(&right.0.bytes));
            let key_type = entries
                .first()
                .map(|entry| entry.0.runtime_type.clone())
                .unwrap_or(runtime_type_for_schema(key)?);
            let value_type = entries
                .first()
                .map(|entry| entry.1.runtime_type.clone())
                .unwrap_or(runtime_type_for_schema(value)?);
            let mut bytes = checked_u32(entries.len(), "map entry count")?
                .to_le_bytes()
                .to_vec();
            for (key, value) in entries {
                append_framed(&mut bytes, &key)?;
                append_framed(&mut bytes, &value)?;
            }
            scalar(
                RuntimeType::Map {
                    key: Box::new(key_type),
                    value: Box::new(value_type),
                },
                4,
                bytes,
            )
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(table)) => {
            let rows = table.column(0).map(SequenceView::len).unwrap_or(0);
            let mut encoded_columns = Vec::with_capacity(columns.len());
            for (index, column) in columns.iter().enumerate() {
                let values = table.column(index).ok_or_else(|| {
                    invalid::<()>("canonical table column is missing").unwrap_err()
                })?;
                let encoded = sequence_data(values)
                    .into_iter()
                    .map(|value| nested(&column.schema, &value))
                    .collect::<MResult<Vec<_>>>()?;
                if encoded.len() != rows {
                    return invalid("canonical table columns have different row counts");
                }
                encoded_columns.push((column.name.clone(), encoded));
            }
            let runtime_columns = columns
                .iter()
                .zip(&encoded_columns)
                .map(|(column, (_, values))| {
                    Ok((
                        column.name.clone(),
                        values
                            .first()
                            .map(|value| value.runtime_type.clone())
                            .unwrap_or(runtime_type_for_schema(&column.schema)?),
                    ))
                })
                .collect::<MResult<Vec<_>>>()?;
            let mut bytes = checked_u32(rows, "table row count")?.to_le_bytes().to_vec();
            bytes.extend_from_slice(
                &checked_u32(columns.len(), "table column count")?.to_le_bytes(),
            );
            for row in 0..rows {
                for (_, column) in &encoded_columns {
                    append_framed(&mut bytes, &column[row])?;
                }
            }
            scalar(
                RuntimeType::Table {
                    columns: runtime_columns,
                    primary_key: 0,
                },
                4,
                bytes,
            )
        }
        (SchemaBody::Atom(_), ValueData::Atom) => {
            invalid("canonical Atom constants require the authoritative semantic nominal resolver")
        }
        (SchemaBody::Enum { .. }, ValueData::Enum(_)) => {
            invalid("canonical Enum constants require the authoritative complete enum schema")
        }
        (SchemaBody::ReifiedType, ValueData::Type(_)) => {
            invalid("canonical reified-type constant encoding is not yet available")
        }
        _ => invalid("canonical constant data does not match its schema"),
    }
}

#[cfg(feature = "semantic-compiler")]
fn runtime_type_for_schema(schema: &SchemaBody) -> MResult<RuntimeType> {
    Ok(match schema {
        SchemaBody::Dynamic => RuntimeType::Any,
        SchemaBody::Bool => RuntimeType::Bool,
        SchemaBody::UnsignedInteger(IntegerWidth::W8) => RuntimeType::U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W16) => RuntimeType::U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W32) => RuntimeType::U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W64) => RuntimeType::U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W128) => RuntimeType::U128,
        SchemaBody::SignedInteger(IntegerWidth::W8) => RuntimeType::I8,
        SchemaBody::SignedInteger(IntegerWidth::W16) => RuntimeType::I16,
        SchemaBody::SignedInteger(IntegerWidth::W32) => RuntimeType::I32,
        SchemaBody::SignedInteger(IntegerWidth::W64) => RuntimeType::I64,
        SchemaBody::SignedInteger(IntegerWidth::W128) => RuntimeType::I128,
        SchemaBody::FloatingPoint(FloatWidth::W32) => RuntimeType::F32,
        SchemaBody::FloatingPoint(FloatWidth::W64) => RuntimeType::F64,
        SchemaBody::Complex(FloatWidth::W64) => RuntimeType::C64,
        SchemaBody::Rational64 => RuntimeType::R64,
        SchemaBody::String => RuntimeType::String,
        SchemaBody::Id => RuntimeType::Id,
        SchemaBody::Index => RuntimeType::Index,
        SchemaBody::Option(inner) => RuntimeType::Option(Box::new(runtime_type_for_schema(inner)?)),
        SchemaBody::Tuple(elements) if elements.is_empty() => RuntimeType::Empty,
        SchemaBody::Tuple(elements) => RuntimeType::Tuple(
            elements
                .iter()
                .map(runtime_type_for_schema)
                .collect::<MResult<Vec<_>>>()?,
        ),
        SchemaBody::Record(fields) => RuntimeType::Record(
            fields
                .iter()
                .map(|field| Ok((field.name.clone(), runtime_type_for_schema(&field.schema)?)))
                .collect::<MResult<Vec<_>>>()?,
        ),
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let [
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ] = dimensions.as_ref()
            else {
                return invalid("canonical matrix type dimensions are unresolved");
            };
            RuntimeType::Matrix {
                element: Box::new(runtime_type_for_schema(element)?),
                storage: MatrixStorage::MatrixD,
                rows: u32::try_from(*rows)
                    .map_err(|_| invalid::<()>("matrix row count exceeds u32").unwrap_err())?,
                cols: u32::try_from(*columns)
                    .map_err(|_| invalid::<()>("matrix column count exceeds u32").unwrap_err())?,
            }
        }
        SchemaBody::Table { columns, .. } => RuntimeType::Table {
            columns: columns
                .iter()
                .map(|column| {
                    Ok((
                        column.name.clone(),
                        runtime_type_for_schema(&column.schema)?,
                    ))
                })
                .collect::<MResult<Vec<_>>>()?,
            primary_key: 0,
        },
        SchemaBody::Set {
            element,
            cardinality,
        } => RuntimeType::Set {
            element: Box::new(runtime_type_for_schema(element)?),
            max_len: cardinality_upper_bound(cardinality)?,
        },
        SchemaBody::Map { key, value, .. } => RuntimeType::Map {
            key: Box::new(runtime_type_for_schema(key)?),
            value: Box::new(runtime_type_for_schema(value)?),
        },
        SchemaBody::Atom(_) | SchemaBody::Enum { .. } => {
            return invalid("nominal bytecode types require an authoritative semantic resolver");
        }
        SchemaBody::ReifiedType => {
            return invalid("canonical reified-type bytecode encoding is not yet available");
        }
        SchemaBody::Complex(FloatWidth::W32) => {
            return invalid("bytecode v1 does not support Complex32 constants");
        }
    })
}

#[cfg(feature = "semantic-compiler")]
fn matrix_storage(
    representation: Option<FunctionValueRepresentation>,
    rows: u32,
    columns: u32,
) -> MResult<MatrixStorage> {
    let storage = match representation {
        Some(FunctionValueRepresentation::Matrix {
            storage: FunctionMatrixStoragePattern::Exact(storage),
            ..
        }) => match storage {
            FunctionMatrixRepresentation::Matrix1 => MatrixStorage::Matrix1,
            FunctionMatrixRepresentation::Matrix2 => MatrixStorage::Matrix2,
            FunctionMatrixRepresentation::Matrix3 => MatrixStorage::Matrix3,
            FunctionMatrixRepresentation::Matrix4 => MatrixStorage::Matrix4,
            FunctionMatrixRepresentation::Matrix2x3 => MatrixStorage::Matrix2x3,
            FunctionMatrixRepresentation::Matrix3x2 => MatrixStorage::Matrix3x2,
            FunctionMatrixRepresentation::RowVector2 => MatrixStorage::RowVector2,
            FunctionMatrixRepresentation::RowVector3 => MatrixStorage::RowVector3,
            FunctionMatrixRepresentation::RowVector4 => MatrixStorage::RowVector4,
            FunctionMatrixRepresentation::Vector2 => MatrixStorage::Vector2,
            FunctionMatrixRepresentation::Vector3 => MatrixStorage::Vector3,
            FunctionMatrixRepresentation::Vector4 => MatrixStorage::Vector4,
            FunctionMatrixRepresentation::RowVectorD => MatrixStorage::RowVectorD,
            FunctionMatrixRepresentation::VectorD => MatrixStorage::VectorD,
            FunctionMatrixRepresentation::MatrixD => MatrixStorage::MatrixD,
        },
        _ => MatrixStorage::MatrixD,
    };
    if !storage.validate_dimensions(rows, columns) {
        return invalid("canonical matrix storage disagrees with its dimensions");
    }
    Ok(storage)
}

#[cfg(feature = "semantic-compiler")]
fn cardinality_upper_bound(cardinality: &CardinalitySpec) -> MResult<Option<u32>> {
    let bound = match cardinality {
        CardinalitySpec::Exact(DimensionExpr::Constant(value))
        | CardinalitySpec::Dynamic {
            upper_bound: Some(DimensionExpr::Constant(value)),
        } => Some(*value),
        CardinalitySpec::Dynamic { upper_bound: None } => None,
        _ => return invalid("canonical collection extent is unresolved"),
    };
    bound
        .map(|value| {
            u32::try_from(value)
                .map_err(|_| invalid::<()>("collection extent exceeds u32").unwrap_err())
        })
        .transpose()
}

#[cfg(feature = "semantic-compiler")]
fn framed_composite(
    runtime_type: RuntimeType,
    children: Vec<EncodedConstant>,
) -> MResult<EncodedConstant> {
    let mut bytes = checked_u32(children.len(), "composite child count")?
        .to_le_bytes()
        .to_vec();
    for child in &children {
        append_framed(&mut bytes, child)?;
    }
    Ok(EncodedConstant {
        runtime_type,
        alignment: 4,
        bytes,
    })
}

#[cfg(feature = "semantic-compiler")]
fn append_framed(bytes: &mut Vec<u8>, child: &EncodedConstant) -> MResult<()> {
    bytes.extend_from_slice(&checked_u32(child.bytes.len(), "child payload length")?.to_le_bytes());
    bytes.extend_from_slice(&child.bytes);
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
fn checked_u32(value: usize, label: &'static str) -> MResult<u32> {
    u32::try_from(value).map_err(|_| invalid::<()>(format!("{label} exceeds u32")).unwrap_err())
}

#[cfg(feature = "semantic-compiler")]
fn sequence_data(sequence: SequenceView<'_>) -> Vec<ValueData> {
    macro_rules! values {
        ($values:expr, $variant:ident) => {
            $values.iter().cloned().map(ValueData::$variant).collect()
        };
    }
    match sequence {
        SequenceView::U8(values) => values!(values, U8),
        SequenceView::U16(values) => values!(values, U16),
        SequenceView::U32(values) => values!(values, U32),
        SequenceView::U64(values) => values!(values, U64),
        SequenceView::U128(values) => values!(values, U128),
        SequenceView::I8(values) => values!(values, I8),
        SequenceView::I16(values) => values!(values, I16),
        SequenceView::I32(values) => values!(values, I32),
        SequenceView::I64(values) => values!(values, I64),
        SequenceView::I128(values) => values!(values, I128),
        SequenceView::F32(values) => values!(values, F32),
        SequenceView::F64(values) => values!(values, F64),
        SequenceView::Complex32(values) => values!(values, Complex32),
        SequenceView::Complex64(values) => values!(values, Complex64),
        SequenceView::Rational64(values) => values!(values, Rational64),
        SequenceView::Bool(values) => values!(values, Bool),
        SequenceView::String(values) => values.iter().cloned().map(ValueData::String).collect(),
        SequenceView::Id(values) => values!(values, Id),
        SequenceView::Index(values) => values!(values, Index),
        SequenceView::Unit(count) => (0..count).map(|_| ValueData::Atom).collect(),
        SequenceView::Values(values) => values.to_vec(),
    }
}

#[cfg(feature = "semantic-compiler")]
fn encode_matrix_sequence(
    sequence: SequenceView<'_>,
    runtime_type: &RuntimeType,
    bytes: &mut Vec<u8>,
) -> MResult<()> {
    for value in sequence_data(sequence) {
        let encoded =
            encode_canonical_data(&runtime_schema_body(runtime_type)?, &value, None, 1, false)?;
        if encoded.runtime_type != *runtime_type {
            return invalid("matrix element encoding changed its declared runtime type");
        }
        if matches!(runtime_type, RuntimeType::String) {
            bytes.extend_from_slice(
                &checked_u32(encoded.bytes.len(), "matrix string length")?.to_le_bytes(),
            );
        }
        bytes.extend_from_slice(&encoded.bytes);
    }
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
fn matrix_element_alignment(element_type: &RuntimeType) -> u8 {
    match element_type {
        RuntimeType::Bool | RuntimeType::U8 | RuntimeType::I8 => 1,
        RuntimeType::U16 | RuntimeType::I16 => 2,
        RuntimeType::U32 | RuntimeType::I32 | RuntimeType::F32 | RuntimeType::String => 4,
        RuntimeType::U64
        | RuntimeType::I64
        | RuntimeType::F64
        | RuntimeType::C64
        | RuntimeType::R64
        | RuntimeType::Index => 8,
        RuntimeType::U128 | RuntimeType::I128 => 16,
        _ => 1,
    }
}

struct BytecodeNamedKinds(BTreeMap<KindId, CanonicalNominalPath>);

impl NamedKindPathResolver for BytecodeNamedKinds {
    fn canonical_path(&self, id: KindId) -> Option<&CanonicalNominalPath> {
        self.0.get(&id)
    }
}

impl ConstantCodecContext {
    fn decode_canonical_child(&mut self, ty: &RuntimeType, bytes: &[u8]) -> MResult<DecodedDraft> {
        if self.depth >= super::MAX_CONSTANT_NESTING {
            return Err(super::super::depth_exceeded(super::MAX_CONSTANT_NESTING));
        }
        self.depth += 1;
        let value = decode_value_payload(ty, bytes, self);
        self.depth -= 1;
        value
    }
}

pub(super) fn decode_constants(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<Value>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(entries.len())
        .map_err(|_| invalid::<()>("unable to allocate decoded constants").unwrap_err())?;
    for entry in entries {
        values.push(decode_constant(types, entry, blob)?);
    }
    Ok(values)
}

pub(super) fn decode_encoded_constants(
    constants: &[super::EncodedConstant],
) -> MResult<Vec<Value>> {
    constants
        .iter()
        .map(|constant| {
            let mut context = ConstantCodecContext::new();
            decode_value_payload(&constant.runtime_type, &constant.bytes, &mut context)?.finalize()
        })
        .collect()
}

pub(super) fn decode_constant_cells(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<Vec<ValueCell>> {
    let values = decode_constants(types, entries, blob)?;
    entries
        .iter()
        .zip(values)
        .map(|(entry, value)| {
            let ty = types
                .get(checked_usize(u64::from(entry.type_id), "constant type ID")?)
                .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
            cell_from_value(value, ty)
        })
        .collect()
}

pub(super) fn decode_encoded_constant_cells(
    constants: &[super::EncodedConstant],
) -> MResult<Vec<ValueCell>> {
    let values = decode_encoded_constants(constants)?;
    constants
        .iter()
        .zip(values)
        .map(|(constant, value)| cell_from_value(value, &constant.runtime_type))
        .collect()
}

fn cell_from_value(value: Value, ty: &RuntimeType) -> MResult<ValueCell> {
    #[cfg(not(feature = "matrix"))]
    {
        let _ = ty;
        return ValueCell::from_snapshot(value);
    }
    #[cfg(feature = "matrix")]
    {
        let RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } = ty
        else {
            return ValueCell::from_snapshot(value);
        };
        let ValueData::Matrix(matrix) = value.data() else {
            return ValueCell::from_snapshot(value);
        };
        let sequence = matrix.elements();
        macro_rules! matrix {
            ($type:ty) => {{
                let values = (0..sequence_len(sequence))
                    .map(|index| {
                        <$type as CanonicalMatrixElementBacking>::from_sequence(sequence, index)
                            .ok_or_else(|| {
                                invalid::<()>("canonical matrix element changed representation")
                                    .unwrap_err()
                            })
                    })
                    .collect::<MResult<Vec<_>>>()?;
                return exact_matrix_cell(value, *storage, *rows as usize, *cols as usize, values);
            }};
        }
        match element.as_ref() {
            #[cfg(feature = "bool")]
            RuntimeType::Bool => matrix!(bool),
            #[cfg(feature = "u8")]
            RuntimeType::U8 => matrix!(u8),
            #[cfg(feature = "u16")]
            RuntimeType::U16 => matrix!(u16),
            #[cfg(feature = "u32")]
            RuntimeType::U32 => matrix!(u32),
            #[cfg(feature = "u64")]
            RuntimeType::U64 => matrix!(u64),
            #[cfg(feature = "u128")]
            RuntimeType::U128 => matrix!(u128),
            #[cfg(feature = "i8")]
            RuntimeType::I8 => matrix!(i8),
            #[cfg(feature = "i16")]
            RuntimeType::I16 => matrix!(i16),
            #[cfg(feature = "i32")]
            RuntimeType::I32 => matrix!(i32),
            #[cfg(feature = "i64")]
            RuntimeType::I64 => matrix!(i64),
            #[cfg(feature = "i128")]
            RuntimeType::I128 => matrix!(i128),
            #[cfg(feature = "f32")]
            RuntimeType::F32 => matrix!(f32),
            #[cfg(feature = "f64")]
            RuntimeType::F64 => matrix!(f64),
            #[cfg(feature = "complex")]
            RuntimeType::C64 => matrix!(crate::C64),
            #[cfg(feature = "rational")]
            RuntimeType::R64 => matrix!(crate::R64),
            #[cfg(feature = "string")]
            RuntimeType::String => matrix!(String),
            RuntimeType::Index => matrix!(usize),
            _ => ValueCell::from_snapshot(value),
        }
    }
}

#[cfg(feature = "matrix")]
fn sequence_len(sequence: SequenceView<'_>) -> usize {
    match sequence {
        SequenceView::U8(values) => values.len(),
        SequenceView::U16(values) => values.len(),
        SequenceView::U32(values) => values.len(),
        SequenceView::U64(values) => values.len(),
        SequenceView::U128(values) => values.len(),
        SequenceView::I8(values) => values.len(),
        SequenceView::I16(values) => values.len(),
        SequenceView::I32(values) => values.len(),
        SequenceView::I64(values) => values.len(),
        SequenceView::I128(values) => values.len(),
        SequenceView::F32(values) => values.len(),
        SequenceView::F64(values) => values.len(),
        SequenceView::Complex32(values) => values.len(),
        SequenceView::Complex64(values) => values.len(),
        SequenceView::Rational64(values) => values.len(),
        SequenceView::Bool(values) => values.len(),
        SequenceView::String(values) => values.len(),
        SequenceView::Id(values) => values.len(),
        SequenceView::Index(values) => values.len(),
        SequenceView::Unit(count) => count as usize,
        SequenceView::Values(values) => values.len(),
    }
}

#[cfg(feature = "matrix")]
fn exact_matrix_cell<T>(
    value: Value,
    storage: super::super::MatrixStorage,
    rows: usize,
    columns: usize,
    values: Vec<T>,
) -> MResult<ValueCell>
where
    T: CanonicalMatrixElementBacking + na::Scalar,
{
    let schemas = value
        .schemas()
        .ok_or_else(|| invalid::<()>("canonical constant has no schema table").unwrap_err())?;
    let schemas = Rc::new((*schemas).clone());
    let schema = value.schema();
    let shape = value.shape().clone();
    macro_rules! fixed {
        ($feature:literal, $variant:ident, $constructor:expr) => {{
            #[cfg(feature = $feature)]
            {
                return ValueCell::from_ref(Ref::new($constructor), schema, shape, schemas);
            }
            #[cfg(not(feature = $feature))]
            {
                return invalid(concat!(
                    stringify!($variant),
                    " matrix storage is unavailable"
                ));
            }
        }};
    }
    match storage {
        super::super::MatrixStorage::Matrix1 => {
            fixed!("matrix1", Matrix1, na::Matrix1::from_row_slice(&values))
        }
        super::super::MatrixStorage::Matrix2 => {
            fixed!("matrix2", Matrix2, na::Matrix2::from_row_slice(&values))
        }
        super::super::MatrixStorage::Matrix3 => {
            fixed!("matrix3", Matrix3, na::Matrix3::from_row_slice(&values))
        }
        super::super::MatrixStorage::Matrix4 => {
            fixed!("matrix4", Matrix4, na::Matrix4::from_row_slice(&values))
        }
        super::super::MatrixStorage::Matrix2x3 => fixed!(
            "matrix2x3",
            Matrix2x3,
            na::Matrix2x3::from_row_slice(&values)
        ),
        super::super::MatrixStorage::Matrix3x2 => fixed!(
            "matrix3x2",
            Matrix3x2,
            na::Matrix3x2::from_row_slice(&values)
        ),
        super::super::MatrixStorage::RowVector2 => fixed!(
            "row_vector2",
            RowVector2,
            na::RowVector2::from_row_slice(&values)
        ),
        super::super::MatrixStorage::RowVector3 => fixed!(
            "row_vector3",
            RowVector3,
            na::RowVector3::from_row_slice(&values)
        ),
        super::super::MatrixStorage::RowVector4 => fixed!(
            "row_vector4",
            RowVector4,
            na::RowVector4::from_row_slice(&values)
        ),
        super::super::MatrixStorage::Vector2 => {
            fixed!("vector2", Vector2, na::Vector2::from_column_slice(&values))
        }
        super::super::MatrixStorage::Vector3 => {
            fixed!("vector3", Vector3, na::Vector3::from_column_slice(&values))
        }
        super::super::MatrixStorage::Vector4 => {
            fixed!("vector4", Vector4, na::Vector4::from_column_slice(&values))
        }
        super::super::MatrixStorage::RowVectorD => fixed!(
            "row_vectord",
            RowVectorD,
            na::RowDVector::from_row_slice(&values)
        ),
        super::super::MatrixStorage::VectorD => {
            fixed!("vectord", VectorD, na::DVector::from_column_slice(&values))
        }
        super::super::MatrixStorage::MatrixD => {
            #[cfg(feature = "matrixd")]
            {
                return ValueCell::from_ref(
                    Ref::new(na::DMatrix::from_row_slice(rows, columns, &values)),
                    schema,
                    shape,
                    schemas,
                );
            }
            #[cfg(not(feature = "matrixd"))]
            {
                invalid("MatrixD storage is unavailable")
            }
        }
    }
}

fn decode_constant(types: &[RuntimeType], entry: &ConstantEntry, blob: &[u8]) -> MResult<Value> {
    if entry.encoding != 1 {
        return invalid("unsupported bytecode constant encoding");
    }
    if entry.flags != 0 {
        return invalid("constant entry flags must be zero");
    }
    if !matches!(entry.alignment, 1 | 2 | 4 | 8 | 16) {
        return invalid("invalid constant alignment");
    }
    if entry.offset % u64::from(entry.alignment) != 0 {
        return invalid("misaligned constant entry");
    }
    let start = checked_usize(entry.offset, "constant offset")?;
    let length = checked_usize(entry.length, "constant length")?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid::<()>("constant range overflow").unwrap_err())?;
    let bytes = blob
        .get(start..end)
        .ok_or_else(|| invalid::<()>("constant entry is outside ConstantBlob").unwrap_err())?;
    let ty = types
        .get(checked_usize(u64::from(entry.type_id), "constant type ID")?)
        .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
    let mut context = ConstantCodecContext::new();
    decode_value_payload(ty, bytes, &mut context)?.finalize()
}

fn decode_value_payload(
    ty: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    if let Some(value) = decode_scalar(ty, bytes) {
        return value;
    }
    match ty {
        RuntimeType::Matrix {
            element,
            storage,
            rows,
            cols,
        } => decode_matrix(element, *storage, *rows, *cols, bytes),
        RuntimeType::Tuple(types) => decode_tuple(types, bytes, context),
        RuntimeType::Record(fields) => decode_record(fields, bytes, context),
        RuntimeType::Map { key, value } => decode_map(key, value, bytes, context),
        RuntimeType::Set { element, max_len } => decode_set(element, *max_len, bytes, context),
        RuntimeType::Table {
            columns,
            primary_key,
        } => decode_table(columns, *primary_key, bytes, context),
        RuntimeType::Reference(child) => {
            let mut reader = ByteReader::new(bytes);
            let value = context.decode_canonical_child(
                child,
                read_child_payload(&mut reader, "reference child")?,
            )?;
            ensure_empty(&reader, "reference constant")?;
            Ok(value)
        }
        RuntimeType::Option(inner) => decode_option(inner, bytes, context),
        RuntimeType::Atom { id, name } => decode_atom(*id, name, bytes),
        RuntimeType::Enum { id, name } => decode_enum(*id, name, bytes, context),
        RuntimeType::Kind(kind) => {
            if !bytes.is_empty() {
                return invalid("Kind constants must have zero payload bytes");
            }
            let mut named = BTreeMap::new();
            let kind = canonical_kind(kind, &mut named)?;
            Ok(DecodedDraft {
                body: SchemaBody::ReifiedType,
                data: ValueDataDraft::Type(ReifiedTypeDraft::Kind {
                    kind,
                    dimension_parameters: Vec::new().into_boxed_slice(),
                }),
                named_kinds: named,
            })
        }
        RuntimeType::Any | RuntimeType::None => {
            if !bytes.is_empty() {
                return invalid("Any and None constants must have zero payload bytes");
            }
            let kind = if matches!(ty, RuntimeType::Any) {
                KindExpr::Wildcard
            } else {
                KindExpr::Never
            };
            Ok(DecodedDraft::new(
                SchemaBody::ReifiedType,
                ValueDataDraft::Type(ReifiedTypeDraft::Kind {
                    kind,
                    dimension_parameters: Vec::new().into_boxed_slice(),
                }),
            ))
        }
        _ => unreachable!("scalar runtime types are handled above"),
    }
}

pub(super) fn validate_payload(ty: &RuntimeType, bytes: &[u8]) -> MResult<()> {
    let mut context = ConstantCodecContext::new();
    decode_value_payload(ty, bytes, &mut context)?.finalize()?;
    Ok(())
}

fn decode_scalar(ty: &RuntimeType, bytes: &[u8]) -> Option<MResult<DecodedDraft>> {
    if !matches!(
        ty,
        RuntimeType::Empty
            | RuntimeType::Bool
            | RuntimeType::String
            | RuntimeType::U8
            | RuntimeType::U16
            | RuntimeType::U32
            | RuntimeType::U64
            | RuntimeType::U128
            | RuntimeType::I8
            | RuntimeType::I16
            | RuntimeType::I32
            | RuntimeType::I64
            | RuntimeType::I128
            | RuntimeType::F32
            | RuntimeType::F64
            | RuntimeType::Id
            | RuntimeType::Index
            | RuntimeType::C64
            | RuntimeType::R64
    ) {
        return None;
    }
    macro_rules! fixed {
        ($body:expr, $variant:ident, $type:ty, $width:literal, $label:literal) => {{
            let raw: [u8; $width] = bytes
                .try_into()
                .map_err(|_| invalid::<()>($label).unwrap_err())?;
            Ok(DecodedDraft::new(
                $body,
                ValueDataDraft::$variant(<$type>::from_le_bytes(raw)),
            ))
        }};
    }
    Some((|| match ty {
        RuntimeType::Empty => {
            if !bytes.is_empty() {
                return invalid("Empty constant must have zero payload bytes");
            }
            Ok(DecodedDraft::new(
                SchemaBody::Tuple(Vec::new().into_boxed_slice()),
                ValueDataDraft::Tuple(Vec::new().into_boxed_slice()),
            ))
        }
        RuntimeType::Bool => match bytes {
            [0] => Ok(DecodedDraft::new(
                SchemaBody::Bool,
                ValueDataDraft::Bool(false),
            )),
            [1] => Ok(DecodedDraft::new(
                SchemaBody::Bool,
                ValueDataDraft::Bool(true),
            )),
            _ => invalid("Bool constant must be exactly 0x00 or 0x01"),
        },
        RuntimeType::String => Ok(DecodedDraft::new(
            SchemaBody::String,
            ValueDataDraft::String(
                core::str::from_utf8(bytes)
                    .map_err(|_| invalid::<()>("invalid UTF-8 String constant").unwrap_err())?
                    .to_owned(),
            ),
        )),
        RuntimeType::U8 => fixed!(
            SchemaBody::UnsignedInteger(IntegerWidth::W8),
            U8,
            u8,
            1,
            "U8 constant has an invalid byte length"
        ),
        RuntimeType::U16 => fixed!(
            SchemaBody::UnsignedInteger(IntegerWidth::W16),
            U16,
            u16,
            2,
            "U16 constant has an invalid byte length"
        ),
        RuntimeType::U32 => fixed!(
            SchemaBody::UnsignedInteger(IntegerWidth::W32),
            U32,
            u32,
            4,
            "U32 constant has an invalid byte length"
        ),
        RuntimeType::U64 => fixed!(
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            U64,
            u64,
            8,
            "U64 constant has an invalid byte length"
        ),
        RuntimeType::U128 => fixed!(
            SchemaBody::UnsignedInteger(IntegerWidth::W128),
            U128,
            u128,
            16,
            "U128 constant has an invalid byte length"
        ),
        RuntimeType::I8 => fixed!(
            SchemaBody::SignedInteger(IntegerWidth::W8),
            I8,
            i8,
            1,
            "I8 constant has an invalid byte length"
        ),
        RuntimeType::I16 => fixed!(
            SchemaBody::SignedInteger(IntegerWidth::W16),
            I16,
            i16,
            2,
            "I16 constant has an invalid byte length"
        ),
        RuntimeType::I32 => fixed!(
            SchemaBody::SignedInteger(IntegerWidth::W32),
            I32,
            i32,
            4,
            "I32 constant has an invalid byte length"
        ),
        RuntimeType::I64 => fixed!(
            SchemaBody::SignedInteger(IntegerWidth::W64),
            I64,
            i64,
            8,
            "I64 constant has an invalid byte length"
        ),
        RuntimeType::I128 => fixed!(
            SchemaBody::SignedInteger(IntegerWidth::W128),
            I128,
            i128,
            16,
            "I128 constant has an invalid byte length"
        ),
        RuntimeType::F32 => {
            let raw: [u8; 4] = bytes.try_into().map_err(|_| {
                invalid::<()>("F32 constant has an invalid byte length").unwrap_err()
            })?;
            Ok(DecodedDraft::new(
                SchemaBody::FloatingPoint(FloatWidth::W32),
                ValueDataDraft::F32(F32Bits::from_bits(u32::from_le_bytes(raw))),
            ))
        }
        RuntimeType::F64 => {
            let raw: [u8; 8] = bytes.try_into().map_err(|_| {
                invalid::<()>("F64 constant has an invalid byte length").unwrap_err()
            })?;
            Ok(DecodedDraft::new(
                SchemaBody::FloatingPoint(FloatWidth::W64),
                ValueDataDraft::F64(F64Bits::from_bits(u64::from_le_bytes(raw))),
            ))
        }
        RuntimeType::Id => fixed!(
            SchemaBody::Id,
            Id,
            u64,
            8,
            "Id constant must contain eight bytes"
        ),
        RuntimeType::Index => fixed!(
            SchemaBody::Index,
            Index,
            u64,
            8,
            "Index constant must contain eight bytes"
        ),
        RuntimeType::C64 => {
            let raw: [u8; 16] = bytes.try_into().map_err(|_| {
                invalid::<()>("C64 constant must contain sixteen bytes").unwrap_err()
            })?;
            Ok(DecodedDraft::new(
                SchemaBody::Complex(FloatWidth::W64),
                ValueDataDraft::Complex64(Complex64Bits::new(
                    F64Bits::from_bits(u64::from_le_bytes(raw[..8].try_into().unwrap())),
                    F64Bits::from_bits(u64::from_le_bytes(raw[8..].try_into().unwrap())),
                )),
            ))
        }
        RuntimeType::R64 => {
            let raw: [u8; 16] = bytes.try_into().map_err(|_| {
                invalid::<()>("R64 constant must contain sixteen bytes").unwrap_err()
            })?;
            let numerator = i64::from_le_bytes(raw[..8].try_into().unwrap());
            let denominator = i64::from_le_bytes(raw[8..].try_into().unwrap());
            if denominator <= 0 {
                return invalid("R64 constant denominator must be positive and nonzero");
            }
            if gcd_i64(numerator, denominator) != 1 {
                return invalid("R64 constant is not reduced");
            }
            Ok(DecodedDraft::new(
                SchemaBody::Rational64,
                ValueDataDraft::Rational64 {
                    numerator,
                    denominator: denominator as u64,
                },
            ))
        }
        RuntimeType::Matrix { .. }
        | RuntimeType::Tuple(_)
        | RuntimeType::Record(_)
        | RuntimeType::Map { .. }
        | RuntimeType::Set { .. }
        | RuntimeType::Table { .. }
        | RuntimeType::Reference(_)
        | RuntimeType::Option(_)
        | RuntimeType::Atom { .. }
        | RuntimeType::Enum { .. }
        | RuntimeType::Kind(_)
        | RuntimeType::Any
        | RuntimeType::None => unreachable!("non-scalar runtime type passed to scalar decoder"),
    })())
}

fn decode_tuple(
    types: &[RuntimeType],
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("tuple element count")?),
        "tuple element count",
    )?;
    if count != types.len() {
        return invalid("tuple element count does not match RuntimeType");
    }
    let mut bodies = Vec::with_capacity(count);
    let mut values = Vec::with_capacity(count);
    let mut named = BTreeMap::new();
    for ty in types {
        let child = context
            .decode_canonical_child(ty, read_child_payload(&mut reader, "tuple element")?)?;
        bodies.push(child.body);
        values.push(child.data);
        named.extend(child.named_kinds);
    }
    ensure_empty(&reader, "tuple constant")?;
    Ok(DecodedDraft {
        body: SchemaBody::Tuple(bodies.into_boxed_slice()),
        data: ValueDataDraft::Tuple(values.into_boxed_slice()),
        named_kinds: named,
    })
}

fn decode_record(
    fields: &[(String, RuntimeType)],
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("record field count")?),
        "record field count",
    )?;
    if count != fields.len() {
        return invalid("record field count does not match RuntimeType");
    }
    let mut schemas = Vec::with_capacity(count);
    let mut values = Vec::with_capacity(count);
    let mut named = BTreeMap::new();
    for (name, ty) in fields {
        if name.is_empty() {
            return invalid("record field name must not be empty");
        }
        let child =
            context.decode_canonical_child(ty, read_child_payload(&mut reader, "record field")?)?;
        schemas.push(SchemaField {
            name: name.clone(),
            schema: child.body,
        });
        values.push(NamedValueDraft {
            name: name.clone(),
            value: child.data,
        });
        named.extend(child.named_kinds);
    }
    ensure_empty(&reader, "record constant")?;
    Ok(DecodedDraft {
        body: SchemaBody::Record(schemas.into_boxed_slice()),
        data: ValueDataDraft::Record(values.into_boxed_slice()),
        named_kinds: named,
    })
}

fn decode_map(
    key_type: &RuntimeType,
    value_type: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("map entry count")?),
        "map entry count",
    )?;
    let mut previous = None::<Vec<u8>>;
    let mut entries = Vec::with_capacity(count);
    let mut key_body = None;
    let mut value_body = None;
    let mut named = BTreeMap::new();
    for _ in 0..count {
        let key_payload = read_child_payload(&mut reader, "map key")?;
        let value_payload = read_child_payload(&mut reader, "map value")?;
        if previous.as_deref() >= Some(key_payload) {
            return invalid("map keys are not in strict canonical payload order");
        }
        previous = Some(key_payload.to_vec());
        let key = context.decode_canonical_child(key_type, key_payload)?;
        let value = context.decode_canonical_child(value_type, value_payload)?;
        key_body.get_or_insert_with(|| key.body.clone());
        value_body.get_or_insert_with(|| value.body.clone());
        named.extend(key.named_kinds);
        named.extend(value.named_kinds);
        entries.push(MapEntryDraft {
            items: vec![key.data, value.data].into_boxed_slice(),
        });
    }
    ensure_empty(&reader, "map constant")?;
    let key_body = key_body.unwrap_or(runtime_schema_body(key_type)?);
    let value_body = value_body.unwrap_or(runtime_schema_body(value_type)?);
    Ok(DecodedDraft {
        body: SchemaBody::Map {
            key: Box::new(key_body),
            value: Box::new(value_body),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(count as u64)),
        },
        data: ValueDataDraft::Map(entries.into_boxed_slice()),
        named_kinds: named,
    })
}

fn decode_set(
    element_type: &RuntimeType,
    max_len: Option<u32>,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    let mut reader = ByteReader::new(bytes);
    let count = checked_usize(
        u64::from(reader.read_u32("set element count")?),
        "set element count",
    )?;
    if max_len.is_some_and(|limit| count > limit as usize) {
        return invalid("set element count exceeds its RuntimeType limit");
    }
    let mut previous = None::<Vec<u8>>;
    let mut values = Vec::with_capacity(count);
    let mut element_body = None;
    let mut named = BTreeMap::new();
    for _ in 0..count {
        let payload = read_child_payload(&mut reader, "set element")?;
        if previous.as_deref() >= Some(payload) {
            return invalid("set elements are not in strict canonical payload order");
        }
        previous = Some(payload.to_vec());
        let child = context.decode_canonical_child(element_type, payload)?;
        element_body.get_or_insert_with(|| child.body.clone());
        values.push(child.data);
        named.extend(child.named_kinds);
    }
    ensure_empty(&reader, "set constant")?;
    Ok(DecodedDraft {
        body: SchemaBody::Set {
            element: Box::new(element_body.unwrap_or(runtime_schema_body(element_type)?)),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(count as u64)),
        },
        data: ValueDataDraft::Set(values.into_boxed_slice()),
        named_kinds: named,
    })
}

fn decode_table(
    columns: &[(String, RuntimeType)],
    primary_key: u32,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    let mut reader = ByteReader::new(bytes);
    let rows = checked_usize(
        u64::from(reader.read_u32("table row count")?),
        "table row count",
    )?;
    let count = checked_usize(
        u64::from(reader.read_u32("table column count")?),
        "table column count",
    )?;
    if count != columns.len() || primary_key != 0 {
        return invalid("table schema is unsupported or does not match RuntimeType");
    }
    validate_table_payload_shape(rows, count, reader.remaining())?;
    let mut data = (0..count)
        .map(|_| Vec::with_capacity(rows))
        .collect::<Vec<_>>();
    let mut bodies = vec![None; count];
    let mut named = BTreeMap::new();
    for _ in 0..rows {
        for (index, (_, ty)) in columns.iter().enumerate() {
            let child = context
                .decode_canonical_child(ty, read_child_payload(&mut reader, "table cell")?)?;
            bodies[index].get_or_insert_with(|| child.body.clone());
            data[index].push(child.data);
            named.extend(child.named_kinds);
        }
    }
    ensure_empty(&reader, "table constant")?;
    let schemas = columns
        .iter()
        .enumerate()
        .map(|(index, (name, ty))| {
            Ok(SchemaField {
                name: name.clone(),
                schema: bodies[index].clone().unwrap_or(runtime_schema_body(ty)?),
            })
        })
        .collect::<MResult<Vec<_>>>()?;
    let values = columns
        .iter()
        .zip(data)
        .map(|((name, _), values)| TableColumnDraft {
            name: name.clone(),
            values: values.into_boxed_slice(),
        })
        .collect::<Vec<_>>();
    Ok(DecodedDraft {
        body: SchemaBody::Table {
            columns: schemas.into_boxed_slice(),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(rows as u64)),
        },
        data: ValueDataDraft::Table(values.into_boxed_slice()),
        named_kinds: named,
    })
}

fn decode_option(
    inner: &RuntimeType,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    let mut reader = ByteReader::new(bytes);
    let (body, data, named) = match reader.read_u8("option presence")? {
        0 => (runtime_schema_body(inner)?, None, BTreeMap::new()),
        1 => {
            let child = context
                .decode_canonical_child(inner, read_child_payload(&mut reader, "option child")?)?;
            (child.body, Some(Box::new(child.data)), child.named_kinds)
        }
        _ => return invalid("option presence must be exactly 0x00 or 0x01"),
    };
    ensure_empty(&reader, "option constant")?;
    Ok(DecodedDraft {
        body: SchemaBody::Option(Box::new(body)),
        data: ValueDataDraft::Option(OptionDraft {
            present: data.is_some(),
            value: data,
        }),
        named_kinds: named,
    })
}

fn decode_atom(id: u64, name: &str, bytes: &[u8]) -> MResult<DecodedDraft> {
    if !bytes.is_empty() {
        return invalid("Atom constants must have zero payload bytes");
    }
    validate_named_id("Atom", id, name)?;
    invalid("canonical Atom constants require an authoritative semantic nominal resolver")
}

fn decode_enum(
    id: u64,
    name: &str,
    bytes: &[u8],
    context: &mut ConstantCodecContext,
) -> MResult<DecodedDraft> {
    validate_named_id("Enum", id, name)?;
    let _ = (bytes, context);
    invalid(
        "canonical Enum constants require the authoritative complete enum schema; a selected variant is insufficient",
    )
}

fn decode_matrix(
    element: &RuntimeType,
    storage: super::super::MatrixStorage,
    rows: u32,
    cols: u32,
    bytes: &[u8],
) -> MResult<DecodedDraft> {
    validate_matrix_payload_feasibility(element, rows, cols, bytes)?;
    let mut reader = ByteReader::new(bytes);
    if (
        reader.read_u32("matrix constant rows")?,
        reader.read_u32("matrix constant columns")?,
    ) != (rows, cols)
        || !storage.validate_dimensions(rows, cols)
    {
        return invalid("matrix constant shape disagrees with RuntimeType");
    }
    let (_, _, count) = matrix::element_count(rows, cols)?;
    if matches!(element, RuntimeType::Any) && count != 0 {
        return invalid(
            "Any-element matrix constants are reserved for empty value matrices in bytecode v1",
        );
    }
    let body = if matches!(element, RuntimeType::Any) {
        SchemaBody::Tuple(Vec::new().into_boxed_slice())
    } else {
        runtime_schema_body(element)?
    };
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decode_matrix_element(element, &mut reader)?);
    }
    ensure_empty(&reader, "matrix constant")?;
    Ok(DecodedDraft::new(
        SchemaBody::Matrix {
            element: Box::new(body),
            dimensions: vec![
                DimensionExpr::Constant(u64::from(rows)),
                DimensionExpr::Constant(u64::from(cols)),
            ]
            .into_boxed_slice(),
        },
        ValueDataDraft::Matrix(values.into_boxed_slice()),
    ))
}

fn decode_matrix_element(ty: &RuntimeType, reader: &mut ByteReader<'_>) -> MResult<ValueDataDraft> {
    macro_rules! fixed {
        ($variant:ident, $type:ty, $width:literal, $label:literal) => {{
            let raw: [u8; $width] = reader.read_exact($width, $label)?.try_into().unwrap();
            ValueDataDraft::$variant(<$type>::from_le_bytes(raw))
        }};
    }
    Ok(match ty {
        RuntimeType::Bool => match reader.read_u8("Bool matrix element")? {
            0 => ValueDataDraft::Bool(false),
            1 => ValueDataDraft::Bool(true),
            _ => return invalid("Bool matrix elements must be exactly 0x00 or 0x01"),
        },
        RuntimeType::U8 => fixed!(U8, u8, 1, "U8 matrix element"),
        RuntimeType::U16 => fixed!(U16, u16, 2, "U16 matrix element"),
        RuntimeType::U32 => fixed!(U32, u32, 4, "U32 matrix element"),
        RuntimeType::U64 => fixed!(U64, u64, 8, "U64 matrix element"),
        RuntimeType::U128 => fixed!(U128, u128, 16, "U128 matrix element"),
        RuntimeType::I8 => fixed!(I8, i8, 1, "I8 matrix element"),
        RuntimeType::I16 => fixed!(I16, i16, 2, "I16 matrix element"),
        RuntimeType::I32 => fixed!(I32, i32, 4, "I32 matrix element"),
        RuntimeType::I64 => fixed!(I64, i64, 8, "I64 matrix element"),
        RuntimeType::I128 => fixed!(I128, i128, 16, "I128 matrix element"),
        RuntimeType::F32 => {
            ValueDataDraft::F32(F32Bits::from_bits(reader.read_u32("F32 matrix element")?))
        }
        RuntimeType::F64 => {
            ValueDataDraft::F64(F64Bits::from_bits(reader.read_u64("F64 matrix element")?))
        }
        RuntimeType::String => {
            let length = checked_usize(
                u64::from(reader.read_u32("String matrix element length")?),
                "String matrix element length",
            )?;
            ValueDataDraft::String(reader.read_utf8(length, "String matrix element")?)
        }
        RuntimeType::C64 => {
            let raw = reader.read_exact(16, "C64 matrix element")?;
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_bits(u64::from_le_bytes(raw[..8].try_into().unwrap())),
                F64Bits::from_bits(u64::from_le_bytes(raw[8..].try_into().unwrap())),
            ))
        }
        RuntimeType::R64 => {
            let raw = reader.read_exact(16, "R64 matrix element")?;
            let numerator = i64::from_le_bytes(raw[..8].try_into().unwrap());
            let denominator = i64::from_le_bytes(raw[8..].try_into().unwrap());
            if denominator <= 0 {
                return invalid("R64 matrix element denominator must be positive and nonzero");
            }
            ValueDataDraft::Rational64 {
                numerator,
                denominator: denominator as u64,
            }
        }
        RuntimeType::Index => ValueDataDraft::Index(reader.read_u64("Index matrix element")?),
        RuntimeType::Any => ValueDataDraft::Tuple(Vec::new().into_boxed_slice()),
        _ => {
            return invalid(format!(
                "matrix constants do not support element type {ty:?} in this runtime"
            ));
        }
    })
}

pub(crate) fn runtime_schema_body(ty: &RuntimeType) -> MResult<SchemaBody> {
    Ok(match ty {
        RuntimeType::Empty => SchemaBody::Tuple(Vec::new().into_boxed_slice()),
        RuntimeType::Bool => SchemaBody::Bool,
        RuntimeType::String => SchemaBody::String,
        RuntimeType::U8 => SchemaBody::UnsignedInteger(IntegerWidth::W8),
        RuntimeType::U16 => SchemaBody::UnsignedInteger(IntegerWidth::W16),
        RuntimeType::U32 => SchemaBody::UnsignedInteger(IntegerWidth::W32),
        RuntimeType::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
        RuntimeType::U128 => SchemaBody::UnsignedInteger(IntegerWidth::W128),
        RuntimeType::I8 => SchemaBody::SignedInteger(IntegerWidth::W8),
        RuntimeType::I16 => SchemaBody::SignedInteger(IntegerWidth::W16),
        RuntimeType::I32 => SchemaBody::SignedInteger(IntegerWidth::W32),
        RuntimeType::I64 => SchemaBody::SignedInteger(IntegerWidth::W64),
        RuntimeType::I128 => SchemaBody::SignedInteger(IntegerWidth::W128),
        RuntimeType::F32 => SchemaBody::FloatingPoint(FloatWidth::W32),
        RuntimeType::F64 => SchemaBody::FloatingPoint(FloatWidth::W64),
        RuntimeType::C64 => SchemaBody::Complex(FloatWidth::W64),
        RuntimeType::R64 => SchemaBody::Rational64,
        RuntimeType::Id => SchemaBody::Id,
        RuntimeType::Index => SchemaBody::Index,
        RuntimeType::Matrix {
            element,
            rows,
            cols,
            ..
        } => SchemaBody::Matrix {
            element: Box::new(runtime_schema_body(element)?),
            dimensions: vec![
                DimensionExpr::Constant(u64::from(*rows)),
                DimensionExpr::Constant(u64::from(*cols)),
            ]
            .into_boxed_slice(),
        },
        RuntimeType::Tuple(types) => SchemaBody::Tuple(
            types
                .iter()
                .map(runtime_schema_body)
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        RuntimeType::Record(fields) => SchemaBody::Record(
            fields
                .iter()
                .map(|(name, ty)| {
                    Ok(SchemaField {
                        name: name.clone(),
                        schema: runtime_schema_body(ty)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        RuntimeType::Option(inner) => SchemaBody::Option(Box::new(runtime_schema_body(inner)?)),
        RuntimeType::Reference(inner) => runtime_schema_body(inner)?,
        RuntimeType::Atom { .. } => {
            return invalid(
                "canonical Atom types require an authoritative semantic nominal resolver",
            );
        }
        RuntimeType::Map { key, value } => SchemaBody::Map {
            key: Box::new(runtime_schema_body(key)?),
            value: Box::new(runtime_schema_body(value)?),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(0)),
        },
        RuntimeType::Set { element, max_len } => SchemaBody::Set {
            element: Box::new(runtime_schema_body(element)?),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(u64::from(
                max_len.unwrap_or(0),
            ))),
        },
        RuntimeType::Table { columns, .. } => SchemaBody::Table {
            columns: columns
                .iter()
                .map(|(name, ty)| {
                    Ok(SchemaField {
                        name: name.clone(),
                        schema: runtime_schema_body(ty)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(0)),
        },
        RuntimeType::Enum { .. } => {
            return invalid("canonical Enum types require the authoritative complete enum schema");
        }
        RuntimeType::Kind(_) | RuntimeType::Any | RuntimeType::None => SchemaBody::ReifiedType,
    })
}

fn canonical_kind(
    kind: &crate::BytecodeKind,
    named: &mut BTreeMap<KindId, CanonicalNominalPath>,
) -> MResult<KindExpr> {
    use crate::BytecodeKind;
    Ok(match kind {
        BytecodeKind::Any => KindExpr::Wildcard,
        BytecodeKind::None => KindExpr::Never,
        BytecodeKind::Empty => {
            return invalid("unresolved Empty kind cannot be a canonical type constant");
        }
        BytecodeKind::Scalar(id) => {
            let (id, path) = crate::builtin_scalar_named_kind(*id).map_err(|_| {
                invalid::<()>("Kind scalar ID does not identify a canonical runtime scalar")
                    .unwrap_err()
            })?;
            named.insert(id, path);
            KindExpr::Named(id)
        }
        BytecodeKind::Id => KindExpr::Id,
        BytecodeKind::Index => KindExpr::Index,
        BytecodeKind::Atom(_, _) => {
            return invalid(
                "canonical Atom kinds require an authoritative semantic nominal resolver",
            );
        }
        BytecodeKind::Enum(_, _) => {
            return invalid(
                "canonical Enum kinds require an authoritative semantic nominal resolver",
            );
        }
        BytecodeKind::Matrix(element, dimensions) => KindExpr::Matrix {
            element: Box::new(canonical_kind(element, named)?),
            dimensions: dimensions
                .iter()
                .map(|value| DimensionExpr::Constant(*value as u64))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        },
        BytecodeKind::Option(inner) => KindExpr::Option(Box::new(canonical_kind(inner, named)?)),
        BytecodeKind::Tuple(types) => KindExpr::Tuple(
            types
                .iter()
                .map(|ty| canonical_kind(ty, named))
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        BytecodeKind::Record(fields) => KindExpr::Record(
            fields
                .iter()
                .map(|(name, ty)| {
                    Ok(KindField {
                        name: name.clone(),
                        kind: canonical_kind(ty, named)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        BytecodeKind::Table(columns, rows) => KindExpr::Table {
            columns: columns
                .iter()
                .map(|(name, ty)| {
                    Ok(KindField {
                        name: name.clone(),
                        kind: canonical_kind(ty, named)?,
                    })
                })
                .collect::<MResult<Vec<_>>>()?
                .into_boxed_slice(),
            rows: DimensionExpr::Constant(*rows as u64),
        },
        BytecodeKind::Set(element, size) => KindExpr::Set {
            element: Box::new(canonical_kind(element, named)?),
            cardinality: DimensionExpr::Constant(size.unwrap_or(0) as u64),
        },
        BytecodeKind::Map(key, value) => KindExpr::Map {
            key: Box::new(canonical_kind(key, named)?),
            value: Box::new(canonical_kind(value, named)?),
            cardinality: DimensionExpr::Constant(0),
        },
        BytecodeKind::Reference(inner) => {
            KindExpr::Reference(Box::new(canonical_kind(inner, named)?))
        }
        BytecodeKind::Kind(inner) => KindExpr::TypeOf(Box::new(canonical_kind(inner, named)?)),
    })
}

fn gcd_i64(left: i64, right: i64) -> i64 {
    let mut left = left.unsigned_abs();
    let mut right = right.unsigned_abs();
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left as i64
}

fn validate_named_id(label: &str, id: u64, name: &str) -> MResult<()> {
    if name.is_empty() || crate::hash_str(name) != id {
        return invalid(format!("{label} name does not match its stable ID"));
    }
    Ok(())
}

fn ensure_empty(reader: &ByteReader<'_>, label: &str) -> MResult<()> {
    if !reader.is_empty() {
        return invalid(format!("{label} has trailing bytes"));
    }
    Ok(())
}
