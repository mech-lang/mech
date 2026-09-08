use crate::*;
use std::sync::LazyLock;

static PURE_TRANSPOSE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
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
    });

// Transpose ------------------------------------------------------------------

trait ManagedTransposeElement: FunctionPortBacking {
    fn planned_output_footprint(
        _input: &ManagedPort<Self>,
        _output: &ManagedPort<Self>,
    ) -> MResult<Option<CurrentMemoryFootprint>>
    where
        Self: Sized,
    {
        Ok(None)
    }

    fn transpose(
        frame: &mut KernelMemoryFrame<'_>,
        input: &ManagedPort<Self>,
        output: &ManagedPort<Self>,
    ) -> MResult<()>
    where
        Self: Sized;
}

macro_rules! managed_transpose_elements {
    ($($type:ty),+ $(,)?) => {$(
        impl ManagedTransposeElement for $type {
            fn transpose(
                frame: &mut KernelMemoryFrame<'_>,
                input: &ManagedPort<Self>,
                output: &ManagedPort<Self>,
            ) -> MResult<()> {
                frame.with_unary_port_views(input, output, |input, output| {
                    let output_rows = output.rows();
                    output.try_fill_column_major(|index| {
                        let output_row = index % output_rows;
                        let output_column = index / output_rows;
                        input.get(output_column, output_row).ok_or_else(|| {
                            MechError::from(MemoryRuntimeError::InvalidLayout {
                                object: None,
                                size: input.len() as u64,
                                alignment: core::mem::align_of::<Self>() as u32,
                                reason: "transpose input and output geometry disagree",
                            })
                        })
                    })
                })
            }
        }
    )+};
}

#[cfg(feature = "u8")]
managed_transpose_elements!(u8);
#[cfg(feature = "u16")]
managed_transpose_elements!(u16);
#[cfg(feature = "u32")]
managed_transpose_elements!(u32);
#[cfg(feature = "u64")]
managed_transpose_elements!(u64);
#[cfg(feature = "u128")]
managed_transpose_elements!(u128);
#[cfg(feature = "i8")]
managed_transpose_elements!(i8);
#[cfg(feature = "i16")]
managed_transpose_elements!(i16);
#[cfg(feature = "i32")]
managed_transpose_elements!(i32);
#[cfg(feature = "i64")]
managed_transpose_elements!(i64);
#[cfg(feature = "i128")]
managed_transpose_elements!(i128);
#[cfg(feature = "f32")]
managed_transpose_elements!(f32);
#[cfg(feature = "f64")]
managed_transpose_elements!(f64);
managed_transpose_elements!(usize);
#[cfg(feature = "bool")]
managed_transpose_elements!(bool);
#[cfg(feature = "complex")]
managed_transpose_elements!(C64);
#[cfg(feature = "rational")]
managed_transpose_elements!(R64);

#[cfg(feature = "string")]
impl ManagedTransposeElement for String {
    fn planned_output_footprint(
        input: &ManagedPort<Self>,
        _output: &ManagedPort<Self>,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(Some(input.cell().current_memory_footprint()?))
    }

    fn transpose(
        frame: &mut KernelMemoryFrame<'_>,
        input: &ManagedPort<Self>,
        output: &ManagedPort<Self>,
    ) -> MResult<()> {
        let footprint = input.cell().current_memory_footprint()?;
        frame.with_admitted_canonical_output(output.cell(), footprint, |frame| {
            let value = frame.snapshot_canonical_port_value(input)?;
            let extents = input.cell().resolved_descriptor()?.current_extents()?;
            let [rows, columns] = extents.as_ref() else {
                return Err(MechError::from(MemoryRuntimeError::InvalidLayout {
                    object: None,
                    size: extents.len() as u64,
                    alignment: 1,
                    reason: "String transpose requires rank-two canonical input",
                }));
            };
            let rows = usize::try_from(*rows).map_err(|_| {
                function_shape_contract_violation(
                    "matrix/transpose",
                    "row extent exceeds the host index range",
                )
            })?;
            let columns = usize::try_from(*columns).map_err(|_| {
                function_shape_contract_violation(
                    "matrix/transpose",
                    "column extent exceeds the host index range",
                )
            })?;
            let values = match value
                .matrix_view()
                .ok_or_else(|| {
                    MechError::from(MemoryRuntimeError::InvalidLayout {
                        object: None,
                        size: 0,
                        alignment: 1,
                        reason: "String transpose input has no canonical matrix data",
                    })
                })?
                .elements()
            {
                mech_core::snapshot::SequenceView::String(values) => values,
                _ => {
                    return Err(MechError::from(MemoryRuntimeError::InvalidLayout {
                        object: None,
                        size: 0,
                        alignment: 1,
                        reason: "String transpose input has a non-String canonical sequence",
                    }));
                }
            };
            let mut next = Vec::new();
            next.try_reserve_exact(values.len()).map_err(|_| {
                MechError::from(MemoryRuntimeError::AllocationFailed {
                    object: None,
                    requested: values.len() as u64,
                    alignment: core::mem::align_of::<ValueDataDraft>() as u32,
                    space: MemorySpace::Host,
                })
            })?;
            for output_row in 0..columns {
                for output_column in 0..rows {
                    next.push(ValueDataDraft::String(
                        values[output_column * columns + output_row].to_string(),
                    ));
                }
            }
            Ok((
                (),
                output.cell().rebuild_matrix_drafts(
                    vec![columns as u64, rows as u64].into_boxed_slice(),
                    next.into_boxed_slice(),
                )?,
            ))
        })
    }
}

#[macro_export]
macro_rules! impl_transpose {
    ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name<T> {
            arg: ManagedPort<T>,
            out: ManagedPort<T>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + FunctionRuntimeType
                + FunctionPortBacking
                + ManagedTransposeElement
                + PartialEq
                + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking + CompileConst + ConstElem,
            $arg_type: FunctionPortBacking,
            $out_type: FunctionStateBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_TRANSPOSE_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg) = invocation.expect_unary()?;
                let arg = arg.try_managed_element::<T>()?;
                let out = out.try_managed_element::<T>()?;
                Ok(Box::new($struct_name { arg, out }))
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug
                + Clone
                + Sync
                + Send
                + 'static
                + FunctionPortBacking
                + ManagedTransposeElement
                + PartialEq
                + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CanonicalMatrixElementBacking,
            $out_type: FunctionStateBacking,
        {
            fn planned_output_footprints(
                &self,
            ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_output_footprint(&self.arg, &self.out)?
                    .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::transpose(frame, &self.arg, &self.out)?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(self.out.cell()))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
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
            T: CanonicalMatrixElementBacking
                + ConstElem
                + CompileConst
                + FunctionRuntimeType
                + FunctionPortBacking
                + ManagedTransposeElement,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION
                );
                let out = compile_value_cell_register(self.out.cell(), ctx)?;
                let arg = compile_value_cell_register(self.arg.cell(), ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_unop(function, out, arg);
                Ok(out)
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
        crate::test_managed_factory::<TransposeM2<f64>>(
            FunctionInvocation::unary(
                ValueCell::from_exact_matrix_ref(fixed_out.clone(), 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(matrix, 2, 2).unwrap(),
            ),
            "test/matrix-transpose-fixed",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_eq!(*fixed_out.borrow(), Matrix2::new(1.0, 3.0, 2.0, 4.0));

        let rectangular = Ref::new(Matrix2x3::new(1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0));
        let rectangular_out = Ref::new(Matrix3x2::zeros());
        crate::test_managed_factory::<TransposeM2x3<f64>>(
            FunctionInvocation::unary(
                ValueCell::from_exact_matrix_ref(rectangular_out.clone(), 3, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(rectangular, 2, 3).unwrap(),
            ),
            "test/matrix-transpose-rectangular",
        )
        .instance()
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
        let function = crate::test_managed_factory::<TransposeMD<f64>>(
            FunctionInvocation::unary(
                ValueCell::from_exact_matrix_ref(dynamic_out.clone(), 3, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(dynamic, 2, 3).unwrap(),
            ),
            "test/matrix-transpose-dynamic",
        );
        function.instance().solve_result().unwrap();
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
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
        crate::test_managed_factory::<TransposeM2<bool>>(
            FunctionInvocation::unary(
                ValueCell::from_exact_matrix_ref(bool_out.clone(), 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(bool_arg.clone(), 2, 2).unwrap(),
            ),
            "test/matrix-transpose-bool",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert_eq!(*bool_out.borrow(), *bool_arg.borrow());
    }

    #[test]
    fn transpose_rejects_wrong_storage_and_layout() {
        let output =
            ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::<f64>::zeros()), 2, 2).unwrap();
        let wrong =
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::<f64>::zeros(2, 2)), 2, 2).unwrap();
        assert!(
            TransposeM2::<f64>::new_invocation(FunctionInvocation::unary(output.clone(), wrong,))
                .is_err()
        );
        let arg =
            ValueCell::from_exact_matrix_ref(Ref::new(Matrix2::<f64>::identity()), 2, 2).unwrap();
        assert!(
            TransposeM2::<f64>::new_invocation(FunctionInvocation::binary(
                output,
                arg.clone(),
                arg,
            ))
            .is_err()
        );
    }

    #[test]
    fn fixed_string_transpose_uses_prospective_canonical_admission() {
        let input = ValueCell::from_exact(Matrix2::new(
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "d".to_owned(),
        ))
        .unwrap();
        let output = ValueCell::from_exact(Matrix2::from_element(String::new())).unwrap();
        let expected_footprint = input.current_memory_footprint().unwrap();
        let function = crate::test_managed_factory::<TransposeM2<String>>(
            FunctionInvocation::unary(output.clone(), input),
            "matrix/transpose",
        );

        function.instance().solve_result().unwrap();

        crate::assert_test_value(
            &output,
            ValueCell::from_exact(Matrix2::new(
                "a".to_owned(),
                "c".to_owned(),
                "b".to_owned(),
                "d".to_owned(),
            ))
            .unwrap(),
        );
        assert_eq!(
            output.current_memory_footprint().unwrap().payload_bytes,
            expected_footprint.payload_bytes,
        );
    }

    #[test]
    fn canonical_numeric_matrix_import_uses_dense_managed_backing() {
        let input = ValueCell::from_exact(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0)).unwrap();
        let output = ValueCell::from_exact(Matrix2::<f64>::zeros()).unwrap();
        crate::test_managed_factory::<TransposeM2<f64>>(
            FunctionInvocation::unary(output.clone(), input),
            "matrix/transpose",
        )
        .instance()
        .solve_result()
        .unwrap();

        crate::assert_test_value(
            &output,
            ValueCell::from_exact(Matrix2::new(1.0, 3.0, 2.0, 4.0)).unwrap(),
        );
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
        let FunctionValueRepresentation::Matrix { element, storage } = input
            .representation()
            .expect("matrix descriptor has representation")
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
                    FunctionMatrixRepresentation::Matrix2x3 => {
                        FunctionMatrixRepresentation::Matrix3x2
                    }
                    FunctionMatrixRepresentation::Matrix3x2 => {
                        FunctionMatrixRepresentation::Matrix2x3
                    }
                    FunctionMatrixRepresentation::RowVector2 => {
                        FunctionMatrixRepresentation::Vector2
                    }
                    FunctionMatrixRepresentation::RowVector3 => {
                        FunctionMatrixRepresentation::Vector3
                    }
                    FunctionMatrixRepresentation::RowVector4 => {
                        FunctionMatrixRepresentation::Vector4
                    }
                    FunctionMatrixRepresentation::RowVectorD => {
                        FunctionMatrixRepresentation::VectorD
                    }
                    FunctionMatrixRepresentation::Vector2 => {
                        FunctionMatrixRepresentation::RowVector2
                    }
                    FunctionMatrixRepresentation::Vector3 => {
                        FunctionMatrixRepresentation::RowVector3
                    }
                    FunctionMatrixRepresentation::Vector4 => {
                        FunctionMatrixRepresentation::RowVector4
                    }
                    FunctionMatrixRepresentation::VectorD => {
                        FunctionMatrixRepresentation::RowVectorD
                    }
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
        let _ = (element, storage);
        context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![descriptor.cols as u64, descriptor.rows as u64].into_boxed_slice()]
                .into_boxed_slice(),
            &[input],
        )
    }
}
