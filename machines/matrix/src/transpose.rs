use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
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
            T: Debug + Clone + Sync + Send + 'static + AsValueKind + PartialEq + PartialOrd,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + ConstElem,
            Ref<$out_type>: ToValue,
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

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Debug + Clone + Sync + Send + 'static + PartialEq + PartialOrd,
            Ref<$out_type>: ToValue,
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
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_TRANSPOSE_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
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
    feature = "vector3",
    feature = "row_vector3",
    feature = "matrixd"
))]
mod invocation_port_tests {
    use super::*;

    fn unary_args<I, O>(out: &Ref<O>, arg: &Ref<I>) -> FunctionArgs
    where
        Ref<I>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Unary(out.to_value(), arg.to_value())
    }

    #[test]
    fn numeric_fixed_vector_and_dynamic_transposes_use_exact_ports() {
        let matrix = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let legacy_out = Ref::new(Matrix2::zeros());
        let invocation_out = Ref::new(Matrix2::zeros());
        let legacy = TransposeM2::<f64>::new(unary_args(&legacy_out, &matrix)).unwrap();
        let invocation = TransposeM2::<f64>::new_invocation(
            unary_args(&invocation_out, &matrix).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), Matrix2::new(1.0, 3.0, 2.0, 4.0));
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());

        let rectangular = Ref::new(Matrix2x3::new(1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0));
        let rectangular_out = Ref::new(Matrix3x2::zeros());
        TransposeM2x3::<f64>::new_invocation(
            unary_args(&rectangular_out, &rectangular).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *rectangular_out.borrow(),
            Matrix3x2::new(1.0, 4.0, 2.0, 5.0, 3.0, 6.0)
        );

        let vector = Ref::new(Vector3::new(1.0_f64, 2.0, 3.0));
        let vector_out = Ref::new(RowVector3::zeros());
        TransposeV3::<f64>::new_invocation(unary_args(&vector_out, &vector).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(*vector_out.borrow(), RowVector3::new(1.0, 2.0, 3.0));

        let dynamic = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_out = Ref::new(DMatrix::zeros(3, 2));
        let dynamic_function =
            TransposeMD::<f64>::new_invocation(unary_args(&dynamic_out, &dynamic).into()).unwrap();
        dynamic_function.solve_result().unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(3, 2, &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0])
        );
        assert_eq!(
            dynamic_function.reactive_output_cell_ids(),
            dynamic_function.out().reactive_root_cell_ids(),
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*dynamic_function)?;
            *dynamic_out.borrow_mut() = DMatrix::from_element(1, 4, -1.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(dynamic_out.borrow().shape(), (3, 2));
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(3, 2, &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0])
        );
    }

    #[test]
    fn bool_and_string_transposes_preserve_element_backings() {
        let bool_arg = Ref::new(Matrix2::new(true, false, false, true));
        let bool_out = Ref::new(Matrix2::from_element(false));
        TransposeM2::<bool>::new_invocation(unary_args(&bool_out, &bool_arg).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(*bool_out.borrow(), *bool_arg.borrow());

        let string_arg = Ref::new(Matrix2::new(
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ));
        let string_out = Ref::new(Matrix2::from_element(String::new()));
        TransposeM2::<String>::new_invocation(unary_args(&string_out, &string_arg).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(
            *string_out.borrow(),
            Matrix2::new(
                "a".to_string(),
                "c".to_string(),
                "b".to_string(),
                "d".to_string(),
            )
        );
    }

    #[test]
    fn transpose_invocation_rejects_wrong_storage_and_layout() {
        let out = Ref::new(Matrix2::<f64>::zeros());
        let wrong_storage = Ref::new(DMatrix::<f64>::zeros(2, 2));
        let type_error = TransposeM2::<f64>::new_invocation(
            unary_args(&out, &wrong_storage).into(),
        )
        .err()
        .expect("wrong exact matrix storage must be rejected");
        assert_eq!(type_error.kind_name(), "FunctionArgumentTypeMismatch");

        let arg = Ref::new(Matrix2::<f64>::identity());
        let arity_error = TransposeM2::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), arg.to_value(), arg.to_value()).into(),
        )
        .err()
        .expect("wrong transpose invocation layout must be rejected");
        assert_eq!(arity_error.kind_name(), "IncorrectNumberOfArguments");
    }
}

#[cfg(feature = "source")]
macro_rules! impl_transpose_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = "row_vector4", feature = "vector4", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(TransposeR4{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default())) })),
            #[cfg(all(feature = "row_vector3", feature = "vector3", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(TransposeR3{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default())) })),
            #[cfg(all(feature = "row_vector2", feature = "vector2", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(TransposeR2{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default())) })),
            #[cfg(all(feature = "vector4", feature = "row_vector4", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(TransposeV4{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = "vector3", feature = "row_vector3", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(TransposeV3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = "vector2", feature = "row_vector2", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(TransposeV2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = "matrix4", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(TransposeM4{arg: arg.clone(), out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix3", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(TransposeM3{arg: arg.clone(), out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix2", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(TransposeM2{arg: arg.clone(), out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix1", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(TransposeM1{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(TransposeM2x3{arg: arg.clone(), out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(TransposeM3x2{arg: arg.clone(), out: Ref::new(Matrix2x3::from_element($target_type::default()))})),
            #[cfg(all(feature = "vectord", feature = "row_vectord", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(TransposeVD{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$target_type::default())) })),
            #[cfg(all(feature = "vectord", feature = "row_vectord", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(TransposeRD{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$target_type::default())) })),
            #[cfg(all(feature = "matrixd", feature = $value_string))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new(TransposeMD{arg, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
          )+
        )+
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 { arg: x.kind(), fxn_name: "MatrixTranspose".to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
}

#[cfg(feature = "source")]
fn impl_transpose_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_transpose_match_arms!(
      lhs_value,
      Bool,   bool,   "bool";
      I8,     i8,     "i8";
      I16,    i16,    "i16";
      I32,    i32,    "i32";
      I64,    i64,    "i64";
      I128,   i128,   "i128";
      U8,     u8,     "u8";
      U16,    u16,    "u16";
      U32,    u32,    "u32";
      U64,    u64,    "u64";
      U128,   u128,   "u128";
      F32,    f32,    "f32";
      F64,    f64,    "f64";
      String, String, "string";
      C64, C64, "complex";
      R64, R64, "rational";
    )
}

#[cfg(feature = "source")]
impl_mech_urnop_fxn!(MatrixTranspose, impl_transpose_fxn, "matrix/transpose");
