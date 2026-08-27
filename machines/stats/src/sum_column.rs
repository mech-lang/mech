#[cfg(any(
    feature = "matrix1",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord",
    all(feature = "matrixd", feature = "row_vectord")
))]
use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
#[cfg(any(
    feature = "matrix1",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord",
    all(feature = "matrixd", feature = "row_vectord")
))]
use num_traits::*;

// Stats Sum Column -----------------------------------------------------------

#[cfg(any(
    feature = "matrix1",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord",
    all(feature = "matrixd", feature = "row_vectord")
))]
macro_rules! sum_column_op {
    ($arg:expr, $out:expr) => {
        {
            for row in 0..($arg).nrows() {
                let mut sum = T::zero();
                for column in 0..($arg).ncols() {
                    sum = checked_sum_add(sum, ($arg)[(row, column)])?;
                }
                ($out)[row] = sum;
            }
            Ok::<(), MechError>(())
        }
    };
}

#[cfg(all(feature = "matrix1", feature = "matrix1"))]
impls_stas!(StatsSumColumnM1, Matrix1<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "matrix2", feature = "vector2"))]
impls_stas!(StatsSumColumnM2, Matrix2<T>, Vector2<T>, sum_column_op);
#[cfg(all(feature = "matrix3", feature = "vector3"))]
impls_stas!(StatsSumColumnM3, Matrix3<T>, Vector3<T>, sum_column_op);
#[cfg(all(feature = "matrix4", feature = "vector4"))]
impls_stas!(StatsSumColumnM4, Matrix4<T>, Vector4<T>, sum_column_op);
#[cfg(all(feature = "matrix2x3", feature = "vector2"))]
impls_stas!(StatsSumColumnM2x3, Matrix2x3<T>, Vector2<T>, sum_column_op);
#[cfg(all(feature = "matrix3x2", feature = "vector3"))]
impls_stas!(StatsSumColumnM3x2, Matrix3x2<T>, Vector3<T>, sum_column_op);
#[cfg(all(feature = "matrixd", feature = "vectord"))]
impls_stas!(StatsSumColumnMD, DMatrix<T>, DVector<T>, sum_column_op);
#[cfg(all(feature = "vector2", feature = "vector2"))]
impls_stas!(StatsSumColumnV2, Vector2<T>, Vector2<T>, sum_column_op);
#[cfg(all(feature = "vector3", feature = "vector3"))]
impls_stas!(StatsSumColumnV3, Vector3<T>, Vector3<T>, sum_column_op);
#[cfg(all(feature = "vector4", feature = "vector4"))]
impls_stas!(StatsSumColumnV4, Vector4<T>, Vector4<T>, sum_column_op);
#[cfg(all(feature = "vectord", feature = "vectord"))]
impls_stas!(StatsSumColumnVD, DVector<T>, DVector<T>, sum_column_op);
#[cfg(all(feature = "row_vector2", feature = "matrix1"))]
impls_stas!(StatsSumColumnR2, RowVector2<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "row_vector3", feature = "matrix1"))]
impls_stas!(StatsSumColumnR3, RowVector3<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impls_stas!(StatsSumColumnR4, RowVector4<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "row_vectord", feature = "matrix1"))]
impls_stas!(StatsSumColumnRD, RowDVector<T>, Matrix1<T>, sum_column_op);

#[cfg(all(feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
#[derive(Debug)]
pub(crate) struct StatsSumColumnRD2<T> {
    arg: Ref<RowDVector<T>>,
    out: Ref<DMatrix<T>>,
}

#[cfg(all(feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
impl<T> MechFunctionFactory for StatsSumColumnRD2<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + Add<Output = T>
        + AddAssign
        + AsValueKind
        + Zero
        + One
        + PartialEq
        + PartialOrd,
    T: StatsCheckedAdd,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    Ref<DMatrix<T>>: ToValue,
    RowDVector<T>: FunctionPortBacking,
    DMatrix<T>: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        <DMatrix<T> as FunctionRuntimeType>::REPRESENTATION,
        <RowDVector<T> as FunctionRuntimeType>::REPRESENTATION,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<RowDVector<T>> = arg.try_ref()?;
        let out: Ref<DMatrix<T>> = out.try_ref()?;
        Ok(Box::new(StatsSumColumnRD2 { arg, out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
#[cfg(all(feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
impl<T> MechFunctionImpl for StatsSumColumnRD2<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + 'static
        + Add<Output = T>
        + AddAssign
        + Zero
        + One
        + PartialEq
        + PartialOrd,
    T: StatsCheckedAdd,
    Ref<DMatrix<T>>: ToValue,
    DMatrix<T>: FunctionStateBacking,
{
    fn solve_result(&self) -> MResult<()> {
        let mut next = self.out.borrow().clone();
        {
            let arg = self.arg.borrow();
            sum_column_op!(&*arg, &mut next)?;
        }
        *self.out.borrow_mut() = next;
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
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(all(feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for StatsSumColumnRD2<T>
where
    T: CompileConst + ConstElem + AsValueKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!(StatsSumColumnRD2), T::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx);
    }
}
#[cfg(feature = "source")]
macro_rules! impl_stats_sum_column_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(StatsSumColumnR4{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(StatsSumColumnR3{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(StatsSumColumnR2{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(StatsSumColumnV4{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(StatsSumColumnV3{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(StatsSumColumnV2{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(StatsSumColumnM4{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(StatsSumColumnM3{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(StatsSumColumnM2{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(StatsSumColumnM1{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(StatsSumColumnM2x3{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(StatsSumColumnM3x2{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(StatsSumColumnVD{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumColumnRD{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumColumnRD2{arg: arg.clone(), out: Ref::new(DMatrix::from_element(1,1,$target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) => Ok(Box::new(StatsSumColumnMD{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().nrows(),$target_type::default())) })),
          )+
        )+
        _ => Err(MechError::new(
          UnhandledFunctionArgumentKind1 {arg: $arg.kind(), fxn_name: stringify!(StatsSumColumn).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

#[cfg(feature = "source")]
fn impl_stats_sum_column_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_stats_sum_column_match_arms!(
      lhs_value,
      I8,   i8,   "i8";
      I16,  i16,  "i16";
      I32,  i32,  "i32";
      I64,  i64,  "i64";
      I128, i128, "i128";
      U8,   u8,   "u8";
      U16,  u16,  "u16";
      U32,  u32,  "u32";
      U64,  u64,  "u64";
      U128, u128, "u128";
      F32,  f32,  "f32";
      F64,  f64,  "f64";
      C64, C64, "complex";
      R64, R64, "rational"
    )
}

#[cfg(feature = "source")]
impl_mech_urnop_fxn!(
    StatsSumColumn,
    impl_stats_sum_column_fxn,
    "stats/sum/column"
);

#[cfg(all(test, any(feature = "u8", feature = "rational")))]
mod checked_sum_tests {
    use super::*;

    #[cfg(feature = "u8")]
    #[test]
    fn integer_column_sum_rejects_reactive_overflow_and_retains_output() {
        let arg = Ref::new(DMatrix::from_row_slice(1, 2, &[1u8, 2]));
        let out = Ref::new(DVector::from_vec(vec![99u8]));
        let function = StatsSumColumnMD::<u8> {
            arg: arg.clone(),
            out: out.clone(),
        };
        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[3]);
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&function)?;
            *arg.borrow_mut() = DMatrix::from_row_slice(1, 2, &[u8::MAX, 1]);
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "StatsArithmeticOverflow");
            assert_eq!(out.borrow().as_slice(), &[3]);
            *out.borrow_mut() = DVector::from_vec(vec![17, 18]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(out.borrow().as_slice(), &[3]);
    }

    #[cfg(feature = "rational")]
    #[test]
    fn bounded_rational_column_sum_is_checked() {
        let arg = Ref::new(DMatrix::from_row_slice(
            1,
            2,
            &[R64::new(i64::MAX, 1), R64::new(1, 1)],
        ));
        let out = Ref::new(DVector::from_vec(vec![R64::new(7, 1)]));
        let function = StatsSumColumnMD::<R64> {
            arg,
            out: out.clone(),
        };
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "StatsArithmeticOverflow");
        assert_eq!(out.borrow().as_slice(), &[R64::new(7, 1)]);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "matrix2",
    feature = "vector2",
    feature = "matrixd",
    feature = "vectord"
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
    fn fixed_and_dynamic_column_sums_preserve_factory_behavior_and_state() {
        let fixed_arg = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let legacy_out = Ref::new(Vector2::zeros());
        let invocation_out = Ref::new(Vector2::zeros());
        let legacy =
            StatsSumColumnM2::<f64>::new(unary_args(&legacy_out, &fixed_arg)).unwrap();
        let invocation = StatsSumColumnM2::<f64>::new_invocation(
            unary_args(&invocation_out, &fixed_arg).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), Vector2::new(3.0, 7.0));
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());

        let dynamic_arg = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_out = Ref::new(DVector::zeros(2));
        let dynamic = StatsSumColumnMD::<f64>::new_invocation(
            unary_args(&dynamic_out, &dynamic_arg).into(),
        )
        .unwrap();
        dynamic.solve_result().unwrap();
        assert_eq!(*dynamic_out.borrow(), DVector::from_vec(vec![6.0, 15.0]));
        assert_eq!(
            dynamic.reactive_output_cell_ids(),
            dynamic.out().reactive_root_cell_ids(),
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*dynamic)?;
            *dynamic_out.borrow_mut() = DVector::from_vec(vec![-1.0, -2.0, -3.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*dynamic_out.borrow(), DVector::from_vec(vec![6.0, 15.0]));
    }

    #[test]
    fn column_sum_rejects_wrong_exact_storage_and_reports_unary_arity() {
        let out = Ref::new(Vector2::<f64>::zeros());
        let wrong_arg = Ref::new(DMatrix::<f64>::zeros(2, 2));
        let type_error = StatsSumColumnM2::<f64>::new_invocation(
            unary_args(&out, &wrong_arg).into(),
        )
        .err()
        .expect("wrong exact statistics input must be rejected");
        assert_eq!(type_error.kind_name(), "FunctionArgumentTypeMismatch");

        let fixed_arg = Ref::new(Matrix2::<f64>::zeros());
        let arity_error = StatsSumColumnM2::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), fixed_arg.to_value(), fixed_arg.to_value()).into(),
        )
        .err()
        .expect("binary layout must be rejected for unary statistics functions");
        let arity = arity_error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!(arity.expected, 1);
        assert_eq!(arity.found, 2);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "row_vectord",
    feature = "matrixd",
    not(feature = "matrix1")
))]
mod direct_dynamic_invocation_tests {
    use super::*;

    #[test]
    fn row_vector_column_sum_uses_direct_dynamic_factory_and_state_port() {
        let arg = Ref::new(RowDVector::from_vec(vec![1.0_f64, 2.0, 3.0]));
        let out = Ref::new(DMatrix::zeros(1, 1));
        let function = StatsSumColumnRD2::<f64>::new_invocation(
            FunctionArgs::Unary(out.to_value(), arg.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), DMatrix::from_element(1, 1, 6.0));
        assert_eq!(
            function.reactive_output_cell_ids(),
            function.out().reactive_root_cell_ids(),
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *out.borrow_mut() = DMatrix::from_element(2, 2, -1.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), DMatrix::from_element(1, 1, 6.0));
    }
}
