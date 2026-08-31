use crate::*;

#[derive(Debug)]
pub struct StrictNotEqValue {
    lhs: FunctionValueInput,
    rhs: FunctionValueInput,
    pub out: Ref<bool>,
}

impl MechFunctionImpl for StrictNotEqValue {
    fn solve_result(&self) -> MResult<()> {
        *self.out.borrow_mut() = !self.lhs.snapshot_eq(&self.rhs)?;
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::compare_full_write_contract(
            FunctionValueRepresentation::Bool,
        ))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
}

impl MechFunctionFactory for StrictNotEqValue {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        Ok(Box::new(Self {
            lhs: lhs.value(),
            rhs: rhs.value(),
            out: out.try_ref()?,
        }))
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for StrictNotEqValue {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_register_brrw!(self.out, ctx);
        let lhs = self.lhs.compile_register(ctx)?;
        let rhs = self.rhs.compile_register(ctx)?;
        ctx.emit_binop(hash_str("compare/sneq"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
pub struct CompareStrictNotEqual;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for CompareStrictNotEqual {
    fn specialize_invocation(
        &self,
        specialization: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if specialization.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: specialization.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let output = ValueCell::from_exact(false)?;
        let invocation = FunctionInvocation::binary(
            output,
            specialization.input(0).expect("validated lhs").cell()?.clone(),
            specialization.input(1).expect("validated rhs").cell()?.clone(),
        );
        let implementation = StrictNotEqValue::new_invocation(invocation.clone())?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            implementation,
            invocation,
        )))
    }
}
