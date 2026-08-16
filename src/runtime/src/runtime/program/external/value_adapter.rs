use mech_core::{
    LegacyMaterializationContext, LegacySnapshotError, LegacyValue, MResult, MechError,
    MechErrorKind, NominalKey, NominalKind, SchemaBody, SchemaId, SchemaTable, ShapeInstance,
    Value, legacy_from_snapshot,
    matrix::ToMatrix,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft},
};

pub fn captured_value_from_legacy(
    value: &LegacyValue,
    schema: SchemaId,
    shape: &ShapeInstance,
    schemas: &SchemaTable,
) -> MResult<Value> {
    let body = schemas
        .entry(schema)
        .ok_or_else(|| unsupported("captured schema is absent"))?
        .schema()
        .body();
    let data = legacy_data(value, body, shape)?;
    ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|error| unsupported(&format!("captured value does not match schema: {error:?}")))
}

pub fn provider_value_from_canonical(value: &Value, schemas: &SchemaTable) -> MResult<LegacyValue> {
    let mut context = ResidentLegacyMaterializationContext;
    let legacy = legacy_from_snapshot(value, schemas, &mut context).map_err(|error| {
        unsupported(&format!(
            "resident provider value cannot be materialized: {error:?}"
        ))
    })?;
    let Some(schema) = schemas.entry(value.schema()) else {
        return Err(unsupported("resident provider schema is absent"));
    };
    if matches!(
        schema.schema().body(),
        SchemaBody::Matrix { element, .. }
            if matches!(element.as_ref(), SchemaBody::FloatingPoint(mech_core::FloatWidth::W64))
    ) {
        let LegacyValue::MatrixValue(matrix) = legacy else {
            return Ok(legacy);
        };
        let shape = matrix.shape();
        let [rows, columns] = shape.as_slice() else {
            return Err(unsupported(
                "resident provider f64 matrix must have exactly two dimensions",
            ));
        };
        let values = matrix
            .as_vec()
            .into_iter()
            .map(|value| value.expect_f64().map(|value| *value.borrow()))
            .collect::<MResult<Vec<_>>>()?;
        return Ok(LegacyValue::MatrixF64(ToMatrix::to_matrixd(
            values, *rows, *columns,
        )));
    }
    Ok(legacy)
}

fn legacy_data(
    value: &LegacyValue,
    body: &SchemaBody,
    shape: &ShapeInstance,
) -> MResult<ValueDataDraft> {
    match body {
        SchemaBody::Bool => legacy_bool(value).map(ValueDataDraft::Bool),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            legacy_f64(value).map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
        }
        SchemaBody::Index => legacy_index(value).map(ValueDataDraft::Index),
        SchemaBody::String => legacy_string(value).map(ValueDataDraft::String),
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(
            element.as_ref(),
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
        ) =>
        {
            let values = canonical_matrix_values(value, dimensions, shape, legacy_f64_values)?;
            Ok(ValueDataDraft::Matrix(
                values
                    .into_iter()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ))
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(element.as_ref(), SchemaBody::Bool) => {
            let values = canonical_matrix_values(value, dimensions, shape, legacy_bool_values)?;
            Ok(ValueDataDraft::Matrix(
                values
                    .into_iter()
                    .map(ValueDataDraft::Bool)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ))
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(element.as_ref(), SchemaBody::Index) => {
            let values = canonical_matrix_values(value, dimensions, shape, legacy_index_values)?;
            Ok(ValueDataDraft::Matrix(
                values
                    .into_iter()
                    .map(ValueDataDraft::Index)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ))
        }
        _ => Err(unsupported(
            "resident provider adapter does not support this schema yet",
        )),
    }
}

fn canonical_matrix_values<T>(
    value: &LegacyValue,
    dimensions: &[mech_core::DimensionExpr],
    shape: &ShapeInstance,
    extract: impl FnOnce(&LegacyValue) -> MResult<Vec<T>>,
) -> MResult<Vec<T>>
where
    T: Clone,
{
    if dimensions.len() != 2 {
        return Err(unsupported(
            "resident provider matrices must have exactly two dimensions",
        ));
    }
    let expected = dimensions
        .iter()
        .map(|dimension| {
            shape
                .resolve_dimension(dimension)
                .map_err(|error| unsupported(&format!("matrix shape resolution failed: {error:?}")))
                .and_then(|extent| {
                    usize::try_from(extent)
                        .map_err(|_| unsupported("matrix extent does not fit usize"))
                })
        })
        .collect::<MResult<Vec<_>>>()?;
    let actual = value.shape();
    if actual != expected {
        return Err(unsupported(&format!(
            "provider matrix shape {actual:?} does not match required shape {expected:?}"
        )));
    }
    let rows = expected[0];
    let columns = expected[1];
    let column_major = extract(value)?;
    let mut row_major = Vec::with_capacity(column_major.len());
    for row in 0..rows {
        for column in 0..columns {
            row_major.push(column_major[column * rows + row].clone());
        }
    }
    Ok(row_major)
}

struct ResidentLegacyMaterializationContext;

impl LegacyMaterializationContext for ResidentLegacyMaterializationContext {
    fn resolve_nominal(
        &mut self,
        _kind: NominalKind,
        _key: NominalKey,
    ) -> Result<(u64, String), LegacySnapshotError> {
        Err(LegacySnapshotError::UnsupportedLegacyMaterialization)
    }
}

#[cfg(feature = "bool")]
fn legacy_bool(value: &LegacyValue) -> MResult<bool> {
    match value {
        LegacyValue::Bool(value) => Ok(*value.borrow()),
        _ => Err(unsupported("provider returned a non-bool value")),
    }
}

#[cfg(not(feature = "bool"))]
fn legacy_bool(_value: &LegacyValue) -> MResult<bool> {
    Err(unsupported("bool provider adapter feature is disabled"))
}

#[cfg(all(feature = "matrix", feature = "bool"))]
fn legacy_bool_values(value: &LegacyValue) -> MResult<Vec<bool>> {
    value.as_vecbool()
}

#[cfg(not(all(feature = "matrix", feature = "bool")))]
fn legacy_bool_values(_value: &LegacyValue) -> MResult<Vec<bool>> {
    Err(unsupported(
        "bool matrix provider adapter feature is disabled",
    ))
}

#[cfg(feature = "f64")]
fn legacy_f64(value: &LegacyValue) -> MResult<f64> {
    match value {
        LegacyValue::F64(value) => Ok(*value.borrow()),
        _ => Err(unsupported("provider returned a non-f64 value")),
    }
}

#[cfg(not(feature = "f64"))]
fn legacy_f64(_value: &LegacyValue) -> MResult<f64> {
    Err(unsupported("f64 provider adapter feature is disabled"))
}

fn legacy_index(value: &LegacyValue) -> MResult<u64> {
    match value {
        LegacyValue::Index(value) => u64::try_from(*value.borrow())
            .map_err(|_| unsupported("provider index does not fit canonical u64")),
        _ => Err(unsupported("provider returned a non-index value")),
    }
}

fn legacy_string(value: &LegacyValue) -> MResult<String> {
    match value {
        LegacyValue::String(value) => Ok(value.borrow().clone()),
        _ => Err(unsupported("provider returned a non-string value")),
    }
}

#[cfg(feature = "matrix")]
fn legacy_index_values(value: &LegacyValue) -> MResult<Vec<u64>> {
    value
        .as_vecusize()?
        .into_iter()
        .map(|value| {
            u64::try_from(value)
                .map_err(|_| unsupported("provider matrix index does not fit canonical u64"))
        })
        .collect()
}

#[cfg(not(feature = "matrix"))]
fn legacy_index_values(_value: &LegacyValue) -> MResult<Vec<u64>> {
    Err(unsupported(
        "index matrix provider adapter feature is disabled",
    ))
}

#[cfg(all(feature = "matrix", feature = "f64"))]
fn legacy_f64_values(value: &LegacyValue) -> MResult<Vec<f64>> {
    value.as_vecf64()
}

#[cfg(not(all(feature = "matrix", feature = "f64")))]
fn legacy_f64_values(_value: &LegacyValue) -> MResult<Vec<f64>> {
    Err(unsupported(
        "f64 matrix provider adapter feature is disabled",
    ))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentValueAdapterUnsupported {
    pub reason: String,
}

impl MechErrorKind for ResidentValueAdapterUnsupported {
    fn name(&self) -> &str {
        "ResidentValueAdapterUnsupported"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}

fn unsupported(reason: &str) -> MechError {
    MechError::new(
        ResidentValueAdapterUnsupported {
            reason: reason.to_owned(),
        },
        None,
    )
}
