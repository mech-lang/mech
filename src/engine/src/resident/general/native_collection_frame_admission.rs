//! A successful-path recipe for ONE private flat frame. No retained arguments,
//! extents, debit, permit, owner pin or previous-visit result is cached here.
use super::NativePreparedLibraryCall;
use crate::{
    native_artifact::{NativeScalarKind, NativeScalarValue as Scalar},
    resident::budget::{
        BORROWED_FOOTPRINT_METADATA_WORK, ResidentBudgetMeter, projected_canonical_value_footprint,
    },
};
use mech_core::{
    ManagedMemoryBudget, ResidentKernelError, SchemaBody, SchemaId, SchemaTable, Value, ValueData,
    snapshot::{SequenceView, ValueFootprint, visit_canonical_data_work},
};

#[derive(Debug)]
pub(super) struct FrameAdmissionRecipe {
    fields: Box<[Field]>,
}

#[derive(Debug)]
struct Field {
    schema: SchemaId,
    layout: Layout,
    schema_work: u64,
}

#[derive(Debug)]
enum Layout {
    Scalar(NativeScalarKind),
    Packed(NativeScalarKind),
}

impl Layout {
    fn accepts(&self, data: &ValueData) -> bool {
        let Self::Packed(kind) = self else {
            return true;
        };
        let ValueData::Matrix(matrix) = data else {
            return false;
        };
        matches!(
            (kind, matrix.elements()),
            (NativeScalarKind::Bool, SequenceView::Bool(_))
                | (NativeScalarKind::U8, SequenceView::U8(_))
                | (NativeScalarKind::U16, SequenceView::U16(_))
                | (NativeScalarKind::U32, SequenceView::U32(_))
                | (NativeScalarKind::U64, SequenceView::U64(_))
                | (NativeScalarKind::I64, SequenceView::I64(_))
                | (NativeScalarKind::F64, SequenceView::F64(_))
        )
    }
}

impl FrameAdmissionRecipe {
    pub(super) fn derive(inputs: &[SchemaId], schemas: &SchemaTable) -> Option<Self> {
        let fields = inputs
            .iter()
            .map(|id| {
                let schema = schemas.get(*id)?;
                let (layout, schema_nodes) = match schema.body() {
                    body if schema.dimension_parameters().is_empty()
                        && NativeScalarKind::from_schema_body(body).is_some() =>
                    {
                        (Layout::Scalar(NativeScalarKind::from_schema_body(body)?), 1)
                    }
                    SchemaBody::Matrix { element, .. } => (
                        Layout::Packed(NativeScalarKind::from_schema_body(element)?),
                        2,
                    ),
                    _ => return None,
                };
                Some(Field {
                    schema: *id,
                    layout,
                    // Same outer borrowed-value inspection and schema variants as
                    // the ordinary footprint visitor. DATA chunks are added live.
                    schema_work: (schema_nodes + 1) * BORROWED_FOOTPRINT_METADATA_WORK,
                })
            })
            .collect::<Option<Box<[_]>>>()?;
        Some(Self { fields })
    }

    pub(super) fn try_admit(
        &self,
        call: &NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        words: Option<&[u64]>,
        memory: &ManagedMemoryBudget,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<Option<u64>, ResidentKernelError> {
        // The enclosing selected call has already paid its metadata preflight
        // and reserved its original constructor envelope. This preview reads
        // only a fixed number of headers per flat scalar/packed numeric field;
        // Core supplies actual payload CAPACITY, not merely its logical length.
        let Some((logical, work, nodes)) = self.measure(call, inputs, words, memory) else {
            return Ok(None);
        };
        if !meter.try_charge_borrowed_footprint(work, nodes)? {
            return Ok(None);
        }
        Ok(Some(logical))
    }

    fn measure(
        &self,
        call: &NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        words: Option<&[u64]>,
        memory: &ManagedMemoryBudget,
    ) -> Option<(u64, u64, u64)> {
        if inputs.len() != self.fields.len() {
            return None;
        }
        let (mut logical, mut work, mut nodes) = (0u64, 0u64, 0u64);
        for (index, (field, input)) in self.fields.iter().zip(inputs).enumerate() {
            let schema = call.schemas.get(field.schema)?;
            let word_data;
            let (data, parameters) = if let Some(value) = input {
                if words.is_some_and(|words| words[index] != 0)
                    || value.schema() != field.schema
                    || value.schema_key() != call.input_keys[index]
                    || value.memory_budget_retained_bytes(memory).is_none()
                    || value.validate_against(&call.schemas).is_err()
                    || !field.layout.accepts(value.data())
                {
                    return None;
                }
                (value.data(), value.shape().parameter_values().len())
            } else {
                let Layout::Scalar(kind) = field.layout else {
                    return None;
                };
                let word = Scalar::from_bits(kind, words?[index])?;
                word_data = match word {
                    Scalar::Bool(v) => ValueData::Bool(v),
                    Scalar::U8(v) => ValueData::U8(v),
                    Scalar::U16(v) => ValueData::U16(v),
                    Scalar::U32(v) => ValueData::U32(v),
                    Scalar::U64(v) => ValueData::U64(v),
                    Scalar::I64(v) => ValueData::I64(v),
                    Scalar::F64(v) => ValueData::F64(v),
                };
                (&word_data, 0)
            };
            let mut footprint = ValueFootprint::zero();
            let mut field_work = field.schema_work;
            // For these proven layouts this is one scalar chunk or one matrix
            // header plus one packed-sequence chunk, never a payload traversal.
            visit_canonical_data_work(schema.body(), data, |chunk| {
                field_work = field_work
                    .checked_add(
                        BORROWED_FOOTPRINT_METADATA_WORK
                            .checked_mul(chunk.node_count.max(1))
                            .ok_or(())?,
                    )
                    .ok_or(())?;
                footprint = footprint
                    .checked_add(ValueFootprint {
                        encoded_bytes: chunk.encoded_bytes,
                        retained_bytes: chunk.retained_bytes,
                        node_count: chunk.node_count,
                    })
                    .map_err(|_| ())?;
                Ok::<_, ()>(())
            })
            .ok()?;
            let footprint = projected_canonical_value_footprint(footprint, parameters).ok()?;
            logical = logical.checked_add(footprint.retained_bytes)?;
            work = work.checked_add(field_work)?;
            nodes = nodes.checked_add(footprint.node_count)?;
        }
        Some((logical, work, nodes))
    }
}
