use mech_core::{LegacyValue, MResult, Ref};
use mech_engine::resident::{ReactiveInstance, ResidentValueBorrow};

use crate::RuntimeValueSnapshot;

pub(crate) fn initial_value(instance: &ReactiveInstance) -> MResult<RuntimeValueSnapshot> {
    let Some(value) = instance.output_borrow(0) else {
        return Ok(RuntimeValueSnapshot::empty());
    };
    let legacy = match value {
        ResidentValueBorrow::Bool { shape: _, values } if values.len() == 1 => {
            LegacyValue::Bool(Ref::new(values[0] != 0))
        }
        ResidentValueBorrow::Index { shape: _, values } if values.len() == 1 => {
            LegacyValue::Index(Ref::new(values[0] as usize))
        }
        ResidentValueBorrow::F64 { shape: _, values } if values.len() == 1 => {
            LegacyValue::F64(Ref::new(values[0]))
        }
        ResidentValueBorrow::F64 { shape, values } => {
            use mech_core::matrix::ToMatrix;
            let rows = shape.rows as usize;
            let columns = shape.columns as usize;
            let mut column_major = vec![0.0; values.len()];
            for row in 0..rows {
                for column in 0..columns {
                    column_major[column * rows + row] = values[row * columns + column];
                }
            }
            LegacyValue::MatrixF64(ToMatrix::to_matrix(column_major, rows, columns))
        }
        ResidentValueBorrow::Bool { .. } | ResidentValueBorrow::Index { .. } => {
            return Ok(RuntimeValueSnapshot::empty());
        }
    };
    RuntimeValueSnapshot::try_capture(&legacy)
}
