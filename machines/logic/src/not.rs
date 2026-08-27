use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Not;

// Not ------------------------------------------------------------------------

// NotS -----------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct NotS<T> {
    pub arg: Ref<T>,
    pub out: Ref<T>,
    pub _marker: PhantomData<T>,
}
impl<T> MechFunctionFactory for NotS<T>
where
    T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static + AsValueKind + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    Ref<T>: ToValue,
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

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
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
    Ref<T>: ToValue,
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
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(T::REPRESENTATION))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for NotS<T>
where
    T: CompileConst + ConstElem + AsValueKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NotS<{}>", T::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx);
    }
}
// NotV -----------------------------------------------------------------------

#[derive(Debug)]
pub struct NotV<T, MatA> {
    pub arg: Ref<MatA>,
    pub out: Ref<MatA>,
    pub _marker: PhantomData<T>,
}
impl<T, MatA> MechFunctionFactory for NotV<T, MatA>
where
    T: Debug + Clone + Sync + Send + 'static + AsValueKind + Not<Output = T>,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + ConstElem,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + AsValueKind + 'static,
    #[cfg(feature = "semantic-compiler")]
    MatA: CompileConst + ConstElem,
    Ref<MatA>: ToValue,
    MatA: FunctionStateBacking,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(MatA::REPRESENTATION, MatA::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg) = invocation.expect_unary()?;
        let arg: Ref<MatA> = arg.try_ref()?;
        let out: Ref<MatA> = out.try_ref()?;
        Ok(Box::new(Self {
            arg,
            out,
            _marker: PhantomData::default(),
        }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
impl<T, MatA> MechFunctionImpl for NotV<T, MatA>
where
    Ref<MatA>: ToValue,
    T: Debug + Clone + Sync + Send + 'static + AsValueKind + Not<Output = T>,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + FunctionStateBacking,
{
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let sink_ptr = self.out.as_mut_ptr();
            let source_ptr = self.arg.as_ptr();
            let sink_ref: &mut MatA = &mut *sink_ptr;
            let source_ref: &MatA = &*source_ptr;
            for (dst, src) in sink_ref.into_iter().zip(source_ref.into_iter()) {
                *dst = !src.clone();
            }
        };
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::logic_unary_full_write_contract(MatA::REPRESENTATION))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, MatA> MechFunctionCompiler for NotV<T, MatA>
where
    T: CompileConst + ConstElem + AsValueKind,
    MatA: CompileConst + ConstElem + AsValueKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("NotV<{}{}>", T::as_value_kind(), MatA::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

#[cfg(feature = "source")]
fn impl_not_fxn(arg_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_urnop_match_arms!(
      Not,
      arg_value,
      Bool, bool, "bool";
    )
}

#[cfg(feature = "source")]
impl_mech_urnop_fxn!(LogicNot, impl_not_fxn, "logic/not");
