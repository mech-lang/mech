use std::collections::BTreeSet;

#[cfg(test)]
use mech_core::MatrixStorage;
use mech_core::{
    MResult, NativeValueFeature, RuntimeType, SchemaBody, SchemaTable,
    native_features_for_runtime_type,
};

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
    let mut native_features = BTreeSet::new();
    for runtime_type in &runtime_types {
        native_features_for_runtime_type(runtime_type, &mut native_features);
    }
    let cargo_features = native_features
        .into_iter()
        .map(NativeValueFeature::cargo_feature)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(RuntimeTypeAnalysis {
        runtime_types: runtime_types.into_iter().collect(),
        cargo_features,
    })
}

pub(crate) fn artifact_schema_features(schemas: &SchemaTable) -> Vec<String> {
    let mut native_features = BTreeSet::new();
    for entry in schemas.entries() {
        collect_schema_features(entry.schema().body(), &mut native_features);
    }
    native_features
        .into_iter()
        .map(NativeValueFeature::cargo_feature)
        .map(str::to_owned)
        .collect()
}

fn collect_schema_features(body: &SchemaBody, features: &mut BTreeSet<NativeValueFeature>) {
    use NativeValueFeature as Feature;
    match body {
        SchemaBody::Dynamic | SchemaBody::Id | SchemaBody::Index => {}
        SchemaBody::Bool => {
            features.insert(Feature::Bool);
        }
        SchemaBody::IntegerInterval(interval) => {
            collect_schema_features(&interval.base_body(), features);
        }
        SchemaBody::UnsignedInteger(width) => {
            features.insert(match width {
                mech_core::IntegerWidth::W8 => Feature::U8,
                mech_core::IntegerWidth::W16 => Feature::U16,
                mech_core::IntegerWidth::W32 => Feature::U32,
                mech_core::IntegerWidth::W64 => Feature::U64,
                mech_core::IntegerWidth::W128 => Feature::U128,
            });
        }
        SchemaBody::SignedInteger(width) => {
            features.insert(match width {
                mech_core::IntegerWidth::W8 => Feature::I8,
                mech_core::IntegerWidth::W16 => Feature::I16,
                mech_core::IntegerWidth::W32 => Feature::I32,
                mech_core::IntegerWidth::W64 => Feature::I64,
                mech_core::IntegerWidth::W128 => Feature::I128,
            });
        }
        SchemaBody::FloatingPoint(width) => {
            features.insert(match width {
                mech_core::FloatWidth::W32 => Feature::F32,
                mech_core::FloatWidth::W64 => Feature::F64,
            });
        }
        SchemaBody::Complex(_) => {
            features.insert(Feature::C64);
        }
        SchemaBody::Rational64 => {
            features.insert(Feature::R64);
        }
        SchemaBody::String => {
            features.insert(Feature::String);
        }
        SchemaBody::Atom(_) => {
            features.insert(Feature::Atom);
        }
        SchemaBody::Enum { variants, .. } => {
            features.insert(Feature::Enum);
            for payload in variants
                .iter()
                .filter_map(|variant| variant.payload.as_ref())
            {
                collect_schema_features(payload, features);
            }
        }
        SchemaBody::Option(inner) => collect_schema_features(inner, features),
        SchemaBody::Tuple(elements) => {
            if !elements.is_empty() {
                features.insert(Feature::Tuple);
            }
            for element in elements {
                collect_schema_features(element, features);
            }
        }
        SchemaBody::Record(fields) => {
            features.insert(Feature::Record);
            for field in fields {
                collect_schema_features(&field.schema, features);
            }
        }
        SchemaBody::Matrix { element, .. } => {
            features.insert(Feature::Matrix);
            collect_schema_features(element, features);
        }
        SchemaBody::Table { columns, .. } => {
            features.insert(Feature::Table);
            for column in columns {
                collect_schema_features(&column.schema, features);
            }
        }
        SchemaBody::Set { element, .. } => {
            features.insert(Feature::Set);
            collect_schema_features(element, features);
        }
        SchemaBody::Map { key, value, .. } => {
            features.insert(Feature::Map);
            collect_schema_features(key, features);
            collect_schema_features(value, features);
        }
        SchemaBody::ReifiedType => {
            features.insert(Feature::KindAnnotation);
        }
    }
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
            let features = analyze_runtime_types(&[runtime_type])
                .unwrap()
                .cargo_features;
            assert_eq!(
                features,
                ["f64".to_owned(), expected.to_owned()],
                "{storage:?}"
            );
        }
    }

    #[test]
    fn recursively_selects_composite_child_features() {
        let analysis = analyze_runtime_types(&[RuntimeType::Map {
            key: Box::new(RuntimeType::String),
            value: Box::new(RuntimeType::Tuple(vec![
                RuntimeType::F64,
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
            ["f64", "map", "matrix2", "string", "tuple", "u8"]
        );
    }
}
