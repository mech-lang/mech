use crate::*;
use std::sync::LazyLock;

static PURE_TRANSPOSE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::TransposeOf { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
});

// Transpose ------------------------------------------------------------------

macro_rules! transpose_op {
    ($arg:expr, $out:expr) => {
        unsafe {
            *$out = (*$arg).transpose();
        }
    };
}

#[macro_export]
macro_rules! impl_transpose {
    ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            arg: Ref<$arg_type>,
            out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + PartialEq + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + CompileConst + ConstElem,
            $arg_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
                let (out, arg) = invocation.expect_unary()?;
                let arg: Ref<$arg_type> = arg.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new($struct_name { arg, out }))
            }

        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + 'static + PartialEq + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn solve_result(&self) -> MResult<()> {
                let arg_ptr = self.arg.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(arg_ptr, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_TRANSPOSE_CONTRACT)
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
                let name = format!("{}<{}>", stringify!($struct_name), <T as FunctionRuntimeType>::REPRESENTATION);
                compile_unop!(name, self.out, self.arg, ctx);
            }
        }
    };
}

#[cfg(feature = "matrix1")]
impl_transpose!(TransposeM1, Matrix1<T>, Matrix1<T>, transpose_op);
#[cfg(feature = "matrix2")]
impl_transpose!(TransposeM2, Matrix2<T>, Matrix2<T>, transpose_op);
#[cfg(feature = "matrix3")]
impl_transpose!(TransposeM3, Matrix3<T>, Matrix3<T>, transpose_op);
#[cfg(feature = "matrix4")]
impl_transpose!(TransposeM4, Matrix4<T>, Matrix4<T>, transpose_op);
#[cfg(all(feature = "matrix2x3", feature = "matrix3x2"))]
impl_transpose!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>, transpose_op);
#[cfg(all(feature = "matrix3x2", feature = "matrix2x3"))]
impl_transpose!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>, transpose_op);
#[cfg(feature = "matrixd")]
impl_transpose!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op);
#[cfg(all(feature = "vector2", feature = "row_vector2"))]
impl_transpose!(TransposeV2, Vector2<T>, RowVector2<T>, transpose_op);
#[cfg(all(feature = "vector3", feature = "row_vector3"))]
impl_transpose!(TransposeV3, Vector3<T>, RowVector3<T>, transpose_op);
#[cfg(all(feature = "vector4", feature = "row_vector4"))]
impl_transpose!(TransposeV4, Vector4<T>, RowVector4<T>, transpose_op);
#[cfg(all(feature = "vectord", feature = "row_vectord"))]
impl_transpose!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op);
#[cfg(all(feature = "row_vector2", feature = "vector2"))]
impl_transpose!(TransposeR2, RowVector2<T>, Vector2<T>, transpose_op);
#[cfg(all(feature = "row_vector3", feature = "vector3"))]
impl_transpose!(TransposeR3, RowVector3<T>, Vector3<T>, transpose_op);
#[cfg(all(feature = "row_vector4", feature = "vector4"))]
impl_transpose!(TransposeR4, RowVector4<T>, Vector4<T>, transpose_op);
#[cfg(all(feature = "row_vectord", feature = "vectord"))]
impl_transpose!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op);

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "bool",
    feature = "string",
    feature = "matrix2",
    feature = "matrix2x3",
    feature = "matrix3x2",
    feature = "matrixd"
))]
mod canonical_port_tests {
    use super::*;

    #[test]
    fn fixed_dynamic_and_non_numeric_transposes_use_exact_ports() {
        let matrix = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let fixed_out = Ref::new(Matrix2::zeros());
        TransposeM2::<f64>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact_matrix_ref(fixed_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(matrix, 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*fixed_out.borrow(), Matrix2::new(1.0, 3.0, 2.0, 4.0));

        let rectangular = Ref::new(Matrix2x3::new(1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0));
        let rectangular_out = Ref::new(Matrix3x2::zeros());
        TransposeM2x3::<f64>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact_matrix_ref(rectangular_out.clone(), 3, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(rectangular, 2, 3).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *rectangular_out.borrow(),
            Matrix3x2::new(1.0, 4.0, 2.0, 5.0, 3.0, 6.0)
        );

        let dynamic = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_out = Ref::new(DMatrix::zeros(3, 2));
        let alias = dynamic_out.clone();
        let function = TransposeMD::<f64>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact_matrix_ref(dynamic_out.clone(), 3, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(dynamic, 2, 3).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *dynamic_out.borrow_mut() = DMatrix::from_element(1, 4, -1.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(dynamic_out.same_handle(&alias));
        assert_eq!(dynamic_out.borrow().shape(), (3, 2));

        let bool_arg = Ref::new(Matrix2::new(true, false, false, true));
        let bool_out = Ref::new(Matrix2::from_element(false));
        TransposeM2::<bool>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact_matrix_ref(bool_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(bool_arg.clone(), 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*bool_out.borrow(), *bool_arg.borrow());
    }

    #[test]
    fn transpose_rejects_wrong_storage_and_layout() {
        let output = ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::<f64>::zeros()), 2, 2)
            .unwrap();
        let wrong = ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::<f64>::zeros(2, 2)), 2, 2)
            .unwrap();
        assert!(TransposeM2::<f64>::new_invocation(FunctionInvocation::unary(
            output.clone(),
            wrong,
        ))
        .is_err());
        let arg = ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::<f64>::identity()), 2, 2)
            .unwrap();
        assert!(TransposeM2::<f64>::new_invocation(FunctionInvocation::binary(
            output,
            arg.clone(),
            arg,
        ))
        .is_err());
    }
}

#[cfg(feature = "source")]
pub struct MatrixTranspose;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for MatrixTranspose {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = invocation.input(0).expect("validated transpose input");
        let descriptor = input.matrix_descriptor()?.ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(0),
                    expected: "exact matrix input".into(),
                    found: format!("{:?}", input.representation()),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let FunctionValueRepresentation::Matrix { element, storage } =
            input.representation().expect("matrix descriptor has representation")
        else {
            unreachable!("matrix descriptor requires matrix representation")
        };
        let storage = match storage {
            FunctionMatrixStoragePattern::Exact(storage) => {
                FunctionMatrixStoragePattern::Exact(match storage {
                    FunctionMatrixRepresentation::Matrix1 => FunctionMatrixRepresentation::Matrix1,
                    FunctionMatrixRepresentation::Matrix2 => FunctionMatrixRepresentation::Matrix2,
                    FunctionMatrixRepresentation::Matrix3 => FunctionMatrixRepresentation::Matrix3,
                    FunctionMatrixRepresentation::Matrix4 => FunctionMatrixRepresentation::Matrix4,
                    FunctionMatrixRepresentation::Matrix2x3 => FunctionMatrixRepresentation::Matrix3x2,
                    FunctionMatrixRepresentation::Matrix3x2 => FunctionMatrixRepresentation::Matrix2x3,
                    FunctionMatrixRepresentation::RowVector2 => FunctionMatrixRepresentation::Vector2,
                    FunctionMatrixRepresentation::RowVector3 => FunctionMatrixRepresentation::Vector3,
                    FunctionMatrixRepresentation::RowVector4 => FunctionMatrixRepresentation::Vector4,
                    FunctionMatrixRepresentation::RowVectorD => FunctionMatrixRepresentation::VectorD,
                    FunctionMatrixRepresentation::Vector2 => FunctionMatrixRepresentation::RowVector2,
                    FunctionMatrixRepresentation::Vector3 => FunctionMatrixRepresentation::RowVector3,
                    FunctionMatrixRepresentation::Vector4 => FunctionMatrixRepresentation::RowVector4,
                    FunctionMatrixRepresentation::VectorD => FunctionMatrixRepresentation::RowVectorD,
                    FunctionMatrixRepresentation::MatrixD => FunctionMatrixRepresentation::MatrixD,
                })
            }
            FunctionMatrixStoragePattern::AnyStorage => {
                return Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Input(0),
                        expected: "exact matrix storage".into(),
                        found: format!("{storage:?}"),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };
        context.bind_runtime_factory(
            "Transpose",
            FunctionValueRepresentation::Matrix { element, storage },
            Some((descriptor.cols, descriptor.rows)),
            &[input],
        )
    }
}
