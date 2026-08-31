#![cfg(feature = "serde")]

use mech_core::*;
use serde_json::Value;
use std::{fs, path::Path};

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/architecture/value-system/canonical-encoding-v1-vectors.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn required<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or_else(|| panic!("missing {key}"))
}

fn required_str<'a>(value: &'a Value, key: &str) -> &'a str {
    required(value, key)
        .as_str()
        .unwrap_or_else(|| panic!("{key} is not a string"))
}

fn required_u64(value: &Value, key: &str) -> u64 {
    required(value, key)
        .as_u64()
        .unwrap_or_else(|| panic!("{key} is not a u64"))
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
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn dimension(value: &Value) -> DimensionExpr {
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

fn dimensions(value: &Value) -> Box<[DimensionExpr]> {
    value
        .as_array()
        .expect("dimensions are an array")
        .iter()
        .map(dimension)
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn integer_width(value: u64) -> Result<IntegerWidth, &'static str> {
    match value {
        8 => Ok(IntegerWidth::W8),
        16 => Ok(IntegerWidth::W16),
        32 => Ok(IntegerWidth::W32),
        64 => Ok(IntegerWidth::W64),
        128 => Ok(IntegerWidth::W128),
        _ => Err("InvalidSchemaWidthV1"),
    }
}

fn float_width(value: u64) -> Result<FloatWidth, &'static str> {
    match value {
        32 => Ok(FloatWidth::W32),
        64 => Ok(FloatWidth::W64),
        _ => Err("InvalidSchemaWidthV1"),
    }
}

fn schema_body(value: &Value) -> Result<SchemaBody, &'static str> {
    Ok(match required_str(value, "kind") {
        "Bool" => SchemaBody::Bool,
        "UnsignedInteger" => {
            SchemaBody::UnsignedInteger(integer_width(required_u64(value, "bit_width"))?)
        }
        "SignedInteger" => {
            SchemaBody::SignedInteger(integer_width(required_u64(value, "bit_width"))?)
        }
        "FloatingPoint" => {
            SchemaBody::FloatingPoint(float_width(required_u64(value, "bit_width"))?)
        }
        "Complex" => SchemaBody::Complex(float_width(required_u64(value, "component_bit_width"))?),
        "Rational" => {
            if required_u64(value, "numerator_width") != 64
                || required_u64(value, "denominator_width") != 64
            {
                return Err("InvalidSchemaWidthV1");
            }
            SchemaBody::Rational64
        }
        "String" => SchemaBody::String,
        "Id" => SchemaBody::Id,
        "Index" => SchemaBody::Index,
        "Atom" => SchemaBody::Atom(nominal_key(value, NominalKind::Atom)),
        "Enum" => SchemaBody::Enum {
            key: nominal_key(value, NominalKind::Enum),
            variants: required(value, "variants")
                .as_array()
                .expect("variants are an array")
                .iter()
                .map(|variant| {
                    Ok(EnumVariantSchema {
                        name: required_str(variant, "name").to_owned(),
                        payload: variant.get("payload").map(schema_body).transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, &'static str>>()?
                .into_boxed_slice(),
        },
        "Option" => SchemaBody::Option(Box::new(schema_body(required(value, "element"))?)),
        "Tuple" => SchemaBody::Tuple(
            required(value, "elements")
                .as_array()
                .expect("elements are an array")
                .iter()
                .map(schema_body)
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice(),
        ),
        "Record" => SchemaBody::Record(schema_fields(required(value, "fields"))?),
        "Matrix" => SchemaBody::Matrix {
            element: Box::new(schema_body(required(value, "element"))?),
            dimensions: dimensions(required(value, "dimensions")),
        },
        "Table" => SchemaBody::Table {
            columns: schema_fields(required(value, "columns"))?,
            rows: CardinalitySpec::Exact(dimension(required(value, "row_count"))),
        },
        "Set" => SchemaBody::Set {
            element: Box::new(schema_body(required(value, "element"))?),
            cardinality: dimension(required(value, "cardinality")).into(),
        },
        "Map" => SchemaBody::Map {
            key: Box::new(schema_body(required(value, "key"))?),
            value: Box::new(schema_body(required(value, "value"))?),
            cardinality: CardinalitySpec::Exact(dimension(required(value, "cardinality"))),
        },
        "ReifiedType" => SchemaBody::ReifiedType,
        other => panic!("unknown schema body {other}"),
    })
}

fn schema_fields(value: &Value) -> Result<Box<[SchemaField]>, &'static str> {
    value
        .as_array()
        .expect("fields are an array")
        .iter()
        .map(|field| {
            Ok(SchemaField {
                name: required_str(field, "name").to_owned(),
                schema: schema_body(required(field, "schema"))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn nominal_key(value: &Value, kind: NominalKind) -> NominalKey {
    let path = required(value, "nominal_path")
        .as_array()
        .expect("nominal path is an array")
        .iter()
        .map(|segment| segment.as_str().unwrap().to_owned())
        .collect::<Vec<_>>()
        .into_boxed_slice();
    NominalKey::from_path(kind, &CanonicalNominalPath::new(path).unwrap())
}

fn dimension_parameters(input: &Value) -> Box<[DimensionParameterDeclaration]> {
    input
        .get("dimension_parameters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(index, parameter)| DimensionParameterDeclaration {
            id: DimensionParameterId::new(index.try_into().unwrap()),
            origin: if parameter
                .get("explicit")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                DimensionParameterOrigin::Explicit
            } else {
                DimensionParameterOrigin::Inferred
            },
            lifetime: match required_str(parameter, "lifetime") {
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

fn semantic_error_name(error: &SemanticModelError) -> &'static str {
    match error {
        SemanticModelError::DuplicateSchemaNameV1 { .. } => "DuplicateSchemaNameV1",
        other => panic!("unexpected semantic error {other:?}"),
    }
}

#[test]
fn every_positive_c0_value_vector_matches_schema_key_and_shape_bytes() {
    let vectors = vectors();
    let vectors = required(&vectors, "value_vectors")
        .as_array()
        .expect("value vectors are an array");
    assert_eq!(vectors.len(), 11);
    for vector in vectors {
        let id = required_str(vector, "id");
        let input = required(vector, "input");
        let expected = required(vector, "expected");
        let schema = SchemaDraft {
            dimension_parameters: dimension_parameters(input),
            body: schema_body(required(input, "schema")).unwrap(),
        }
        .finalize()
        .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(
            schema.canonical_bytes().as_ref(),
            decode_hex(required_str(expected, "schema_hex")),
            "{id}: schema bytes"
        );
        assert_eq!(
            schema.key().as_bytes().as_slice(),
            decode_hex(required_str(expected, "schema_key_hex")),
            "{id}: schema key"
        );
        let shape_values = required(input, "shape_values")
            .as_array()
            .expect("shape values are an array")
            .iter()
            .map(|value| value.as_u64().unwrap())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let shape = schema
            .instantiate_shape(shape_values)
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(
            shape.canonical_bytes().as_ref(),
            decode_hex(required_str(expected, "shape_hex")),
            "{id}: shape bytes"
        );
    }
}

#[test]
fn every_invalid_c0_schema_vector_returns_its_frozen_error() {
    let vectors = vectors();
    let vectors = required(&vectors, "invalid_schema_vectors")
        .as_array()
        .expect("invalid schema vectors are an array");
    assert_eq!(vectors.len(), 8);
    for vector in vectors {
        let id = required_str(vector, "id");
        let expected = required_str(required(vector, "expected"), "error");
        let input = required(vector, "input");
        let body = match schema_body(required(input, "schema")) {
            Ok(body) => body,
            Err(error) => {
                assert_eq!(error, expected, "{id}");
                continue;
            }
        };
        let error = SchemaDraft {
            dimension_parameters: dimension_parameters(input),
            body,
        }
        .finalize()
        .expect_err("invalid schema unexpectedly finalized");
        assert_eq!(semantic_error_name(&error), expected, "{id}");
    }
}
