//! Preserved materializing encoder from IR236 as an independent byte oracle.
//! Its separate child buffers make length-prefix and key compatibility observable.
use super::{
    EnumVariantSchema, FloatWidth, IntegerInterval, IntegerWidth, Schema, SchemaBody, SchemaField,
};
use crate::{
    DimensionExpr, DimensionLifetime, DimensionParameter, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, NominalKey, SchemaDraft, SchemaKey,
    SchemaTableBuilder,
};
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec, vec::Vec};
use sha2::{Digest, Sha256};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec, vec::Vec};
struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16_le(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32_le(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64_le(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn write_utf8(&mut self, value: &str) {
        self.write_u64_le(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    fn write_node(&mut self, value: &[u8]) {
        self.write_u64_le(value.len() as u64);
        self.write_bytes(value);
    }

    fn finish(self) -> Box<[u8]> {
        self.bytes.into_boxed_slice()
    }
}

fn original_bytes(schema: &Schema) -> Box<[u8]> {
    let mut writer = CanonicalWriter::new();
    writer.write_u8(0x01);
    writer.write_u32_le(schema.dimension_parameters().len() as u32);
    encode_dimension_parameters(schema.dimension_parameters(), &mut writer.bytes);
    let body = encode_schema_body(schema.body());
    writer.write_node(&body);
    writer.finish()
}
fn original_key(bytes: &[u8]) -> SchemaKey {
    let mut hash = Sha256::new();
    hash.update(b"mech-schema-v1\0");
    hash.update(bytes);
    SchemaKey::from_bytes(hash.finalize().into())
}
fn encode_schema_body(body: &SchemaBody) -> Box<[u8]> {
    let mut writer = CanonicalWriter::new();
    match body {
        SchemaBody::Dynamic => writer.write_u8(0x14),
        SchemaBody::Bool => writer.write_u8(0x01),
        SchemaBody::UnsignedInteger(width) => {
            writer.write_u8(0x02);
            writer.write_u16_le(integer_width(*width));
        }
        SchemaBody::SignedInteger(width) => {
            writer.write_u8(0x03);
            writer.write_u16_le(integer_width(*width));
        }
        SchemaBody::IntegerInterval(interval) => {
            writer.write_u8(0x15);
            match interval {
                IntegerInterval::Unsigned {
                    width,
                    lower,
                    upper,
                    upper_inclusive,
                } => {
                    writer.write_u8(0);
                    writer.write_u16_le(integer_width(*width));
                    writer.write_bytes(&lower.to_le_bytes());
                    writer.write_bytes(&upper.to_le_bytes());
                    writer.write_u8(u8::from(*upper_inclusive));
                }
                IntegerInterval::Signed {
                    width,
                    lower,
                    upper,
                    upper_inclusive,
                } => {
                    writer.write_u8(1);
                    writer.write_u16_le(integer_width(*width));
                    writer.write_bytes(&lower.to_le_bytes());
                    writer.write_bytes(&upper.to_le_bytes());
                    writer.write_u8(u8::from(*upper_inclusive));
                }
            }
        }
        SchemaBody::FloatingPoint(width) => {
            writer.write_u8(0x04);
            writer.write_u16_le(float_width(*width));
        }
        SchemaBody::Complex(width) => {
            writer.write_u8(0x05);
            writer.write_u16_le(float_width(*width));
        }
        SchemaBody::Rational64 => {
            writer.write_u8(0x06);
            writer.write_u16_le(64);
            writer.write_u16_le(64);
        }
        SchemaBody::String => writer.write_u8(0x07),
        SchemaBody::Id => writer.write_u8(0x08),
        SchemaBody::Index => writer.write_u8(0x09),
        SchemaBody::Atom(key) => {
            writer.write_u8(0x0a);
            writer.write_bytes(key.as_bytes());
        }
        SchemaBody::Enum { key, variants } => {
            writer.write_u8(0x0b);
            writer.write_bytes(key.as_bytes());
            encode_variants(&mut writer, variants);
        }
        SchemaBody::Option(element) => {
            writer.write_u8(0x0c);
            writer.write_node(&encode_schema_body(element));
        }
        SchemaBody::Tuple(elements) => {
            writer.write_u8(0x0d);
            writer.write_u32_le(elements.len() as u32);
            for element in elements {
                writer.write_node(&encode_schema_body(element));
            }
        }
        SchemaBody::Record(fields) => {
            writer.write_u8(0x0e);
            encode_fields(&mut writer, fields);
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            writer.write_u8(0x0f);
            writer.write_node(&encode_schema_body(element));
            writer.write_u32_le(dimensions.len() as u32);
            for dimension in dimensions {
                writer.write_node(&encode_normalized_dimension(dimension));
            }
        }
        SchemaBody::Table { columns, rows } => {
            writer.write_u8(0x10);
            encode_fields(&mut writer, columns);
            writer.write_node(&encode_extent(rows));
        }
        SchemaBody::Set {
            element,
            cardinality,
        } => {
            writer.write_u8(0x11);
            writer.write_node(&encode_schema_body(element));
            writer.write_node(&encode_extent(cardinality));
        }
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => {
            writer.write_u8(0x12);
            writer.write_node(&encode_schema_body(key));
            writer.write_node(&encode_schema_body(value));
            writer.write_node(&encode_extent(cardinality));
        }
        SchemaBody::ReifiedType => writer.write_u8(0x13),
    }
    writer.finish()
}

fn encode_extent(extent: &crate::ExtentSpec) -> Box<[u8]> {
    match extent {
        crate::ExtentSpec::Exact(value) => encode_normalized_dimension(value).into_boxed_slice(),
        dynamic @ crate::ExtentSpec::Dynamic { .. } => encode_dynamic_extent(dynamic),
    }
}

fn encode_dynamic_extent(cardinality: &crate::ExtentSpec) -> Box<[u8]> {
    let mut writer = CanonicalWriter::new();
    match cardinality {
        crate::CardinalitySpec::Exact(_) => unreachable!("exact cardinality uses the v1 encoding"),
        crate::CardinalitySpec::Dynamic { upper_bound: None } => writer.write_u8(0x80),
        crate::CardinalitySpec::Dynamic {
            upper_bound: Some(value),
        } => {
            writer.write_u8(0x81);
            writer.write_node(&encode_normalized_dimension(value));
        }
    }
    writer.finish()
}

fn encode_variants(writer: &mut CanonicalWriter, variants: &[EnumVariantSchema]) {
    writer.write_u32_le(variants.len() as u32);
    for variant in variants {
        writer.write_utf8(&variant.name);
        match &variant.payload {
            None => writer.write_u8(0),
            Some(payload) => {
                writer.write_u8(1);
                writer.write_node(&encode_schema_body(payload));
            }
        }
    }
}

fn encode_fields(writer: &mut CanonicalWriter, fields: &[SchemaField]) {
    writer.write_u32_le(fields.len() as u32);
    for field in fields {
        writer.write_utf8(&field.name);
        writer.write_node(&encode_schema_body(&field.schema));
    }
}

const fn integer_width(width: IntegerWidth) -> u16 {
    width as u16
}

const fn float_width(width: FloatWidth) -> u16 {
    width as u16
}
fn encode_normalized_dimension(expression: &DimensionExpr) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_normalized_dimension_into(expression, &mut bytes);
    bytes
}

fn encode_normalized_dimension_into(expression: &DimensionExpr, bytes: &mut Vec<u8>) {
    match expression {
        DimensionExpr::Hole => unreachable!("holes are not canonical"),
        DimensionExpr::Constant(value) => {
            bytes.push(0x01);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        DimensionExpr::Parameter(id) => {
            bytes.push(0x02);
            bytes.extend_from_slice(&id.get().to_le_bytes());
        }
        DimensionExpr::Add(children)
        | DimensionExpr::Multiply(children)
        | DimensionExpr::Min(children)
        | DimensionExpr::Max(children) => {
            bytes.push(match expression {
                DimensionExpr::Add(_) => 0x03,
                DimensionExpr::Multiply(_) => 0x04,
                DimensionExpr::Min(_) => 0x05,
                DimensionExpr::Max(_) => 0x06,
                _ => unreachable!(),
            });
            bytes.extend_from_slice(&(children.len() as u32).to_le_bytes());
            for child in children {
                let encoded = encode_normalized_dimension(child);
                bytes.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
                bytes.extend_from_slice(&encoded);
            }
        }
    }
}

fn encode_dimension_parameters(parameters: &[DimensionParameter], bytes: &mut Vec<u8>) {
    for parameter in parameters {
        bytes.push(match parameter.lifetime() {
            DimensionLifetime::Activation => 0x01,
            DimensionLifetime::Turn => 0x02,
            DimensionLifetime::CompileTime => unreachable!("compile-time parameters are rejected"),
        });
        let lower = encode_normalized_dimension(parameter.lower_bound());
        bytes.extend_from_slice(&(lower.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&lower);
        match parameter.upper_bound() {
            None => bytes.push(0),
            Some(upper) => {
                bytes.push(1);
                let upper = encode_normalized_dimension(upper);
                bytes.extend_from_slice(&(upper.len() as u64).to_le_bytes());
                bytes.extend_from_slice(&upper);
            }
        }
    }
}

fn finalized(body: SchemaBody) -> Schema {
    SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap()
}
fn fields() -> Box<[SchemaField]> {
    vec![
        SchemaField {
            name: "alpha".into(),
            schema: SchemaBody::Bool,
        },
        SchemaField {
            name: "é界".into(),
            schema: SchemaBody::Option(Box::new(SchemaBody::String)),
        },
    ]
    .into_boxed_slice()
}
fn schemas() -> Vec<Schema> {
    let mut bodies = vec![
        SchemaBody::Dynamic,
        SchemaBody::Bool,
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        SchemaBody::SignedInteger(IntegerWidth::W16),
        SchemaBody::SignedInteger(IntegerWidth::W64),
        SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
            width: IntegerWidth::W8,
            lower: 7,
            upper: 37,
            upper_inclusive: true,
        }),
        SchemaBody::IntegerInterval(IntegerInterval::Signed {
            width: IntegerWidth::W16,
            lower: -13,
            upper: 42,
            upper_inclusive: false,
        }),
        SchemaBody::FloatingPoint(FloatWidth::W32),
        SchemaBody::FloatingPoint(FloatWidth::W64),
        SchemaBody::Complex(FloatWidth::W64),
        SchemaBody::Rational64,
        SchemaBody::String,
        SchemaBody::Id,
        SchemaBody::Index,
        SchemaBody::Atom(NominalKey::from_bytes([17; 32])),
        SchemaBody::Enum {
            key: NominalKey::from_bytes([29; 32]),
            variants: vec![
                EnumVariantSchema {
                    name: "empty".into(),
                    payload: None,
                },
                EnumVariantSchema {
                    name: "payload界".into(),
                    payload: Some(SchemaBody::Record(fields())),
                },
            ]
            .into_boxed_slice(),
        },
        SchemaBody::Option(Box::new(SchemaBody::Bool)),
        SchemaBody::Tuple(Box::new([])),
        SchemaBody::Tuple(vec![SchemaBody::Bool, SchemaBody::Record(fields())].into_boxed_slice()),
        SchemaBody::Record(fields()),
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: vec![
                DimensionExpr::Constant(0),
                DimensionExpr::Constant(1),
                DimensionExpr::Constant(4),
            ]
            .into_boxed_slice(),
        },
        SchemaBody::ReifiedType,
    ];
    for cardinality in [
        crate::CardinalitySpec::Exact(DimensionExpr::Constant(7)),
        crate::CardinalitySpec::Dynamic { upper_bound: None },
        crate::CardinalitySpec::Dynamic {
            upper_bound: Some(DimensionExpr::Constant(19)),
        },
    ] {
        bodies.push(SchemaBody::Table {
            columns: fields(),
            rows: cardinality.clone(),
        });
        bodies.push(SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: cardinality.clone(),
        });
        bodies.push(SchemaBody::Map {
            key: Box::new(SchemaBody::Id),
            value: Box::new(SchemaBody::Record(fields())),
            cardinality,
        });
    }
    let mut nested = SchemaBody::Bool;
    for depth in 0..24 {
        nested = SchemaBody::Tuple(
            vec![
                SchemaBody::Option(Box::new(nested)),
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    dimensions: vec![DimensionExpr::Constant(depth)].into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        );
        bodies.push(nested.clone());
    }
    bodies.push(SchemaBody::Tuple(
        (0..300)
            .map(|i| SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![DimensionExpr::Constant(i)].into_boxed_slice(),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    ));
    let mut result = bodies.into_iter().map(finalized).collect::<Vec<_>>();
    let parameter = |id| DimensionExpr::Parameter(DimensionParameterId::new(id));
    let declarations = vec![
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(64)),
        },
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(1),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Max(
                vec![parameter(0), DimensionExpr::Constant(3)].into_boxed_slice(),
            ),
            upper_bound: Some(DimensionExpr::Add(
                vec![parameter(0), DimensionExpr::Constant(128)].into_boxed_slice(),
            )),
        },
    ];
    let expression = DimensionExpr::Max(
        vec![
            DimensionExpr::Min(vec![parameter(1), DimensionExpr::Constant(31)].into_boxed_slice()),
            DimensionExpr::Multiply(
                vec![DimensionExpr::Constant(2), parameter(0)].into_boxed_slice(),
            ),
        ]
        .into_boxed_slice(),
    );
    result.push(
        SchemaDraft {
            dimension_parameters: declarations.into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Record(fields())),
                dimensions: vec![parameter(0), expression].into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap(),
    );
    result
}

#[test]
fn canonical_encoding_preserves_materialized_nested_bytes_and_keys() {
    for schema in schemas() {
        let expected = original_bytes(&schema);
        assert_eq!(schema.canonical_bytes(), expected, "{:?}", schema.body());
        assert_eq!(schema.key(), original_key(&expected), "{:?}", schema.body());
    }
}

#[test]
fn canonical_encoding_preserves_table_order_duplicate_handles_and_keys() {
    let mut builder = SchemaTableBuilder::new();
    let mut handles = Vec::new();
    for schema in schemas().into_iter().rev() {
        let expected = original_bytes(&schema);
        handles.push((builder.insert(schema.clone()).unwrap(), expected.clone()));
        handles.push((builder.insert(schema).unwrap(), expected));
    }
    let mut ordered = handles
        .iter()
        .map(|(_, bytes)| bytes.clone())
        .collect::<Vec<_>>();
    ordered.sort();
    ordered.dedup();
    let build = builder.finish().unwrap();
    assert_eq!(build.table.len(), ordered.len());
    for (handle, expected) in handles {
        let id = build.resolve(handle).unwrap();
        assert_eq!(id.get() as usize, ordered.binary_search(&expected).unwrap());
        let entry = build.table.entry(id).unwrap();
        assert_eq!(entry.canonical_bytes(), expected.as_ref());
        assert_eq!(entry.key(), original_key(&expected));
    }
}
