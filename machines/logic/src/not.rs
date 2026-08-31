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
    T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static + FunctionRuntimeType + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    T: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, T::REPRESENTATION);

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

#[cfg(feature = "source")]
fn specialize_not_factory<F>(input: &SpecializationInput) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let output = input.cell()?.detached_clone()?;
    let invocation = FunctionInvocation::unary(output, input.cell()?.clone());
    let implementation = F::new_invocation(invocation.clone())?;
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        invocation,
    )))
}

#[cfg(feature = "source")]
pub struct LogicNot;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for LogicNot {
    fn specialize_invocation(
        &self,
        specialization: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
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
        match input.representation() {
            #[cfg(feature = "bool")]
            Some(FunctionValueRepresentation::Bool) => {
                specialize_not_factory::<NotS<bool>>(input)
            }
            found => Err(MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(0),
                    expected: "Bool scalar".into(),
                    found: format!("{found:?}"),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
