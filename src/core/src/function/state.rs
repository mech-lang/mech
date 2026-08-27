//! Opaque capabilities over exact typed cells owned by runtime functions.

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(not(feature = "no_std"))]
use std::{fmt, string::String};

#[cfg(feature = "no_std")]
use core::fmt;

use crate::{FunctionPortBacking, FunctionRuntimeType, FunctionValueRepresentation, Ref, ToValue};

mod function_state_sealed {
    pub trait Sealed {}
}

/// An exact runtime backing whose clone is a complete independent checkpoint.
///
/// The sealed implementation list deliberately excludes universal values,
/// mutable value cells, and aggregates that can contain nested cell identity.
///
/// ```compile_fail
/// use mech_core::{FunctionStateBacking, LegacyValue};
/// fn require_state_backing<T: FunctionStateBacking>() {}
/// require_state_backing::<LegacyValue>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStateBacking, ValueCell};
/// fn require_state_backing<T: FunctionStateBacking>() {}
/// require_state_backing::<ValueCell>();
/// ```
pub trait FunctionStateBacking:
    function_state_sealed::Sealed + FunctionPortBacking + FunctionRuntimeType + Clone + 'static
{
}

impl<T> FunctionStateBacking for T where
    T: function_state_sealed::Sealed + FunctionPortBacking + FunctionRuntimeType + Clone + 'static
{
}

macro_rules! scalar_function_state_backing {
    ($type:ty, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl function_state_sealed::Sealed for $type {}
    };
}

scalar_function_state_backing!(u8, "u8");
scalar_function_state_backing!(u16, "u16");
scalar_function_state_backing!(u32, "u32");
scalar_function_state_backing!(u64, "u64");
scalar_function_state_backing!(u128, "u128");
scalar_function_state_backing!(i8, "i8");
scalar_function_state_backing!(i16, "i16");
scalar_function_state_backing!(i32, "i32");
scalar_function_state_backing!(i64, "i64");
scalar_function_state_backing!(i128, "i128");
scalar_function_state_backing!(f32, "f32");
scalar_function_state_backing!(f64, "f64");
scalar_function_state_backing!(bool, "bool");
scalar_function_state_backing!(String, "string");

impl function_state_sealed::Sealed for usize {}

#[cfg(feature = "complex")]
impl function_state_sealed::Sealed for crate::C64 {}

#[cfg(feature = "rational")]
impl function_state_sealed::Sealed for crate::R64 {}

macro_rules! exact_matrix_function_state_backing {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T: FunctionStateBacking> function_state_sealed::Sealed for crate::$type<T> {}
    };
}

exact_matrix_function_state_backing!(Matrix1, "matrix1");
exact_matrix_function_state_backing!(Matrix2, "matrix2");
exact_matrix_function_state_backing!(Matrix3, "matrix3");
exact_matrix_function_state_backing!(Matrix4, "matrix4");
exact_matrix_function_state_backing!(Matrix2x3, "matrix2x3");
exact_matrix_function_state_backing!(Matrix3x2, "matrix3x2");
exact_matrix_function_state_backing!(RowVector2, "row_vector2");
exact_matrix_function_state_backing!(RowVector3, "row_vector3");
exact_matrix_function_state_backing!(RowVector4, "row_vector4");
exact_matrix_function_state_backing!(RowDVector, "row_vectord");
exact_matrix_function_state_backing!(Vector2, "vector2");
exact_matrix_function_state_backing!(Vector3, "vector3");
exact_matrix_function_state_backing!(Vector4, "vector4");
exact_matrix_function_state_backing!(DVector, "vectord");
exact_matrix_function_state_backing!(DMatrix, "matrixd");

trait ErasedFunctionState {
    fn representation(&self) -> FunctionValueRepresentation;
}

impl<T> ErasedFunctionState for Ref<T>
where
    T: FunctionStateBacking,
    Ref<T>: ToValue,
{
    fn representation(&self) -> FunctionValueRepresentation {
        T::REPRESENTATION
    }
}

/// A borrowed, opaque capability over one exact typed function-owned cell.
///
/// State ports reveal only the cell's declared representation. They do not
/// expose its payload, typed reference, physical identity, or a legacy value.
#[derive(Clone, Copy)]
pub struct FunctionStatePort<'a> {
    inner: &'a dyn ErasedFunctionState,
}

impl<'a> FunctionStatePort<'a> {
    /// Borrows an exact typed cell without cloning either its handle or payload.
    pub fn from_ref<T>(reference: &'a Ref<T>) -> Self
    where
        T: FunctionStateBacking,
        Ref<T>: ToValue,
    {
        Self { inner: reference }
    }

    pub fn representation(self) -> FunctionValueRepresentation {
        self.inner.representation()
    }
}

impl fmt::Debug for FunctionStatePort<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FunctionStatePort")
            .field("representation", &self.representation())
            .finish()
    }
}

#[cfg(all(test, feature = "f64", feature = "matrix2", feature = "matrixd"))]
mod tests {
    use super::*;
    use crate::{DMatrix, FunctionMatrixRepresentation, FunctionMatrixStoragePattern, Matrix2};

    #[test]
    fn scalar_state_port_borrows_the_existing_cell_and_reports_exact_representation() {
        let reference = Ref::new(1.25_f64);
        let existing_alias = reference.clone();

        let port = FunctionStatePort::from_ref(&reference);

        assert!(reference.same_handle(&existing_alias));
        assert_eq!(*reference.borrow(), 1.25);
        assert_eq!(port.representation(), FunctionValueRepresentation::F64);
    }

    #[test]
    fn matrix_state_ports_report_exact_storage() {
        let fixed = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let dynamic = Ref::new(DMatrix::from_vec(2, 2, vec![1.0_f64, 2.0, 3.0, 4.0]));

        assert_eq!(
            FunctionStatePort::from_ref(&fixed).representation(),
            FunctionValueRepresentation::Matrix {
                element: crate::FunctionMatrixElement::F64,
                storage: FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::Matrix2,),
            },
        );
        assert_eq!(
            FunctionStatePort::from_ref(&dynamic).representation(),
            FunctionValueRepresentation::Matrix {
                element: crate::FunctionMatrixElement::F64,
                storage: FunctionMatrixStoragePattern::Exact(FunctionMatrixRepresentation::MatrixD,),
            },
        );
    }

    #[test]
    fn state_port_debug_exposes_only_the_representation() {
        let reference = Ref::new(1234.56789_f64);
        let debug = format!("{:?}", FunctionStatePort::from_ref(&reference));

        assert_eq!(debug, "FunctionStatePort { representation: F64 }",);
        assert!(!debug.contains("1234.56789"));
        assert!(!debug.contains("LegacyValue"));
        assert!(!debug.contains("ReactiveCellId"));
        assert!(!debug.contains("0x"));
        assert!(!debug.contains('@'));
    }
}
