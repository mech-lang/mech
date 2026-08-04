#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

use core::any::type_name;

use crate::{MResult, MechError, MechErrorKind, Ref, Value};

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
