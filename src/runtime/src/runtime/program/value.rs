use mech_core::{FloatWidth, LegacyValue, MResult, Ref, ResidentShape, SchemaBody, matrix::Matrix};
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
    let output = &instance.plan.outputs[output_index];
    let schema = instance
        .plan
        .schemas()
        .get(output.schema)
        .expect("activated output schema must exist");
    let legacy = match value {
        ResidentValueBorrow::Bool { shape: _, values }
            if values.len() == 1 && matches!(schema.body(), SchemaBody::Bool) =>
        {
            LegacyValue::Bool(Ref::new(values[0] != 0))
        }
        ResidentValueBorrow::Index { shape: _, values }
            if values.len() == 1 && matches!(schema.body(), SchemaBody::Index) =>
        {
            LegacyValue::Index(Ref::new(values[0] as usize))
        }
        ResidentValueBorrow::F64 { shape: _, values }
            if values.len() == 1
                && matches!(schema.body(), SchemaBody::FloatingPoint(FloatWidth::W64)) =>
        {
            LegacyValue::F64(Ref::new(values[0]))
        }
        ResidentValueBorrow::String { shape: _, values }
            if values.len() == 1 && matches!(schema.body(), SchemaBody::String) =>
        {
            LegacyValue::String(Ref::new(values[0].clone()))
        }
        ResidentValueBorrow::Bool { shape, values } => LegacyValue::MatrixBool(Matrix::from_vec(
            values.iter().map(|value| *value != 0).collect(),
            shape.rows as usize,
            shape.columns as usize,
        )),
        ResidentValueBorrow::Index { shape, values } => LegacyValue::MatrixIndex(Matrix::from_vec(
            values.iter().map(|value| *value as usize).collect(),
            shape.rows as usize,
            shape.columns as usize,
        )),
        ResidentValueBorrow::F64 { shape, values } => LegacyValue::MatrixF64(Matrix::from_vec(
            values.to_vec(),
            shape.rows as usize,
            shape.columns as usize,
        )),
        ResidentValueBorrow::String { shape, values } => string_matrix_value(shape, values),
        ResidentValueBorrow::Snapshot {
            values: [Some(value)],
            ..
        } => super::external::provider_value_from_canonical(value, instance.plan.schemas())?,
        ResidentValueBorrow::Snapshot { values: [None], .. } => return Ok(None),
        ResidentValueBorrow::Snapshot { .. } => {
            unreachable!("resident snapshot matrices are rejected during activation")
        }
    };
    RuntimeValueSnapshot::try_capture(&legacy).map(Some)
}

fn string_matrix_value(shape: ResidentShape, values: &[String]) -> LegacyValue {
    LegacyValue::MatrixString(Matrix::from_vec(
        values.to_vec(),
        shape.rows as usize,
        shape.columns as usize,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_string_matrix_output_materializes_without_scalar_fallback() {
        let value = string_matrix_value(
            ResidentShape {
                rows: 1,
                columns: 2,
            },
            &["Fizz".to_owned(), "Buzz".to_owned()],
        );
        let LegacyValue::MatrixString(matrix) = value else {
            panic!("resident string matrix output changed representation")
        };
        assert_eq!(matrix.as_vec(), ["Fizz", "Buzz"]);
    }
}
