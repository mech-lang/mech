use std::collections::BTreeSet;

use mech_core::{MResult, MatrixStorage, MechError, RuntimeType};

use crate::error::{NativeBuildErrorKind, native_build_error};

/// The exact Phase 1 value and shape features selected by a bytecode type
/// table.  The same sorted feature vector is applied to `mech-core`,
/// `mech-engine`, and (for hosted applications) `mech-runtime`.  Selected
/// machine packages merge these features with their function linkage features.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeTypeAnalysis {
    pub runtime_types: Vec<RuntimeType>,
    pub cargo_features: Vec<String>,
}

pub(crate) fn analyze_runtime_types(types: &[RuntimeType]) -> MResult<RuntimeTypeAnalysis> {
    let runtime_types = types.iter().cloned().collect::<BTreeSet<_>>();
    let mut cargo_features = BTreeSet::new();
    for runtime_type in &runtime_types {
        cargo_features.extend(
            features_for_runtime_type(runtime_type)?
                .iter()
                .map(|feature| (*feature).to_owned()),
        );
    }

    Ok(RuntimeTypeAnalysis {
        runtime_types: runtime_types.into_iter().collect(),
        cargo_features: cargo_features.into_iter().collect(),
    })
}

fn features_for_runtime_type(runtime_type: &RuntimeType) -> MResult<&'static [&'static str]> {
    match runtime_type {
        RuntimeType::Empty | RuntimeType::Index => Ok(&[]),
        RuntimeType::Bool => Ok(&["bool"]),
        RuntimeType::String => Ok(&["string"]),
        RuntimeType::F64 => Ok(&["f64"]),
        RuntimeType::Matrix {
            element, storage, ..
        } if element.as_ref() == &RuntimeType::F64 => Ok(match storage {
            MatrixStorage::Matrix1 => &["f64", "matrix1"],
            MatrixStorage::Matrix2 => &["f64", "matrix2"],
            MatrixStorage::Matrix3 => &["f64", "matrix3"],
            MatrixStorage::Matrix4 => &["f64", "matrix4"],
            MatrixStorage::Matrix2x3 => &["f64", "matrix2x3"],
            MatrixStorage::Matrix3x2 => &["f64", "matrix3x2"],
            MatrixStorage::RowVector2 => &["f64", "row_vector2"],
            MatrixStorage::RowVector3 => &["f64", "row_vector3"],
            MatrixStorage::RowVector4 => &["f64", "row_vector4"],
            MatrixStorage::Vector2 => &["f64", "vector2"],
            MatrixStorage::Vector3 => &["f64", "vector3"],
            MatrixStorage::Vector4 => &["f64", "vector4"],
            MatrixStorage::RowVectorD => &["f64", "row_vectord"],
            MatrixStorage::VectorD => &["f64", "vectord"],
            MatrixStorage::MatrixD => &["f64", "matrixd"],
        }),
        _ => Err(unsupported_runtime_type(runtime_type)),
    }
}

fn unsupported_runtime_type(runtime_type: &RuntimeType) -> MechError {
    native_build_error(
        NativeBuildErrorKind::NativeRuntimeTypeUnsupported {
            runtime_type: format!("{runtime_type:?}"),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_supported_scalar_features_exactly() {
        let analysis = analyze_runtime_types(&[
            RuntimeType::String,
            RuntimeType::Empty,
            RuntimeType::Bool,
            RuntimeType::Index,
            RuntimeType::F64,
            RuntimeType::Bool,
        ])
        .unwrap();

        assert_eq!(analysis.cargo_features, ["bool", "f64", "string"]);
        assert_eq!(analysis.runtime_types.len(), 5);
    }

    #[test]
    fn maps_every_f64_matrix_shape_to_its_exact_feature() {
        let cases = [
            (MatrixStorage::Matrix1, 1, 1, "matrix1"),
            (MatrixStorage::Matrix2, 2, 2, "matrix2"),
            (MatrixStorage::Matrix3, 3, 3, "matrix3"),
            (MatrixStorage::Matrix4, 4, 4, "matrix4"),
            (MatrixStorage::Matrix2x3, 2, 3, "matrix2x3"),
            (MatrixStorage::Matrix3x2, 3, 2, "matrix3x2"),
            (MatrixStorage::RowVector2, 1, 2, "row_vector2"),
            (MatrixStorage::RowVector3, 1, 3, "row_vector3"),
            (MatrixStorage::RowVector4, 1, 4, "row_vector4"),
            (MatrixStorage::Vector2, 2, 1, "vector2"),
            (MatrixStorage::Vector3, 3, 1, "vector3"),
            (MatrixStorage::Vector4, 4, 1, "vector4"),
            (MatrixStorage::RowVectorD, 1, 7, "row_vectord"),
            (MatrixStorage::VectorD, 7, 1, "vectord"),
            (MatrixStorage::MatrixD, 7, 9, "matrixd"),
        ];

        for (storage, rows, cols, expected) in cases {
            let runtime_type = RuntimeType::Matrix {
                element: Box::new(RuntimeType::F64),
                storage,
                rows,
                cols,
            };
            assert_eq!(
                features_for_runtime_type(&runtime_type).unwrap(),
                ["f64", expected],
                "{storage:?}"
            );
        }
    }

    #[test]
    fn official_type_without_phase1_analysis_is_structured() {
        let error = analyze_runtime_types(&[RuntimeType::F32]).unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeTypeUnsupported");
    }
}
