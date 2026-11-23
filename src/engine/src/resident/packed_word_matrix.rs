//! One admitted packed copy through the core's existing checked constructors.
use mech_core::{ResidentKernelError, SchemaId, SchemaTable, ShapeInstance, Value, ValueData};
use std::sync::Arc;

pub(super) fn copy(
    schema: SchemaId,
    shape: ShapeInstance,
    data: &ValueData,
    schemas: Arc<SchemaTable>,
) -> Result<Value, ResidentKernelError> {
    use mech_core::snapshot::SequenceView;
    let ValueData::Matrix(matrix) = data else {
        return Err(ResidentKernelError::InvalidInput);
    };
    // The caller prepays payload and owner construction before entering here.
    // Core validates the exact kind, current shape and cardinality.
    match matrix.elements() {
        SequenceView::U8(words) => Value::from_u8_matrix(schema, shape, words.into(), schemas),
        SequenceView::U16(words) => Value::from_u16_matrix(schema, shape, words.into(), schemas),
        SequenceView::U32(words) => Value::from_u32_matrix(schema, shape, words.into(), schemas),
        SequenceView::U64(words) => Value::from_u64_matrix(schema, shape, words.into(), schemas),
        SequenceView::U128(words) => Value::from_u128_matrix(schema, shape, words.into(), schemas),
        _ => return Err(ResidentKernelError::InvalidInput),
    }
    .map_err(|_| ResidentKernelError::InvalidOutput)
}
