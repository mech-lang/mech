//! Concrete and erased sinks share exact bytes, lengths and write ordering.
use super::*;
use crate::snapshot::{Complex32Bits, Complex64Bits, F32Bits, F64Bits, Rational64Value};

fn cases() -> Vec<(SchemaBody, SequenceStorage)> {
    let mut cases = Vec::new();
    macro_rules! integer {
        ($kind:ident, $width:ident, $storage:ident, $ty:ty) => {
            cases.push((
                SchemaBody::$kind(IntegerWidth::$width),
                SequenceStorage::$storage(vec![<$ty>::MIN, 0, 1, <$ty>::MAX].into()),
            ));
        };
    }
    integer!(UnsignedInteger, W8, U8, u8);
    integer!(UnsignedInteger, W16, U16, u16);
    integer!(UnsignedInteger, W32, U32, u32);
    integer!(UnsignedInteger, W64, U64, u64);
    integer!(UnsignedInteger, W128, U128, u128);
    integer!(SignedInteger, W8, I8, i8);
    integer!(SignedInteger, W16, I16, i16);
    integer!(SignedInteger, W32, I32, i32);
    integer!(SignedInteger, W64, I64, i64);
    integer!(SignedInteger, W128, I128, i128);
    let f32s = [0, 0x8000_0000, 0x7fc0_0001, 0xff80_0000].map(F32Bits::from_bits);
    let f64s = [
        0,
        0x8000_0000_0000_0000,
        0x7ff8_0000_0000_0001,
        0xfff0_0000_0000_0000,
    ]
    .map(F64Bits::from_bits);
    cases.extend([
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            SequenceStorage::F32(f32s.into()),
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            SequenceStorage::F64(f64s.into()),
        ),
        (
            SchemaBody::Complex(FloatWidth::W32),
            SequenceStorage::Complex32(f32s.map(|value| Complex32Bits::new(value, f32s[1])).into()),
        ),
        (
            SchemaBody::Complex(FloatWidth::W64),
            SequenceStorage::Complex64(f64s.map(|value| Complex64Bits::new(value, f64s[1])).into()),
        ),
        (
            SchemaBody::Rational64,
            SequenceStorage::Rational64(
                [(0, 1), (-1, 3), (i64::MIN, 1), (i64::MAX, u64::MAX)]
                    .map(|(a, b)| Rational64Value::new(a, b).unwrap())
                    .into(),
            ),
        ),
        (
            SchemaBody::Bool,
            SequenceStorage::Bool([false, true].into()),
        ),
        (
            SchemaBody::String,
            SequenceStorage::String(
                ["", "\0\r\n", "é", "e\u{301}", "👩‍💻", "ordinary"]
                    .map(Box::<str>::from)
                    .into(),
            ),
        ),
        (SchemaBody::Id, SequenceStorage::Id([0, 1, u64::MAX].into())),
        (
            SchemaBody::Index,
            SequenceStorage::Index([0, 1, u64::MAX].into()),
        ),
        (
            SchemaBody::Atom(crate::NominalKey::from_bytes([7; 32])),
            SequenceStorage::Unit(3),
        ),
        (
            SchemaBody::Option(Box::new(SchemaBody::String)),
            SequenceStorage::Values(
                [
                    ValueData::Option(None),
                    ValueData::Option(Some(Box::new(ValueData::String("x\0é".into())))),
                ]
                .into(),
            ),
        ),
    ]);
    cases
}

#[derive(Default)]
struct TraceSink {
    writes: Vec<Vec<u8>>,
}
impl SnapshotByteSink for TraceSink {
    fn write(&mut self, bytes: &[u8]) {
        self.writes.push(bytes.to_vec());
    }
}

#[test]
fn concrete_and_erased_sinks_keep_every_packed_sequence_encoding() {
    for (schema, sequence) in cases() {
        let mut concrete = TraceSink::default();
        encode_sequence(&schema, &sequence, &mut concrete);
        let mut erased = TraceSink::default();
        encode_sequence(&schema, &sequence, &mut erased as &mut dyn SnapshotByteSink);
        assert_eq!(concrete.writes, erased.writes, "{schema:?}");
        let expected = concrete.writes.concat();
        let mut scalar_bytes = VecSnapshotSink::new();
        for value in sequence.view().to_values() {
            encode_data(&schema, &value, &mut scalar_bytes);
        }
        assert_eq!(scalar_bytes.finish().as_ref(), expected, "{schema:?}");
        let mut length = LengthSnapshotSink { len: 0 };
        encode_sequence(&schema, &sequence, &mut length);
        assert_eq!(length.len, expected.len(), "{schema:?}");
        let mut actual = vec![0; length.len];
        let mut fixed = FixedSnapshotSink {
            bytes: &mut actual,
            offset: 0,
        };
        encode_sequence(&schema, &sequence, &mut fixed);
        assert_eq!(fixed.offset, expected.len());
        assert_eq!(actual, expected);
        let mut hash = Sha256SnapshotSink::new();
        encode_sequence(&schema, &sequence, &mut hash);
        assert_eq!(
            hash.finish().as_slice(),
            Sha256::digest(&expected).as_slice()
        );
    }
}

#[test]
fn concrete_length_sink_preserves_saturation_and_empty_sequences() {
    for (schema, sequence) in cases() {
        for initial in [0, usize::MAX - 33, usize::MAX - 1, usize::MAX] {
            let mut concrete = LengthSnapshotSink { len: initial };
            let mut erased = LengthSnapshotSink { len: initial };
            encode_sequence(&schema, &sequence, &mut concrete);
            encode_sequence(&schema, &sequence, &mut erased as &mut dyn SnapshotByteSink);
            let mut bytes = VecSnapshotSink::new();
            encode_sequence(&schema, &sequence, &mut bytes);
            assert_eq!(concrete.len, initial.saturating_add(bytes.finish().len()));
            assert_eq!(concrete.len, erased.len);
        }
    }
    for (schema, sequence) in [
        (
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            SequenceStorage::U64(Vec::new()),
        ),
        (SchemaBody::String, SequenceStorage::String(Box::new([]))),
        (
            SchemaBody::Atom(crate::NominalKey::from_bytes([7; 32])),
            SequenceStorage::Unit(0),
        ),
        (
            SchemaBody::Option(Box::new(SchemaBody::String)),
            SequenceStorage::Values(Box::new([])),
        ),
    ] {
        let mut sink = TraceSink::default();
        encode_sequence(&schema, &sequence, &mut sink);
        assert!(sink.writes.is_empty());
        let mut length = LengthSnapshotSink { len: 19 };
        encode_sequence(&schema, &sequence, &mut length);
        assert_eq!(length.len, 19);
    }
}

#[test]
fn concrete_recursive_encoder_keeps_nested_tags_lengths_and_raw_float_bits() {
    let schema = SchemaBody::Tuple(
        [
            SchemaBody::UnsignedInteger(IntegerWidth::W16),
            SchemaBody::Option(Box::new(SchemaBody::String)),
            SchemaBody::Tuple(
                [SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool].into(),
            ),
        ]
        .into(),
    );
    let data = ValueData::Tuple(
        [
            ValueData::U16(0x1234),
            ValueData::Option(Some(Box::new(ValueData::String("é\0".into())))),
            ValueData::Tuple(
                [
                    ValueData::F64(F64Bits::from_bits(0x8000_0000_0000_0000)),
                    ValueData::Bool(true),
                ]
                .into(),
            ),
        ]
        .into(),
    );
    let expected: Vec<u8> = [0x34, 0x12, 1]
        .into_iter()
        .chain(3u64.to_le_bytes())
        .chain([0xc3, 0xa9, 0])
        .chain(0x8000_0000_0000_0000u64.to_le_bytes())
        .chain([1])
        .collect();
    let mut bytes = VecSnapshotSink::new();
    encode_data(&schema, &data, &mut bytes);
    assert_eq!(bytes.finish().as_ref(), expected);
    assert_eq!(canonical_data_payload_len(&schema, &data), expected.len());
    assert_eq!(canonical_material(&schema, &data).as_ref(), expected);
    let mut erased = TraceSink::default();
    encode_data(&schema, &data, &mut erased as &mut dyn SnapshotByteSink);
    assert_eq!(erased.writes.concat(), expected);
}
