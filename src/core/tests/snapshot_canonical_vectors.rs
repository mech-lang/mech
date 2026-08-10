#![cfg(feature = "serde")]

use core::cmp::Ordering;
use mech_core::{snapshot::*, *};
use serde_json::Value as JsonValue;
use std::{fs, path::Path};

fn vectors() -> JsonValue {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/architecture/value-system/canonical-encoding-v1-vectors.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn required<'a>(value: &'a JsonValue, key: &str) -> &'a JsonValue {
    value.get(key).unwrap_or_else(|| panic!("missing {key}"))
}

fn required_str<'a>(value: &'a JsonValue, key: &str) -> &'a str {
    required(value, key).as_str().unwrap()
}

fn required_u64(value: &JsonValue, key: &str) -> u64 {
    required(value, key).as_u64().unwrap()
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |value| match value {
                b'0'..=b'9' => value - b'0',
                b'a'..=b'f' => value - b'a' + 10,
                _ => panic!("invalid test hex"),
            };
            digit(pair[0]) << 4 | digit(pair[1])
        })
        .collect()
}

fn dimension(value: &JsonValue) -> DimensionExpr {
    match required_str(value, "kind") {
        "Hole" => DimensionExpr::Hole,
        "Constant" => DimensionExpr::Constant(required_u64(value, "value")),
        "Parameter" => DimensionExpr::Parameter(DimensionParameterId::new(
            required_u64(value, "ordinal").try_into().unwrap(),
        )),
        "Add" => DimensionExpr::Add(dimensions(required(value, "operands"))),
        "Multiply" => DimensionExpr::Multiply(dimensions(required(value, "operands"))),
        "Min" => DimensionExpr::Min(dimensions(required(value, "operands"))),
        "Max" => DimensionExpr::Max(dimensions(required(value, "operands"))),
        other => panic!("unknown dimension expression {other}"),
    }
}

fn dimensions(value: &JsonValue) -> Box<[DimensionExpr]> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(dimension)
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn integer_width(value: u64) -> IntegerWidth {
    match value {
        8 => IntegerWidth::W8,
        16 => IntegerWidth::W16,
        32 => IntegerWidth::W32,
        64 => IntegerWidth::W64,
        128 => IntegerWidth::W128,
        _ => panic!("invalid integer width"),
    }
}

fn float_width(value: u64) -> FloatWidth {
    match value {
        32 => FloatWidth::W32,
        64 => FloatWidth::W64,
        _ => panic!("invalid float width"),
    }
}

fn nominal_key(value: &JsonValue, kind: NominalKind) -> NominalKey {
    let path = required(value, "nominal_path")
        .as_array()
        .unwrap()
        .iter()
        .map(|segment| segment.as_str().unwrap().to_owned())
        .collect::<Vec<_>>()
        .into_boxed_slice();
    NominalKey::from_path(kind, &CanonicalNominalPath::new(path).unwrap())
}

fn schema_body(value: &JsonValue) -> SchemaBody {
    match required_str(value, "kind") {
        "Bool" => SchemaBody::Bool,
        "UnsignedInteger" => {
            SchemaBody::UnsignedInteger(integer_width(required_u64(value, "bit_width")))
        }
        "SignedInteger" => {
            SchemaBody::SignedInteger(integer_width(required_u64(value, "bit_width")))
        }
        "FloatingPoint" => SchemaBody::FloatingPoint(float_width(required_u64(value, "bit_width"))),
        "Complex" => SchemaBody::Complex(float_width(required_u64(value, "component_bit_width"))),
        "Rational" => SchemaBody::Rational64,
        "String" => SchemaBody::String,
        "Id" => SchemaBody::Id,
        "Index" => SchemaBody::Index,
        "Atom" => SchemaBody::Atom(nominal_key(value, NominalKind::Atom)),
        "Enum" => SchemaBody::Enum {
            key: nominal_key(value, NominalKind::Enum),
            variants: required(value, "variants")
                .as_array()
                .unwrap()
                .iter()
                .map(|variant| EnumVariantSchema {
                    name: required_str(variant, "name").to_owned(),
                    payload: variant.get("payload").map(schema_body),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        },
        "Option" => SchemaBody::Option(Box::new(schema_body(required(value, "element")))),
        "Tuple" => SchemaBody::Tuple(
            required(value, "elements")
                .as_array()
                .unwrap()
                .iter()
                .map(schema_body)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "Record" => SchemaBody::Record(schema_fields(required(value, "fields"))),
        "Matrix" => SchemaBody::Matrix {
            element: Box::new(schema_body(required(value, "element"))),
            dimensions: dimensions(required(value, "dimensions")),
        },
        "Table" => SchemaBody::Table {
            columns: schema_fields(required(value, "columns")),
            rows: dimension(required(value, "row_count")),
        },
        "Set" => SchemaBody::Set {
            element: Box::new(schema_body(required(value, "element"))),
            cardinality: dimension(required(value, "cardinality")),
        },
        "Map" => SchemaBody::Map {
            key: Box::new(schema_body(required(value, "key"))),
            value: Box::new(schema_body(required(value, "value"))),
            cardinality: dimension(required(value, "cardinality")),
        },
        "ReifiedType" => SchemaBody::ReifiedType,
        other => panic!("unknown schema body {other}"),
    }
}

fn schema_fields(value: &JsonValue) -> Box<[SchemaField]> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|field| SchemaField {
            name: required_str(field, "name").to_owned(),
            schema: schema_body(required(field, "schema")),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn dimension_parameters(input: &JsonValue) -> Box<[DimensionParameterDeclaration]> {
    input
        .get("dimension_parameters")
        .and_then(JsonValue::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(ordinal, parameter)| DimensionParameterDeclaration {
            id: DimensionParameterId::new(ordinal as u32),
            origin: if parameter
                .get("explicit")
                .and_then(JsonValue::as_bool)
                .unwrap_or(true)
            {
                DimensionParameterOrigin::Explicit
            } else {
                DimensionParameterOrigin::Inferred
            },
            lifetime: match required_str(parameter, "lifetime") {
                "CompileTime" => DimensionLifetime::CompileTime,
                "Activation" => DimensionLifetime::Activation,
                "Turn" => DimensionLifetime::Turn,
                other => panic!("unknown dimension lifetime {other}"),
            },
            lower_bound: dimension(required(parameter, "lower_bound")),
            upper_bound: parameter.get("upper_bound").map(dimension),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn data(schema: &JsonValue, value: &JsonValue) -> ValueDataDraft {
    match required_str(schema, "kind") {
        "Bool" => ValueDataDraft::Bool(value.as_bool().unwrap()),
        "UnsignedInteger" => match required_u64(schema, "bit_width") {
            8 => ValueDataDraft::U8(value.as_u64().unwrap().try_into().unwrap()),
            16 => ValueDataDraft::U16(value.as_u64().unwrap().try_into().unwrap()),
            32 => ValueDataDraft::U32(value.as_u64().unwrap().try_into().unwrap()),
            64 => ValueDataDraft::U64(value.as_u64().unwrap()),
            128 => ValueDataDraft::U128(value.as_u64().unwrap() as u128),
            _ => unreachable!(),
        },
        "SignedInteger" => match required_u64(schema, "bit_width") {
            8 => ValueDataDraft::I8(value.as_i64().unwrap().try_into().unwrap()),
            16 => ValueDataDraft::I16(value.as_i64().unwrap().try_into().unwrap()),
            32 => ValueDataDraft::I32(value.as_i64().unwrap().try_into().unwrap()),
            64 => ValueDataDraft::I64(value.as_i64().unwrap()),
            128 => ValueDataDraft::I128(value.as_i64().unwrap() as i128),
            _ => unreachable!(),
        },
        "FloatingPoint" => match required_u64(schema, "bit_width") {
            32 => ValueDataDraft::F32(F32Bits::from_bits(float32_bits(value))),
            64 => ValueDataDraft::F64(F64Bits::from_bits(float_bits(value))),
            _ => unreachable!(),
        },
        "Complex" => match required_u64(schema, "component_bit_width") {
            32 => ValueDataDraft::Complex32(Complex32Bits::new(
                F32Bits::from_bits(float32_bits(required(value, "real"))),
                F32Bits::from_bits(float32_bits(required(value, "imaginary"))),
            )),
            64 => ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_bits(float_bits(required(value, "real"))),
                F64Bits::from_bits(float_bits(required(value, "imaginary"))),
            )),
            _ => unreachable!(),
        },
        "Rational" => ValueDataDraft::Rational64 {
            numerator: required(value, "numerator").as_i64().unwrap(),
            denominator: required_u64(value, "denominator"),
        },
        "String" => ValueDataDraft::String(value.as_str().unwrap().to_owned()),
        "Id" => ValueDataDraft::Id(value.as_u64().unwrap()),
        "Index" => ValueDataDraft::Index(value.as_u64().unwrap()),
        "Atom" => ValueDataDraft::Atom,
        "Option" => ValueDataDraft::Option(OptionDraft {
            present: value
                .get("present")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false),
            value: value
                .get("value")
                .map(|value| Box::new(data(required(schema, "element"), value))),
        }),
        "Enum" => {
            let ordinal = required_u64(value, "ordinal") as u32;
            let payload_schema = required(schema, "variants")
                .as_array()
                .unwrap()
                .get(ordinal as usize)
                .and_then(|variant| variant.get("payload"));
            ValueDataDraft::Enum(EnumDraft {
                ordinal,
                payload: value.get("payload").map(|payload| {
                    Box::new(match payload_schema {
                        Some(payload_schema) => data(payload_schema, payload),
                        None => ValueDataDraft::Bool(payload.as_bool().unwrap()),
                    })
                }),
            })
        }
        "Tuple" => ValueDataDraft::Tuple(
            required(schema, "elements")
                .as_array()
                .unwrap()
                .iter()
                .zip(value.as_array().unwrap())
                .map(|(schema, value)| data(schema, value))
                .chain(
                    value
                        .as_array()
                        .unwrap()
                        .iter()
                        .skip(required(schema, "elements").as_array().unwrap().len())
                        .map(|value| ValueDataDraft::Bool(value.as_bool().unwrap())),
                )
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "Record" => ValueDataDraft::Record(
            value
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, value)| {
                    let field = required(schema, "fields")
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|field| required_str(field, "name") == name);
                    NamedValueDraft {
                        name: name.clone(),
                        value: match field {
                            Some(field) => data(required(field, "schema"), value),
                            None => ValueDataDraft::Bool(value.as_bool().unwrap()),
                        },
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "Matrix" => ValueDataDraft::Matrix(
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|value| data(required(schema, "element"), value))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "Table" => ValueDataDraft::Table(
            value
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, values)| {
                    let column = required(schema, "columns")
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|column| required_str(column, "name") == name);
                    TableColumnDraft {
                        name: name.clone(),
                        values: values
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|value| match column {
                                Some(column) => data(required(column, "schema"), value),
                                None => ValueDataDraft::Bool(value.as_bool().unwrap()),
                            })
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "Set" => ValueDataDraft::Set(
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|value| data(required(schema, "element"), value))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "Map" => ValueDataDraft::Map(
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| {
                    let items = entry.as_array().unwrap();
                    MapEntryDraft {
                        items: items
                            .iter()
                            .enumerate()
                            .map(|(index, value)| {
                                data(
                                    if index == 0 {
                                        required(schema, "key")
                                    } else {
                                        required(schema, "value")
                                    },
                                    value,
                                )
                            })
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        "ReifiedType" => {
            assert_eq!(required_str(value, "kind"), "Schema");
            let schema = SchemaDraft {
                dimension_parameters: Box::new([]),
                body: schema_body(required(value, "schema")),
            }
            .finalize()
            .unwrap();
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(schema.key()))
        }
        other => panic!("unknown value kind {other}"),
    }
}

fn float_bits(value: &JsonValue) -> u64 {
    value
        .get("bits_hex")
        .and_then(JsonValue::as_str)
        .map(|bits| u64::from_str_radix(bits, 16).unwrap())
        .unwrap_or_else(|| value.as_f64().unwrap().to_bits())
}

fn float32_bits(value: &JsonValue) -> u32 {
    value
        .get("bits_hex")
        .and_then(JsonValue::as_str)
        .map(|bits| u32::from_str_radix(bits, 16).unwrap())
        .unwrap_or_else(|| (value.as_f64().unwrap() as f32).to_bits())
}

fn finalize_input(input: &JsonValue) -> Result<(Value, SchemaTable), SnapshotValueError> {
    let schema_json = required(input, "schema");
    let schema = SchemaDraft {
        dimension_parameters: dimension_parameters(input),
        body: schema_body(schema_json),
    }
    .finalize()?;
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema)?;
    let build = builder.finish()?;
    let schema_id = build.resolve(handle)?;
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema: schema_id,
        shape_values: input
            .get("shape_values")
            .and_then(JsonValue::as_array)
            .map(|values| {
                values
                    .iter()
                    .map(|value| value.as_u64().unwrap())
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            })
            .unwrap_or_else(|| Box::new([])),
        data: data(schema_json, required(input, "value")),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))?;
    Ok((value, schemas))
}

fn error_name(error: &SnapshotValueError) -> &'static str {
    match error {
        SnapshotValueError::AggregateArityMismatchV1 { .. } => "AggregateArityMismatchV1",
        SnapshotValueError::AggregateFieldMismatchV1 { .. } => "AggregateFieldMismatchV1",
        SnapshotValueError::PayloadCardinalityMismatchV1 { .. } => "PayloadCardinalityMismatchV1",
        SnapshotValueError::EnumOrdinalOutOfRangeV1 { .. } => "EnumOrdinalOutOfRangeV1",
        SnapshotValueError::EnumPayloadMismatchV1 { .. } => "EnumPayloadMismatchV1",
        SnapshotValueError::MapEntryArityMismatchV1 { .. } => "MapEntryArityMismatchV1",
        SnapshotValueError::DuplicateCanonicalKeyV1 { .. } => "DuplicateCanonicalKeyV1",
        SnapshotValueError::Semantic(SemanticModelError::ShapeParameterCountMismatchV1 {
            ..
        }) => "ShapeParameterCountMismatchV1",
        SnapshotValueError::Semantic(SemanticModelError::ShapeBoundViolationV1 { .. }) => {
            "ShapeBoundViolationV1"
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn all_eleven_payload_and_value_hash_vectors_match() {
    let vectors = vectors();
    let vectors = required(&vectors, "value_vectors").as_array().unwrap();
    assert_eq!(vectors.len(), 11);
    for vector in vectors {
        let (value, schemas) = finalize_input(required(vector, "input")).unwrap();
        let expected = required(vector, "expected");
        assert_eq!(
            value.canonical_payload_bytes(&schemas).unwrap().as_ref(),
            decode_hex(required_str(expected, "payload_hex")),
            "{} payload",
            required_str(vector, "id")
        );
        assert_eq!(
            value.value_hash(&schemas).unwrap().as_bytes(),
            decode_hex(required_str(expected, "value_hash_hex")).as_slice(),
            "{} hash",
            required_str(vector, "id")
        );
    }
}

#[test]
fn all_seventeen_invalid_value_vectors_return_the_frozen_error() {
    let vectors = vectors();
    let vectors = required(&vectors, "invalid_value_vectors")
        .as_array()
        .unwrap();
    assert_eq!(vectors.len(), 17);
    for vector in vectors {
        let error = finalize_input(required(vector, "input")).unwrap_err();
        assert_eq!(
            error_name(&error),
            required_str(required(vector, "expected"), "error"),
            "{}",
            required_str(vector, "id")
        );
    }
}

#[test]
fn all_five_key_vectors_match() {
    let vectors = vectors();
    let vectors = required(&vectors, "key_vectors").as_array().unwrap();
    assert_eq!(vectors.len(), 5);
    for vector in vectors {
        let id = required_str(vector, "id");
        let input = required(vector, "input");
        let expected = required(vector, "expected");
        match id {
            "f64-signed-zero-equivalence" | "f64-nan-equivalence" => {
                let schema_json = required(input, "schema");
                let values = required(input, "values").as_array().unwrap();
                let (schemas, schema) = {
                    let schema = SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body: schema_body(schema_json),
                    }
                    .finalize()
                    .unwrap();
                    let mut builder = SchemaTableBuilder::new();
                    let handle = builder.insert(schema).unwrap();
                    let build = builder.finish().unwrap();
                    let id = build.resolve(handle).unwrap();
                    let (table, _) = build.into_parts();
                    (table, id)
                };
                let make = |json| {
                    ValueDraft {
                        schema,
                        shape_values: Box::new([]),
                        data: data(schema_json, json),
                    }
                    .finalize(&SnapshotValidationContext::new(&schemas))
                    .unwrap()
                };
                let left = make(&values[0]);
                let right = make(&values[1]);
                assert_eq!(
                    left.key_cmp(&schemas, &right, &schemas).unwrap(),
                    Ordering::Equal
                );
                assert_eq!(
                    left.key_hash(&schemas).unwrap(),
                    right.key_hash(&schemas).unwrap()
                );
                assert_eq!(
                    left.key_hash(&schemas).unwrap().as_bytes(),
                    decode_hex(required_str(expected, "key_hash_hex")).as_slice()
                );
            }
            "duplicate-canonical-set-keys" => {
                let scalar_schema = required(input, "schema");
                let set_schema = SchemaBody::Set {
                    element: Box::new(schema_body(scalar_schema)),
                    cardinality: DimensionExpr::Constant(2),
                };
                let schema = SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: set_schema,
                }
                .finalize()
                .unwrap();
                let mut builder = SchemaTableBuilder::new();
                let handle = builder.insert(schema).unwrap();
                let build = builder.finish().unwrap();
                let schema = build.resolve(handle).unwrap();
                let (schemas, _) = build.into_parts();
                let error = ValueDraft {
                    schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::Set(
                        required(input, "values")
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|value| data(scalar_schema, value))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    ),
                }
                .finalize(&SnapshotValidationContext::new(&schemas))
                .unwrap_err();
                assert_eq!(error_name(&error), required_str(expected, "error"));
            }
            "rational64-order" => {
                let schema_json = required(input, "schema");
                let make_input = |value: &JsonValue| {
                    serde_json::json!({
                        "schema": schema_json,
                        "shape_values": [],
                        "value": value,
                    })
                };
                let (left, left_schemas) =
                    finalize_input(&make_input(required(input, "left"))).unwrap();
                let (right, right_schemas) =
                    finalize_input(&make_input(required(input, "right"))).unwrap();
                assert_eq!(
                    left.key_cmp(&left_schemas, &right, &right_schemas).unwrap(),
                    Ordering::Less
                );
            }
            "complex-not-keyable" => {
                let (value, schemas) = finalize_input(input).unwrap();
                assert!(matches!(
                    value.key_cmp(&schemas, &value, &schemas),
                    Err(SnapshotValueError::SchemaNotKeyableV1)
                ));
            }
            _ => panic!("unexpected key vector {id}"),
        }
    }
}
