use crate::*;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Not;

pub(crate) fn not_vector_runtime_name<MatA: FunctionRuntimeType>() -> String {
    format!("NotV<bool{}>", MatA::REPRESENTATION)
}

#[derive(Debug)]
pub(crate) struct NotS<T> {
    pub arg: ManagedPort<T>,
    pub out: ManagedPort<T>,
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
    T: ManagedElement + FunctionPortBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, T::REPRESENTATION);

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg = arg.try_managed::<T>()?;
        let out = out.try_managed::<T>()?;
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
        + FunctionStateBacking
        + ManagedElement,
{
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_unary_port_views(&self.arg, &self.out, |arg, out| {
            if arg.rows() != out.rows() || arg.columns() != out.columns() {
                return Err(MechError::new(
                    GenericError {
                        msg: "logic/not managed input and output geometry disagree".into(),
                    },
                    None,
                ));
            }
            out.try_fill_column_major(|index| {
                Ok(!arg
                    .get_column_major(index)
                    .expect("validated logic/not input lane"))
            })
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(T::REPRESENTATION))
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for NotS<T>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NotS<{}>", <T as FunctionRuntimeType>::REPRESENTATION);
        let out = compile_value_cell_register(self.out.cell(), ctx)?;
        let arg = compile_value_cell_register(self.arg.cell(), ctx)?;
        let function = ctx.function_id(&name)?;
        ctx.emit_unop(function, out, arg);
        Ok(out)
    }
}

#[derive(Debug)]
pub struct NotV<T, MatA> {
    pub arg: ManagedPort<T>,
    pub out: ManagedPort<T>,
    pub _marker: PhantomData<MatA>,
}

impl<T, MatA> MechFunctionFactory for NotV<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + FunctionRuntimeType + Not<Output = T>,
    T: ManagedElement + FunctionPortBacking,
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
        let _ = arg.try_managed::<MatA>()?;
        let _ = out.try_managed::<MatA>()?;
        Ok(Box::new(Self {
            arg: arg.try_managed_element::<T>()?,
            out: out.try_managed_element::<T>()?,
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
    T: ManagedElement,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + FunctionRuntimeType + FunctionStateBacking,
{
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_unary_port_views(&self.arg, &self.out, |arg, out| {
            if arg.rows() != out.rows() || arg.columns() != out.columns() {
                return Err(MechError::new(
                    GenericError {
                        msg: "logic/not managed input and output geometry disagree".into(),
                    },
                    None,
                ));
            }
            out.try_fill_column_major(|index| {
                Ok(!arg
                    .get_column_major(index)
                    .expect("validated logic/not input lane"))
            })
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.out.cell()))
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(MatA::REPRESENTATION))
    }

    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.out.cell())]))
    }
}

#[cfg(feature = "semantic-compiler")]
impl<T, MatA> MechFunctionCompiler for NotV<T, MatA>
where
    T: CompileConst + ConstElem + FunctionRuntimeType,
    MatA: CompileConst + ConstElem + FunctionRuntimeType,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = not_vector_runtime_name::<MatA>();
        let out = compile_value_cell_register(self.out.cell(), ctx)?;
        let arg = compile_value_cell_register(self.arg.cell(), ctx)?;
        let function = ctx.function_id(&name)?;
        ctx.emit_unop(function, out, arg);
        Ok(out)
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
