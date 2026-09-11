use crate::*;
use nalgebra::ComplexField;
use num_traits::{One, Zero};

fn solve_planned<T>(
    lhs: ManagedValueView<'_, T>,
    rhs: ManagedValueView<'_, T>,
    out: &mut ManagedValueViewMut<'_, T>,
    coefficients: &mut ManagedValueViewMut<'_, T>,
    solution: &mut ManagedValueViewMut<'_, T>,
    pivots: &mut ManagedValueViewMut<'_, usize>,
) -> MResult<()>
where
    T: Copy + ComplexField,
{
    let rows = lhs.rows();
    let columns = rhs.columns();
    let invalid = || {
        MechError::from(MemoryRuntimeError::InvalidLayout {
            object: None,
            size: rows as u64,
            alignment: core::mem::align_of::<T>() as u32,
            reason: "matrix solve workspace geometry disagrees with the admitted plan",
        })
    };
    coefficients.try_fill_column_major(|index| lhs.get_column_major(index).ok_or_else(invalid))?;
    solution.try_fill_column_major(|index| rhs.get_column_major(index).ok_or_else(invalid))?;
    pivots.try_fill_column_major(Ok)?;

    // The maintained algorithm is LU with partial row pivoting. Its exact
    // coefficient scaling and column-update order match the previous nalgebra
    // implementation, but every mutable lane now belongs to R5 scratch.
    for pivot_column in 0..rows {
        let mut pivot_row = pivot_column;
        let mut magnitude = coefficients
            .get_column_major(pivot_column * rows + pivot_column)
            .ok_or_else(invalid)?
            .norm1();
        for row in pivot_column + 1..rows {
            let candidate = coefficients
                .get_column_major(pivot_column * rows + row)
                .ok_or_else(invalid)?
                .norm1();
            if candidate > magnitude {
                pivot_row = row;
                magnitude = candidate;
            }
        }
        let diagonal = coefficients
            .get_column_major(pivot_column * rows + pivot_row)
            .ok_or_else(invalid)?;
        if diagonal.is_zero() {
            continue;
        }
        pivots.try_set_column_major(pivot_column, pivot_row)?;
        if pivot_row != pivot_column {
            for column in 0..rows {
                let current = column * rows + pivot_column;
                let selected = column * rows + pivot_row;
                let a = coefficients.get_column_major(current).ok_or_else(invalid)?;
                let b = coefficients
                    .get_column_major(selected)
                    .ok_or_else(invalid)?;
                coefficients.try_set_column_major(current, b)?;
                coefficients.try_set_column_major(selected, a)?;
            }
        }
        let inverse = T::one() / diagonal;
        for row in pivot_column + 1..rows {
            let index = pivot_column * rows + row;
            let value = coefficients.get_column_major(index).ok_or_else(invalid)? * inverse;
            coefficients.try_set_column_major(index, value)?;
        }
        for column in pivot_column + 1..rows {
            let factor = -coefficients
                .get_column_major(column * rows + pivot_column)
                .ok_or_else(invalid)?;
            for row in pivot_column + 1..rows {
                let index = column * rows + row;
                let value = factor
                    * coefficients
                        .get_column_major(pivot_column * rows + row)
                        .ok_or_else(invalid)?
                    + coefficients.get_column_major(index).ok_or_else(invalid)?;
                coefficients.try_set_column_major(index, value)?;
            }
        }
    }

    for pivot_column in 0..rows {
        let pivot_row = pivots.get_column_major(pivot_column).ok_or_else(invalid)?;
        if pivot_row != pivot_column {
            for column in 0..columns {
                let current = column * rows + pivot_column;
                let selected = column * rows + pivot_row;
                let a = solution.get_column_major(current).ok_or_else(invalid)?;
                let b = solution.get_column_major(selected).ok_or_else(invalid)?;
                solution.try_set_column_major(current, b)?;
                solution.try_set_column_major(selected, a)?;
            }
        }
    }
    for column in 0..columns {
        for pivot in 0..rows.saturating_sub(1) {
            let factor = -solution
                .get_column_major(column * rows + pivot)
                .ok_or_else(invalid)?;
            for row in pivot + 1..rows {
                let index = column * rows + row;
                let value = factor
                    * coefficients
                        .get_column_major(pivot * rows + row)
                        .ok_or_else(invalid)?
                    + solution.get_column_major(index).ok_or_else(invalid)?;
                solution.try_set_column_major(index, value)?;
            }
        }
    }
    for column in 0..columns {
        for pivot in (0..rows).rev() {
            let diagonal = coefficients
                .get_column_major(pivot * rows + pivot)
                .ok_or_else(invalid)?;
            if diagonal.is_zero() {
                return Err(MechError::new(MatrixSolveSingular, None).with_compiler_loc());
            }
            let index = column * rows + pivot;
            let value = solution.get_column_major(index).ok_or_else(invalid)? / diagonal;
            solution.try_set_column_major(index, value)?;
            for row in 0..pivot {
                let index = column * rows + row;
                let value = -value
                    * coefficients
                        .get_column_major(pivot * rows + row)
                        .ok_or_else(invalid)?
                    + solution.get_column_major(index).ok_or_else(invalid)?;
                solution.try_set_column_major(index, value)?;
            }
        }
    }
    out.try_fill_column_major(|index| solution.get_column_major(index).ok_or_else(invalid))
}

static PURE_MATRIX_SOLVE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
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
                shape: ShapeRule::SameAsInput { input: 1 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSolveSingular;

impl MechErrorKind for MatrixSolveSingular {
    fn name(&self) -> &str {
        "MatrixSolveSingular"
    }

    fn message(&self) -> String {
        "Matrix solve requires a nonsingular coefficient matrix".to_string()
    }
}

// Solve  ------------------------------------------------------------------

#[macro_export]
macro_rules! impl_binop_solve {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            lhs: ManagedPort<T>,
            rhs: ManagedPort<T>,
            out: ManagedPort<T>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            #[cfg(feature = "semantic-compiler")]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ComplexField
                + FunctionRuntimeType
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One
                + ConstElem
                + CompileConst
                + CanonicalMatrixElementBacking
                + FunctionPortBacking
                + ManagedElement
                + FunctionRuntimeType,
            #[cfg(not(feature = "semantic-compiler"))]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ComplexField
                + FunctionRuntimeType
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One
                + FunctionPortBacking
                + ManagedElement,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::MatrixSolve
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_SOLVE_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let _ = lhs.try_managed::<$arg1_type>()?;
                let _ = rhs.try_managed::<$arg2_type>()?;
                let _ = out.try_managed::<$out_type>()?;
                let lhs = lhs.try_managed_element::<T>()?;
                let rhs = rhs.try_managed_element::<T>()?;
                let out = out.try_managed_element::<T>()?;
                Ok(Box::new(Self { lhs, rhs, out }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ComplexField
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One
                + ManagedElement,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                frame.with_matrix_solve_port_views(
                    &self.lhs,
                    &self.rhs,
                    &self.out,
                    solve_planned,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_SOLVE_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: CanonicalMatrixElementBacking + ConstElem + CompileConst + FunctionRuntimeType,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
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

macro_rules! impl_solve {
    ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
        impl_binop_solve!($name, $type1, $type2, $out_type, solve_op);
    };
}

#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_solve!(MatrixSolveMDVD, DMatrix<T>, DVector<T>, DVector<T>);

#[cfg(feature = "matrixd")]
impl_solve!(MatrixSolveMDMD, DMatrix<T>, DMatrix<T>, DMatrix<T>);

// Source matrices with one row use RowDVector storage, including a 1x1
// coefficient matrix. Retain that exact representation for the valid
// one-equation, multiple-right-hand-side solve.
#[cfg(feature = "row_vectord")]
impl_solve!(
    MatrixSolveRDRD,
    RowDVector<T>,
    RowDVector<T>,
    RowDVector<T>
);

// Keep fixed-shape source mathematical. The semantic compiler sees the
// ordinary solve operation and compute backends can scalarize it without the
// program spelling out an inverse or splitting a matrix right-hand side into
// columns.
#[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
impl_solve!(MatrixSolveM2M2x3, Matrix2<T>, Matrix2x3<T>, Matrix2x3<T>);

#[cfg(all(
    test,
    feature = "f64",
    feature = "matrixd",
    feature = "vectord",
    feature = "row_vectord"
))]
mod canonical_port_tests {
    use super::*;

    fn managed<F: MechFunctionFactory>(invocation: FunctionInvocation) -> SpecializedFunction {
        let implementation = F::new_invocation(invocation.clone()).unwrap();
        SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name(
                "matrix/solve",
                F::declared_operation_contract().unwrap().clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("matrix/solve"),
            ExecutionTarget::DirectRuntime,
            F::implementation_memory_class(),
        )
        .unwrap()
    }

    fn values(cell: &ValueCell) -> Vec<f64> {
        let value = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("matrix output")
        };
        let snapshot::SequenceView::F64(elements) = matrix.elements() else {
            panic!("F64 output")
        };
        elements.iter().map(|value| value.to_f64()).collect()
    }

    fn assert_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1.0e-12,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn vector_and_matrix_rhs_use_managed_ports_and_planned_scratch() {
        let coefficients = DMatrix::from_row_slice(2, 2, &[4.0, 1.0, 2.0, 3.0]);
        let lhs = ValueCell::from_exact(coefficients.clone()).unwrap();
        let vector_rhs = ValueCell::from_exact(DVector::from_vec(vec![9.0, 8.0])).unwrap();
        let vector_out = ValueCell::from_exact(DVector::<f64>::zeros(2)).unwrap();
        let vector = managed::<MatrixSolveMDVD<f64>>(FunctionInvocation::binary(
            vector_out.clone(),
            lhs.clone(),
            vector_rhs,
        ));
        vector.instance().solve_result().unwrap();
        assert_close(&values(&vector_out), &[1.9, 1.4]);

        let rhs = DMatrix::<f64>::identity(2, 2);
        let matrix_out = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 2)).unwrap();
        let matrix = managed::<MatrixSolveMDMD<f64>>(FunctionInvocation::binary(
            matrix_out.clone(),
            lhs,
            ValueCell::from_exact(rhs.clone()).unwrap(),
        ));
        matrix.instance().solve_result().unwrap();
        let expected = coefficients.lu().solve(&rhs).unwrap();
        let actual = DMatrix::from_row_slice(2, 2, &values(&matrix_out));
        assert!((actual - expected).norm() < 1.0e-12);
    }

    #[test]
    fn one_by_one_coefficients_support_a_row_of_right_hand_sides() {
        let lhs = ValueCell::from_exact(RowDVector::from_vec(vec![2.0_f64])).unwrap();
        let rhs = ValueCell::from_exact(RowDVector::from_vec(vec![2.0, 4.0, 8.0])).unwrap();
        let out = ValueCell::from_exact(RowDVector::<f64>::zeros(3)).unwrap();
        let function = managed::<MatrixSolveRDRD<f64>>(FunctionInvocation::binary(
            out.clone(),
            lhs,
            rhs,
        ));
        function.instance().solve_result().unwrap();
        assert_close(&values(&out), &[1.0, 2.0, 4.0]);
    }

    #[test]
    fn singular_resolve_is_atomic_and_checkpointed() {
        let lhs = ValueCell::from_exact(DMatrix::<f64>::identity(2, 2)).unwrap();
        let rhs = ValueCell::from_exact(DVector::from_vec(vec![3.0, 4.0])).unwrap();
        let out = ValueCell::from_exact(DVector::<f64>::from_element(2, -1.0)).unwrap();
        let alias = out.clone();
        let function = managed::<MatrixSolveMDVD<f64>>(FunctionInvocation::binary(
            out.clone(),
            lhs.clone(),
            rhs,
        ));
        function.instance().solve_result().unwrap();
        let previous = out.snapshot().unwrap();
        let version = out.published_version();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            lhs.replace(
                &ValueCell::from_exact(DMatrix::from_row_slice(2, 2, &[1.0_f64, 2.0, 2.0, 4.0]))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )?;
            assert_eq!(
                function.instance().solve_result().unwrap_err().kind_name(),
                "MatrixSolveSingular"
            );
            assert_close(&values(&out), &[3.0, 4.0]);
            assert_eq!(out.published_version(), version);
            out.replace(
                &ValueCell::from_exact(DVector::from_vec(vec![99.0_f64]))
                    .unwrap()
                    .snapshot()
                    .unwrap(),
            )?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(out.same_logical_cell(&alias));
        assert_close(&values(&alias), &[3.0, 4.0]);
        assert_eq!(out.snapshot().unwrap().shape(), previous.shape());
    }

    #[test]
    fn bound_solve_follows_dynamic_growth_without_rebinding() {
        let lhs = ValueCell::from_exact(DMatrix::<f64>::identity(2, 2)).unwrap();
        let rhs = ValueCell::from_exact(DMatrix::from_row_slice(2, 1, &[2.0_f64, 3.0])).unwrap();
        let out = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 1)).unwrap();
        let alias = out.clone();
        let function = managed::<MatrixSolveMDMD<f64>>(FunctionInvocation::binary(
            out.clone(),
            lhs.clone(),
            rhs.clone(),
        ));
        function.instance().solve_result().unwrap();
        let before = out.snapshot().unwrap();
        lhs.replace(
            &ValueCell::from_exact(DMatrix::<f64>::identity(3, 3))
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        rhs.replace(
            &ValueCell::from_exact(DMatrix::from_row_slice(
                3,
                2,
                &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
            ))
            .unwrap()
            .snapshot()
            .unwrap(),
        )
        .unwrap();
        function.instance().solve_result().unwrap();
        assert!(out.same_logical_cell(&alias));
        assert_close(&values(&alias), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(
            alias.snapshot().unwrap().shape().parameter_values(),
            &[3, 2]
        );
        assert_eq!(before.shape().parameter_values(), &[2, 1]);
    }

    #[test]
    fn pivoting_and_triangular_updates_match_direct_lu() {
        for rows in 1..=6 {
            for columns in 1..=3 {
                let coefficients = DMatrix::from_fn(rows, rows, |row, column| {
                    let value = ((row * 7 + column * 11 + 3) % 17) as f64 - 8.0;
                    if row == column { value + 0.125 } else { value }
                });
                let rhs = DMatrix::from_fn(rows, columns, |row, column| {
                    (row + column * 2) as f64 + 0.25
                });
                let expected = coefficients.clone().lu().solve(&rhs);
                let lhs = ValueCell::from_exact(coefficients).unwrap();
                let rhs = ValueCell::from_exact(rhs).unwrap();
                let out = ValueCell::from_exact(DMatrix::<f64>::zeros(rows, columns)).unwrap();
                let function = managed::<MatrixSolveMDMD<f64>>(FunctionInvocation::binary(
                    out.clone(),
                    lhs,
                    rhs,
                ));
                let result = function.instance().solve_result();
                match expected {
                    Some(expected) => {
                        result.unwrap();
                        let actual = DMatrix::from_row_slice(rows, columns, &values(&out));
                        assert!((actual - expected).norm() < 1.0e-10);
                    }
                    None => assert_eq!(result.unwrap_err().kind_name(), "MatrixSolveSingular"),
                }
            }
        }
    }

    #[cfg(feature = "f32")]
    #[test]
    fn f32_solve_matches_direct_lu() {
        let coefficients =
            DMatrix::from_row_slice(3, 3, &[0.0_f32, 1.0, 4.0, 4.0, -2.0, 1.0, 1.0, 1.0, 2.0]);
        let rhs = DVector::from_vec(vec![3.0_f32, 1.0, -1.0]);
        let expected = coefficients.clone().lu().solve(&rhs).unwrap();
        let out = ValueCell::from_exact(DVector::<f32>::zeros(3)).unwrap();
        let function = managed::<MatrixSolveMDVD<f32>>(FunctionInvocation::binary(
            out.clone(),
            ValueCell::from_exact(coefficients).unwrap(),
            ValueCell::from_exact(rhs).unwrap(),
        ));
        function.instance().solve_result().unwrap();
        let value = out.snapshot().unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("matrix output")
        };
        let snapshot::SequenceView::F32(actual) = matrix.elements() else {
            panic!("F32 output")
        };
        for (actual, expected) in actual.iter().zip(expected.iter()) {
            assert_eq!(actual.to_f32().to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn solve_rejects_wrong_rhs_representation_and_layout() {
        let lhs = ValueCell::from_exact(DMatrix::<f64>::identity(2, 2)).unwrap();
        let wrong_rhs = ValueCell::from_exact(DMatrix::<f64>::identity(2, 2)).unwrap();
        let output = ValueCell::from_exact(DVector::<f64>::zeros(2)).unwrap();
        assert!(
            MatrixSolveMDVD::<f64>::new_invocation(FunctionInvocation::binary(
                output.clone(),
                lhs.clone(),
                wrong_rhs
            ))
            .is_err()
        );
        assert!(
            MatrixSolveMDVD::<f64>::new_invocation(FunctionInvocation::unary(output, lhs)).is_err()
        );
    }

    #[test]
    fn solve_rejects_an_unrelated_scratch_authority_before_publication() {
        let lhs = ValueCell::from_exact(DMatrix::<f64>::identity(2, 2)).unwrap();
        let rhs = ValueCell::from_exact(DVector::from_vec(vec![3.0_f64, 4.0])).unwrap();
        let out = ValueCell::from_exact(DVector::<f64>::from_element(2, -1.0)).unwrap();
        let invocation = FunctionInvocation::binary(out.clone(), lhs, rhs);
        let implementation = MatrixSolveMDVD::<f64>::new_invocation(invocation.clone()).unwrap();
        let function = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name(
                "matrix/solve",
                PURE_MATRIX_SOLVE_CONTRACT.clone(),
            )
            .unwrap(),
            RuntimeFunctionId::from_name("matrix/solve"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::NoAdditionalScratch,
        )
        .unwrap();
        let version = out.published_version();
        assert!(function.instance().solve_result().is_err());
        assert_close(&values(&out), &[-1.0, -1.0]);
        assert_eq!(out.published_version(), version);
    }

    #[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
    #[test]
    fn fixed_matrix_rhs_uses_managed_ports() {
        let rhs = ValueCell::from_exact(Matrix2x3::new(1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0)).unwrap();
        let out = ValueCell::from_exact(Matrix2x3::<f64>::zeros()).unwrap();
        let function = managed::<MatrixSolveM2M2x3<f64>>(FunctionInvocation::binary(
            out.clone(),
            ValueCell::from_exact(Matrix2::<f64>::identity()).unwrap(),
            rhs,
        ));
        function.instance().solve_result().unwrap();
        assert_close(&values(&out), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }
}

#[cfg(feature = "source")]
pub struct MatrixSolve;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for MatrixSolve {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = invocation.input(0).expect("validated solve lhs");
        let rhs = invocation.input(1).expect("validated solve rhs");
        let lhs_shape = lhs.matrix_descriptor()?.ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(0),
                    expected: "matrix coefficient input".into(),
                    found: format!("{:?}", lhs.representation()),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let rhs_shape = rhs.matrix_descriptor()?.ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(1),
                    expected: "matrix right-hand side".into(),
                    found: format!("{:?}", rhs.representation()),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        if lhs_shape.rows != lhs_shape.cols || lhs_shape.rows != rhs_shape.rows {
            return Err(MechError::new(
                DimensionMismatch {
                    dims: vec![
                        lhs_shape.rows,
                        lhs_shape.cols,
                        rhs_shape.rows,
                        rhs_shape.cols,
                    ],
                },
                Some(
                    "Matrix solve requires a square coefficient matrix whose rows match the right-hand side"
                        .into(),
                ),
            )
            .with_compiler_loc());
        }
        context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![rhs_shape.rows as u64, rhs_shape.cols as u64].into_boxed_slice()]
                .into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}
