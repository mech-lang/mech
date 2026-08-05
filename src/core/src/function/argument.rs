#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

use core::any::type_name;

#[cfg(feature = "matrix")]
use crate::structures::Matrix;
use crate::{MResult, MechError, MechErrorKind, ReactiveCellId, Ref, Value};

/// Identifies the argument whose exact runtime representation was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionArgumentRole {
    Output,
    Input(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionArgumentTypeMismatch {
    pub role: FunctionArgumentRole,
    pub expected: String,
    pub found: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionMatrixRepresentation {
    Matrix1,
    Matrix2,
    Matrix3,
    Matrix4,
    Matrix2x3,
    Matrix3x2,
    RowVector2,
    RowVector3,
    RowVector4,
    Vector2,
    Vector3,
    Vector4,
    RowVectorD,
    VectorD,
    MatrixD,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionMatrixDescriptor {
    pub representation: FunctionMatrixRepresentation,
    pub rows: usize,
    pub cols: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionArgumentAliasViolation {
    pub input: usize,
    pub cell: ReactiveCellId,
}

impl MechErrorKind for FunctionArgumentAliasViolation {
    fn name(&self) -> &str {
        "FunctionArgumentAliasViolation"
    }

    fn message(&self) -> String {
        format!(
            "function output aliases input {} through reactive root cell {}",
            self.input,
            self.cell.get(),
        )
    }
}

#[cfg(feature = "matrix")]
fn matrix_descriptor<T>(matrix: &Matrix<T>) -> FunctionMatrixDescriptor
where
    T: core::fmt::Debug + Clone + PartialEq + 'static,
{
    let representation = match matrix {
        #[cfg(feature = "matrix1")]
        Matrix::Matrix1(_) => FunctionMatrixRepresentation::Matrix1,
        #[cfg(feature = "matrix2")]
        Matrix::Matrix2(_) => FunctionMatrixRepresentation::Matrix2,
        #[cfg(feature = "matrix3")]
        Matrix::Matrix3(_) => FunctionMatrixRepresentation::Matrix3,
        #[cfg(feature = "matrix4")]
        Matrix::Matrix4(_) => FunctionMatrixRepresentation::Matrix4,
        #[cfg(feature = "matrix2x3")]
        Matrix::Matrix2x3(_) => FunctionMatrixRepresentation::Matrix2x3,
        #[cfg(feature = "matrix3x2")]
        Matrix::Matrix3x2(_) => FunctionMatrixRepresentation::Matrix3x2,
        #[cfg(feature = "row_vector2")]
        Matrix::RowVector2(_) => FunctionMatrixRepresentation::RowVector2,
        #[cfg(feature = "row_vector3")]
        Matrix::RowVector3(_) => FunctionMatrixRepresentation::RowVector3,
        #[cfg(feature = "row_vector4")]
        Matrix::RowVector4(_) => FunctionMatrixRepresentation::RowVector4,
        #[cfg(feature = "vector2")]
        Matrix::Vector2(_) => FunctionMatrixRepresentation::Vector2,
        #[cfg(feature = "vector3")]
        Matrix::Vector3(_) => FunctionMatrixRepresentation::Vector3,
        #[cfg(feature = "vector4")]
        Matrix::Vector4(_) => FunctionMatrixRepresentation::Vector4,
        #[cfg(feature = "row_vectord")]
        Matrix::RowDVector(_) => FunctionMatrixRepresentation::RowVectorD,
        #[cfg(feature = "vectord")]
        Matrix::DVector(_) => FunctionMatrixRepresentation::VectorD,
        #[cfg(feature = "matrixd")]
        Matrix::DMatrix(_) => FunctionMatrixRepresentation::MatrixD,
    };
    FunctionMatrixDescriptor {
        representation,
        rows: matrix.rows(),
        cols: matrix.cols(),
    }
}

impl Value {
    pub fn function_matrix_descriptor(
        &self,
        role: FunctionArgumentRole,
    ) -> MResult<Option<FunctionMatrixDescriptor>> {
        let descriptor = match self {
            #[cfg(feature = "matrix")]
            Value::MatrixIndex(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            Value::MatrixBool(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            Value::MatrixU8(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            Value::MatrixU16(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            Value::MatrixU32(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            Value::MatrixU64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            Value::MatrixU128(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            Value::MatrixI8(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            Value::MatrixI16(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            Value::MatrixI32(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            Value::MatrixI64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            Value::MatrixI128(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            Value::MatrixF32(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            Value::MatrixF64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "string"))]
            Value::MatrixString(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            Value::MatrixR64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            Value::MatrixC64(matrix) => Some(matrix_descriptor(matrix)),
            #[cfg(feature = "matrix")]
            Value::MatrixValue(matrix) => Some(matrix_descriptor(matrix)),
            Value::Typed(_, _) | Value::MutableReference(_) => {
                return Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role,
                        expected: "an unwrapped scalar, nonmatrix, or exact matrix backing"
                            .to_string(),
                        found: self.exact_runtime_representation_name(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            _ => None,
        };
        Ok(descriptor)
    }
}

impl MechErrorKind for FunctionArgumentTypeMismatch {
    fn name(&self) -> &str {
        "FunctionArgumentTypeMismatch"
    }

    fn message(&self) -> String {
        format!(
            "function argument {:?} requires exact runtime representation {}, found {}",
            self.role, self.expected, self.found,
        )
    }
}

/// Extracts only an exact `Ref<T>` backing representation.
///
/// This deliberately performs no scalar conversion, matrix reshaping, or
/// unwrapping of `Typed` and `MutableReference` values.
pub fn require_function_ref<T: 'static>(
    value: &Value,
    role: FunctionArgumentRole,
) -> MResult<Ref<T>> {
    value
        .exact_ref_any()
        .and_then(|backing| backing.downcast_ref::<Ref<T>>())
        .cloned()
        .ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role,
                    expected: type_name::<Ref<T>>().to_string(),
                    found: value.exact_runtime_representation_name(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToValue;

    #[cfg(feature = "f64")]
    #[test]
    fn exact_scalar_refs_are_accepted_without_conversion() {
        let source = Ref::new(1.5_f64);
        let extracted =
            require_function_ref::<f64>(&source.to_value(), FunctionArgumentRole::Input(0))
                .unwrap();
        assert!(source.same_handle(&extracted));

        #[cfg(feature = "i8")]
        {
            let error = require_function_ref::<f64>(
                &Value::I8(Ref::new(1)),
                FunctionArgumentRole::Input(0),
            )
            .unwrap_err();
            assert_eq!(error.kind_name(), "FunctionArgumentTypeMismatch");
            let mismatch = error.kind_as::<FunctionArgumentTypeMismatch>().unwrap();
            assert_eq!(mismatch.role, FunctionArgumentRole::Input(0));
            assert!(mismatch.expected.contains("f64"));
            assert!(mismatch.found.contains("i8"));
        }
    }

    #[cfg(feature = "f64")]
    #[test]
    fn wrappers_are_not_implicitly_unwrapped() {
        let scalar = Ref::new(2.0_f64).to_value();
        let typed = Value::Typed(Box::new(scalar), crate::ValueKind::F64);
        assert!(require_function_ref::<f64>(&typed, FunctionArgumentRole::Output).is_err());

        let mutable = Value::MutableReference(Ref::new(Ref::new(2.0_f64).to_value()));
        assert!(require_function_ref::<f64>(&mutable, FunctionArgumentRole::Output).is_err());
        assert!(require_function_ref::<Value>(&mutable, FunctionArgumentRole::Output).is_ok());
    }

    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        feature = "matrix2",
        feature = "matrixd"
    ))]
    #[test]
    fn matrix_storage_is_part_of_the_exact_contract() {
        use crate::matrix::Matrix;
        use nalgebra::{DMatrix, Matrix2};

        let fixed = Value::MatrixF64(Matrix::Matrix2(Ref::new(Matrix2::identity())));
        let dynamic = Value::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::identity(2, 2))));

        assert!(require_function_ref::<Matrix2<f64>>(&fixed, FunctionArgumentRole::Output).is_ok());
        assert!(
            require_function_ref::<DMatrix<f64>>(&fixed, FunctionArgumentRole::Output).is_err()
        );
        assert!(
            require_function_ref::<Matrix2<f64>>(&dynamic, FunctionArgumentRole::Output).is_err()
        );
    }
}
