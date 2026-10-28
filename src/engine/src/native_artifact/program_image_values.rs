//! Canonical data reconstruction for the private build-image loader.
//! Unlike selected compiler proofs, these values use ordinary safe finalizers.
#[cfg(any(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::*;
use serde::Deserialize;
use serde_json::Value as Json;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parameter {
    lifetime: DimensionLifetime,
    lower_bound: DimensionExpr,
    upper_bound: Option<DimensionExpr>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaImage {
    dimension_parameters: Vec<Parameter>,
    body: SchemaBody,
}

pub(crate) fn schema(image: &Json) -> Result<Schema, String> {
    let wire: SchemaImage = super::canonical_json::decode(image)?;
    let dimension_parameters = wire
        .dimension_parameters
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, p)| {
            Ok(DimensionParameterDeclaration {
                id: DimensionParameterId::new(
                    u32::try_from(id).map_err(|_| "too many dimensions")?,
                ),
                // Same reconstruction as canonical artifact bytecode: finalized
                // parameter ordinals are explicit, not new inference variables.
                origin: DimensionParameterOrigin::Explicit,
                lifetime: p.lifetime,
                lower_bound: p.lower_bound,
                upper_bound: p.upper_bound,
            })
        })
        .collect::<Result<Box<[_]>, String>>()?;
    let schema = SchemaDraft {
        dimension_parameters,
        body: wire.body.clone(),
    }
    .finalize()
    .map_err(|e| format!("image schema: {e:?}"))?;
    if schema.body() != &wire.body
        || schema.dimension_parameters().len() != wire.dimension_parameters.len()
        || schema
            .dimension_parameters()
            .iter()
            .zip(&wire.dimension_parameters)
            .any(|(actual, expected)| {
                actual.lifetime() != expected.lifetime
                    || actual.lower_bound() != &expected.lower_bound
                    || actual.upper_bound() != expected.upper_bound.as_ref()
            })
    {
        return Err("image schema is not canonical".into());
    }
    Ok(schema)
}

/// Finalize an ordered schema image through the canonical schema validator.
/// This restores data types only and confers no native execution authority.
pub fn schemas(image: &Json) -> Result<SchemaTable, String> {
    let entries = image.as_array().ok_or("image schemas must be an array")?;
    let mut builder = SchemaTableBuilder::new();
    let handles = entries
        .iter()
        .map(|entry| {
            builder
                .insert(schema(entry)?)
                .map_err(|e| format!("image schema table: {e:?}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let build = builder
        .finish()
        .map_err(|e| format!("image schema table: {e:?}"))?;
    if build.table.len() != handles.len() {
        return Err("duplicate image schema".into());
    }
    for (ordinal, handle) in handles.into_iter().enumerate() {
        let id = build
            .resolve(handle)
            .map_err(|e| format!("image schema ID: {e:?}"))?;
        if id.get() as usize != ordinal {
            return Err("image schema order differs".into());
        }
    }
    Ok(build.table)
}

#[cfg(any(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub(super) fn descriptor(image: &Json) -> Result<ResolvedValueDescriptor, String> {
    let (definition, parameters) =
        <(Json, Box<[u64]>)>::deserialize(image).map_err(|e| e.to_string())?;
    let definition = schema(&definition)?;
    let shape = definition
        .instantiate_shape(parameters)
        .map_err(|e| format!("image shape: {e:?}"))?;
    ResolvedValueDescriptor::from_schema(definition, shape)
        .map_err(|e| format!("image type: {e:?}"))
}

#[cfg(test)]
pub(super) fn constant(image: &Json, schemas: &SchemaTable) -> Result<Value, String> {
    let draft: ValueDraft = super::canonical_json::decode(image)?;
    draft
        .finalize(&SnapshotValidationContext::new(schemas))
        .map_err(|e| format!("image constant: {e:?}"))
}

#[cfg(any(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub(super) fn constant_bytes(bytes: &[u8], schemas: &SchemaTable) -> Result<Value, String> {
    let draft: ValueDraft = super::raw_json::decode(bytes)?;
    draft
        .finalize(&SnapshotValidationContext::new(schemas))
        .map_err(|e| format!("image constant: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scalar() -> Schema {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
        }
        .finalize()
        .unwrap()
    }
    #[test]
    fn canonical_schema_table_and_constant_roundtrip() {
        let definition = scalar();
        let image = serde_json::json!([definition]);
        let table = schemas(&image).unwrap();
        let draft = ValueDraft {
            schema: SchemaId::new(0),
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(42),
        };
        let value = constant_bytes(&serde_json::to_vec(&draft).unwrap(), &table).unwrap();
        assert_eq!(value.canonical_data_draft().unwrap(), draft.data);
        assert_eq!(
            serde_json::to_value(table.get(SchemaId::new(0)).unwrap()).unwrap(),
            image[0]
        );
        let resolved = descriptor(&serde_json::json!([definition, []])).unwrap();
        assert_eq!(resolved.schema(), &scalar());
    }
    #[test]
    fn duplicate_schema_invalid_shape_and_mistyped_constant_are_rejected() {
        let definition = scalar();
        assert!(schemas(&serde_json::json!([definition, definition])).is_err());
        assert!(descriptor(&serde_json::json!([definition, [1]])).is_err());
        let table = schemas(&serde_json::json!([definition])).unwrap();
        let bad = ValueDraft {
            schema: SchemaId::new(0),
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(true),
        };
        assert!(constant(&serde_json::to_value(bad).unwrap(), &table).is_err());
        let noncanonical = serde_json::json!({
            "dimension_parameters": [],
            "body": {"Matrix": {"element": {"UnsignedInteger": "W64"},
                "dimensions": [{"Add": [{"Constant": 1}, {"Constant": 1}]}]}},
        });
        assert!(schema(&noncanonical).is_err());
    }

    #[test]
    fn canonical_value_reader_preserves_all_existing_payload_families() {
        // Compare with the existing public codec, including open/type values.
        // These are wire fixtures only; none is admitted or executed.
        for image in [
            serde_json::json!({"Dynamic": null}),
            serde_json::json!({"Dynamic": {"schema": 0, "shape_values": [], "data": {"U64": 1}}}),
            serde_json::json!({"U8": 1}),
            serde_json::json!({"U16": 1}),
            serde_json::json!({"U32": 1}),
            serde_json::json!({"U64": u64::MAX}),
            serde_json::json!({"U128": 1}),
            serde_json::json!({"I8": -1}),
            serde_json::json!({"I16": -1}),
            serde_json::json!({"I32": -1}),
            serde_json::json!({"I64": i64::MIN}),
            serde_json::json!({"I128": -1}),
            serde_json::json!({"F32": 0}),
            serde_json::json!({"F64": u64::MAX}),
            serde_json::json!({"Complex32": {"real": 0, "imaginary": 0}}),
            serde_json::json!({"Complex64": {"real": 0, "imaginary": 0}}),
            serde_json::json!({"Rational64": {"numerator": 1, "denominator": 2}}),
            serde_json::json!({"Bool": true}),
            serde_json::json!({"String": "x"}),
            serde_json::json!({"Id": 1}),
            serde_json::json!({"Index": 1}),
            serde_json::json!("Atom"),
            serde_json::json!({"Enum": {"ordinal": 0, "payload": null}}),
            serde_json::json!({"Option": {"present": false, "value": null}}),
            serde_json::json!({"Tuple": []}),
            serde_json::json!({"Record": [{"name": "x", "value": {"U64": 1}}]}),
            serde_json::json!({"Matrix": [{"U64": 1}]}),
            serde_json::json!({"Table": [{"name": "x", "values": [{"U64": 1}]}]}),
            serde_json::json!({"Set": []}),
            serde_json::json!({"Map": [{"items": [{"String": "x"}, {"U64": 1}]}]}),
            serde_json::json!({"Type": {"CanonicalKind": []}}),
            serde_json::json!({"Type": {"Kind": {"kind": "Wildcard", "dimension_parameters": []}}}),
        ] {
            let original: ValueDataDraft = serde_json::from_value(image.clone()).unwrap();
            let restored: ValueDataDraft = super::super::canonical_json::decode(&image).unwrap();
            assert_eq!(restored, original);
            let raw = serde_json::to_vec(&original).unwrap();
            let restored_raw: ValueDataDraft = super::super::raw_json::decode(&raw).unwrap();
            assert_eq!(restored_raw, original);
            assert_eq!(serde_json::to_value(restored).unwrap(), image);
        }
    }
}
