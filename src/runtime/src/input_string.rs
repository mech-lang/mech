//! Consume detached text through canonical finalization without a mutable cell.
use mech_core::{
    MResult, MechError, SchemaBody, SchemaDraft, SchemaTableBuilder, Value,
    ValueCellSnapshotFailure,
    snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft},
};

pub(super) fn snapshot(text: String) -> MResult<Value> {
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
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String(text),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .map_err(|error| MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc())
}
