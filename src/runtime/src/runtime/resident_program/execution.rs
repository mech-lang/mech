use mech_core::{LegacyValue, MResult, Ref, matrix::Matrix};
use mech_engine::resident::{ReactiveInstance, ResidentValueBorrow};

use crate::RuntimeValueSnapshot;

pub(crate) fn initial_value(instance: &ReactiveInstance) -> MResult<RuntimeValueSnapshot> {
    output_value(instance, 0).map(|value| value.unwrap_or_else(RuntimeValueSnapshot::empty))
}

pub(crate) fn output_value(
    instance: &ReactiveInstance,
    output_index: usize,
) -> MResult<Option<RuntimeValueSnapshot>> {
    let Some(value) = instance.output_borrow(output_index) else {
        return Ok(None);
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
        ResidentValueBorrow::Bool { shape, values } => LegacyValue::MatrixBool(Matrix::from_vec(
            to_column_major(
                &values.iter().map(|value| *value != 0).collect::<Vec<_>>(),
                shape.rows as usize,
                shape.columns as usize,
            ),
            shape.rows as usize,
            shape.columns as usize,
        )),
        ResidentValueBorrow::Index { shape, values } => LegacyValue::MatrixIndex(Matrix::from_vec(
            to_column_major(
                &values
                    .iter()
                    .map(|value| *value as usize)
                    .collect::<Vec<_>>(),
                shape.rows as usize,
                shape.columns as usize,
            ),
            shape.rows as usize,
            shape.columns as usize,
        )),
        ResidentValueBorrow::F64 { shape, values } => LegacyValue::MatrixF64(Matrix::from_vec(
            to_column_major(values, shape.rows as usize, shape.columns as usize),
            shape.rows as usize,
            shape.columns as usize,
        )),
    };
    RuntimeValueSnapshot::try_capture(&legacy).map(Some)
}

fn to_column_major<T: Copy>(values: &[T], rows: usize, columns: usize) -> Vec<T> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut column_major = vec![values[0]; values.len()];
    for row in 0..rows {
        for column in 0..columns {
            column_major[column * rows + row] = values[row * columns + column];
        }
    }
    column_major
}
