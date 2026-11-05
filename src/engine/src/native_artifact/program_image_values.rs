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
    finalize_schema_image(wire)
}

fn finalize_schema_image(wire: SchemaImage) -> Result<Schema, String> {
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
    validate_schema_image(&wire, &schema)?;
    Ok(schema)
}

fn validate_schema_image(wire: &SchemaImage, schema: &Schema) -> Result<(), String> {
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
    Ok(())
}

/// An exact copy of an already finalized schema needs no second finalization.
/// Decode through the strict canonical reader before comparing every field;
/// neither a matching key nor a schema that merely normalizes to the same type
/// is sufficient. The canonical schema table remains the source of authority.
#[cfg(test)]
pub(super) fn matching_schema(image: &Json, definition: &Schema) -> Result<(), String> {
    let wire: SchemaImage = super::canonical_json::decode(image)?;
    validate_schema_image(&wire, definition)
}

pub(crate) fn schema_record(
    image: crate::native_artifact::BuildOwnedImageRecord<'_>,
) -> Result<Schema, String> {
    finalize_schema_image(super::canonical_record(image)?)
}

pub(crate) fn matching_schema_record(
    image: crate::native_artifact::BuildOwnedImageRecord<'_>,
    definition: &Schema,
) -> Result<(), String> {
    let wire = super::canonical_record(image)?;
    validate_schema_image(&wire, definition)
}

/// Finalize an ordered schema image through the canonical schema validator.
/// This restores data types only and confers no native execution authority.
pub fn schemas(image: &Json) -> Result<SchemaTable, String> {
    let entries = image.as_array().ok_or("image schemas must be an array")?;
    schema_table(entries.iter().map(schema))
}

pub fn schemas_record(
    image: crate::native_artifact::BuildOwnedImageRecord<'_>,
) -> Result<SchemaTable, String> {
    let entries: Vec<crate::native_artifact::BuildOwnedImageRecord<'_>> =
        image.decode().map_err(|e| e.to_string())?;
    schema_table(entries.into_iter().map(schema_record))
}

fn schema_table(
    entries: impl Iterator<Item = Result<Schema, String>>,
) -> Result<SchemaTable, String> {
    let mut builder = SchemaTableBuilder::new();
    let handles = entries
        .map(|entry| {
            builder
                .insert(entry?)
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

#[cfg(test)]
pub(super) fn descriptor(
    image: &Json,
    definition: &Schema,
) -> Result<ResolvedValueDescriptor, String> {
    let fields = image
        .as_array()
        .filter(|fields| fields.len() == 2)
        .ok_or("image descriptor must contain a schema and parameters")?;
    matching_schema(&fields[0], definition)?;
    let parameters: Box<[u64]> = super::canonical_json::decode(&fields[1])?;
    let shape = definition
        .instantiate_shape(parameters)
        .map_err(|e| format!("image shape: {e:?}"))?;
    ResolvedValueDescriptor::from_schema(definition.clone(), shape)
        .map_err(|e| format!("image type: {e:?}"))
}

#[cfg(any(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub(super) fn descriptor_record(
    image: crate::native_artifact::BuildOwnedImageRecord<'_>,
    definition: &Schema,
) -> Result<ResolvedValueDescriptor, String> {
    let (schema, parameters): (
        crate::native_artifact::BuildOwnedImageRecord<'_>,
        Box<[u64]>,
    ) = image.decode().map_err(|e| e.to_string())?;
    matching_schema_record(schema, definition)?;
    let shape = definition
        .instantiate_shape(parameters)
        .map_err(|e| format!("image shape: {e:?}"))?;
    ResolvedValueDescriptor::from_schema(definition.clone(), shape)
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

#[cfg(any(
    test,
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub(super) fn constant_record(
    image: crate::native_artifact::BuildOwnedImageRecord<'_>,
    schemas: &SchemaTable,
) -> Result<Value, String> {
    use crate::native_artifact::BuildOwnedImageRecord;
    match image {
        BuildOwnedImageRecord::Json(text) => constant_bytes(text.as_bytes(), schemas),
        BuildOwnedImageRecord::Native(_) | BuildOwnedImageRecord::UnsignedByte(_) => {
            let mut reader = image.reader().canonical();
            let draft = ValueDraft::deserialize(&mut reader).map_err(|e| e.to_string())?;
            reader.end().map_err(|e| e.to_string())?;
            draft
                .finalize(&SnapshotValidationContext::new(schemas))
                .map_err(|e| format!("image constant: {e:?}"))
        }
    }
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
        let resolved = descriptor(&serde_json::json!([definition, []]), &definition).unwrap();
        assert_eq!(resolved.schema(), &scalar());
    }

    #[test]
    fn borrowed_schema_tables_preserve_wide_interval_bounds() {
        let definition = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
                width: IntegerWidth::W128,
                lower: 1u128 << 80,
                upper: (1u128 << 80) + 100,
                upper_inclusive: false,
            }),
        }
        .finalize()
        .unwrap();
        let json = serde_json::to_vec(&[&definition]).unwrap();
        let native = crate::native_artifact::encode_native_metadata_image(&json).unwrap();
        for bytes in [&json, &native] {
            let mut reader =
                crate::native_artifact::BuildOwnedImageReader::from_image(bytes).unwrap();
            let image =
                crate::native_artifact::BuildOwnedImageRecord::deserialize(&mut reader).unwrap();
            reader.end().unwrap();
            let table = schemas_record(image).unwrap();
            assert_eq!(table.get(SchemaId::new(0)), Some(&definition));
        }
    }
    #[test]
    fn duplicate_schema_invalid_shape_and_mistyped_constant_are_rejected() {
        let definition = scalar();
        assert!(schemas(&serde_json::json!([definition, definition])).is_err());
        assert!(descriptor(&serde_json::json!([definition, [1]]), &definition).is_err());
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
    fn schema_reuse_requires_exact_canonical_fields_and_descriptor_shape() {
        let definition = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                dimensions: vec![DimensionExpr::Constant(2)].into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let image = serde_json::to_value(&definition).unwrap();
        assert!(matching_schema(&image, &definition).is_ok());
        let resolved = descriptor(&serde_json::json!([image, []]), &definition).unwrap();
        assert_eq!(resolved.schema(), &definition);
        let mut changed = image.clone();
        changed["body"]["Matrix"]["dimensions"][0] =
            serde_json::json!({"Add": [{"Constant": 1}, {"Constant": 1}]});
        // This normalizes to the same type, but is not the canonical image.
        assert!(matching_schema(&changed, &definition).is_err());
        assert!(descriptor(&serde_json::json!([changed, []]), &definition).is_err());
        changed = image.clone();
        changed["body"]["Matrix"]["dimensions"][0] = serde_json::json!({"Constant": 3});
        assert!(matching_schema(&changed, &definition).is_err());
        changed = image.clone();
        changed
            .as_object_mut()
            .unwrap()
            .remove("dimension_parameters");
        assert!(matching_schema(&changed, &definition).is_err());
        changed = image.clone();
        changed["extra"] = serde_json::json!(null);
        assert!(matching_schema(&changed, &definition).is_err());
        changed = image.clone();
        changed["body"]["Matrix"]["extra"] = serde_json::json!(0);
        assert!(matching_schema(&changed, &definition).is_err());
        for malformed in [
            serde_json::json!([image]),
            serde_json::json!([image, [], null]),
            serde_json::json!([image, [1]]),
            serde_json::json!([image, [-1]]),
            serde_json::json!([image, null]),
        ] {
            assert!(descriptor(&malformed, &definition).is_err(), "{malformed}");
        }
    }

    #[test]
    fn schema_reuse_checks_dimension_lifetimes_bounds_and_ordinals() {
        let definition = SchemaDraft {
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(1),
                upper_bound: Some(DimensionExpr::Constant(5)),
            }]
            .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                    .into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let image = serde_json::to_value(&definition).unwrap();
        assert!(matching_schema(&image, &definition).is_ok());
        assert!(descriptor(&serde_json::json!([image, [3]]), &definition).is_ok());
        for parameters in [
            serde_json::json!([]),
            serde_json::json!([0]),
            serde_json::json!([6]),
        ] {
            assert!(descriptor(&serde_json::json!([image, parameters]), &definition).is_err());
        }
        for (field, replacement) in [
            ("lifetime", serde_json::json!("Activation")),
            ("lower_bound", serde_json::json!({"Constant": 0})),
            ("upper_bound", serde_json::json!(null)),
        ] {
            let mut changed = image.clone();
            changed["dimension_parameters"][0][field] = replacement;
            assert!(matching_schema(&changed, &definition).is_err());
        }
        let mut changed = image;
        changed["body"]["Matrix"]["dimensions"][0] = serde_json::json!({"Parameter": 1});
        assert!(matching_schema(&changed, &definition).is_err());
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
