//! Prepare all fallible String metadata before consuming a caller's backing.
use super::*;

/// An unpublished canonical String construction. Preparation validates its
/// schema/shape and funds the same allocations as ordinary finalization. No
/// Value or frozen identity exposes the private, empty metadata placeholder.
/// The caller retains its String until preparation has succeeded.
#[derive(Debug)]
pub struct StringSnapshotConstructor {
    schema: SchemaId,
    key: SchemaKey,
    length: usize,
    shape: Arc<ShapeInstance>,
    schemas: Arc<SchemaTable>,
    root: Arc<FrozenSnapshotStorage>,
}

impl StringSnapshotConstructor {
    pub fn new(
        schema: SchemaId,
        shape_values: Box<[u64]>,
        length: usize,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        let entry = context
            .schemas
            .entry(schema)
            .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
        let shape = entry.schema().instantiate_shape(shape_values)?;
        if !matches!(entry.schema().body(), SchemaBody::String) {
            return Err(data_mismatch_kind(
                entry.schema().body(),
                super::super::ValueDataKind::String,
                &SnapshotPath::root(),
            ));
        }
        Self::from_validated_shape(schema, entry.key(), shape, length, context)
    }

    pub(super) fn from_validated_shape(
        schema: SchemaId,
        key: SchemaKey,
        shape: ShapeInstance,
        length: usize,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Self, SnapshotValueError> {
        // Match the ordinary String finalizer's admission order exactly.
        context.admit_string_backing(length)?;
        let schemas = context.try_clone_schemas()?;
        let (shape, root) =
            prepared_snapshot_storage(shape, ValueData::String(Box::<str>::default()), context)?;
        Ok(Self {
            schema,
            key,
            length,
            shape,
            schemas,
            root,
        })
    }

    /// On error, return the caller's original String unchanged. On success,
    /// consume it only after all fallible metadata allocation is complete.
    pub fn finish(mut self, text: String) -> Result<Value, (String, SnapshotValueError)> {
        if text.len() != self.length {
            return Err((
                text,
                invalid_prepared_string("prepared String length changed"),
            ));
        }
        let Some(data) =
            Arc::get_mut(&mut self.root).and_then(|storage| Arc::get_mut(&mut storage.data))
        else {
            return Err((
                text,
                invalid_prepared_string("prepared String metadata was shared"),
            ));
        };
        // Neither Arc can escape this constructor. No canonical Value has
        // existed, so this initialization cannot mutate a frozen identity.
        data.data = ValueData::String(text.into_boxed_str());
        Ok(Value {
            schema: self.schema,
            schema_key: self.key,
            shape: self.shape,
            root: self.root,
            schemas: Some(self.schemas),
        })
    }
}

fn invalid_prepared_string(reason: &str) -> SnapshotValueError {
    crate::MemoryRuntimeError::CandidateValidationFailed {
        object: None,
        reason: reason.into(),
    }
    .into()
}

#[cfg(test)]
#[path = "string_constructor_tests.rs"]
mod tests;
