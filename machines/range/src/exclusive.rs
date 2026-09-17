use mech_core::*;
use std::sync::LazyLock;

static PURE_EXCLUSIVE_RANGE_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| {
        mech_core::maintained_operation_contract("range/exclusive", 2, true)
            .expect("maintained range operation contract")
    });

// Exclusive ------------------------------------------------------------------

crate::impl_managed_binary_range!(RangeExclusiveScalar, &PURE_EXCLUSIVE_RANGE_CONTRACT, false);

#[cfg(all(test, feature = "u128", feature = "matrixd"))]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn exclusive_range_revalidates_reactive_cardinality() {
        let to = ValueCell::from_exact(3_u128).unwrap();
        let output = ValueCell::from_exact(DMatrix::from_element(1, 2, 0_u128)).unwrap();
        let function = crate::test_managed_factory::<RangeExclusiveScalar<u128, DMatrix<u128>>>(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(1_u128).unwrap(),
                to.clone(),
            ),
            "test/range-exclusive",
        );

        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[1_u128, 2])).unwrap(),
        );

        to.replace(&ValueCell::from_exact(4_u128).unwrap().snapshot().unwrap())
            .unwrap();
        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 3, &[1_u128, 2, 3])).unwrap(),
        );
    }
}

#[cfg(feature = "source")]
pub struct RangeExclusive;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for RangeExclusive {
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
                        RangeExclusiveScalar,
                        $scalar,
                        from,
                        to,
                        false,
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
