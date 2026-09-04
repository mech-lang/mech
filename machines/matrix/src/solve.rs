use crate::*;
use nalgebra::ComplexField;
use num_traits::{One, Zero};

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
            pub lhs: Ref<$arg1_type>,
            pub rhs: Ref<$arg2_type>,
            pub out: Ref<$out_type>,
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
                + One,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_SOLVE_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs: Ref<$arg1_type> = lhs.try_ref()?;
                let rhs: Ref<$arg2_type> = rhs.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
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
                + One,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
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
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

macro_rules! solve_op {
    ($a:expr, $b:expr, $out:expr) => {
        unsafe {
            let solution = (*$a)
                .clone()
                .lu()
                .solve(&*$b)
                .ok_or_else(|| MechError::new(MatrixSolveSingular, None).with_compiler_loc())?;
            *$out = solution;
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

// Keep fixed-shape source mathematical. The semantic compiler sees the
// ordinary solve operation and compute backends can scalarize it without the
// program spelling out an inverse or splitting a matrix right-hand side into
// columns.
#[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
impl_solve!(MatrixSolveM2M2x3, Matrix2<T>, Matrix2x3<T>, Matrix2x3<T>);

#[cfg(all(test, feature = "f64", feature = "matrixd", feature = "vectord"))]
mod canonical_port_tests {
    use super::*;

    #[test]
    fn vector_and_matrix_rhs_use_exact_ports() {
        let lhs = Ref::new(DMatrix::from_row_slice(2, 2, &[4.0, 1.0, 2.0, 3.0]));
        let vector_rhs = Ref::new(DVector::from_vec(vec![9.0, 8.0]));
        let vector_out = Ref::new(DVector::<f64>::zeros(2));
        MatrixSolveMDVD::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(vector_out.clone(), 2, 1).unwrap(),
            ValueCell::from_exact_matrix_ref(lhs.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(vector_rhs, 2, 1).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();

        let matrix_rhs = Ref::new(DMatrix::<f64>::identity(2, 2));
        let matrix_out = Ref::new(DMatrix::<f64>::zeros(2, 2));
        MatrixSolveMDMD::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(matrix_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(lhs.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(matrix_rhs.clone(), 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        let residual = lhs.borrow().clone() * matrix_out.borrow().clone();
        assert!((residual - matrix_rhs.borrow().clone()).norm() < 1.0e-12);
    }

    #[test]
    fn singular_resolve_is_atomic_and_checkpointed() {
        let lhs = Ref::new(DMatrix::identity(2, 2));
        let rhs = Ref::new(DVector::from_vec(vec![3.0, 4.0]));
        let out = Ref::new(DVector::from_element(2, -1.0));
        let alias = out.clone();
        let function = MatrixSolveMDVD::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 1).unwrap(),
            ValueCell::from_exact_matrix_ref(lhs.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(rhs, 2, 1).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        let previous = out.borrow().clone();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *lhs.borrow_mut() = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 4.0]);
            assert_eq!(
                function.solve_result().unwrap_err().kind_name(),
                "MatrixSolveSingular"
            );
            assert_eq!(*out.borrow(), previous);
            *out.borrow_mut() = DVector::from_vec(vec![99.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(out.same_handle(&alias));
        assert_eq!(*out.borrow(), previous);
    }

    #[test]
    fn solve_rejects_wrong_rhs_representation_and_layout() {
        let lhs = ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::<f64>::identity(2, 2)), 2, 2)
            .unwrap();
        let wrong_rhs =
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::<f64>::identity(2, 2)), 2, 2)
                .unwrap();
        let output =
            ValueCell::from_exact_matrix_ref(Ref::new(DVector::<f64>::zeros(2)), 2, 1).unwrap();
        assert!(
            MatrixSolveMDVD::<f64>::new_invocation(FunctionInvocation::binary(
                output.clone(),
                lhs.clone(),
                wrong_rhs,
            ))
            .is_err()
        );
        assert!(
            MatrixSolveMDVD::<f64>::new_invocation(FunctionInvocation::unary(output, lhs)).is_err()
        );
    }
}

#[cfg(all(test, feature = "f64", feature = "matrix2", feature = "matrix2x3"))]
mod fixed_port_tests {
    use super::*;

    #[test]
    fn fixed_matrix_rhs_uses_exact_ports() {
        let rhs = Ref::new(Matrix2x3::new(1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0));
        let out = Ref::new(Matrix2x3::<f64>::zeros());
        MatrixSolveM2M2x3::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 3).unwrap(),
            ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::<f64>::identity()), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(rhs.clone(), 2, 3).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*out.borrow(), *rhs.borrow());
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
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![rhs_shape.rows as u64, rhs_shape.cols as u64].into_boxed_slice()]
                .into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}
