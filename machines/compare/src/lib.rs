#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

extern crate paste;

use mech_core::*;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;

use std::sync::LazyLock;

#[cfg(feature = "source")]
pub(crate) fn semantic_compare_extents(inputs: &[&SpecializationInput]) -> MResult<Box<[u64]>> {
    let mut result: Option<[u64; 2]> = None;
    for input in inputs {
        let extents = input
            .cell()?
            .resolved_descriptor()?
            .current_extents()
            .map_err(MechError::from)?;
        if !extents.is_empty() {
            let [rows, columns] = extents.as_ref() else {
                return Err(MechError::new(
                    GenericError {
                        msg: "comparison requires scalar or rank-two inputs".into(),
                    },
                    None,
                )
                .with_compiler_loc());
            };
            result = Some(match result {
                None => [*rows, *columns],
                Some([left_rows, left_columns]) => {
                    let axis = |left: u64, right: u64| {
                        if left == right {
                            Some(left)
                        } else if left == 1 {
                            Some(right)
                        } else if right == 1 {
                            Some(left)
                        } else {
                            None
                        }
                    };
                    [
                        axis(left_rows, *rows).ok_or_else(|| {
                            MechError::new(
                                DimensionMismatch {
                                    dims: vec![
                                        left_rows as usize,
                                        left_columns as usize,
                                        *rows as usize,
                                        *columns as usize,
                                    ],
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?,
                        axis(left_columns, *columns).ok_or_else(|| {
                            MechError::new(
                                DimensionMismatch {
                                    dims: vec![
                                        left_rows as usize,
                                        left_columns as usize,
                                        *rows as usize,
                                        *columns as usize,
                                    ],
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?,
                    ]
                }
            });
        }
    }
    Ok(result.map_or_else(
        || Vec::<u64>::new().into_boxed_slice(),
        |shape| shape.into_iter().collect::<Vec<_>>().into_boxed_slice(),
    ))
}

static PURE_COMPARE_SCALAR_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_compare_contract(ChangeDetectionPolicy::ExactScalar));
static PURE_COMPARE_MATRIX_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| pure_compare_contract(ChangeDetectionPolicy::KernelReported));

fn pure_compare_contract(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn compare_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_COMPARE_MATRIX_CONTRACT,
        _ => &PURE_COMPARE_SCALAR_CONTRACT,
    }
}

#[macro_export]
macro_rules! impl_canonical_numeric_compare_specializer {
    ($specializer:ident, $module:ident, $lib:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer;

        #[cfg(feature = "source")]
        impl CanonicalFunctionSpecializer for $specializer {
            fn specialize_invocation(
                &self,
                specialization: &SpecializationInvocation,
                context: &mut SpecializationContext<'_>,
            ) -> MResult<SpecializedFunction> {
                if specialization.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: specialization.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let first = specialization.input(0).expect("validated comparison lhs");
                let second = specialization.input(1).expect("validated comparison rhs");
                let extents = $crate::semantic_compare_extents(&[first, second])?;
                context.bind_resolved_runtime(
                    RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
                    ExecutionTarget::DirectRuntime,
                    vec![extents].into_boxed_slice(),
                    &[first, second],
                )
            }
        }
    };
}

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "eq")]
pub mod eq;
#[cfg(feature = "gt")]
pub mod gt;
#[cfg(feature = "gte")]
pub mod gte;
#[cfg(feature = "lt")]
pub mod lt;
#[cfg(feature = "lte")]
pub mod lte;
#[cfg(feature = "max")]
pub mod max;
#[cfg(feature = "min")]
pub mod min;
#[cfg(feature = "neq")]
pub mod neq;
#[cfg(feature = "seq")]
pub mod seq;
#[cfg(feature = "sneq")]
pub mod sneq;

#[cfg(all(feature = "eq", feature = "source"))]
pub use self::eq::*;
#[cfg(all(feature = "eq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::eq::*;
#[cfg(all(feature = "gt", feature = "source"))]
pub use self::gt::*;
#[cfg(all(feature = "gt", feature = "runtime", not(feature = "source")))]
pub(crate) use self::gt::*;
#[cfg(all(feature = "gte", feature = "source"))]
pub use self::gte::*;
#[cfg(all(feature = "gte", feature = "runtime", not(feature = "source")))]
pub(crate) use self::gte::*;
#[cfg(all(feature = "lt", feature = "source"))]
pub use self::lt::*;
#[cfg(all(feature = "lt", feature = "runtime", not(feature = "source")))]
pub(crate) use self::lt::*;
#[cfg(all(feature = "lte", feature = "source"))]
pub use self::lte::*;
#[cfg(all(feature = "lte", feature = "runtime", not(feature = "source")))]
pub(crate) use self::lte::*;
#[cfg(all(feature = "max", feature = "source"))]
pub use self::max::*;
#[cfg(all(feature = "max", feature = "runtime", not(feature = "source")))]
pub(crate) use self::max::*;
#[cfg(all(feature = "min", feature = "source"))]
pub use self::min::*;
#[cfg(all(feature = "min", feature = "runtime", not(feature = "source")))]
pub(crate) use self::min::*;
#[cfg(all(feature = "neq", feature = "source"))]
pub use self::neq::*;
#[cfg(all(feature = "neq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::neq::*;
#[cfg(all(feature = "seq", feature = "source"))]
pub use self::seq::*;
#[cfg(all(feature = "seq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::seq::*;
#[cfg(all(feature = "sneq", feature = "source"))]
pub use self::sneq::*;
#[cfg(all(feature = "sneq", feature = "runtime", not(feature = "source")))]
pub(crate) use self::sneq::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

pub(crate) trait FixedComparisonElement:
    ManagedElement + FunctionPortBacking + std::fmt::Debug + PartialOrd
{
}

macro_rules! fixed_comparison_element {
    ($feature:literal, $type:ty) => {
        #[cfg(feature = $feature)]
        impl FixedComparisonElement for $type {}
    };
}
fixed_comparison_element!("bool", bool);
fixed_comparison_element!("u8", u8);
fixed_comparison_element!("u16", u16);
fixed_comparison_element!("u32", u32);
fixed_comparison_element!("u64", u64);
fixed_comparison_element!("u128", u128);
fixed_comparison_element!("i8", i8);
fixed_comparison_element!("i16", i16);
fixed_comparison_element!("i32", i32);
fixed_comparison_element!("i64", i64);
fixed_comparison_element!("i128", i128);
fixed_comparison_element!("f32", f32);
fixed_comparison_element!("f64", f64);
fixed_comparison_element!("r64", R64);
fixed_comparison_element!("c64", C64);

/// Comparison implementations retain logical ports only. Fixed-width values
/// are opened as typed frame views; String values remain immutable canonical
/// roots and are rebuilt into the transaction-selected output.
pub(crate) trait ComparisonPort<P, E> {
    type Port: std::fmt::Debug;
    fn input(port: FunctionInputPort<'_>) -> MResult<Self::Port>;
    fn output(port: FunctionOutputPort<'_>) -> MResult<Self::Port>;
}

impl<T, P, E> ComparisonPort<P, E> for T
where
    T: FixedComparisonElement,
    P: FunctionPortBacking,
    E: ManagedElement + FunctionPortBacking,
{
    type Port = ManagedPort<E>;
    fn input(port: FunctionInputPort<'_>) -> MResult<Self::Port> {
        let _ = port.try_managed::<P>()?;
        port.try_managed_element::<E>()
    }
    fn output(port: FunctionOutputPort<'_>) -> MResult<Self::Port> {
        let _ = port.try_managed::<P>()?;
        port.try_managed_element::<E>()
    }
}

#[cfg(feature = "string")]
impl<P, E> ComparisonPort<P, E> for String
where
    P: FunctionPortBacking + std::fmt::Debug,
    E: FunctionPortBacking,
{
    type Port = ManagedPort<E>;
    fn input(port: FunctionInputPort<'_>) -> MResult<Self::Port> {
        let _ = port.try_managed::<P>()?;
        port.try_managed_element::<E>()
    }
    fn output(port: FunctionOutputPort<'_>) -> MResult<Self::Port> {
        let _ = port.try_managed::<P>()?;
        port.try_managed_element::<E>()
    }
}

#[derive(Clone, Copy)]
// The shared factory traversal enables only the modes present in a profile.
#[allow(
    dead_code,
    reason = "feature profiles enable only a subset of comparison broadcast modes"
)]
enum ComparisonBroadcast {
    Exact,
    LeftScalar,
    RightScalar,
    LeftColumn,
    RightColumn,
    LeftRow,
    RightRow,
}

fn apply_managed_comparison<T: ManagedElement, O: ManagedElement>(
    lhs: ManagedValueView<'_, T>,
    rhs: ManagedValueView<'_, T>,
    out: &mut ManagedValueViewMut<'_, O>,
    broadcast: ComparisonBroadcast,
    operation: impl Fn(T, T) -> O,
) -> MResult<()> {
    let same_shape = |view: &ManagedValueView<'_, T>| {
        view.rows() == out.rows() && view.columns() == out.columns()
    };
    let geometry_valid = match broadcast {
        ComparisonBroadcast::Exact => same_shape(&lhs) && same_shape(&rhs),
        ComparisonBroadcast::LeftScalar => lhs.len() == 1 && same_shape(&rhs),
        ComparisonBroadcast::RightScalar => same_shape(&lhs) && rhs.len() == 1,
        ComparisonBroadcast::LeftColumn => {
            lhs.columns() == 1 && lhs.rows() == out.rows() && same_shape(&rhs)
        }
        ComparisonBroadcast::RightColumn => {
            same_shape(&lhs) && rhs.columns() == 1 && rhs.rows() == out.rows()
        }
        ComparisonBroadcast::LeftRow => {
            lhs.rows() == 1 && lhs.columns() == out.columns() && same_shape(&rhs)
        }
        ComparisonBroadcast::RightRow => {
            same_shape(&lhs) && rhs.rows() == 1 && rhs.columns() == out.columns()
        }
    };
    if !geometry_valid {
        return Err(MechError::new(
            GenericError {
                msg: "comparison managed broadcast geometry is invalid".into(),
            },
            None,
        ));
    }
    let output_rows = out.rows();
    out.try_fill_column_major(|index| {
        let row = if output_rows == 0 {
            0
        } else {
            index % output_rows
        };
        let column = if output_rows == 0 {
            0
        } else {
            index / output_rows
        };
        let (lhs_index, rhs_index) = match broadcast {
            ComparisonBroadcast::Exact => (index, index),
            ComparisonBroadcast::LeftScalar => (0, index),
            ComparisonBroadcast::RightScalar => (index, 0),
            ComparisonBroadcast::LeftColumn => (row, index),
            ComparisonBroadcast::RightColumn => (index, row),
            ComparisonBroadcast::LeftRow => (column, index),
            ComparisonBroadcast::RightRow => (index, column),
        };
        let bad_geometry = || {
            MechError::new(
                GenericError {
                    msg: "comparison managed broadcast geometry is invalid".into(),
                },
                None,
            )
        };
        Ok(operation(
            lhs.get_column_major(lhs_index).ok_or_else(bad_geometry)?,
            rhs.get_column_major(rhs_index).ok_or_else(bad_geometry)?,
        ))
    })
}

#[cfg(feature = "string")]
trait CanonicalStringComparisonResult: FunctionPortBacking {
    fn into_draft(self) -> ValueDataDraft;
}

#[cfg(feature = "string")]
impl CanonicalStringComparisonResult for bool {
    fn into_draft(self) -> ValueDataDraft {
        ValueDataDraft::Bool(self)
    }
}

#[cfg(feature = "string")]
impl CanonicalStringComparisonResult for String {
    fn into_draft(self) -> ValueDataDraft {
        ValueDataDraft::String(self)
    }
}

#[cfg(feature = "string")]
fn canonical_string_extents(value: &Value, cell: &ValueCell) -> MResult<Option<(usize, usize)>> {
    match value.data() {
        ValueData::String(_) => Ok(None),
        ValueData::Matrix(_) => {
            let extents = cell
                .resolved_descriptor()?
                .current_extents()
                .map_err(MechError::from)?;
            let [rows, columns] = extents.as_ref() else {
                return Err(function_shape_contract_violation(
                    "comparison",
                    "String matrix input must have rank two",
                ));
            };
            Ok(Some((
                usize::try_from(*rows).map_err(|_| {
                    function_shape_contract_violation("comparison", "row extent exceeds usize")
                })?,
                usize::try_from(*columns).map_err(|_| {
                    function_shape_contract_violation(
                        "comparison",
                        "column extent exceeds usize",
                    )
                })?,
            )))
        }
        _ => Err(function_shape_contract_violation(
            "comparison",
            "canonical String input disagrees with its schema",
        )),
    }
}

#[cfg(feature = "string")]
fn canonical_string_at(
    value: &Value,
    extents: Option<(usize, usize)>,
    row: usize,
    column: usize,
) -> MResult<&str> {
    match (value.data(), extents) {
        (ValueData::String(value), None) => Ok(value.as_ref()),
        (ValueData::Matrix(matrix), Some((rows, columns))) => {
            let mech_core::snapshot::SequenceView::String(values) = matrix.elements() else {
                return Err(function_shape_contract_violation(
                    "comparison",
                    "matrix payload is not String-backed",
                ));
            };
            let row = if rows == 1 { 0 } else { row };
            let column = if columns == 1 { 0 } else { column };
            values
                .get(row.saturating_mul(columns).saturating_add(column))
                .map(|value| value.as_ref())
                .ok_or_else(|| {
                    function_shape_contract_violation(
                        "comparison",
                        "broadcast coordinate is outside the String input",
                    )
                })
        }
        _ => Err(function_shape_contract_violation(
            "comparison",
            "canonical String input disagrees with its schema",
        )),
    }
}

#[cfg(feature = "string")]
fn apply_canonical_string_comparison<O: CanonicalStringComparisonResult>(
    frame: &mut KernelMemoryFrame<'_>,
    lhs: &ManagedPort<String>,
    rhs: &ManagedPort<String>,
    out: &ManagedPort<O>,
    broadcast: ComparisonBroadcast,
    operation: impl Fn(&str, &str) -> O,
) -> MResult<()> {
    let lhs_value = frame.snapshot_canonical_port_value(lhs)?;
    let rhs_value = frame.snapshot_canonical_port_value(rhs)?;
    let lhs_extents = canonical_string_extents(&lhs_value, lhs.cell())?;
    let rhs_extents = canonical_string_extents(&rhs_value, rhs.cell())?;
    let output_shape = out
        .cell()
        .resolved_descriptor()?
        .current_extents()
        .map_err(MechError::from)?;
    let output_extents = match output_shape.as_ref() {
        [] => None,
        [rows, columns] => Some((
            usize::try_from(*rows).map_err(|_| {
                function_shape_contract_violation("comparison", "row extent exceeds usize")
            })?,
            usize::try_from(*columns).map_err(|_| {
                function_shape_contract_violation("comparison", "column extent exceeds usize")
            })?,
        )),
        _ => {
            return Err(function_shape_contract_violation(
                "comparison",
                "output must be scalar or rank two",
            ));
        }
    };
    let valid = match broadcast {
        ComparisonBroadcast::Exact => {
            lhs_extents == output_extents && rhs_extents == output_extents
        }
        ComparisonBroadcast::LeftScalar => {
            lhs_extents.is_none() && rhs_extents == output_extents
        }
        ComparisonBroadcast::RightScalar => {
            lhs_extents == output_extents && rhs_extents.is_none()
        }
        ComparisonBroadcast::LeftColumn => {
            matches!((lhs_extents, output_extents), (Some((left_rows, 1)), Some((rows, _))) if left_rows == rows)
                && rhs_extents == output_extents
        }
        ComparisonBroadcast::RightColumn => {
            lhs_extents == output_extents
                && matches!((rhs_extents, output_extents), (Some((right_rows, 1)), Some((rows, _))) if right_rows == rows)
        }
        ComparisonBroadcast::LeftRow => {
            matches!((lhs_extents, output_extents), (Some((1, left_columns)), Some((_, columns))) if left_columns == columns)
                && rhs_extents == output_extents
        }
        ComparisonBroadcast::RightRow => {
            lhs_extents == output_extents
                && matches!((rhs_extents, output_extents), (Some((1, right_columns)), Some((_, columns))) if right_columns == columns)
        }
    };
    if !valid {
        return Err(function_shape_contract_violation(
            "comparison",
            "String broadcast geometry is invalid",
        ));
    }

    let next = match output_extents {
        None => out.cell().rebuild_data_draft(
            operation(
                canonical_string_at(&lhs_value, lhs_extents, 0, 0)?,
                canonical_string_at(&rhs_value, rhs_extents, 0, 0)?,
            )
            .into_draft(),
        )?,
        Some((rows, columns)) => {
            let count = rows.checked_mul(columns).ok_or_else(|| {
                function_shape_contract_violation("comparison", "output cardinality overflow")
            })?;
            let mut drafts = Vec::new();
            drafts.try_reserve_exact(count).map_err(|_| {
                function_shape_contract_violation(
                    "comparison",
                    "output staging allocation failed",
                )
            })?;
            for row in 0..rows {
                for column in 0..columns {
                    drafts.push(
                        operation(
                            canonical_string_at(&lhs_value, lhs_extents, row, column)?,
                            canonical_string_at(&rhs_value, rhs_extents, row, column)?,
                        )
                        .into_draft(),
                    );
                }
            }
            out.cell().rebuild_matrix_drafts(
                vec![rows as u64, columns as u64].into_boxed_slice(),
                drafts.into_boxed_slice(),
            )?
        }
    };
    frame.stage_output_value(out.cell(), next)
}

#[macro_export]
macro_rules! impl_compare_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_compare_typed_binop!($struct_name, $arg1_type, $arg2_type, $out_type, bool, $op);
    };
}

#[macro_export]
macro_rules! impl_compare_value_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        impl_compare_typed_binop!($struct_name, $arg1_type, $arg2_type, $out_type, T, $op);
    };
}

#[macro_export]
macro_rules! impl_compare_typed_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $out_element:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T>
        where
            T: ComparisonPort<$arg1_type, T>
                + ComparisonPort<$arg2_type, T>
                + ComparisonPort<$out_type, $out_element>,
        {
            lhs: <T as ComparisonPort<$arg1_type, T>>::Port,
            rhs: <T as ComparisonPort<$arg2_type, T>>::Port,
            out: <T as ComparisonPort<$out_type, $out_element>>::Port,
            invocation: FunctionInvocation,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: std::fmt::Debug + Clone + 'static + FunctionRuntimeType + PartialEq + PartialOrd,
            T: ComparisonPort<$arg1_type, T>
                + ComparisonPort<$arg2_type, T>
                + ComparisonPort<$out_type, $out_element>,
            Self: MechFunction,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
            $out_type: FunctionStateBacking + FunctionPortBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(compare_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs = <T as ComparisonPort<$arg1_type, T>>::input(lhs)?;
                let rhs = <T as ComparisonPort<$arg2_type, T>>::input(rhs)?;
                let out = <T as ComparisonPort<$out_type, $out_element>>::output(out)?;
                Ok(Box::new(Self {
                    lhs,
                    rhs,
                    out,
                    invocation,
                }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: std::fmt::Debug + Clone + 'static + PartialEq + PartialOrd,
            T: FixedComparisonElement,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking + FunctionPortBacking,
        {
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_binary_typed_port_views(
                    &self.lhs,
                    &self.rhs,
                    &self.out,
                    |lhs, rhs, out| $op!(managed, lhs, rhs, out),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.invocation.output_cell()))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(compare_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(
                    self.invocation.output_cell(),
                )]))
            }
        }
        #[cfg(feature = "string")]
        impl MechFunctionImpl for $struct_name<String> {
            fn solve_managed(
                &self,
                frame: &mut KernelMemoryFrame<'_>,
                _services: &mut dyn MechExecutionServices,
            ) -> MResult<ReactiveSolveStatus> {
                $op!(canonical, frame, &self.lhs, &self.rhs, &self.out)?;
                Ok(ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.invocation.output_cell()))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(compare_full_write_contract(
                    self.invocation.output_cell().representation(),
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(
                    self.invocation.output_cell(),
                )]))
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst + FunctionRuntimeType,
            T: ComparisonPort<$arg1_type, T>
                + ComparisonPort<$arg2_type, T>
                + ComparisonPort<$out_type, $out_element>,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                let out = compile_value_cell_register(self.invocation.output_cell(), ctx)?;
                let lhs = compile_value_cell_register(&self.invocation.input_cells()[0], ctx)?;
                let rhs = compile_value_cell_register(&self.invocation.input_cells()[1], ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, out, lhs, rhs);
                Ok(out)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_compare_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, bool, impl_compare_binop);
    };
}

#[macro_export]
macro_rules! impl_compare_fxns2 {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_compare_value_binop);
    };
}

#[cfg(all(test, feature = "runtime"))]
fn managed_test_function<F: MechFunctionFactory>(
    invocation: FunctionInvocation,
    operation: &'static str,
) -> SpecializedFunction {
    let implementation = F::new_invocation(invocation.clone()).unwrap();
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(
            operation,
            F::declared_operation_contract().unwrap().clone(),
        )
        .unwrap(),
        RuntimeFunctionId::from_name(operation),
        ExecutionTarget::DirectRuntime,
        F::implementation_memory_class(),
    )
    .unwrap()
}
