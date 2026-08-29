use super::sequence::SequenceStorage;
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

struct Sha256SnapshotSink {
    hash: Sha256,
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
}
