//! Snapshot conversion at the boundary of the compact resident representation.

use mech_core::snapshot::{F64Bits, SequenceView, SnapshotValidationContext};
use mech_core::{
    GenericError, MResult, MechError, ResidentShape, ResidentValueMut, ResidentValueRef,
    SchemaBody, SchemaId, ShapeInstance, Value, ValueData, ValueDataDraft, ValueDraft,
};

use crate::resident::general::{
    ReactiveInstance, ResidentActivationError, ResidentRegion, ResidentValueBorrow,
    TypedResidentArena,
};

impl ReactiveInstance {
    pub fn copied_output(&self, output: usize) -> Result<Value, ResidentActivationError> {
        let declaration = self
            .plan
            .outputs
            .get(output)
            .ok_or(ResidentActivationError::UnknownOutput { output })?;
        let borrowed = self
            .output_borrow(output)
            .ok_or(ResidentActivationError::UnknownOutput { output })?;
        let scalar = !matches!(
            self.plan
                .schemas
                .entry(declaration.schema)
                .expect("activated output schema remains present")
                .schema()
                .body(),
            SchemaBody::Matrix { .. }
        );
        let data = match borrowed {
            ResidentValueBorrow::Bool { values, .. } if scalar => {
                ValueDataDraft::Bool(values[0] != 0)
            }
            ResidentValueBorrow::Index { values, .. } if scalar => ValueDataDraft::Index(values[0]),
            ResidentValueBorrow::F64 { values, .. } if scalar => {
                ValueDataDraft::F64(F64Bits::from_f64(values[0]))
            }
            ResidentValueBorrow::Bool { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::Bool(values[index] != 0))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::Index { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::Index(values[index]))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::F64 { values, .. } => ValueDataDraft::Matrix(
                canonical_matrix_indices(declaration.region.shape)
                    .map(|index| ValueDataDraft::F64(F64Bits::from_f64(values[index])))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        };
        ValueDraft {
            schema: declaration.schema,
            shape_values: declaration
                .shape
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&self.plan.schemas))
        .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)
    }
}

pub(crate) fn materialize_resident_value(
    schemas: &mech_core::SchemaTable,
    schema: SchemaId,
    shape: &ShapeInstance,
    region: ResidentRegion,
    borrowed: ResidentValueRef<'_>,
) -> MResult<Value> {
    let scalar = !matches!(
        schemas
            .entry(schema)
            .expect("activated schema remains present")
            .schema()
            .body(),
        SchemaBody::Matrix { .. }
    );
    let data = match borrowed {
        ResidentValueRef::Bool(values) if scalar => ValueDataDraft::Bool(values[0] != 0),
        ResidentValueRef::Index(values) if scalar => ValueDataDraft::Index(values[0]),
        ResidentValueRef::F64(values) if scalar => {
            ValueDataDraft::F64(F64Bits::from_f64(values[0]))
        }
        ResidentValueRef::Bool(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::Bool(values[index] != 0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        ResidentValueRef::Index(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::Index(values[index]))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        ResidentValueRef::F64(values) => ValueDataDraft::Matrix(
            canonical_matrix_indices(region.shape)
                .map(|index| ValueDataDraft::F64(F64Bits::from_f64(values[index])))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    };
    ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|error| {
        MechError::new(
            GenericError {
                msg: format!("resident value materialization failed: {error:?}"),
            },
            None,
        )
    })
}

pub(crate) fn write_value(
    arena: &mut TypedResidentArena,
    region: ResidentRegion,
    value: &Value,
) -> Result<(), ResidentActivationError> {
    match (arena.write(region), value.data()) {
        (ResidentValueMut::Bool(target), ValueData::Bool(value)) if target.len() == 1 => {
            target[0] = u8::from(*value);
        }
        (ResidentValueMut::Index(target), ValueData::Index(value)) if target.len() == 1 => {
            target[0] = *value;
        }
        (ResidentValueMut::F64(target), ValueData::F64(value)) if target.len() == 1 => {
            target[0] = value.to_f64();
        }
        (ResidentValueMut::Bool(target), ValueData::Matrix(matrix)) => {
            let SequenceView::Bool(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = u8::from(source[canonical]);
            }
        }
        (ResidentValueMut::Index(target), ValueData::Matrix(matrix)) => {
            let SequenceView::Index(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = source[canonical];
            }
        }
        (ResidentValueMut::F64(target), ValueData::Matrix(matrix)) => {
            let SequenceView::F64(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (canonical, physical) in canonical_matrix_indices(region.shape).enumerate() {
                target[physical] = source[canonical].to_f64();
            }
        }
        _ => return Err(ResidentActivationError::InvalidSnapshotRepresentation),
    }
    Ok(())
}

fn canonical_matrix_indices(shape: ResidentShape) -> impl ExactSizeIterator<Item = usize> {
    let rows = shape.rows as usize;
    let columns = shape.columns as usize;
    (0..rows * columns).map(move |canonical| {
        let row = canonical / columns;
        let column = canonical % columns;
        column * rows + row
    })
}
