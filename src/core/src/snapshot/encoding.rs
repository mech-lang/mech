use super::sequence::{SequenceStorage, SequenceView};
use super::{KeyHash, ReifiedType, SnapshotValueError, Value, ValueData, ValueHash};
use crate::{FloatWidth, IntegerWidth, SchemaBody, SchemaTable};
use sha2::{Digest, Sha256};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

pub(super) trait SnapshotByteSink {
    fn write(&mut self, bytes: &[u8]);
}

struct VecSnapshotSink {
    bytes: Vec<u8>,
}

impl VecSnapshotSink {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Box<[u8]> {
        self.bytes.into_boxed_slice()
    }
}

impl SnapshotByteSink for VecSnapshotSink {
    fn write(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }
}

struct LengthSnapshotSink {
    len: usize,
}

impl SnapshotByteSink for LengthSnapshotSink {
    fn write(&mut self, bytes: &[u8]) {
        self.len = self.len.saturating_add(bytes.len());
    }
}

struct Sha256SnapshotSink {
    hash: Sha256,
}

/// Conservative, schema-directed accounting for one immutable canonical
/// value. `encoded_bytes` measures the canonical payload, while
/// `retained_bytes` includes owned in-memory containers and payloads. The
/// latter is deliberately conservative: inline root storage may be counted
/// alongside its enclosing value so admission never mistakes serialized size
/// for peak retained memory.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValueFootprint {
    pub encoded_bytes: u64,
    pub retained_bytes: u64,
    pub node_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueFootprintError {
    InvalidValue(SnapshotValueError),
    IndexOutOfRange,
    ArithmeticOverflow,
}

/// One incrementally chargeable piece of a canonical value traversal.
///
/// Aggregate containers are reported before their children, and packed
/// sequences report their complete byte work before any element walk. This
/// lets execution targets stop a recursively expensive inspection at the
/// shared budget boundary instead of discovering the cost only after a full
/// footprint has already been computed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CanonicalDataWork {
    pub encoded_bytes: u64,
    pub retained_bytes: u64,
    pub node_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalDataWorkError<E> {
    ArithmeticOverflow,
    UnknownDynamicSchema,
    InvalidValue,
    Visitor(E),
}

impl From<SnapshotValueError> for ValueFootprintError {
    fn from(error: SnapshotValueError) -> Self {
        Self::InvalidValue(error)
    }
}

impl ValueFootprint {
    pub const fn zero() -> Self {
        Self {
            encoded_bytes: 0,
            retained_bytes: 0,
            node_count: 0,
        }
    }

    pub fn checked_add(self, other: Self) -> Result<Self, ValueFootprintError> {
        Ok(Self {
            encoded_bytes: self
                .encoded_bytes
                .checked_add(other.encoded_bytes)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            retained_bytes: self
                .retained_bytes
                .checked_add(other.retained_bytes)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            node_count: self
                .node_count
                .checked_add(other.node_count)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
        })
    }

    /// Accounts for cloning this value `multiplicity` times. Selection
    /// planning uses this operation so repeated selectors charge every
    /// retained copy rather than the source payload once.
    pub fn checked_multiply(self, multiplicity: u64) -> Result<Self, ValueFootprintError> {
        Ok(Self {
            encoded_bytes: self
                .encoded_bytes
                .checked_mul(multiplicity)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            retained_bytes: self
                .retained_bytes
                .checked_mul(multiplicity)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            node_count: self
                .node_count
                .checked_mul(multiplicity)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
        })
    }
}

impl Sha256SnapshotSink {
    fn new() -> Self {
        Self {
            hash: Sha256::new(),
        }
    }

    fn finish(self) -> [u8; 32] {
        self.hash.finalize().into()
    }
}

impl SnapshotByteSink for Sha256SnapshotSink {
    fn write(&mut self, bytes: &[u8]) {
        self.hash.update(bytes);
    }
}

impl Value {
    /// Encodes this immutable value without including any process-local cell
    /// identity. The schema key and shape precede the canonical payload so the
    /// bytes are suitable for public save/export boundaries.
    pub fn canonical_snapshot_bytes(
        &self,
        schemas: &SchemaTable,
    ) -> Result<Box<[u8]>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let shape = self.shape().canonical_bytes();
        let mut payload = VecSnapshotSink::new();
        encode_data(schema.body(), self.data(), &mut payload);
        let payload = payload.finish();

        let mut sink = VecSnapshotSink::new();
        sink.write(b"mech-snapshot-v1\0");
        sink.write(self.schema_key().as_bytes());
        write_u64(&mut sink, shape.len() as u64);
        sink.write(&shape);
        write_u64(&mut sink, payload.len() as u64);
        sink.write(&payload);
        Ok(sink.finish())
    }

    pub fn canonical_payload_bytes(
        &self,
        schemas: &SchemaTable,
    ) -> Result<Box<[u8]>, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let mut sink = VecSnapshotSink::new();
        encode_data(schema.body(), self.data(), &mut sink);
        Ok(sink.finish())
    }

    /// Returns the canonical encoded payload length without allocating the
    /// encoded payload. A saturated length is sufficient for callers that use
    /// this value to enforce a bounded-work limit.
    pub fn canonical_payload_len(
        &self,
        schemas: &SchemaTable,
    ) -> Result<usize, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let mut sink = LengthSnapshotSink { len: 0 };
        encode_data(schema.body(), self.data(), &mut sink);
        Ok(sink.len)
    }

    /// Traverses the validated value without cloning it and returns a
    /// conservative in-memory footprint in a checked `u64` accounting
    /// domain.
    pub fn retained_footprint(
        &self,
        schemas: &SchemaTable,
    ) -> Result<ValueFootprint, ValueFootprintError> {
        let schema = self.validate_against(schemas)?;
        let encoded_bytes = u64::try_from(canonical_data_payload_len(schema.body(), self.data()))
            .map_err(|_| ValueFootprintError::ArithmeticOverflow)?;
        let data = retained_data_footprint(schema.body(), self.data())?;
        let shape_bytes = checked_bytes(
            self.shape().parameter_values().len(),
            core::mem::size_of::<u64>(),
        )?;
        let retained_bytes = checked_size_of::<Value>()?
            .checked_add(shape_bytes)
            .and_then(|bytes| bytes.checked_add(data.retained_bytes))
            .ok_or(ValueFootprintError::ArithmeticOverflow)?;
        Ok(ValueFootprint {
            encoded_bytes,
            retained_bytes,
            node_count: data
                .node_count
                .checked_add(1)
                .ok_or(ValueFootprintError::ArithmeticOverflow)?,
        })
    }

    pub fn clone_footprint(
        &self,
        multiplicity: u64,
        schemas: &SchemaTable,
    ) -> Result<ValueFootprint, ValueFootprintError> {
        self.retained_footprint(schemas)?
            .checked_multiply(multiplicity)
    }

    pub fn value_hash(&self, schemas: &SchemaTable) -> Result<ValueHash, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let mut sink = Sha256SnapshotSink::new();
        sink.write(b"mech-value-v1\0");
        sink.write(self.schema_key().as_bytes());
        sink.write(&self.shape().canonical_bytes());
        encode_data(schema.body(), self.data(), &mut sink);
        Ok(ValueHash::from_bytes(sink.finish()))
    }

    pub fn key_hash(&self, schemas: &SchemaTable) -> Result<KeyHash, SnapshotValueError> {
        let schema = self.validate_against(schemas)?;
        let key = super::relations::normalized_key_data(schema.body(), self.data().clone())?;
        let mut sink = Sha256SnapshotSink::new();
        sink.write(b"mech-key-v1\0");
        sink.write(self.schema_key().as_bytes());
        sink.write(&self.shape().canonical_bytes());
        encode_data(schema.body(), &key, &mut sink);
        Ok(KeyHash::from_bytes(sink.finish()))
    }
}

pub(super) fn canonical_material(schema: &SchemaBody, data: &ValueData) -> Box<[u8]> {
    let mut sink = VecSnapshotSink::new();
    encode_data(schema, data, &mut sink);
    sink.finish()
}

/// Returns the canonical payload length for already validated schema-directed
/// data without allocating its encoded representation.
pub fn canonical_data_payload_len(schema: &SchemaBody, data: &ValueData) -> usize {
    let mut sink = LengthSnapshotSink { len: 0 };
    encode_data(schema, data, &mut sink);
    sink.len
}

/// Returns the conservative retained footprint of already validated
/// schema-directed data without allocating or cloning it.
pub fn canonical_data_retained_footprint(
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<ValueFootprint, ValueFootprintError> {
    let encoded_bytes = u64::try_from(canonical_data_payload_len(schema, data))
        .map_err(|_| ValueFootprintError::ArithmeticOverflow)?;
    let retained = retained_data_footprint(schema, data)?;
    Ok(ValueFootprint {
        encoded_bytes,
        ..retained
    })
}

/// Visits canonical data in pre-order, reporting bounded work before
/// descending into recursively stored children.
///
/// The data is expected to come from an already-finalized [`Value`]. This is
/// a work-admission traversal rather than a second semantic validator, but it
/// rejects physical schema/data mismatches encountered during the walk.
pub fn visit_canonical_data_work<E>(
    schema: &SchemaBody,
    data: &ValueData,
    mut visitor: impl FnMut(CanonicalDataWork) -> Result<(), E>,
) -> Result<(), CanonicalDataWorkError<E>> {
    visit_data_work(schema, data, true, &mut visitor)
}

fn work_bytes<E>(count: usize, width: usize) -> Result<u64, CanonicalDataWorkError<E>> {
    let count = u64::try_from(count).map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
    let width = u64::try_from(width).map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
    count
        .checked_mul(width)
        .ok_or(CanonicalDataWorkError::ArithmeticOverflow)
}

fn visit_work_chunk<E>(
    visitor: &mut impl FnMut(CanonicalDataWork) -> Result<(), E>,
    encoded_bytes: u64,
    retained_bytes: u64,
    node_count: u64,
    count_encoded: bool,
) -> Result<(), CanonicalDataWorkError<E>> {
    visitor(CanonicalDataWork {
        encoded_bytes: if count_encoded { encoded_bytes } else { 0 },
        retained_bytes,
        node_count,
    })
    .map_err(CanonicalDataWorkError::Visitor)
}

fn visit_sequence_work<E>(
    schema: &SchemaBody,
    values: &SequenceStorage,
    count_encoded: bool,
    visitor: &mut impl FnMut(CanonicalDataWork) -> Result<(), E>,
) -> Result<(), CanonicalDataWorkError<E>> {
    let fixed = match (schema, values) {
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), SequenceStorage::U8(values)) => {
            Some((values.len(), core::mem::size_of::<u8>()))
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), SequenceStorage::U16(values)) => {
            Some((values.len(), core::mem::size_of::<u16>()))
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), SequenceStorage::U32(values)) => {
            Some((values.len(), core::mem::size_of::<u32>()))
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), SequenceStorage::U64(values)) => {
            Some((values.len(), core::mem::size_of::<u64>()))
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), SequenceStorage::U128(values)) => {
            Some((values.len(), core::mem::size_of::<u128>()))
        }
        (SchemaBody::SignedInteger(IntegerWidth::W8), SequenceStorage::I8(values)) => {
            Some((values.len(), core::mem::size_of::<i8>()))
        }
        (SchemaBody::SignedInteger(IntegerWidth::W16), SequenceStorage::I16(values)) => {
            Some((values.len(), core::mem::size_of::<i16>()))
        }
        (SchemaBody::SignedInteger(IntegerWidth::W32), SequenceStorage::I32(values)) => {
            Some((values.len(), core::mem::size_of::<i32>()))
        }
        (SchemaBody::SignedInteger(IntegerWidth::W64), SequenceStorage::I64(values)) => {
            Some((values.len(), core::mem::size_of::<i64>()))
        }
        (SchemaBody::SignedInteger(IntegerWidth::W128), SequenceStorage::I128(values)) => {
            Some((values.len(), core::mem::size_of::<i128>()))
        }
        (SchemaBody::FloatingPoint(FloatWidth::W32), SequenceStorage::F32(values)) => {
            Some((values.len(), core::mem::size_of::<super::F32Bits>()))
        }
        (SchemaBody::FloatingPoint(FloatWidth::W64), SequenceStorage::F64(values)) => {
            Some((values.len(), core::mem::size_of::<super::F64Bits>()))
        }
        (SchemaBody::Complex(FloatWidth::W32), SequenceStorage::Complex32(values)) => {
            Some((values.len(), core::mem::size_of::<super::Complex32Bits>()))
        }
        (SchemaBody::Complex(FloatWidth::W64), SequenceStorage::Complex64(values)) => {
            Some((values.len(), core::mem::size_of::<super::Complex64Bits>()))
        }
        (SchemaBody::Rational64, SequenceStorage::Rational64(values)) => {
            Some((values.len(), core::mem::size_of::<super::Rational64Value>()))
        }
        (SchemaBody::Bool, SequenceStorage::Bool(values)) => Some((values.len(), 1)),
        (SchemaBody::Id, SequenceStorage::Id(values))
        | (SchemaBody::Index, SequenceStorage::Index(values)) => {
            Some((values.len(), core::mem::size_of::<u64>()))
        }
        (SchemaBody::Atom(_), SequenceStorage::Unit(_))
        | (SchemaBody::String, SequenceStorage::String(_))
        | (_, SequenceStorage::Values(_)) => None,
        _ => return Err(CanonicalDataWorkError::InvalidValue),
    };
    if let Some((count, width)) = fixed {
        let bytes = work_bytes(count, width)?;
        return visit_work_chunk(visitor, bytes, bytes, 1, count_encoded);
    }
    match values {
        SequenceStorage::Unit(_) if matches!(schema, SchemaBody::Atom(_)) => {
            visit_work_chunk(visitor, 0, 0, 1, count_encoded)
        }
        SequenceStorage::String(values) if matches!(schema, SchemaBody::String) => {
            visit_work_chunk(
                visitor,
                0,
                work_bytes(values.len(), core::mem::size_of::<Box<str>>())?,
                1,
                count_encoded,
            )?;
            for value in values.iter() {
                let payload = u64::try_from(value.len())
                    .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
                let encoded = payload
                    .checked_add(8)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?;
                visit_work_chunk(visitor, encoded, payload, 1, count_encoded)?;
            }
            Ok(())
        }
        SequenceStorage::Values(values) => {
            visit_work_chunk(visitor, 0, 0, 1, count_encoded)?;
            for value in values.iter() {
                visit_data_work(schema, value, count_encoded, visitor)?;
            }
            Ok(())
        }
        _ => Err(CanonicalDataWorkError::InvalidValue),
    }
}

fn visit_data_work<E>(
    schema: &SchemaBody,
    data: &ValueData,
    count_encoded: bool,
    visitor: &mut impl FnMut(CanonicalDataWork) -> Result<(), E>,
) -> Result<(), CanonicalDataWorkError<E>> {
    let scalar = match (schema, data) {
        (SchemaBody::Bool, ValueData::Bool(_)) => Some(1),
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueData::U8(_))
        | (SchemaBody::SignedInteger(IntegerWidth::W8), ValueData::I8(_)) => Some(1),
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueData::U16(_))
        | (SchemaBody::SignedInteger(IntegerWidth::W16), ValueData::I16(_)) => Some(2),
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueData::U32(_))
        | (SchemaBody::SignedInteger(IntegerWidth::W32), ValueData::I32(_))
        | (SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(_)) => Some(4),
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueData::U64(_))
        | (SchemaBody::SignedInteger(IntegerWidth::W64), ValueData::I64(_))
        | (SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(_))
        | (SchemaBody::Complex(FloatWidth::W32), ValueData::Complex32(_))
        | (SchemaBody::Id, ValueData::Id(_))
        | (SchemaBody::Index, ValueData::Index(_)) => Some(8),
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueData::U128(_))
        | (SchemaBody::SignedInteger(IntegerWidth::W128), ValueData::I128(_))
        | (SchemaBody::Complex(FloatWidth::W64), ValueData::Complex64(_))
        | (SchemaBody::Rational64, ValueData::Rational64(_)) => Some(16),
        (SchemaBody::Atom(_), ValueData::Atom) => Some(0),
        _ => None,
    };
    if let Some(encoded_bytes) = scalar {
        return visit_work_chunk(
            visitor,
            encoded_bytes,
            u64::try_from(core::mem::size_of::<ValueData>())
                .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?,
            1,
            count_encoded,
        );
    }
    let inline = u64::try_from(core::mem::size_of::<ValueData>())
        .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
    match (schema, data) {
        (SchemaBody::Dynamic, ValueData::Dynamic(value)) => {
            let canonical = u64::try_from(value.canonical.len())
                .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
            visit_work_chunk(
                visitor,
                canonical,
                inline
                    .checked_add(canonical)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                1,
                count_encoded,
            )?;
            if let Some(value) = value.value.as_deref() {
                let schemas = value
                    .schemas()
                    .ok_or(CanonicalDataWorkError::UnknownDynamicSchema)?;
                let nested = schemas
                    .get(value.schema())
                    .ok_or(CanonicalDataWorkError::UnknownDynamicSchema)?;
                let shape_bytes = work_bytes(
                    value.shape().parameter_values().len(),
                    core::mem::size_of::<u64>(),
                )?;
                visit_work_chunk(
                    visitor,
                    shape_bytes,
                    u64::try_from(core::mem::size_of::<Value>())
                        .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?
                        .checked_add(shape_bytes)
                        .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                    1,
                    false,
                )?;
                visit_data_work(nested.body(), value.data(), false, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::String, ValueData::String(value)) => {
            let payload = u64::try_from(value.len())
                .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
            let encoded = payload
                .checked_add(8)
                .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?;
            visit_work_chunk(
                visitor,
                encoded,
                inline
                    .checked_add(payload)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                1,
                count_encoded,
            )
        }
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            visit_work_chunk(visitor, 4, inline, 1, count_encoded)?;
            let payload_schema = variants
                .get(value.ordinal() as usize)
                .ok_or(CanonicalDataWorkError::InvalidValue)?
                .payload
                .as_ref();
            match (payload_schema, value.payload()) {
                (Some(payload_schema), Some(payload)) => {
                    visit_data_work(payload_schema, payload, count_encoded, visitor)?;
                }
                (None, None) => {}
                _ => return Err(CanonicalDataWorkError::InvalidValue),
            }
            Ok(())
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => {
            visit_work_chunk(visitor, 1, inline, 1, count_encoded)?;
            if let Some(value) = value.as_deref() {
                visit_data_work(element, value, count_encoded, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => {
            if elements.len() != values.len() {
                return Err(CanonicalDataWorkError::InvalidValue);
            }
            visit_work_chunk(visitor, 0, inline, 1, count_encoded)?;
            for (element, value) in elements.iter().zip(values.iter()) {
                visit_data_work(element, value, count_encoded, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::Record(fields), ValueData::Record(value)) => {
            if fields.len() != value.fields().len() {
                return Err(CanonicalDataWorkError::InvalidValue);
            }
            visit_work_chunk(visitor, 0, inline, 1, count_encoded)?;
            for (field, value) in fields.iter().zip(value.fields().iter()) {
                visit_data_work(&field.schema, value, count_encoded, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(value)) => {
            visit_work_chunk(visitor, 0, inline, 1, count_encoded)?;
            visit_sequence_work(element, &value.elements, count_encoded, visitor)
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(value)) => {
            if columns.len() != value.columns.len() {
                return Err(CanonicalDataWorkError::InvalidValue);
            }
            let containers =
                work_bytes(value.columns.len(), core::mem::size_of::<SequenceStorage>())?;
            visit_work_chunk(
                visitor,
                0,
                inline
                    .checked_add(containers)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                1,
                count_encoded,
            )?;
            for (column, values) in columns.iter().zip(value.columns.iter()) {
                visit_sequence_work(&column.schema, values, count_encoded, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(value)) => {
            let containers = work_bytes(
                value.elements.len(),
                core::mem::size_of::<super::CanonicalKeyValue>(),
            )?;
            visit_work_chunk(
                visitor,
                8,
                inline
                    .checked_add(containers)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                1,
                count_encoded,
            )?;
            for key in value.elements.iter() {
                visit_data_work(element, key.data(), count_encoded, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            let containers = work_bytes(
                map.entries.len(),
                core::mem::size_of::<super::MapEntryValue>(),
            )?;
            visit_work_chunk(
                visitor,
                8,
                inline
                    .checked_add(containers)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                1,
                count_encoded,
            )?;
            for entry in map.entries.iter() {
                visit_data_work(key, entry.key().data(), count_encoded, visitor)?;
                visit_data_work(value, entry.value(), count_encoded, visitor)?;
            }
            Ok(())
        }
        (SchemaBody::ReifiedType, ValueData::Type(ReifiedType::Kind(kind))) => {
            let payload = u64::try_from(kind.canonical_bytes().len())
                .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?;
            let encoded = payload
                .checked_add(9)
                .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?;
            visit_work_chunk(
                visitor,
                encoded,
                inline
                    .checked_add(payload)
                    .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?,
                1,
                count_encoded,
            )
        }
        (SchemaBody::ReifiedType, ValueData::Type(ReifiedType::Schema(key))) => {
            let bytes = u64::try_from(key.as_bytes().len())
                .map_err(|_| CanonicalDataWorkError::ArithmeticOverflow)?
                .checked_add(1)
                .ok_or(CanonicalDataWorkError::ArithmeticOverflow)?;
            visit_work_chunk(visitor, bytes, inline, 1, count_encoded)
        }
        _ => Err(CanonicalDataWorkError::InvalidValue),
    }
}

/// Returns the retained footprint of one borrowed canonical sequence element
/// without first expanding the packed sequence into owned `ValueData` nodes.
/// Selection planners use this to charge repeated positions with
/// multiplicity before allocating selector or output materialization.
pub fn canonical_sequence_element_retained_footprint(
    schema: &SchemaBody,
    values: SequenceView<'_>,
    index: usize,
) -> Result<ValueFootprint, ValueFootprintError> {
    let fixed = |encoded_bytes: u64, payload_bytes: u64| {
        Ok(ValueFootprint {
            encoded_bytes,
            ..payload_footprint(payload_bytes)?
        })
    };
    match values {
        SequenceView::U8(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(1, 0)),
        SequenceView::U16(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(2, 0)),
        SequenceView::U32(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(4, 0)),
        SequenceView::U64(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(8, 0)),
        SequenceView::U128(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(16, 0)),
        SequenceView::I8(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(1, 0)),
        SequenceView::I16(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(2, 0)),
        SequenceView::I32(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(4, 0)),
        SequenceView::I64(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(8, 0)),
        SequenceView::I128(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(16, 0)),
        SequenceView::F32(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(4, 0)),
        SequenceView::F64(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(8, 0)),
        SequenceView::Complex32(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(8, 0)),
        SequenceView::Complex64(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(16, 0)),
        SequenceView::Rational64(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(16, 0)),
        SequenceView::Bool(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(1, 0)),
        SequenceView::Id(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(8, 0)),
        SequenceView::Index(values) => values
            .get(index)
            .ok_or(ValueFootprintError::IndexOutOfRange)
            .and_then(|_| fixed(8, 0)),
        SequenceView::String(values) => {
            let value = values
                .get(index)
                .ok_or(ValueFootprintError::IndexOutOfRange)?;
            let payload =
                u64::try_from(value.len()).map_err(|_| ValueFootprintError::ArithmeticOverflow)?;
            fixed(
                payload
                    .checked_add(8)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                payload,
            )
        }
        SequenceView::Unit(count) => {
            if u64::try_from(index).map_or(true, |index| index >= count) {
                return Err(ValueFootprintError::IndexOutOfRange);
            }
            fixed(0, 0)
        }
        SequenceView::Values(values) => canonical_data_retained_footprint(
            schema,
            values
                .get(index)
                .ok_or(ValueFootprintError::IndexOutOfRange)?,
        ),
    }
}

fn checked_size_of<T>() -> Result<u64, ValueFootprintError> {
    u64::try_from(core::mem::size_of::<T>()).map_err(|_| ValueFootprintError::ArithmeticOverflow)
}

fn checked_bytes(count: usize, element: usize) -> Result<u64, ValueFootprintError> {
    let count = u64::try_from(count).map_err(|_| ValueFootprintError::ArithmeticOverflow)?;
    let element = u64::try_from(element).map_err(|_| ValueFootprintError::ArithmeticOverflow)?;
    count
        .checked_mul(element)
        .ok_or(ValueFootprintError::ArithmeticOverflow)
}

fn payload_footprint(bytes: u64) -> Result<ValueFootprint, ValueFootprintError> {
    Ok(ValueFootprint {
        encoded_bytes: 0,
        retained_bytes: checked_size_of::<ValueData>()?
            .checked_add(bytes)
            .ok_or(ValueFootprintError::ArithmeticOverflow)?,
        node_count: 1,
    })
}

fn aggregate_footprint<'a>(
    values: impl IntoIterator<Item = (&'a SchemaBody, &'a ValueData)>,
) -> Result<ValueFootprint, ValueFootprintError> {
    values.into_iter().try_fold(
        ValueFootprint {
            encoded_bytes: 0,
            retained_bytes: checked_size_of::<ValueData>()?,
            node_count: 1,
        },
        |total, (schema, value)| total.checked_add(retained_data_footprint(schema, value)?),
    )
}

fn retained_sequence_footprint(
    schema: &SchemaBody,
    values: &SequenceStorage,
) -> Result<ValueFootprint, ValueFootprintError> {
    let (retained_bytes, node_count) = match values {
        SequenceStorage::U8(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<u8>())?, 1)
        }
        SequenceStorage::U16(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<u16>())?, 1)
        }
        SequenceStorage::U32(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<u32>())?, 1)
        }
        SequenceStorage::U64(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<u64>())?, 1)
        }
        SequenceStorage::U128(values) => (
            checked_bytes(values.len(), core::mem::size_of::<u128>())?,
            1,
        ),
        SequenceStorage::I8(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<i8>())?, 1)
        }
        SequenceStorage::I16(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<i16>())?, 1)
        }
        SequenceStorage::I32(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<i32>())?, 1)
        }
        SequenceStorage::I64(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<i64>())?, 1)
        }
        SequenceStorage::I128(values) => (
            checked_bytes(values.len(), core::mem::size_of::<i128>())?,
            1,
        ),
        SequenceStorage::F32(values) => (
            checked_bytes(values.len(), core::mem::size_of::<super::F32Bits>())?,
            1,
        ),
        SequenceStorage::F64(values) => (
            checked_bytes(values.len(), core::mem::size_of::<super::F64Bits>())?,
            1,
        ),
        SequenceStorage::Complex32(values) => (
            checked_bytes(values.len(), core::mem::size_of::<super::Complex32Bits>())?,
            1,
        ),
        SequenceStorage::Complex64(values) => (
            checked_bytes(values.len(), core::mem::size_of::<super::Complex64Bits>())?,
            1,
        ),
        SequenceStorage::Rational64(values) => (
            checked_bytes(values.len(), core::mem::size_of::<super::Rational64Value>())?,
            1,
        ),
        SequenceStorage::Bool(values) => (
            checked_bytes(values.len(), core::mem::size_of::<bool>())?,
            1,
        ),
        SequenceStorage::Id(values) | SequenceStorage::Index(values) => {
            (checked_bytes(values.len(), core::mem::size_of::<u64>())?, 1)
        }
        SequenceStorage::Unit(_) => (0, 1),
        SequenceStorage::String(values) => {
            let containers = checked_bytes(values.len(), core::mem::size_of::<Box<str>>())?;
            let payload = values.iter().try_fold(0_u64, |bytes, value| {
                bytes
                    .checked_add(
                        u64::try_from(value.len())
                            .map_err(|_| ValueFootprintError::ArithmeticOverflow)?,
                    )
                    .ok_or(ValueFootprintError::ArithmeticOverflow)
            })?;
            (
                containers
                    .checked_add(payload)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                u64::try_from(values.len())
                    .map_err(|_| ValueFootprintError::ArithmeticOverflow)?
                    .checked_add(1)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            )
        }
        SequenceStorage::Values(values) => {
            let total = values
                .iter()
                .try_fold(ValueFootprint::zero(), |total, value| {
                    total.checked_add(retained_data_footprint(schema, value)?)
                })?;
            (
                total.retained_bytes,
                total
                    .node_count
                    .checked_add(1)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            )
        }
    };
    Ok(ValueFootprint {
        encoded_bytes: 0,
        retained_bytes,
        node_count,
    })
}

fn retained_data_footprint(
    schema: &SchemaBody,
    data: &ValueData,
) -> Result<ValueFootprint, ValueFootprintError> {
    let inline = checked_size_of::<ValueData>()?;
    match (schema, data) {
        (SchemaBody::Dynamic, ValueData::Dynamic(value)) => {
            let canonical = u64::try_from(value.canonical.len())
                .map_err(|_| ValueFootprintError::ArithmeticOverflow)?;
            let nested = match value.value.as_deref() {
                Some(value) => {
                    let schemas = value.schemas().ok_or_else(|| {
                        ValueFootprintError::InvalidValue(
                            SnapshotValueError::UnknownSnapshotSchema {
                                schema: value.schema(),
                            },
                        )
                    })?;
                    value.retained_footprint(&schemas)?
                }
                None => ValueFootprint::zero(),
            };
            Ok(ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline
                    .checked_add(canonical)
                    .and_then(|bytes| bytes.checked_add(nested.retained_bytes))
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                node_count: nested
                    .node_count
                    .checked_add(1)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            })
        }
        (SchemaBody::String, ValueData::String(value)) => payload_footprint(
            u64::try_from(value.len()).map_err(|_| ValueFootprintError::ArithmeticOverflow)?,
        ),
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            let mut result = ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline,
                node_count: 1,
            };
            if let (Some(payload_schema), Some(payload)) = (
                variants[value.ordinal() as usize].payload.as_ref(),
                value.payload(),
            ) {
                result = result.checked_add(retained_data_footprint(payload_schema, payload)?)?;
            }
            Ok(result)
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => match value.as_deref() {
            Some(value) => ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline,
                node_count: 1,
            }
            .checked_add(retained_data_footprint(element, value)?),
            None => Ok(ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline,
                node_count: 1,
            }),
        },
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => {
            aggregate_footprint(elements.iter().zip(values.iter()))
        }
        (SchemaBody::Record(fields), ValueData::Record(value)) => aggregate_footprint(
            fields
                .iter()
                .map(|field| &field.schema)
                .zip(value.fields().iter()),
        ),
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(value)) => {
            let sequence = retained_sequence_footprint(element, &value.elements)?;
            Ok(ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline
                    .checked_add(sequence.retained_bytes)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                node_count: sequence
                    .node_count
                    .checked_add(1)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
            })
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(value)) => {
            let mut result = ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline
                    .checked_add(checked_bytes(
                        value.columns.len(),
                        core::mem::size_of::<SequenceStorage>(),
                    )?)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                node_count: 1,
            };
            for (column, values) in columns.iter().zip(value.columns.iter()) {
                result =
                    result.checked_add(retained_sequence_footprint(&column.schema, values)?)?;
            }
            Ok(result)
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(value)) => {
            let mut result = ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline
                    .checked_add(checked_bytes(
                        value.elements.len(),
                        core::mem::size_of::<super::CanonicalKeyValue>(),
                    )?)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                node_count: 1,
            };
            for key in value.elements.iter() {
                result = result.checked_add(retained_data_footprint(element, key.data())?)?;
            }
            Ok(result)
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            let mut result = ValueFootprint {
                encoded_bytes: 0,
                retained_bytes: inline
                    .checked_add(checked_bytes(
                        map.entries.len(),
                        core::mem::size_of::<super::MapEntryValue>(),
                    )?)
                    .ok_or(ValueFootprintError::ArithmeticOverflow)?,
                node_count: 1,
            };
            for entry in map.entries.iter() {
                result = result.checked_add(retained_data_footprint(key, entry.key().data())?)?;
                result = result.checked_add(retained_data_footprint(value, entry.value())?)?;
            }
            Ok(result)
        }
        (SchemaBody::ReifiedType, ValueData::Type(ReifiedType::Kind(kind))) => payload_footprint(
            u64::try_from(kind.canonical_bytes().len())
                .map_err(|_| ValueFootprintError::ArithmeticOverflow)?,
        ),
        (SchemaBody::ReifiedType, ValueData::Type(ReifiedType::Schema(_))) => payload_footprint(0),
        _ => payload_footprint(0),
    }
}

pub(super) fn encode_data(schema: &SchemaBody, data: &ValueData, sink: &mut dyn SnapshotByteSink) {
    match (schema, data) {
        (SchemaBody::Dynamic, ValueData::Dynamic(value)) => sink.write(&value.canonical),
        (SchemaBody::Bool, ValueData::Bool(value)) => write_u8(sink, u8::from(*value)),
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), ValueData::U8(value)) => {
            write_u8(sink, *value)
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), ValueData::U16(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), ValueData::U32(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), ValueData::U64(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), ValueData::U128(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W8), ValueData::I8(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W16), ValueData::I16(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W32), ValueData::I32(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W64), ValueData::I64(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::SignedInteger(IntegerWidth::W128), ValueData::I128(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::FloatingPoint(FloatWidth::W32), ValueData::F32(value)) => {
            sink.write(&value.bits().to_le_bytes())
        }
        (SchemaBody::FloatingPoint(FloatWidth::W64), ValueData::F64(value)) => {
            sink.write(&value.bits().to_le_bytes())
        }
        (SchemaBody::Complex(FloatWidth::W32), ValueData::Complex32(value)) => {
            sink.write(&value.real().bits().to_le_bytes());
            sink.write(&value.imaginary().bits().to_le_bytes());
        }
        (SchemaBody::Complex(FloatWidth::W64), ValueData::Complex64(value)) => {
            sink.write(&value.real().bits().to_le_bytes());
            sink.write(&value.imaginary().bits().to_le_bytes());
        }
        (SchemaBody::Rational64, ValueData::Rational64(value)) => {
            sink.write(&value.numerator().to_le_bytes());
            sink.write(&value.denominator().to_le_bytes());
        }
        (SchemaBody::String, ValueData::String(value)) => write_utf8(sink, value),
        (SchemaBody::Id, ValueData::Id(value)) | (SchemaBody::Index, ValueData::Index(value)) => {
            sink.write(&value.to_le_bytes())
        }
        (SchemaBody::Atom(_), ValueData::Atom) => {}
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            sink.write(&value.ordinal().to_le_bytes());
            if let (Some(payload_schema), Some(payload)) = (
                variants[value.ordinal() as usize].payload.as_ref(),
                value.payload(),
            ) {
                encode_data(payload_schema, payload, sink);
            }
        }
        (SchemaBody::Option(element), ValueData::Option(value)) => match value {
            None => write_u8(sink, 0),
            Some(value) => {
                write_u8(sink, 1);
                encode_data(element, value, sink);
            }
        },
        (SchemaBody::Tuple(elements), ValueData::Tuple(values)) => {
            for (element, value) in elements.iter().zip(values) {
                encode_data(element, value, sink);
            }
        }
        (SchemaBody::Record(fields), ValueData::Record(value)) => {
            for (field, value) in fields.iter().zip(value.fields()) {
                encode_data(&field.schema, value, sink);
            }
        }
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(value)) => {
            encode_sequence(element, &value.elements, sink)
        }
        (SchemaBody::Table { columns, .. }, ValueData::Table(value)) => {
            for (column, values) in columns.iter().zip(value.columns.iter()) {
                encode_sequence(&column.schema, values, sink);
            }
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(value)) => {
            write_u64(sink, value.elements.len() as u64);
            for key in &value.elements {
                encode_data(element, key.data(), sink);
            }
        }
        (SchemaBody::Map { key, value, .. }, ValueData::Map(map)) => {
            write_u64(sink, map.entries.len() as u64);
            for entry in &map.entries {
                encode_data(key, entry.key().data(), sink);
                encode_data(value, entry.value(), sink);
            }
        }
        (SchemaBody::ReifiedType, ValueData::Type(value)) => match value {
            ReifiedType::Kind(kind) => {
                write_u8(sink, 1);
                write_u64(sink, kind.canonical_bytes().len() as u64);
                sink.write(kind.canonical_bytes());
            }
            ReifiedType::Schema(key) => {
                write_u8(sink, 2);
                sink.write(key.as_bytes());
            }
        },
        _ => unreachable!("finalized snapshot no longer matches its schema"),
    }
}

fn encode_sequence(schema: &SchemaBody, values: &SequenceStorage, sink: &mut dyn SnapshotByteSink) {
    macro_rules! primitive {
        ($values:expr) => {
            for value in $values.iter() {
                sink.write(&value.to_le_bytes());
            }
        };
    }
    match values {
        SequenceStorage::U8(values) => sink.write(values),
        SequenceStorage::U16(values) => primitive!(values),
        SequenceStorage::U32(values) => primitive!(values),
        SequenceStorage::U64(values) => primitive!(values),
        SequenceStorage::U128(values) => primitive!(values),
        SequenceStorage::I8(values) => {
            for value in values.iter() {
                sink.write(&value.to_le_bytes());
            }
        }
        SequenceStorage::I16(values) => primitive!(values),
        SequenceStorage::I32(values) => primitive!(values),
        SequenceStorage::I64(values) => primitive!(values),
        SequenceStorage::I128(values) => primitive!(values),
        SequenceStorage::F32(values) => {
            for value in values.iter() {
                sink.write(&value.bits().to_le_bytes());
            }
        }
        SequenceStorage::F64(values) => {
            for value in values.iter() {
                sink.write(&value.bits().to_le_bytes());
            }
        }
        SequenceStorage::Complex32(values) => {
            for value in values.iter() {
                sink.write(&value.real().bits().to_le_bytes());
                sink.write(&value.imaginary().bits().to_le_bytes());
            }
        }
        SequenceStorage::Complex64(values) => {
            for value in values.iter() {
                sink.write(&value.real().bits().to_le_bytes());
                sink.write(&value.imaginary().bits().to_le_bytes());
            }
        }
        SequenceStorage::Rational64(values) => {
            for value in values.iter() {
                sink.write(&value.numerator().to_le_bytes());
                sink.write(&value.denominator().to_le_bytes());
            }
        }
        SequenceStorage::Bool(values) => {
            for value in values.iter() {
                write_u8(sink, u8::from(*value));
            }
        }
        SequenceStorage::String(values) => {
            for value in values.iter() {
                write_utf8(sink, value);
            }
        }
        SequenceStorage::Id(values) | SequenceStorage::Index(values) => primitive!(values),
        SequenceStorage::Unit(_) => {}
        SequenceStorage::Values(values) => {
            for value in values.iter() {
                encode_data(schema, value, sink);
            }
        }
    }
}

fn write_u8(sink: &mut dyn SnapshotByteSink, value: u8) {
    sink.write(&[value]);
}

fn write_u64(sink: &mut dyn SnapshotByteSink, value: u64) {
    sink.write(&value.to_le_bytes());
}

fn write_utf8(sink: &mut dyn SnapshotByteSink, value: &str) {
    write_u64(sink, value.len() as u64);
    sink.write(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotValidationContext;
    use crate::{SchemaDraft, SchemaTableBuilder, ValueCell, ValueDataDraft, ValueDraft};
    use std::rc::Rc;

    fn bool_value(value: bool) -> (Value, SchemaTable) {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Bool,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema: id,
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(value),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        (value, schemas)
    }

    #[test]
    fn canonical_snapshot_bytes_include_schema_shape_and_data_but_not_cell_identity() {
        let (value, schemas) = bool_value(true);
        let first = ValueCell::from_value(value.clone(), Rc::new(schemas.clone())).unwrap();
        let second = ValueCell::from_value(value, Rc::new(schemas.clone())).unwrap();
        assert!(!first.same_cell(&second));

        let first = first
            .snapshot()
            .unwrap()
            .canonical_snapshot_bytes(&schemas)
            .unwrap();
        let second = second
            .snapshot()
            .unwrap()
            .canonical_snapshot_bytes(&schemas)
            .unwrap();
        assert_eq!(first, second);
        let prefix = b"mech-snapshot-v1\0";
        assert_eq!(&first[..prefix.len()], prefix);
        assert_eq!(
            &first[prefix.len()..prefix.len() + 32],
            bool_value(true).0.schema_key().as_bytes()
        );

        let (different, different_schemas) = bool_value(false);
        assert_ne!(
            first,
            different
                .canonical_snapshot_bytes(&different_schemas)
                .unwrap()
        );
    }

    #[test]
    fn retained_footprint_counts_containers_and_clone_multiplicity() {
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::String,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let id = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let payload = "x".repeat(1_024);
        let value = ValueDraft {
            schema: id,
            shape_values: Box::new([]),
            data: ValueDataDraft::String(payload),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();

        let single = value.retained_footprint(&schemas).unwrap();
        assert_eq!(single.encoded_bytes, 1_032);
        assert!(single.retained_bytes > single.encoded_bytes);
        assert!(single.node_count >= 2);

        let repeated = value.clone_footprint(4, &schemas).unwrap();
        assert_eq!(repeated.encoded_bytes, single.encoded_bytes * 4);
        assert_eq!(repeated.retained_bytes, single.retained_bytes * 4);
        assert_eq!(repeated.node_count, single.node_count * 4);
    }

    #[test]
    fn incremental_work_matches_canonical_footprint_in_one_pass() {
        let schema = SchemaBody::Tuple(
            vec![
                SchemaBody::String,
                SchemaBody::Option(Box::new(SchemaBody::String)),
            ]
            .into_boxed_slice(),
        );
        let data = ValueData::Tuple(
            vec![
                ValueData::String("left".into()),
                ValueData::Option(Some(Box::new(ValueData::String("right".into())))),
            ]
            .into_boxed_slice(),
        );
        let mut measured = ValueFootprint::zero();
        visit_canonical_data_work(&schema, &data, |work| {
            measured = measured
                .checked_add(ValueFootprint {
                    encoded_bytes: work.encoded_bytes,
                    retained_bytes: work.retained_bytes,
                    node_count: work.node_count,
                })
                .unwrap();
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!(
            measured,
            canonical_data_retained_footprint(&schema, &data).unwrap()
        );
    }

    #[test]
    fn incremental_work_stops_at_the_first_rejected_child() {
        const LIMIT: u64 = 65_536;
        let count = LIMIT as usize + 1;
        let schema = SchemaBody::Tuple(vec![SchemaBody::Bool; count].into_boxed_slice());
        let data = ValueData::Tuple(
            (0..count)
                .map(|_| ValueData::Bool(false))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let mut nodes = 0u64;
        let mut chunks = 0usize;
        assert_eq!(
            visit_canonical_data_work(&schema, &data, |work| {
                chunks += 1;
                nodes = nodes.checked_add(work.node_count).unwrap();
                (nodes <= LIMIT).then_some(()).ok_or(())
            }),
            Err(CanonicalDataWorkError::Visitor(()))
        );
        assert_eq!(nodes, LIMIT + 1);
        assert_eq!(chunks as u64, LIMIT + 1);
        assert!(chunks < count + 1);
    }
}
