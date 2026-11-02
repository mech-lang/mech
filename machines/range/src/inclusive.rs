use mech_core::*;
use std::sync::LazyLock;

static PURE_INCLUSIVE_RANGE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| {
        mech_core::maintained_operation_contract("range/inclusive", 2, true)
            .expect("maintained range operation contract")
    });

// Inclusive ------------------------------------------------------------------

crate::impl_managed_binary_range!(RangeInclusiveScalar, &PURE_INCLUSIVE_RANGE_CONTRACT, true);

#[cfg(all(test, feature = "u128", feature = "matrixd"))]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn inclusive_range_does_not_increment_past_the_final_max_value() {
        let output = ValueCell::from_exact(DMatrix::from_element(1, 2, 0_u128)).unwrap();
        let function = crate::test_managed_factory::<RangeInclusiveScalar<u128, DMatrix<u128>>>(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(u128::MAX - 1).unwrap(),
                ValueCell::from_exact(u128::MAX).unwrap(),
            ),
            "test/range-inclusive",
        );

        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[u128::MAX - 1, u128::MAX]))
                .unwrap(),
        );
    }

    #[test]
    fn inclusive_range_revalidates_extent_and_rolls_back_without_replacing_identity() {
        let to = ValueCell::from_exact(2_u128).unwrap();
        let output = ValueCell::from_exact(DMatrix::from_element(1, 2, 0_u128)).unwrap();
        let output_alias = output.clone();
        let schema = output.schema_key();
        let function = crate::test_managed_factory::<RangeInclusiveScalar<u128, DMatrix<u128>>>(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(1_u128).unwrap(),
                to.clone(),
            ),
            "test/range-inclusive",
        );

        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[1_u128, 2])).unwrap(),
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            to.replace(&ValueCell::from_exact(3_u128).unwrap().snapshot().unwrap())?;
            function.instance().solve_result()?;
            crate::assert_test_value(
                &output,
                ValueCell::from_exact(DMatrix::from_row_slice(1, 3, &[1_u128, 2, 3])).unwrap(),
            );
            assert_eq!(output.shape().parameter_values(), &[1, 3]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(output.same_cell(&output_alias));
        assert_eq!(output.schema_key(), schema);
        assert_eq!(output.shape().parameter_values(), &[1, 2]);
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[1_u128, 2])).unwrap(),
        );
    }
}
#[cfg(feature = "source")]
pub struct RangeInclusive;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for RangeInclusive {
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
        let from = invocation.input(0).expect("validated range start");
        let to = invocation.input(1).expect("validated range end");
        macro_rules! try_scalar {
            ($scalar:ty, $feature:literal) => {
                #[cfg(feature = $feature)]
                if from.representation() == Some(<$scalar as FunctionRuntimeType>::REPRESENTATION)
                    && to.representation() == Some(<$scalar as FunctionRuntimeType>::REPRESENTATION)
                {
                    $crate::bind_dynamic_binary_range!(
                        RangeInclusiveScalar,
                        $scalar,
                        from,
                        to,
                        true,
                        context
                    );
                }
            };
        }
        try_scalar!(f32, "f32");
        try_scalar!(f64, "f64");
        try_scalar!(i8, "i8");
        try_scalar!(i16, "i16");
        try_scalar!(i32, "i32");
        try_scalar!(i64, "i64");
        try_scalar!(i128, "i128");
        try_scalar!(u8, "u8");
        try_scalar!(u16, "u16");
        try_scalar!(u32, "u32");
        try_scalar!(u64, "u64");
        try_scalar!(u128, "u128");
        Err(MechError::new(
            FunctionArgumentTypeMismatch {
                role: FunctionArgumentRole::Input(0),
                expected: "matching numeric scalar range endpoints".into(),
                found: format!("{:?} and {:?}", from.representation(), to.representation()),
            },
            None,
        )
        .with_compiler_loc())
    }
}
