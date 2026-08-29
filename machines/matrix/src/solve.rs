use crate::*;
use nalgebra::ComplexField;
use num_traits::{One, Zero};

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
                + AsValueKind
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
                + AsValueKind,
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
                + AsValueKind
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
            Ref<$out_type>: ToValue,
            $arg1_type: FunctionPortBacking,
            $arg2_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
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
            Ref<$out_type>: ToValue,
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
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

macro_rules! solve_op {
    ($a:expr, $b:expr, $out:expr) => {
        unsafe {
            let solution = (*$a).clone().lu().solve(&*$b).ok_or_else(|| {
                MechError::new(MatrixSolveSingular, None).with_compiler_loc()
            })?;
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
mod tests {
    use super::*;

    fn binary_args<L, R, O>(out: &Ref<O>, lhs: &Ref<L>, rhs: &Ref<R>) -> FunctionArgs
    where
        Ref<L>: ToValue,
        Ref<R>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

    #[test]
    fn vector_and_matrix_rhs_factories_use_invocation_ports() {
        let lhs = Ref::new(DMatrix::from_row_slice(2, 2, &[4.0, 1.0, 2.0, 3.0]));
        let vector_rhs = Ref::new(DVector::from_vec(vec![9.0, 8.0]));
        let legacy_out = Ref::new(DVector::<f64>::zeros(2));
        let invocation_out = Ref::new(DVector::<f64>::zeros(2));
        let legacy = MatrixSolveMDVD::<f64>::new(binary_args(
            &legacy_out,
            &lhs,
            &vector_rhs,
        ))
        .unwrap();
        let invocation = MatrixSolveMDVD::<f64>::new_invocation(
            binary_args(&invocation_out, &lhs, &vector_rhs).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());

        let matrix_rhs = Ref::new(DMatrix::<f64>::identity(2, 2));
        let matrix_out = Ref::new(DMatrix::<f64>::zeros(2, 2));
        MatrixSolveMDMD::<f64>::new_invocation(
            binary_args(&matrix_out, &lhs, &matrix_rhs).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        let residual = lhs.borrow().clone() * matrix_out.borrow().clone();
        assert!((residual - matrix_rhs.borrow().clone()).norm() < 1.0e-12);
    }

    #[test]
    fn solve_invocation_rejects_wrong_rhs_representation_and_layout() {
        let lhs = Ref::new(DMatrix::<f64>::identity(2, 2));
        let wrong_rhs = Ref::new(DMatrix::<f64>::identity(2, 2));
        let out = Ref::new(DVector::<f64>::zeros(2));
        let type_error = MatrixSolveMDVD::<f64>::new_invocation(
            binary_args(&out, &lhs, &wrong_rhs).into(),
        )
        .err()
        .expect("wrong exact right-hand-side type must be rejected");
        assert_eq!(type_error.kind_name(), "FunctionArgumentTypeMismatch");

        let arity_error = MatrixSolveMDVD::<f64>::new_invocation(
            FunctionArgs::Unary(out.to_value(), lhs.to_value()).into(),
        )
        .err()
        .expect("wrong solve invocation layout must be rejected");
        assert_eq!(arity_error.kind_name(), "IncorrectNumberOfArguments");
    }

    #[test]
    fn singular_matrix_is_a_structured_error_on_reactive_resolve() {
        let lhs = Ref::new(DMatrix::identity(2, 2));
        let rhs = Ref::new(DVector::from_vec(vec![3.0, 4.0]));
        let out = Ref::new(DVector::from_element(2, -1.0));
        let function = MatrixSolveMDVD {
            lhs: lhs.clone(),
            rhs,
            out: out.clone(),
        };

        function.solve_result().unwrap();
        let previous = out.borrow().clone();
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&function)?;
            *lhs.borrow_mut() = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 4.0]);

            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MatrixSolveSingular");
            assert_eq!(*out.borrow(), previous);
            *out.borrow_mut() = DVector::from_vec(vec![99.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), previous);
    }

    #[test]
    fn matrix_right_hand_side_is_solved_in_one_operation() {
        let lhs = Ref::new(DMatrix::from_row_slice(2, 2, &[4.0, 1.0, 2.0, 3.0]));
        let rhs = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[9.0, 1.0, 5.0, 8.0, 7.0, 2.0],
        ));
        let out = Ref::new(DMatrix::zeros(2, 3));
        let function = MatrixSolveMDMD {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();

        let residual = lhs.borrow().clone() * out.borrow().clone() - rhs.borrow().clone();
        assert!(residual.norm() < 1.0e-12);
    }
}

#[cfg(all(test, feature = "f64", feature = "matrix2", feature = "matrix2x3"))]
mod fixed_invocation_port_tests {
    use super::*;

    #[test]
    fn fixed_matrix_rhs_uses_exact_invocation_ports() {
        let lhs = Ref::new(Matrix2::<f64>::identity());
        let rhs = Ref::new(Matrix2x3::new(1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0));
        let out = Ref::new(Matrix2x3::<f64>::zeros());
        let function = MatrixSolveM2M2x3::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();

        function.solve_result().unwrap();
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
        context.bind_runtime_factory_derived_output(
            "MatrixSolve",
            Some((rhs_shape.rows, rhs_shape.cols)),
            &[lhs, rhs],
        )
    }
}
