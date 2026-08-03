use std::collections::BTreeSet;

use mech_core::{MResult, MatrixStorage, RuntimeType};

/// The exact value and shape features selected by an official bytecode-v1 type
/// table. The same sorted feature vector is applied to `mech-core`,
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
        features_for_runtime_type(runtime_type, &mut cargo_features)?;
    }

    Ok(RuntimeTypeAnalysis {
        runtime_types: runtime_types.into_iter().collect(),
        cargo_features: cargo_features.into_iter().collect(),
    })
}

fn features_for_runtime_type(
    runtime_type: &RuntimeType,
    features: &mut BTreeSet<String>,
) -> MResult<()> {
    let add = |feature: &'static str, features: &mut BTreeSet<String>| {
        features.insert(feature.to_owned());
    };
    match runtime_type {
        RuntimeType::Empty
        | RuntimeType::Any
        | RuntimeType::None
        | RuntimeType::Id
        | RuntimeType::Index => {}
        RuntimeType::Bool => add("bool", features),
        RuntimeType::String => add("string", features),
        RuntimeType::U8 => add("u8", features),
        RuntimeType::U16 => add("u16", features),
        RuntimeType::U32 => add("u32", features),
        RuntimeType::U64 => add("u64", features),
        RuntimeType::U128 => add("u128", features),
        RuntimeType::I8 => add("i8", features),
        RuntimeType::I16 => add("i16", features),
        RuntimeType::I32 => add("i32", features),
        RuntimeType::I64 => add("i64", features),
        RuntimeType::I128 => add("i128", features),
        RuntimeType::F32 => add("f32", features),
        RuntimeType::F64 => add("f64", features),
        RuntimeType::C64 => add("c64", features),
        RuntimeType::R64 => add("r64", features),
        RuntimeType::Matrix {
            element, storage, ..
        } => {
            add(
                match storage {
                    MatrixStorage::Matrix1 => "matrix1",
                    MatrixStorage::Matrix2 => "matrix2",
                    MatrixStorage::Matrix3 => "matrix3",
                    MatrixStorage::Matrix4 => "matrix4",
                    MatrixStorage::Matrix2x3 => "matrix2x3",
                    MatrixStorage::Matrix3x2 => "matrix3x2",
                    MatrixStorage::RowVector2 => "row_vector2",
                    MatrixStorage::RowVector3 => "row_vector3",
                    MatrixStorage::RowVector4 => "row_vector4",
                    MatrixStorage::Vector2 => "vector2",
                    MatrixStorage::Vector3 => "vector3",
                    MatrixStorage::Vector4 => "vector4",
                    MatrixStorage::RowVectorD => "row_vectord",
                    MatrixStorage::VectorD => "vectord",
                    MatrixStorage::MatrixD => "matrixd",
                },
                features,
            );
            features_for_runtime_type(element, features)?;
        }
        RuntimeType::Record(fields) => {
            add("record", features);
            for (_, child) in fields {
                features_for_runtime_type(child, features)?;
            }
        }
        RuntimeType::Map { key, value } => {
            add("map", features);
            features_for_runtime_type(key, features)?;
            features_for_runtime_type(value, features)?;
        }
        RuntimeType::Set { element, .. } => {
            add("set", features);
            features_for_runtime_type(element, features)?;
        }
        RuntimeType::Table { columns, .. } => {
            add("table", features);
            for (_, child) in columns {
                features_for_runtime_type(child, features)?;
            }
        }
        RuntimeType::Tuple(children) => {
            add("tuple", features);
            for child in children {
                features_for_runtime_type(child, features)?;
            }
        }
        RuntimeType::Atom { .. } => add("atom", features),
        RuntimeType::Enum { .. } => add("enum", features),
        RuntimeType::Reference(child) => features_for_runtime_type(child, features)?,
        RuntimeType::Option(child) => features_for_runtime_type(child, features)?,
        RuntimeType::Kind(_) => add("kind_annotation", features),
    }
    Ok(())
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
            let mut features = BTreeSet::new();
            features_for_runtime_type(&runtime_type, &mut features).unwrap();
            assert_eq!(
                features.into_iter().collect::<Vec<_>>(),
                ["f64".to_owned(), expected.to_owned()],
                "{storage:?}"
            );
        }
    }

    #[test]
    fn recursively_selects_composite_child_features() {
        let analysis = analyze_runtime_types(&[RuntimeType::Map {
            key: Box::new(RuntimeType::U16),
            value: Box::new(RuntimeType::Tuple(vec![
                RuntimeType::String,
                RuntimeType::Matrix {
                    element: Box::new(RuntimeType::U8),
                    storage: MatrixStorage::Matrix2,
                    rows: 2,
                    cols: 2,
                },
            ])),
        }])
        .unwrap();

        assert_eq!(
            analysis.cargo_features,
            ["map", "matrix2", "string", "tuple", "u16", "u8"]
        );
    }
}
