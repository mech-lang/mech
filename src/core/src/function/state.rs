//! Opaque capabilities over exact typed cells owned by runtime functions.

#[cfg(all(feature = "no_std", feature = "string"))]
use alloc::string::String;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(all(not(feature = "no_std"), feature = "string"))]
use std::string::String;
#[cfg(not(feature = "no_std"))]
use std::{fmt, vec::Vec};

#[cfg(feature = "no_std")]
use core::fmt;

use crate::{
    FunctionPortBacking, FunctionReactiveCell, FunctionRuntimeType, FunctionValueRepresentation,
    MResult, Ref, ToValue,
};

mod function_state_sealed {
    pub(super) use crate::ValueStateJournal as Journal;

    pub struct FlatElement;

    pub trait PortSealed {
        type ElementShape;

        fn capture(reference: &crate::Ref<Self>, journal: &mut Journal) -> crate::MResult<()>
        where
            Self: Sized;
    }

    pub trait ExactSealed: PortSealed {}
}

/// A runtime backing that can be exposed through an opaque state port.
///
/// Implementations choose a private capture strategy. Exact scalar and matrix
/// backings clone their payload, while supported graph aggregates delegate to
/// the value-state journal's recursive traversal.
///
/// ```compile_fail
/// use mech_core::{FunctionStatePortBacking, LegacyValue};
/// fn require_state_port_backing<T: FunctionStatePortBacking>() {}
/// require_state_port_backing::<LegacyValue>();
/// ```
///
/// ```compile_fail
/// use mech_core::{FunctionStatePortBacking, ValueCell};
/// fn require_state_port_backing<T: FunctionStatePortBacking>() {}
/// require_state_port_backing::<ValueCell>();
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

#[cfg(feature = "set")]
impl function_state_sealed::PortSealed for crate::MechSet {
    type ElementShape = ();

    fn capture(reference: &Ref<Self>, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        let root = reference.to_value();
        journal.capture_value(&root)
    }
}

trait ErasedFunctionState {
    fn representation(&self) -> FunctionValueRepresentation;
    fn reactive_cell_ids(&self) -> Vec<FunctionReactiveCell>;
    fn capture(&self, journal: &mut function_state_sealed::Journal) -> MResult<()>;
}

impl<T> ErasedFunctionState for Ref<T>
where
    T: FunctionStatePortBacking,
    Ref<T>: ToValue,
{
    fn representation(&self) -> FunctionValueRepresentation {
        T::REPRESENTATION
    }

    fn reactive_cell_ids(&self) -> Vec<FunctionReactiveCell> {
        self.to_value().reactive_root_cell_ids()
    }

    fn capture(&self, journal: &mut function_state_sealed::Journal) -> MResult<()> {
        <T as function_state_sealed::PortSealed>::capture(self, journal)
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
        Ref<T>: ToValue,
    {
        Self { inner: reference }
    }

    pub fn representation(self) -> FunctionValueRepresentation {
        self.inner.representation()
    }

    pub(crate) fn reactive_cell_ids(self) -> Vec<FunctionReactiveCell> {
        self.inner.reactive_cell_ids()
    }

    pub(crate) fn capture_into(self, journal: &mut crate::ValueStateJournal) -> MResult<()> {
        self.inner.capture(journal)
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

#[cfg(all(test, feature = "set", feature = "tuple", feature = "f64"))]
mod set_graph_tests {
    use super::*;
    use crate::{LegacyValue, MechSet, MechTuple};

    fn scalar(value: f64) -> Ref<f64> {
        Ref::new(value)
    }

    fn scalar_value(reference: &Ref<f64>) -> LegacyValue {
        reference.to_value()
    }

    fn scalar_member(value: &LegacyValue) -> Ref<f64> {
        value.expect_f64().unwrap()
    }

    fn require_state_port<T: FunctionStatePortBacking>() {}

    #[test]
    fn mech_set_is_a_graph_state_port_backing_with_legacy_identity() {
        require_state_port::<MechSet>();

        let set = Ref::new(MechSet::from_vec(vec![scalar_value(&scalar(1.0))]));
        let legacy = set.to_value();
        let port = FunctionStatePort::from_ref(&set);

        assert_eq!(port.representation(), FunctionValueRepresentation::Set);
        assert_eq!(port.reactive_cell_ids(), legacy.reactive_root_cell_ids());

        let debug = format!("{port:?}");
        assert_eq!(debug, "FunctionStatePort { representation: Set }");
        assert!(!debug.contains("1.0"));
        assert!(!debug.contains("ReactiveCellId"));
        assert!(!debug.contains("0x"));
        assert!(!debug.contains('@'));
    }

    #[test]
    fn graph_port_restores_set_root_metadata_order_and_nested_cells() {
        let removed = scalar(1.0);
        let retained = scalar(2.0);
        let added = scalar(3.0);
        let set = Ref::new(MechSet {
            kind: scalar_value(&removed).kind(),
            max_elements: Some(2),
            num_elements: 2,
            set: [scalar_value(&removed), scalar_value(&retained)]
                .into_iter()
                .collect(),
        });
        let set_alias = set.clone();
        let mut journal = Default::default();
        FunctionStatePort::from_ref(&set)
            .capture_into(&mut journal)
            .unwrap();

        let changed_kind = set.to_value().kind();
        {
            let mut payload = set.borrow_mut();
            payload.set.shift_remove(&scalar_value(&removed));
            payload.set.insert(scalar_value(&added));
            payload.kind = changed_kind;
            payload.max_elements = Some(9);
            payload.num_elements = 9;
        }
        *retained.borrow_mut() = 20.0;
        journal.restore_before().unwrap();

        assert!(set.same_handle(&set_alias));
        let payload = set.borrow();
        assert_eq!(payload.kind, scalar_value(&removed).kind());
        assert_eq!(payload.max_elements, Some(2));
        assert_eq!(payload.num_elements, 2);
        let members = payload.set.iter().map(scalar_member).collect::<Vec<_>>();
        assert!(members[0].same_handle(&removed));
        assert!(members[1].same_handle(&retained));
        assert_eq!((*members[0].borrow(), *members[1].borrow()), (1.0, 2.0));
    }

    #[test]
    fn graph_port_preserves_nested_sets_and_shared_cells() {
        let shared = scalar(4.0);
        let left = Ref::new(MechTuple::from_vec(vec![
            scalar(1.0).to_value(),
            scalar_value(&shared),
        ]));
        let right = Ref::new(MechTuple::from_vec(vec![
            scalar(2.0).to_value(),
            scalar_value(&shared),
        ]));
        let inner = Ref::new(MechSet::from_vec(vec![left.to_value(), right.to_value()]));
        let outer = Ref::new(MechSet::from_vec(vec![inner.to_value()]));
        let mut journal = Default::default();
        FunctionStatePort::from_ref(&outer)
            .capture_into(&mut journal)
            .unwrap();

        let changed_kind = inner.to_value().kind();
        *shared.borrow_mut() = 40.0;
        inner.borrow_mut().kind = changed_kind.clone();
        outer.borrow_mut().num_elements = 99;
        journal.restore_before().unwrap();

        assert!(outer.borrow().set.contains(&inner.to_value()));
        assert!(inner.borrow().set.contains(&left.to_value()));
        assert!(inner.borrow().set.contains(&right.to_value()));
        let tuples = [left, right];
        let tuple_scalar = |tuple: &Ref<MechTuple>| {
            let tuple = tuple.borrow();
            scalar_member(&tuple.elements[1])
        };
        let left_shared = tuple_scalar(&tuples[0]);
        let right_shared = tuple_scalar(&tuples[1]);
        assert!(left_shared.same_handle(&shared));
        assert!(right_shared.same_handle(&shared));
        assert!(left_shared.same_handle(&right_shared));
        assert_eq!(*shared.borrow(), 4.0);
        assert_eq!(outer.borrow().num_elements, 1);
        assert_ne!(inner.borrow().kind, changed_kind);
    }

    #[test]
    fn graph_ports_deduplicate_and_keep_equal_roots_distinct() {
        let first = Ref::new(MechSet::from_vec(vec![scalar_value(&scalar(1.0))]));
        let second = Ref::new(MechSet::from_vec(vec![scalar_value(&scalar(1.0))]));
        let mut journal = Default::default();

        FunctionStatePort::from_ref(&first)
            .capture_into(&mut journal)
            .unwrap();
        assert_eq!(journal.cell_count(), 2);
        FunctionStatePort::from_ref(&first)
            .capture_into(&mut journal)
            .unwrap();
        assert_eq!(journal.cell_count(), 2);
        FunctionStatePort::from_ref(&second)
            .capture_into(&mut journal)
            .unwrap();
        assert_eq!(journal.cell_count(), 4);
        assert!(!first.same_handle(&second));
    }

    #[test]
    fn graph_port_delta_rewinds_and_replays() {
        let member = scalar(1.0);
        let set = Ref::new(MechSet::from_vec(vec![scalar_value(&member)]));
        let set_alias = set.clone();
        let mut journal = Default::default();
        FunctionStatePort::from_ref(&set)
            .capture_into(&mut journal)
            .unwrap();

        *member.borrow_mut() = 2.0;
        set.borrow_mut().max_elements = Some(7);
        journal.record_after().unwrap();
        let delta = journal.into_delta().unwrap();

        delta.rewind().unwrap();
        assert!(set.same_handle(&set_alias));
        assert_eq!(*member.borrow(), 1.0);
        assert_eq!(set.borrow().max_elements, Some(1));

        delta.replay().unwrap();
        assert!(set.same_handle(&set_alias));
        assert_eq!(*member.borrow(), 2.0);
        assert_eq!(set.borrow().max_elements, Some(7));
    }
}
