#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix"
))]
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;
use std::marker::PhantomData;

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "source")]
fn semantic_logic_extents(inputs: &[&SpecializationInput]) -> MResult<Box<[u64]>> {
    let mut extents = Vec::<u64>::new().into_boxed_slice();
    for input in inputs {
        let current = input
            .cell()?
            .resolved_descriptor()?
            .current_extents()
            .map_err(MechError::from)?;
        if !current.is_empty() {
            if extents.is_empty() {
                extents = current;
            } else if extents.len() == current.len()
                && extents
                    .iter()
                    .zip(current.iter())
                    .all(|(left, right)| left == right || *left == 1 || *right == 1)
            {
                for (extent, current) in extents.iter_mut().zip(current.iter()) {
                    if *extent == 1 {
                        *extent = *current;
                    }
                }
            } else {
                return Err(MechError::new(
                    DimensionMismatch {
                        dims: extents
                            .iter()
                            .chain(current.iter())
                            .map(|extent| *extent as usize)
                            .collect(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
    }
    Ok(extents)
}

#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrixd"
))]
use nalgebra::DMatrix;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vectord"
))]
use nalgebra::DVector;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix1"
))]
use nalgebra::Matrix1;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix2"
))]
use nalgebra::Matrix2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix2x3"
))]
use nalgebra::Matrix2x3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix3"
))]
use nalgebra::Matrix3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix3x2"
))]
use nalgebra::Matrix3x2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "matrix4"
))]
use nalgebra::Matrix4;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vectord"
))]
use nalgebra::RowDVector;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vector2"
))]
use nalgebra::RowVector2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vector3"
))]
use nalgebra::RowVector3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "row_vector4"
))]
use nalgebra::RowVector4;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vector2"
))]
use nalgebra::Vector2;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vector3"
))]
use nalgebra::Vector3;
#[cfg(all(
    any(feature = "and", feature = "or", feature = "xor"),
    feature = "vector4"
))]
use nalgebra::Vector4;

#[cfg(any(feature = "and", feature = "not", feature = "or", feature = "xor"))]
use std::sync::LazyLock;

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
static PURE_LOGIC_BINARY_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(2, ChangeDetectionPolicy::ExactScalar));
#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
static PURE_LOGIC_BINARY_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(2, ChangeDetectionPolicy::KernelReported));
#[cfg(feature = "not")]
static PURE_LOGIC_UNARY_EXACT_SCALAR: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(1, ChangeDetectionPolicy::ExactScalar));
#[cfg(feature = "not")]
static PURE_LOGIC_UNARY_KERNEL_REPORTED: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| logic_full_write_contract(1, ChangeDetectionPolicy::KernelReported));

fn logic_full_write_contract(
    input_count: usize,
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    mech_core::elementwise_operation_contract(input_count, change_detection)
}

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
fn logic_binary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_LOGIC_BINARY_KERNEL_REPORTED,
        _ => &PURE_LOGIC_BINARY_EXACT_SCALAR,
    }
}

#[cfg(feature = "not")]
fn logic_unary_full_write_contract(
    output: FunctionValueRepresentation,
) -> &'static OperationContractDeclaration {
    match output {
        FunctionValueRepresentation::Matrix { .. } => &PURE_LOGIC_UNARY_KERNEL_REPORTED,
        _ => &PURE_LOGIC_UNARY_EXACT_SCALAR,
    }
}

#[cfg(feature = "and")]
pub mod and;
#[cfg(feature = "not")]
pub mod not;
#[cfg(feature = "or")]
pub mod or;
#[cfg(feature = "xor")]
pub mod xor;

#[cfg(all(feature = "and", feature = "source"))]
pub use self::and::*;
#[cfg(all(feature = "not", feature = "source"))]
pub use self::not::*;
#[cfg(all(feature = "or", feature = "source"))]
pub use self::or::*;
#[cfg(all(feature = "xor", feature = "source"))]
pub use self::xor::*;

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
#[derive(Clone, Copy)]
// The shared factory traversal enables only the modes present in a profile.
#[allow(
    dead_code,
    reason = "feature profiles enable only a subset of logic broadcast modes"
)]
enum LogicBroadcast {
    Exact,
    LeftScalar,
    RightScalar,
    LeftColumn,
    RightColumn,
    LeftRow,
    RightRow,
}

#[cfg(any(feature = "and", feature = "or", feature = "xor"))]
fn apply_logic_binary(
    lhs: ManagedValueView<'_, bool>,
    rhs: ManagedValueView<'_, bool>,
    out: &mut ManagedValueViewMut<'_, bool>,
    broadcast: LogicBroadcast,
    operation: impl Fn(bool, bool) -> bool,
) -> MResult<()> {
    let output_rows = out.rows();
    let output_columns = out.columns();
    let same_shape = |view: &ManagedValueView<'_, bool>| {
        view.rows() == output_rows && view.columns() == output_columns
    };
    // Dynamic matrix representations do not encode their live extents in the
    // runtime function ID. A same-representation factory must therefore retain
    // the broadcast selected by the source scheme when one operand happens to
    // use the same storage family as the output (for example RowD 1x1 with
    // RowD 1x5).
    let can_broadcast = |view: &ManagedValueView<'_, bool>| {
        same_shape(view)
            || view.len() == 1
            || (view.columns() == 1 && view.rows() == output_rows)
            || (view.rows() == 1 && view.columns() == output_columns)
    };
    let geometry_valid = match broadcast {
        LogicBroadcast::Exact => {
            (same_shape(&lhs) || same_shape(&rhs)) && can_broadcast(&lhs) && can_broadcast(&rhs)
        }
        LogicBroadcast::LeftScalar => lhs.len() == 1 && same_shape(&rhs),
        LogicBroadcast::RightScalar => same_shape(&lhs) && rhs.len() == 1,
        LogicBroadcast::LeftColumn => {
            lhs.columns() == 1 && lhs.rows() == out.rows() && same_shape(&rhs)
        }
        LogicBroadcast::RightColumn => {
            same_shape(&lhs) && rhs.columns() == 1 && rhs.rows() == out.rows()
        }
        LogicBroadcast::LeftRow => {
            lhs.rows() == 1 && lhs.columns() == out.columns() && same_shape(&rhs)
        }
        LogicBroadcast::RightRow => {
            same_shape(&lhs) && rhs.rows() == 1 && rhs.columns() == out.columns()
        }
    };
    if !geometry_valid {
        return Err(MechError::new(
            GenericError {
                msg: "logic managed broadcast geometry is invalid".into(),
            },
            None,
        ));
    }
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
        let compatible_index = |view: &ManagedValueView<'_, bool>| {
            if same_shape(view) {
                Some(index)
            } else if view.len() == 1 {
                Some(0)
            } else if view.columns() == 1 && view.rows() == output_rows {
                Some(row)
            } else if view.rows() == 1 && view.columns() == output_columns {
                Some(column)
            } else {
                None
            }
        };
        let (lhs_index, rhs_index) = match broadcast {
            LogicBroadcast::Exact => (
                compatible_index(&lhs).ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "logic lhs broadcast geometry is invalid".into(),
                        },
                        None,
                    )
                })?,
                compatible_index(&rhs).ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "logic rhs broadcast geometry is invalid".into(),
                        },
                        None,
                    )
                })?,
            ),
            LogicBroadcast::LeftScalar => (0, index),
            LogicBroadcast::RightScalar => (index, 0),
            LogicBroadcast::LeftColumn => (row, index),
            LogicBroadcast::RightColumn => (index, row),
            LogicBroadcast::LeftRow => (column, index),
            LogicBroadcast::RightRow => (index, column),
        };
        let lhs = lhs.get_column_major(lhs_index).ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: "logic lhs broadcast geometry is invalid".into(),
                },
                None,
            )
        })?;
        let rhs = rhs.get_column_major(rhs_index).ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: "logic rhs broadcast geometry is invalid".into(),
                },
                None,
            )
        })?;
        Ok(operation(lhs, rhs))
    })
}

#[macro_export]
macro_rules! impl_logic_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name {
            lhs: ManagedPort<bool>,
            rhs: ManagedPort<bool>,
            out: ManagedPort<bool>,
            marker: PhantomData<($arg1_type, $arg2_type, $out_type)>,
        }
        impl MechFunctionFactory for $struct_name {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let _ = lhs.try_managed::<$arg1_type>()?;
                let _ = rhs.try_managed::<$arg2_type>()?;
                let _ = out.try_managed::<$out_type>()?;
                let lhs = lhs.try_managed_element::<bool>()?;
                let rhs = rhs.try_managed_element::<bool>()?;
                let out = out.try_managed_element::<bool>()?;
                Ok(Box::new(Self {
                    lhs,
                    rhs,
                    out,
                    marker: PhantomData,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some($crate::logic_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
        }
        impl MechFunctionImpl for $struct_name {
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_binary_port_views(
                    &self.lhs,
                    &self.rhs,
                    &self.out,
                    |lhs, rhs, out| $op!(lhs, rhs, out),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some($crate::logic_binary_full_write_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $struct_name {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<bool>", stringify!($struct_name));
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let lhs = compile_value_cell_register(self.lhs.cell(), ctx)?;
                let rhs = compile_value_cell_register(self.rhs.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, out, lhs, rhs);
                Ok(out)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_logic_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, bool, bool, impl_logic_binop);
    };
}

#[macro_export]
macro_rules! impl_canonical_logic_binop_specializer {
    ($specializer:ident, $module:ident, $lib:ident, $operation:literal) => {
        #[cfg(feature = "source")]
        pub struct $specializer {}

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
                let first = specialization.input(0).expect("validated first input");
                let second = specialization.input(1).expect("validated second input");
                let extents = $crate::semantic_logic_extents(&[first, second])?;
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

#[cfg(all(
    test,
    feature = "runtime",
    feature = "and",
    feature = "not",
    feature = "bool",
    feature = "matrix2",
    feature = "matrixd"
))]
mod invocation_port_tests {
    use super::*;
    use mech_core::snapshot::SequenceView;
    use nalgebra::{DMatrix, Matrix2};

    fn managed<F: MechFunctionFactory>(
        invocation: FunctionInvocation,
        operation: &'static str,
    ) -> SpecializedFunction {
        let implementation = F::new_invocation(invocation.clone()).unwrap();
        let contract = F::declared_operation_contract().unwrap().clone();
        SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name(operation, contract).unwrap(),
            RuntimeFunctionId::from_name(operation),
            ExecutionTarget::DirectRuntime,
            F::implementation_memory_class(),
        )
        .unwrap()
    }

    #[test]
    fn scalar_binary_and_unary_factories_use_canonical_ports() {
        let binary_out = ValueCell::from_exact(true).unwrap();
        let binary = managed::<crate::and::AndSS>(
            FunctionInvocation::binary(
                binary_out.clone(),
                ValueCell::from_exact(true).unwrap(),
                ValueCell::from_exact(false).unwrap(),
            ),
            "logic/and",
        );
        binary.instance().solve_result().unwrap();
        assert!(matches!(
            binary_out.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
        assert_eq!(
            binary.instance().reactive_output_cell_ids(),
            vec![binary_out.reactive_cell_id()]
        );

        let unary_out = ValueCell::from_exact(false).unwrap();
        let unary = managed::<crate::not::NotS<bool>>(
            FunctionInvocation::unary(unary_out.clone(), ValueCell::from_exact(true).unwrap()),
            "logic/not",
        );
        unary.instance().solve_result().unwrap();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(binary.instance())?;
            participant.capture_function_instance(unary.instance())?;
            binary_out.replace(&ValueCell::from_exact(true)?.snapshot()?)?;
            unary_out.replace(&ValueCell::from_exact(true)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(matches!(
            binary_out.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
        assert!(matches!(
            unary_out.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
    }

    #[test]
    fn fixed_and_dynamic_logic_factories_publish_managed_cells() {
        let fixed_lhs = Ref::new(Matrix2::new(true, true, false, false));
        let fixed_rhs = Ref::new(Matrix2::new(true, false, true, false));
        let fixed_out =
            ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::from_element(false)), 2, 2).unwrap();
        managed::<crate::and::AndM2M2>(
            FunctionInvocation::binary(
                fixed_out.clone(),
                ValueCell::from_exact_matrix_ref(fixed_lhs, 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(fixed_rhs, 2, 2).unwrap(),
            ),
            "logic/and",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_bool_matrix(&fixed_out, &[true, false, false, false]);
        let fixed_not_input = Ref::new(Matrix2::new(true, false, false, true));
        let fixed_not_output =
            ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::from_element(false)), 2, 2).unwrap();
        managed::<crate::not::NotV<bool, Matrix2<bool>>>(
            FunctionInvocation::unary(
                fixed_not_output.clone(),
                ValueCell::from_exact_matrix_ref(fixed_not_input, 2, 2).unwrap(),
            ),
            "logic/not",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_bool_matrix(&fixed_not_output, &[false, true, true, false]);

        let dynamic_lhs = Ref::new(DMatrix::from_row_slice(2, 2, &[true, true, false, false]));
        let dynamic_rhs = Ref::new(DMatrix::from_row_slice(2, 2, &[true, false, true, false]));
        let dynamic_out =
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::from_element(2, 2, false)), 2, 2)
                .unwrap();
        let function = managed::<crate::and::AndMDMD>(
            FunctionInvocation::binary(
                dynamic_out.clone(),
                ValueCell::from_exact_matrix_ref(dynamic_lhs, 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(dynamic_rhs, 2, 2).unwrap(),
            ),
            "logic/and",
        );
        function.instance().solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            dynamic_out.replace(
                &ValueCell::from_exact_matrix_ref(
                    Ref::new(DMatrix::from_element(1, 1, true)),
                    1,
                    1,
                )?
                .snapshot()?,
            )?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_bool_matrix(&dynamic_out, &[true, false, false, false]);
        let dynamic_not_input =
            Ref::new(DMatrix::from_row_slice(2, 2, &[true, false, false, true]));
        let dynamic_not_output =
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::from_element(2, 2, false)), 2, 2)
                .unwrap();
        managed::<crate::not::NotV<bool, DMatrix<bool>>>(
            FunctionInvocation::unary(
                dynamic_not_output.clone(),
                ValueCell::from_exact_matrix_ref(dynamic_not_input, 2, 2).unwrap(),
            ),
            "logic/not",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_bool_matrix(&dynamic_not_output, &[false, true, true, false]);
    }

    fn assert_bool_matrix(cell: &ValueCell, expected: &[bool]) {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = snapshot.data() else {
            panic!("expected managed matrix")
        };
        let SequenceView::Bool(elements) = matrix.elements() else {
            panic!("expected Boolean elements")
        };
        assert_eq!(elements, expected);
    }

    #[cfg(all(
        feature = "vectord",
        feature = "row_vectord",
        feature = "or",
        feature = "xor"
    ))]
    #[test]
    fn managed_logic_preserves_scalar_row_and_column_broadcasts() {
        let matrix = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::from_row_slice(
                    2,
                    3,
                    &[true, false, true, false, true, false],
                )),
                2,
                3,
            )
            .unwrap()
        };
        let column = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::DVector::from_vec(vec![true, false])),
                2,
                1,
            )
            .unwrap()
        };
        let row = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::RowDVector::from_vec(vec![false, true, false])),
                1,
                3,
            )
            .unwrap()
        };
        fn check<F: MechFunctionFactory>(
            lhs: ValueCell,
            rhs: ValueCell,
            operation: &'static str,
            expected: &[bool],
        ) {
            let output = ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::from_element(2, 3, false)),
                2,
                3,
            )
            .unwrap();
            let alias = output.clone();
            let function = managed::<F>(
                FunctionInvocation::binary(output.clone(), lhs, rhs),
                operation,
            );
            function.instance().solve_result().unwrap();
            assert_bool_matrix(&alias, expected);
            assert!(alias.same_cell(&output));
        }
        check::<crate::and::AndSMD>(
            ValueCell::from_exact(true).unwrap(),
            matrix(),
            "logic/and",
            &[true, false, true, false, true, false],
        );
        check::<crate::and::AndMDS>(
            matrix(),
            ValueCell::from_exact(false).unwrap(),
            "logic/and",
            &[false; 6],
        );
        check::<crate::and::AndMDVD>(
            matrix(),
            column(),
            "logic/and",
            &[true, false, true, false, false, false],
        );
        check::<crate::and::AndVDMD>(
            column(),
            matrix(),
            "logic/and",
            &[true, false, true, false, false, false],
        );
        check::<crate::or::OrMDRD>(
            matrix(),
            row(),
            "logic/or",
            &[true, true, true, false, true, false],
        );
        check::<crate::or::OrRDMD>(
            row(),
            matrix(),
            "logic/or",
            &[true, true, true, false, true, false],
        );
        check::<crate::xor::XorMDMD>(matrix(), matrix(), "logic/xor", &[false; 6]);

        fn check_degenerate_row<F: MechFunctionFactory>(
            operation: &'static str,
            expected: &[bool],
        ) {
            let lhs = ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::RowDVector::from_vec(vec![
                    true, false, true, false, true,
                ])),
                1,
                5,
            )
            .unwrap();
            let rhs = ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::RowDVector::from_vec(vec![false])),
                1,
                1,
            )
            .unwrap();
            let output = ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::RowDVector::from_element(5, false)),
                1,
                5,
            )
            .unwrap();
            let function = managed::<F>(
                FunctionInvocation::binary(output.clone(), lhs, rhs),
                operation,
            );
            function.instance().solve_result().unwrap();
            assert_bool_matrix(&output, expected);
        }
        check_degenerate_row::<crate::and::AndRDRD>("logic/and", &[false; 5]);
        check_degenerate_row::<crate::or::OrRDRD>("logic/or", &[true, false, true, false, true]);
        check_degenerate_row::<crate::xor::XorRDRD>("logic/xor", &[true, false, true, false, true]);

        fn check_degenerate_column<F: MechFunctionFactory>(
            lhs: ValueCell,
            rhs: ValueCell,
            operation: &'static str,
            expected: &[bool],
        ) {
            let output = ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::DVector::from_element(5, false)),
                5,
                1,
            )
            .unwrap();
            let function = managed::<F>(
                FunctionInvocation::binary(output.clone(), lhs, rhs),
                operation,
            );
            function.instance().solve_result().unwrap();
            assert_bool_matrix(&output, expected);
        }
        let column = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::DVector::from_vec(vec![
                    true, false, true, false, true,
                ])),
                5,
                1,
            )
            .unwrap()
        };
        let singleton = || {
            ValueCell::from_exact_matrix_ref(
                Ref::new(nalgebra::RowDVector::from_vec(vec![false])),
                1,
                1,
            )
            .unwrap()
        };
        check_degenerate_column::<crate::and::AndVDRD>(
            column(),
            singleton(),
            "logic/and",
            &[false; 5],
        );
        check_degenerate_column::<crate::and::AndRDVD>(
            singleton(),
            column(),
            "logic/and",
            &[false; 5],
        );
        check_degenerate_column::<crate::or::OrVDRD>(
            column(),
            singleton(),
            "logic/or",
            &[true, false, true, false, true],
        );
        check_degenerate_column::<crate::or::OrRDVD>(
            singleton(),
            column(),
            "logic/or",
            &[true, false, true, false, true],
        );
        check_degenerate_column::<crate::xor::XorVDRD>(
            column(),
            singleton(),
            "logic/xor",
            &[true, false, true, false, true],
        );
        check_degenerate_column::<crate::xor::XorRDVD>(
            singleton(),
            column(),
            "logic/xor",
            &[true, false, true, false, true],
        );
    }

    #[test]
    fn logic_ports_reject_wrong_types_and_layouts() {
        assert!(
            crate::and::AndSS::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(false).unwrap(),
                ValueCell::from_exact(1_usize).unwrap(),
                ValueCell::from_exact(true).unwrap(),
            ))
            .is_err()
        );
        assert!(
            crate::and::AndSS::new_invocation(FunctionInvocation::unary(
                ValueCell::from_exact(false).unwrap(),
                ValueCell::from_exact(true).unwrap(),
            ))
            .is_err()
        );
    }
}
