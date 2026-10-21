//! Consume detached text through canonical finalization without a mutable cell.
use super::RuntimeHostStringError;
use mech_core::{
    MResult, MechError, SchemaBody, SchemaDraft, SchemaTableBuilder, Value,
    ValueCellSnapshotFailure,
    snapshot::{SnapshotValidationContext, StringSnapshotConstructor},
};

pub(super) fn snapshot(text: String) -> MResult<Value> {
    prepare(text.len())?
        .finish(text)
        .map_err(|(_, error)| snapshot_error(error))
}

fn prepare(length: usize) -> MResult<StringSnapshotConstructor> {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::String,
        }
        .finalize()?,
    )?;
    let built = builder.finish()?;
    let schema = built.resolve(handle)?;
    let (schemas, _) = built.into_parts();
    StringSnapshotConstructor::new(
        schema,
        Box::new([]),
        length,
        &SnapshotValidationContext::new(&schemas),
    )
    .map_err(snapshot_error)
}

fn snapshot_error(error: mech_core::snapshot::SnapshotValueError) -> MechError {
    MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
}

pub(super) fn snapshot_bytes(bytes: &mut Vec<u8>) -> Result<Value, RuntimeHostStringError> {
    snapshot_bytes_with_prepare(bytes, prepare)
}

fn snapshot_bytes_with_prepare(
    bytes: &mut Vec<u8>,
    prepare: impl FnOnce(usize) -> MResult<StringSnapshotConstructor>,
) -> Result<Value, RuntimeHostStringError> {
    let text = match String::from_utf8(core::mem::take(bytes)) {
        Ok(text) => text,
        Err(error) => {
            let diagnostic = error.utf8_error();
            *bytes = error.into_bytes();
            return Err(RuntimeHostStringError::InvalidUtf8(diagnostic));
        }
    };
    let prepared = match prepare(text.len()) {
        Ok(prepared) => prepared,
        Err(error) => {
            *bytes = text.into_bytes();
            return Err(RuntimeHostStringError::Snapshot(error));
        }
    };
    prepared.finish(text).map_err(|(text, error)| {
        *bytes = text.into_bytes();
        RuntimeHostStringError::Snapshot(snapshot_error(error))
    })
}

#[cfg(test)]
#[path = "input_string_tests.rs"]
mod tests;
