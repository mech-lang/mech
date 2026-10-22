//! Canonical data reconstruction for the private build-image loader.
//! Unlike selected compiler proofs, these values use ordinary safe finalizers.
#![allow(dead_code)] // Connected as the complete descriptor reader is assembled.
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::*;
use serde::Deserialize;
use serde_json::Value as Json;

#[derive(Deserialize)]
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
    let wire: SchemaImage = serde_json::from_value(image.clone()).map_err(|e| e.to_string())?;
    let dimension_parameters = wire
        .dimension_parameters
        .into_iter()
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
        body: wire.body,
    }
    .finalize()
    .map_err(|e| format!("image schema: {e:?}"))?;
    if serde_json::to_value(&schema).map_err(|e| e.to_string())? != *image {
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

pub(super) fn descriptor(image: &Json) -> Result<ResolvedValueDescriptor, String> {
    let (definition, parameters): (Json, Box<[u64]>) =
        serde_json::from_value(image.clone()).map_err(|e| e.to_string())?;
    let definition = schema(&definition)?;
    let shape = definition
        .instantiate_shape(parameters)
        .map_err(|e| format!("image shape: {e:?}"))?;
    ResolvedValueDescriptor::from_schema(definition, shape)
        .map_err(|e| format!("image type: {e:?}"))
}

pub(super) fn constant(image: &Json, schemas: &SchemaTable) -> Result<Value, String> {
    let draft: ValueDraft = serde_json::from_value(image.clone()).map_err(|e| e.to_string())?;
    if serde_json::to_value(&draft).map_err(|e| e.to_string())? != *image {
        return Err("unknown constant image fields".into());
    }
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
        let value = constant(&serde_json::to_value(&draft).unwrap(), &table).unwrap();
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
    }
}
