use mech_core::*;
use std::sync::LazyLock;

static PURE_EXCLUSIVE_INCREMENT_RANGE_CONTRACT: LazyLock<OperationContractDeclaration> =
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
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["range".to_owned()].into_boxed_slice(),
                    contract_name: "exclusive-increment-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

// Exclusive ------------------------------------------------------------------

crate::impl_managed_ternary_range!(
    RangeIncrementExclusiveScalar,
    &PURE_EXCLUSIVE_INCREMENT_RANGE_CONTRACT,
    false
);

#[cfg(all(test, feature = "u128", feature = "matrixd"))]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn exclusive_increment_range_revalidates_reactive_cardinality() {
        let to = ValueCell::from_exact(5_u128).unwrap();
        let output = ValueCell::from_exact(DMatrix::from_element(1, 2, 0_u128)).unwrap();
        let function =
            crate::test_managed_factory::<RangeIncrementExclusiveScalar<u128, DMatrix<u128>>>(
                FunctionInvocation::ternary(
                    output.clone(),
                    ValueCell::from_exact(1_u128).unwrap(),
                    ValueCell::from_exact(2_u128).unwrap(),
                    to.clone(),
                ),
                "test/range-exclusive-increment",
            );

        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[1_u128, 3])).unwrap(),
        );
        to.replace(&ValueCell::from_exact(7_u128).unwrap().snapshot().unwrap())
            .unwrap();
        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 3, &[1_u128, 3, 5])).unwrap(),
        );
    }
}

#[cfg(feature = "source")]
pub struct RangeIncrementExclusive;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for RangeIncrementExclusive {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 3 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 3,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let from = invocation.input(0).expect("validated range start");
        let step = invocation.input(1).expect("validated range step");
        let to = invocation.input(2).expect("validated range end");
        macro_rules! try_scalar {
            ($scalar:ty, $feature:literal) => {
                #[cfg(feature = $feature)]
                if from.representation() == Some(<$scalar as FunctionRuntimeType>::REPRESENTATION)
                    && step.representation()
                        == Some(<$scalar as FunctionRuntimeType>::REPRESENTATION)
                    && to.representation() == Some(<$scalar as FunctionRuntimeType>::REPRESENTATION)
                {
                    $crate::bind_dynamic_ternary_range!(
                        RangeIncrementExclusiveScalar,
                        $scalar,
                        from,
                        step,
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
                expected: "matching numeric scalar range inputs".into(),
                found: format!(
                    "{:?}, {:?}, and {:?}",
                    from.representation(),
                    step.representation(),
                    to.representation()
                ),
            },
            None,
        )
        .with_compiler_loc())
    }
}
