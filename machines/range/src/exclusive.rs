use crate::*;
use mech_core::*;
use nalgebra::{
    Dim, Scalar,
    base::{Matrix as naMatrix, StorageMut},
};
use std::marker::PhantomData;
use std::sync::LazyLock;

static PURE_EXCLUSIVE_RANGE_CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    OperationContractDeclaration {
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
            construction: OutputConstruction::Build {
                postcondition: ShapeContractReference {
                    module_path: vec!["range".to_owned()].into_boxed_slice(),
                    contract_name: "exclusive-output".to_owned(),
                },
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
});

// Exclusive ------------------------------------------------------------------

#[derive(Debug)]
pub struct RangeExclusiveScalar<T, MatA> {
    pub from: Ref<T>,
    pub to: Ref<T>,
    pub out: Ref<MatA>,
    output_value: FunctionValueOutput,
    phantom: PhantomData<T>,
}
impl<T, R1, C1, S1> MechFunctionFactory for RangeExclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
    T: Copy
        + CanonicalMatrixElementBacking
        + Debug
        + Clone
        + Sync
        + Send
        + FunctionRuntimeType
        + PartialOrd
        + 'static
        + One
        + Add<Output = T>
        + mech_core::CanonicalRangeScalar,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
    R1: Dim + 'static,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug + 'static,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
        T::REPRESENTATION,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, from, to) = invocation.expect_binary()?;
        let output_value = out.value();
        let from: Ref<T> = from.try_ref()?;
        let to: Ref<T> = to.try_ref()?;
        let out: Ref<naMatrix<T, R1, C1, S1>> = out.try_ref()?;
        Ok(Box::new(Self {
            from,
            to,
            out,
            output_value,
            phantom: PhantomData::default(),
        }))
    }

}
impl<T, R1, C1, S1> MechFunctionImpl for RangeExclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: Copy
        + CanonicalMatrixElementBacking
        + Scalar
        + Clone
        + Debug
        + Sync
        + Send
        + 'static
        + PartialOrd
        + One
        + Add<Output = T>
        + mech_core::CanonicalRangeScalar
        + 'static,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
{
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(self.output_value.state_port())
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![self.output_value.state_port()]))
    }
    fn solve_result(&self) -> MResult<()> {
        let elements = crate::canonical_range_drafts(
            *self.from.borrow(),
            None,
            *self.to.borrow(),
            false,
        )?;
        let output_len = elements.len();
        self.output_value.replace_matrix_drafts(
            vec![1, output_len as u64].into_boxed_slice(),
            elements,
        )
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_EXCLUSIVE_RANGE_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(all(test, feature = "u128", feature = "matrixd"))]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn exclusive_range_revalidates_reactive_cardinality() {
        let to = ValueCell::from_exact(3_u128).unwrap();
        let out = Ref::new(DMatrix::from_element(1, 2, 0_u128));
        let function = RangeExclusiveScalar::<u128, DMatrix<u128>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(out.clone(), 1, 2).unwrap(),
                ValueCell::from_exact(1_u128).unwrap(),
                to.clone(),
            ),
        )
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[1, 2]);

        to.replace(&ValueCell::from_exact(4_u128).unwrap().snapshot().unwrap())
            .unwrap();
        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[1, 2, 3]);
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T, R1, C1, S1> MechFunctionCompiler for RangeExclusiveScalar<T, naMatrix<T, R1, C1, S1>>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
    naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "RangeExclusiveScalar<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>()
        );
        compile_binop!(name, self.out, self.from, self.to, ctx);
    }
}

#[cfg(feature = "source")]
pub struct RangeExclusive;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for RangeExclusive {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
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
                    && to.representation()
                        == Some(<$scalar as FunctionRuntimeType>::REPRESENTATION)
                {
                    bind_dynamic_binary_range!(RangeExclusiveScalar, $scalar, from, to, false);
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
