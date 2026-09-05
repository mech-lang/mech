use crate::*;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Not;

#[derive(Debug)]
pub(crate) struct NotS<T> {
    pub arg: Ref<T>,
    pub out: Ref<T>,
    pub _marker: PhantomData<T>,
}

impl<T> MechFunctionFactory for NotS<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + FunctionRuntimeType
        + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    T: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, T::REPRESENTATION);

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<T> = arg.try_ref()?;
        let out: Ref<T> = out.try_ref()?;
        Ok(Box::new(Self {
            arg,
            out,
            _marker: PhantomData::default(),
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(T::REPRESENTATION))
    }
}

impl<T> MechFunctionImpl for NotS<T>
where
    T: Copy
        + Debug
        + Clone
        + Sync
        + Send
        + PartialEq
        + 'static
        + Not<Output = T>
        + FunctionStateBacking,
{
    fn solve_result(&self) -> MResult<()> {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            *out_ptr = !*arg_ptr;
        };
        Ok(())
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(T::REPRESENTATION))
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for NotS<T>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NotS<{}>", <T as FunctionRuntimeType>::REPRESENTATION);
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

#[derive(Debug)]
pub struct NotV<T, MatA> {
    pub arg: Ref<MatA>,
    pub out: Ref<MatA>,
    pub _marker: PhantomData<T>,
}

impl<T, MatA> MechFunctionFactory for NotV<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + FunctionRuntimeType + FunctionStateBacking + 'static,
    #[cfg(feature = "semantic-compiler")]
    MatA: CompileConst + ConstElem,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(MatA::REPRESENTATION, MatA::REPRESENTATION);

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        Ok(Box::new(Self {
            arg: arg.try_ref()?,
            out: out.try_ref()?,
            _marker: PhantomData,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(MatA::REPRESENTATION))
    }
}

impl<T, MatA> MechFunctionImpl for NotV<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + Not<Output = T>,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + FunctionRuntimeType + FunctionStateBacking,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let output = &mut *self.out.as_mut_ptr();
            let input = &*self.arg.as_ptr();
            for (target, source) in output.into_iter().zip(input.into_iter()) {
                *target = !source.clone();
            }
        }
        Ok(())
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(MatA::REPRESENTATION))
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T, MatA> MechFunctionCompiler for NotV<T, MatA>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
    MatA: CompileConst + ConstElem + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "NotV<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            <MatA as FunctionRuntimeType>::REPRESENTATION,
        );
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

#[cfg(feature = "source")]
pub struct LogicNot;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for LogicNot {
    fn specialize_invocation(
        &self,
        specialization: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if specialization.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: specialization.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = specialization.input(0).expect("validated unary input");
        let extents = input
            .cell()?
            .resolved_descriptor()?
            .current_extents()
            .map_err(MechError::from)?;
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            ExecutionTarget::DirectRuntime,
            vec![extents].into_boxed_slice(),
            &[input],
        )
    }
}
