//! Compiler-resolved immutable types, bound to the enclosing canonical schema
//! table. No Values, current shapes, factories or invocation state are cached.
use super::{NativeDescriptorTemplatePool, SchemaDescriptorEntry, descriptor_for_schema};
use crate::native_artifact::BuildOwnedImageRecord;
use mech_core::{ResolvedType, ResolvedValueDescriptor, SchemaId, SchemaTable};
use serde::{Deserialize, Deserializer, Serializer};

// Private adapter only: ResolvedType itself does not implement Deserialize.
struct ReadType(ResolvedType);
impl<'de> Deserialize<'de> for ReadType {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        // SAFETY: confined to restore below. Independent images are compared
        // against freshly derived complete evidence before the pool can escape;
        // the other entry requires genuine unchanged checked-Object provenance.
        unsafe { ResolvedType::from_build_owned_image_reader(reader).map(Self) }
    }
}

impl NativeDescriptorTemplatePool {
    /// Resolve each canonical schema once during compilation. A null row keeps
    /// the original neutral-shape rejection and ordinary live-shape fallback.
    pub fn serialize_build_owned_schema_templates<S: Serializer>(
        schemas: &SchemaTable,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut rows = serializer.serialize_seq(Some(schemas.len()))?;
        for entry in schemas.entries() {
            let descriptor = descriptor_for_schema(entry.schema());
            rows.serialize_element(&descriptor.as_ref().map(|d| d.resolved_type()))?;
        }
        rows.end()
    }

    /// Independent loading must rederive and compare the complete type,
    /// including predicate evidence. IDs and ordinary ResolvedType equality
    /// cannot establish that evidence.
    pub fn from_independent_schema_templates(
        record: BuildOwnedImageRecord<'_>,
        schemas: &SchemaTable,
    ) -> Result<Self, String> {
        // SAFETY: false selects full independent derivation and comparison;
        // no wire-produced semantic metadata escapes before it matches.
        unsafe { Self::restore_schema_templates::<false>(record, schemas) }
    }

    /// # Safety
    /// All rows and this exact canonical schema table must be unchanged output
    /// of the SAME checked compiler and genuinely bound Object. Confine the pool
    /// until that provenance holds. A serialized selector, schema ID or digest
    /// cannot select this path. Fresh-shape, factory, source-site and finite
    /// runtime admission checks remain mandatory on every actual call.
    pub unsafe fn from_build_owned_schema_templates(
        record: BuildOwnedImageRecord<'_>,
        schemas: &SchemaTable,
    ) -> Result<Self, String> {
        // SAFETY: forwards the complete same-build confinement obligation.
        unsafe { Self::restore_schema_templates::<true>(record, schemas) }
    }

    unsafe fn restore_schema_templates<const COMPILED: bool>(
        record: BuildOwnedImageRecord<'_>,
        schemas: &SchemaTable,
    ) -> Result<Self, String> {
        let rows: Box<[Option<ReadType>]> = record.decode().map_err(|e| e.to_string())?;
        if rows.len() != schemas.len() {
            return Err("compiled type-template inventory differs from schema table".into());
        }
        let mut pool = Self::default();
        pool.schemas.reserve(rows.len());
        for (index, (row, entry)) in rows
            .into_vec()
            .into_iter()
            .zip(schemas.entries())
            .enumerate()
        {
            let schema = entry.schema();
            let descriptor = if COMPILED {
                row.map(|row| {
                    // SAFETY: complete row-to-schema correspondence is part of
                    // the enclosing checked Object, not inferred from this ID.
                    unsafe {
                        ResolvedValueDescriptor::from_build_owned_neutral_type(
                            schema.clone(),
                            row.0,
                        )
                    }
                    .map_err(|e| format!("compiled neutral type differs: {e:?}"))
                })
                .transpose()?
            } else {
                let expected = descriptor_for_schema(schema);
                match (&row, &expected) {
                    (None, None) => {}
                    (Some(row), Some(expected))
                        if row.0.has_same_normalized_evidence(expected.resolved_type()) => {}
                    _ => return Err(
                        "compiled type template differs from independently derived schema evidence"
                            .into(),
                    ),
                }
                expected
            };
            let id = SchemaId::new(
                u32::try_from(index).map_err(|_| "type-template schema index overflows")?,
            );
            pool.schemas.insert(
                id,
                vec![SchemaDescriptorEntry {
                    schema: schema.clone(),
                    descriptor,
                }],
            );
        }
        Ok(pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, IntegerWidth, Schema, SchemaBody, SchemaDraft,
        SchemaTableBuilder,
    };

    struct Image<'a>(&'a SchemaTable);
    impl serde::Serialize for Image<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            NativeDescriptorTemplatePool::serialize_build_owned_schema_templates(self.0, s)
        }
    }
    fn scalar(body: SchemaBody) -> Schema {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
    }
    fn fixtures() -> SchemaTable {
        let mut builder = SchemaTableBuilder::new();
        for body in [
            SchemaBody::Bool,
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            SchemaBody::SignedInteger(IntegerWidth::W64),
            SchemaBody::Tuple(vec![SchemaBody::Bool, SchemaBody::String].into()),
        ] {
            builder.insert(scalar(body)).unwrap();
        }
        for lower in [0, 1] {
            builder
                .insert(
                    SchemaDraft {
                        dimension_parameters: vec![DimensionParameterDeclaration {
                            id: DimensionParameterId::new(0),
                            origin: DimensionParameterOrigin::Explicit,
                            lifetime: DimensionLifetime::Turn,
                            lower_bound: DimensionExpr::Constant(lower),
                            upper_bound: Some(DimensionExpr::Constant(4)),
                        }]
                        .into(),
                        body: SchemaBody::Matrix {
                            element: SchemaBody::Bool.into(),
                            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(
                                0,
                            ))]
                            .into(),
                        },
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
        }
        builder.finish().unwrap().table
    }

    #[test]
    fn frozen_schema_templates_preserve_all_evidence_null_fallback_and_foreign_id_checks() {
        let schemas = fixtures();
        let json = serde_json::to_string(&Image(&schemas)).unwrap();
        let native = crate::native_artifact::encode_native_metadata_image(json.as_bytes()).unwrap();
        for typed in [false, true] {
            let record = if typed {
                let mut reader =
                    crate::native_artifact::BuildOwnedImageReader::from_image(&native).unwrap();
                BuildOwnedImageRecord::deserialize(&mut reader).unwrap()
            } else {
                BuildOwnedImageRecord::Json(&json)
            };
            let independent =
                NativeDescriptorTemplatePool::from_independent_schema_templates(record, &schemas)
                    .unwrap();
            // SAFETY: exact same compiler serialization and schema table; test
            // compares metadata and never invokes native code.
            let mut compiled = unsafe {
                NativeDescriptorTemplatePool::from_build_owned_schema_templates(record, &schemas)
            }
            .unwrap();
            for (i, entry) in schemas.entries().enumerate() {
                let id = SchemaId::new(i as u32);
                let expected = descriptor_for_schema(entry.schema());
                for actual in [
                    &independent.schemas[&id][0].descriptor,
                    &compiled.schemas[&id][0].descriptor,
                ] {
                    match (actual, &expected) {
                        (None, None) => {
                            assert!(entry.schema().instantiate_shape(vec![1].into()).is_ok());
                        }
                        (Some(a), Some(b)) => {
                            assert_eq!(a.schema(), b.schema());
                            assert_eq!(a.shape(), b.shape());
                            assert!(
                                a.resolved_type()
                                    .has_same_normalized_evidence(b.resolved_type())
                            );
                        }
                        _ => panic!("neutral fallback changed"),
                    }
                }
            }
            let id = SchemaId::new(0);
            let foreign = scalar(SchemaBody::String);
            let found = compiled.for_schema(id, &foreign).unwrap();
            assert_eq!(found.schema(), &foreign);
            assert_eq!(compiled.schemas[&id].len(), 2);
        }
    }

    #[test]
    fn independent_frozen_schema_templates_reject_every_type_and_evidence_change() {
        let schemas = fixtures();
        let image = serde_json::to_value(Image(&schemas)).unwrap();
        let live = image
            .as_array()
            .unwrap()
            .iter()
            .position(|r| !r.is_null())
            .unwrap();
        let absent = image
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r.is_null())
            .unwrap();
        for fault in 0..8 {
            let mut changed = image.clone();
            match fault {
                0 => {
                    changed.as_array_mut().unwrap().pop();
                }
                1 => {
                    changed
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::Value::Null);
                }
                2 => changed[live] = serde_json::Value::Null,
                3 => changed[absent] = image[live].clone(),
                4 => changed[live]["evidence"] = serde_json::json!([]),
                5 => {
                    let bits = changed[live]["evidence"][0][1].as_u64().unwrap();
                    changed[live]["evidence"][0][1] = (bits ^ 1).into();
                }
                6 => changed[live]["dimensions"] = serde_json::json!([{}]),
                _ => changed[live]["kind"] = serde_json::json!("Wildcard"),
            }
            let json = serde_json::to_string(&changed).unwrap();
            assert!(
                NativeDescriptorTemplatePool::from_independent_schema_templates(
                    BuildOwnedImageRecord::Json(&json),
                    &schemas
                )
                .is_err(),
                "fault {fault}"
            );
        }
    }
}
