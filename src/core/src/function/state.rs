//! Opaque capabilities over exact typed cells owned by runtime functions.

#[cfg(all(feature = "no_std", feature = "string"))]
use alloc::string::String;
#[cfg(not(feature = "no_std"))]
use std::fmt;
#[cfg(all(not(feature = "no_std"), feature = "string"))]
use std::string::String;

#[cfg(feature = "no_std")]
use core::fmt;

use crate::{FunctionPortBacking, FunctionRuntimeType, FunctionValueRepresentation, MResult, Ref};

mod function_state_sealed {
    pub(crate) use crate::CanonicalStateJournal as Journal;

    pub struct FlatElement;

    pub trait PortSealed {
        type ElementShape;

        fn capture(reference: &crate::Ref<Self>, journal: &mut Journal) -> crate::MResult<()>
        where
            Self: Sized;
    }

    pub trait ExactSealed: PortSealed {}
}

pub(crate) use function_state_sealed::Journal as FunctionCheckpoint;

/// A runtime backing that can be exposed through an opaque state port.
///
/// Normal ports admit only exact scalar and matrix backings whose payload can
/// be checkpointed without traversing legacy aggregate graphs. Compatibility
/// graph capture remains confined to `legacy_adapter`.
///
/// ```compile_fail
/// use mech_core::FunctionStatePortBacking;
/// struct Unsupported;
/// fn require_state_port_backing<T: FunctionStatePortBacking>() {}
/// require_state_port_backing::<Unsupported>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStatePortBacking, ValueCell};
/// fn require_state_port_backing<T: FunctionStatePortBacking>() {}
/// require_state_port_backing::<ValueCell>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStatePortBacking, MechSet};
/// fn require_state_port_backing<T: FunctionStatePortBacking>() {}
/// require_state_port_backing::<MechSet>();
/// ```
pub trait FunctionStatePortBacking:
    function_state_sealed::PortSealed + FunctionPortBacking + FunctionRuntimeType + 'static
{
}

impl<T> FunctionStatePortBacking for T where
    T: function_state_sealed::PortSealed + FunctionPortBacking + FunctionRuntimeType + 'static
{
}

/// An exact runtime backing whose clone is a complete independent checkpoint.
///
/// The sealed implementation list deliberately excludes universal values,
/// mutable value cells, and aggregates that can contain nested cell identity.
///
/// ```compile_fail
/// use mech_core::FunctionStateBacking;
/// struct Unsupported;
/// fn require_state_backing<T: FunctionStateBacking>() {}
/// require_state_backing::<Unsupported>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStateBacking, ValueCell};
/// fn require_state_backing<T: FunctionStateBacking>() {}
/// require_state_backing::<ValueCell>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStateBacking, MechSet};
/// fn require_state_backing<T: FunctionStateBacking>() {}
/// require_state_backing::<MechSet>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStateBacking, Matrix2};
/// fn require_state_backing<T: FunctionStateBacking>() {}
/// require_state_backing::<Matrix2<Matrix2<f64>>>();
/// ```
pub trait FunctionStateBacking:
    FunctionStatePortBacking + function_state_sealed::ExactSealed + Clone + 'static
{
}

impl<T> FunctionStateBacking for T where
    T: FunctionStatePortBacking + function_state_sealed::ExactSealed + Clone + 'static
{
}

macro_rules! scalar_function_state_backing {
    ($type:ty, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl function_state_sealed::PortSealed for $type {
            type ElementShape = function_state_sealed::FlatElement;

            fn capture(
                reference: &Ref<Self>,
                journal: &mut function_state_sealed::Journal,
            ) -> MResult<()> {
                journal.capture_exact_ref(reference)
            }
        }

        #[cfg(feature = $feature)]
        impl function_state_sealed::ExactSealed for $type {}
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

impl function_state_sealed::PortSealed for usize {
    type ElementShape = function_state_sealed::FlatElement;

    fn capture(reference: &Ref<Self>, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        journal.capture_exact_ref(reference)
    }
}
impl function_state_sealed::ExactSealed for usize {}

#[cfg(feature = "complex")]
impl function_state_sealed::PortSealed for crate::C64 {
    type ElementShape = function_state_sealed::FlatElement;

    fn capture(reference: &Ref<Self>, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        journal.capture_exact_ref(reference)
    }
}
#[cfg(feature = "complex")]
impl function_state_sealed::ExactSealed for crate::C64 {}

#[cfg(feature = "rational")]
impl function_state_sealed::PortSealed for crate::R64 {
    type ElementShape = function_state_sealed::FlatElement;

    fn capture(reference: &Ref<Self>, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        journal.capture_exact_ref(reference)
    }
}
#[cfg(feature = "rational")]
impl function_state_sealed::ExactSealed for crate::R64 {}

macro_rules! exact_matrix_function_state_backing {
    ($type:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> function_state_sealed::PortSealed for crate::$type<T>
        where
            T: function_state_sealed::PortSealed<ElementShape = function_state_sealed::FlatElement>
                + FunctionPortBacking
                + FunctionRuntimeType
                + Clone
                + 'static,
        {
            type ElementShape = ();

            fn capture(
                reference: &Ref<Self>,
                journal: &mut function_state_sealed::Journal,
            ) -> MResult<()> {
                journal.capture_exact_ref(reference)
            }
        }

        #[cfg(feature = $feature)]
        impl<T> function_state_sealed::ExactSealed for crate::$type<T> where
            T: function_state_sealed::PortSealed<ElementShape = function_state_sealed::FlatElement>
                + FunctionPortBacking
                + FunctionRuntimeType
                + Clone
                + 'static
        {
        }
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
    fn as_any(&self) -> &dyn core::any::Any;
    fn representation(&self) -> FunctionValueRepresentation;
    fn logical_cell_id(&self) -> crate::CanonicalCellId;
    fn capture(&self, journal: &mut function_state_sealed::Journal) -> MResult<()>;
}

impl<T> ErasedFunctionState for Ref<T>
where
    T: FunctionStatePortBacking,
{
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn representation(&self) -> FunctionValueRepresentation {
        T::REPRESENTATION
    }

    fn logical_cell_id(&self) -> crate::CanonicalCellId {
        self.reactive_cell_id()
    }

    fn capture(&self, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        <T as function_state_sealed::PortSealed>::capture(self, journal)
    }
}

impl ErasedFunctionState for crate::ValueCell {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn representation(&self) -> FunctionValueRepresentation {
        self.representation()
    }

    fn logical_cell_id(&self) -> crate::CanonicalCellId {
        self.reactive_cell_id()
    }

    fn capture(&self, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        journal.capture_value_cell(self)
    }
}

/// A borrowed, opaque capability over one typed function-owned state root.
///
/// State ports reveal only the cell's declared representation. They do not
/// expose its payload, typed reference, physical identity, or a legacy value.
#[derive(Clone, Copy)]
pub struct FunctionStatePort<'a> {
    inner: &'a dyn ErasedFunctionState,
}

impl<'a> FunctionStatePort<'a> {
    /// Borrows a typed state root without cloning either its handle or payload.
    pub fn from_ref<T>(reference: &'a Ref<T>) -> Self
    where
        T: FunctionStatePortBacking,
    {
        Self { inner: reference }
    }

    /// Borrows a canonical value cell as an authoritative state root.
    pub fn from_cell(cell: &'a crate::ValueCell) -> Self {
        Self { inner: cell }
    }

    pub fn representation(self) -> FunctionValueRepresentation {
        self.inner.representation()
    }

    pub(crate) fn logical_cell_id(self) -> crate::CanonicalCellId {
        self.inner.logical_cell_id()
    }

    pub(crate) fn capture_into(self, journal: &mut crate::CanonicalStateJournal) -> MResult<()> {
        self.inner.capture(journal)
    }

    pub(crate) fn backing_any(self) -> &'a dyn core::any::Any {
        self.inner.as_any()
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

    fn require_exact_state<T: FunctionStateBacking>() {}

    #[test]
    fn scalar_and_matrix_backings_remain_exact() {
        require_exact_state::<f64>();
        require_exact_state::<Matrix2<f64>>();
    }

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
    fn exact_ports_restore_the_original_scalar_and_matrix_cells() {
        let scalar = Ref::new(1.25_f64);
        let scalar_alias = scalar.clone();
        let matrix = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let matrix_alias = matrix.clone();
        let mut journal = Default::default();

        FunctionStatePort::from_ref(&scalar)
            .capture_into(&mut journal)
            .unwrap();
        FunctionStatePort::from_ref(&matrix)
            .capture_into(&mut journal)
            .unwrap();
        *scalar.borrow_mut() = 9.0;
        *matrix.borrow_mut() = Matrix2::new(9.0, 8.0, 7.0, 6.0);
        journal.restore_before().unwrap();

        assert!(scalar.same_handle(&scalar_alias));
        assert!(matrix.same_handle(&matrix_alias));
        assert_eq!(*scalar.borrow(), 1.25);
        assert_eq!(*matrix.borrow(), Matrix2::new(1.0, 2.0, 3.0, 4.0));
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
