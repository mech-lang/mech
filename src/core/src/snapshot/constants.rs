use super::{SnapshotValueError, Value};
use crate::{
    ConstantId, SchemaKey, SchemaTable, SemanticIdentityKind, SemanticModelError, ValueHash,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, vec, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// An ephemeral ordinal-plus-[`ValueHash`] handle.
///
/// A handle may resolve against another build result when both its ordinal and
/// value hash match that build result. It is not an authority token and does
/// not prove builder identity. Different-content and out-of-range handles are
/// rejected.
pub struct ConstantHandle {
    ordinal: u32,
    hash: ValueHash,
}

#[derive(Debug)]
pub struct ConstantStoreBuilder<'a> {
    schemas: &'a SchemaTable,
    pending: Vec<PendingConstant>,
}

#[derive(Debug)]
pub struct ConstantStoreBuild {
    pub store: ConstantStore,
    remap: Box<[ConstantId]>,
    handle_hashes: Box<[ValueHash]>,
}

#[derive(Clone, Debug)]
pub struct ConstantStore {
    entries: Box<[ConstantEntry]>,
}

#[derive(Clone, Debug)]
pub struct ConstantEntry {
    value: Value,
    hash: ValueHash,
}

#[derive(Debug)]
struct PendingConstant {
    ordinal: u32,
    value: Value,
    hash: ValueHash,
    material: CanonicalConstantMaterial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalConstantMaterial {
    schema: SchemaKey,
    shape: Box<[u8]>,
    payload: Box<[u8]>,
}

impl Ord for CanonicalConstantMaterial {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.schema
            .cmp(&other.schema)
            .then_with(|| self.shape.cmp(&other.shape))
            .then_with(|| self.payload.cmp(&other.payload))
    }
}

impl PartialOrd for CanonicalConstantMaterial {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> ConstantStoreBuilder<'a> {
    pub const fn new(schemas: &'a SchemaTable) -> Self {
        Self {
            schemas,
            pending: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: Value) -> Result<ConstantHandle, SnapshotValueError> {
        value.validate_against(self.schemas)?;
        let ordinal = u32::try_from(self.pending.len()).map_err(|_| {
            SemanticModelError::IdentityExhausted {
                identity: SemanticIdentityKind::ConstantHandle,
            }
        })?;
        let hash = value.value_hash(self.schemas)?;
        let material = CanonicalConstantMaterial {
            schema: value.schema_key(),
            shape: value.shape().canonical_bytes(),
            payload: value.canonical_payload_bytes(self.schemas)?,
        };
        self.pending.push(PendingConstant {
            ordinal,
            value,
            hash,
            material,
        });
        Ok(ConstantHandle { ordinal, hash })
    }

    pub fn finish(self) -> Result<ConstantStoreBuild, SnapshotValueError> {
        self.finish_with_hash(|pending| pending.hash)
    }

    fn finish_with_hash(
        mut self,
        mut hash_for: impl FnMut(&PendingConstant) -> ValueHash,
    ) -> Result<ConstantStoreBuild, SnapshotValueError> {
        self.pending
            .sort_by(|left, right| left.material.cmp(&right.material));

        let mut seen_hashes: BTreeMap<ValueHash, CanonicalConstantMaterial> = BTreeMap::new();
        let mut remap = vec![ConstantId::new(0); self.pending.len()];
        let mut handle_hashes = vec![ValueHash::from_bytes([0; 32]); self.pending.len()];
        let mut entries: Vec<ConstantEntry> = Vec::new();
        let mut previous_material: Option<CanonicalConstantMaterial> = None;
        let mut previous_id = ConstantId::new(0);

        for pending in self.pending {
            let hash = hash_for(&pending);
            if let Some(existing) = seen_hashes.get(&hash) {
                if existing != &pending.material {
                    return Err(SnapshotValueError::ValueHashCollision { hash });
                }
            } else {
                seen_hashes.insert(hash, pending.material.clone());
            }

            let id = if previous_material.as_ref() == Some(&pending.material) {
                previous_id
            } else {
                let raw = u32::try_from(entries.len()).map_err(|_| {
                    SemanticModelError::IdentityExhausted {
                        identity: SemanticIdentityKind::ConstantId,
                    }
                })?;
                let id = ConstantId::new(raw);
                entries.push(ConstantEntry {
                    value: pending.value,
                    hash,
                });
                previous_material = Some(pending.material);
                previous_id = id;
                id
            };
            let ordinal = pending.ordinal as usize;
            remap[ordinal] = id;
            handle_hashes[ordinal] = hash;
        }

        Ok(ConstantStoreBuild {
            store: ConstantStore {
                entries: entries.into_boxed_slice(),
            },
            remap: remap.into_boxed_slice(),
            handle_hashes: handle_hashes.into_boxed_slice(),
        })
    }
}

impl ConstantStoreBuild {
    pub fn resolve(&self, handle: ConstantHandle) -> Result<ConstantId, SnapshotValueError> {
        let ordinal = handle.ordinal as usize;
        if self.handle_hashes.get(ordinal) != Some(&handle.hash) {
            return Err(SnapshotValueError::InvalidConstantHandleV1);
        }
        self.remap
            .get(ordinal)
            .copied()
            .ok_or(SnapshotValueError::InvalidConstantHandleV1)
    }

    pub fn into_parts(self) -> (ConstantStore, Box<[ConstantId]>) {
        (self.store, self.remap)
    }
}

impl ConstantStore {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: ConstantId) -> Option<&Value> {
        self.entry(id).map(ConstantEntry::value)
    }

    pub fn entry(&self, id: ConstantId) -> Option<&ConstantEntry> {
        self.entries.get(id.get() as usize)
    }
}

impl ConstantEntry {
    pub const fn value(&self) -> &Value {
        &self.value
    }

    pub const fn hash(&self) -> ValueHash {
        self.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SchemaBody, SchemaDraft, SchemaTableBuilder, ValueDataDraft, ValueDraft};

    fn bool_schema_table() -> (SchemaTable, crate::SchemaId) {
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
        (schemas, id)
    }

    fn bool_value(schemas: &SchemaTable, schema: crate::SchemaId, value: bool) -> Value {
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(value),
        }
        .finalize(&super::super::SnapshotValidationContext::new(schemas))
        .unwrap()
    }

    #[test]
    fn forced_hash_collision_rejects_unequal_snapshot_material() {
        let (schemas, schema) = bool_schema_table();
        let mut builder = ConstantStoreBuilder::new(&schemas);
        builder.insert(bool_value(&schemas, schema, false)).unwrap();
        builder.insert(bool_value(&schemas, schema, true)).unwrap();
        let forced = ValueHash::from_bytes([7; 32]);
        assert!(matches!(
            builder.finish_with_hash(|_| forced),
            Err(SnapshotValueError::ValueHashCollision { hash }) if hash == forced
        ));
    }
}
