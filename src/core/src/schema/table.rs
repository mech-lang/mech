use super::Schema;
use crate::{SchemaId, SchemaKey, SemanticIdentityKind, SemanticModelError};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, vec, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SchemaHandle {
    ordinal: u32,
    key: SchemaKey,
}

#[derive(Clone, Debug, Default)]
pub struct SchemaTableBuilder {
    schemas: Vec<Schema>,
}

#[derive(Clone, Debug)]
pub struct SchemaTableBuild {
    pub table: SchemaTable,
    remap: Box<[SchemaId]>,
    handle_keys: Box<[SchemaKey]>,
}

#[derive(Clone, Debug)]
pub struct SchemaTable {
    entries: Box<[SchemaEntry]>,
}

#[derive(Clone, Debug)]
pub struct SchemaEntry {
    schema: Schema,
    key: SchemaKey,
    canonical_bytes: Box<[u8]>,
}

impl SchemaTableBuilder {
    pub const fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    pub fn insert(&mut self, schema: Schema) -> Result<SchemaHandle, SemanticModelError> {
        self.insert_with_limit(schema, usize::MAX)
    }

    fn insert_with_limit(
        &mut self,
        schema: Schema,
        handle_limit: usize,
    ) -> Result<SchemaHandle, SemanticModelError> {
        if self.schemas.len() >= handle_limit {
            return Err(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::SchemaHandle,
            });
        }
        let ordinal = u32::try_from(self.schemas.len()).map_err(|_| {
            SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::SchemaHandle,
            }
        })?;
        let handle = SchemaHandle {
            ordinal,
            key: schema.key(),
        };
        self.schemas.push(schema);
        Ok(handle)
    }

    pub fn finish(self) -> Result<SchemaTableBuild, SemanticModelError> {
        self.finish_with(u32::MAX as usize, Schema::key)
    }

    fn finish_with(
        self,
        unique_limit: usize,
        key_for: impl Fn(&Schema) -> SchemaKey,
    ) -> Result<SchemaTableBuild, SemanticModelError> {
        let handle_keys = self
            .schemas
            .iter()
            .map(Schema::key)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let mut pending = self
            .schemas
            .into_iter()
            .enumerate()
            .map(|(handle, schema)| (schema.canonical_bytes(), handle, schema))
            .collect::<Vec<_>>();
        pending.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

        let mut remap = vec![SchemaId::new(0); pending.len()];
        let mut entries: Vec<SchemaEntry> = Vec::new();
        let mut keys: BTreeMap<SchemaKey, Box<[u8]>> = BTreeMap::new();
        for (canonical_bytes, handle, schema) in pending {
            if entries
                .last()
                .is_some_and(|entry| entry.canonical_bytes == canonical_bytes)
            {
                remap[handle] = SchemaId::new((entries.len() - 1) as u32);
                continue;
            }
            if entries.len() >= unique_limit {
                return Err(SemanticModelError::SchemaIdExhausted);
            }
            let id = u32::try_from(entries.len())
                .map(SchemaId::new)
                .map_err(|_| SemanticModelError::SchemaIdExhausted)?;
            let key = key_for(&schema);
            if let Some(existing) = keys.get(&key) {
                if existing.as_ref() != canonical_bytes.as_ref() {
                    return Err(SemanticModelError::SchemaKeyCollision { key });
                }
            }
            keys.insert(key, canonical_bytes.clone());
            remap[handle] = id;
            entries.push(SchemaEntry {
                schema,
                key,
                canonical_bytes,
            });
        }
        Ok(SchemaTableBuild {
            table: SchemaTable {
                entries: entries.into_boxed_slice(),
            },
            remap: remap.into_boxed_slice(),
            handle_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntegerWidth, SchemaBody, SchemaDraft};

    fn schema(body: SchemaBody) -> Schema {
        SchemaDraft {
            dimension_parameters: Vec::new().into_boxed_slice(),
            body,
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn test_only_limit_proves_schema_id_exhaustion() {
        let mut builder = SchemaTableBuilder::new();
        builder.insert(schema(SchemaBody::Bool)).unwrap();
        builder
            .insert(schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)))
            .unwrap();
        assert!(matches!(
            builder.finish_with(1, Schema::key),
            Err(SemanticModelError::SchemaIdExhausted)
        ));
    }

    #[test]
    fn test_only_limit_proves_schema_handle_identity_exhaustion() {
        let mut builder = SchemaTableBuilder::new();
        builder
            .insert_with_limit(schema(SchemaBody::Bool), 1)
            .unwrap();
        assert!(matches!(
            builder.insert_with_limit(schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)), 1,),
            Err(SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::SchemaHandle,
            })
        ));
        assert_eq!(builder.schemas.len(), 1);
    }

    #[test]
    fn test_only_hash_hook_proves_collision_rejection() {
        let mut builder = SchemaTableBuilder::new();
        builder.insert(schema(SchemaBody::Bool)).unwrap();
        builder
            .insert(schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)))
            .unwrap();
        let forced = SchemaKey::from_bytes([7; 32]);
        assert!(matches!(
            builder.finish_with(u32::MAX as usize, |_| forced),
            Err(SemanticModelError::SchemaKeyCollision { key }) if key == forced
        ));
    }

    #[test]
    fn resolve_rejects_out_of_range_and_foreign_handles() {
        let bool_schema = schema(SchemaBody::Bool);
        let string_schema = schema(SchemaBody::String);

        let mut first = SchemaTableBuilder::new();
        let first_bool = first.insert(bool_schema.clone()).unwrap();
        let first = first.finish().unwrap();

        let mut second = SchemaTableBuilder::new();
        let second_string = second.insert(string_schema).unwrap();
        let second = second.finish().unwrap();

        assert!(first.resolve(first_bool).is_ok());
        assert!(matches!(
            first.resolve(second_string),
            Err(SemanticModelError::InvalidSchemaHandleV1)
        ));
        assert!(matches!(
            second.resolve(SchemaHandle {
                ordinal: u32::MAX,
                key: bool_schema.key(),
            }),
            Err(SemanticModelError::InvalidSchemaHandleV1)
        ));
    }
}

impl SchemaTableBuild {
    pub fn resolve(&self, handle: SchemaHandle) -> Result<SchemaId, SemanticModelError> {
        let ordinal = handle.ordinal as usize;
        if self.handle_keys.get(ordinal) != Some(&handle.key) {
            return Err(SemanticModelError::InvalidSchemaHandleV1);
        }
        self.remap
            .get(ordinal)
            .copied()
            .ok_or(SemanticModelError::InvalidSchemaHandleV1)
    }

    pub fn into_parts(self) -> (SchemaTable, Box<[SchemaId]>) {
        (self.table, self.remap)
    }
}

impl SchemaTable {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: SchemaId) -> Option<&Schema> {
        self.entry(id).map(SchemaEntry::schema)
    }

    pub fn entry(&self, id: SchemaId) -> Option<&SchemaEntry> {
        self.entries.get(id.get() as usize)
    }

    pub fn find_by_key(&self, key: SchemaKey) -> Option<SchemaId> {
        self.entries
            .iter()
            .position(|entry| entry.key == key)
            .map(|index| SchemaId::new(index as u32))
    }
}

impl SchemaEntry {
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    pub const fn key(&self) -> SchemaKey {
        self.key
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}
