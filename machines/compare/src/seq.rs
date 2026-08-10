use crate::*;
use mech_core::*;

#[derive(Debug)]
pub struct StrictEqValue {
    pub lhs: LegacyValue,
    pub rhs: LegacyValue,
    pub out: Ref<bool>,
}

impl MechFunctionImpl for StrictEqValue {
    fn solve_result(&self) -> MResult<()> {
        let lhs = match &self.lhs {
            LegacyValue::MutableReference(v) => v.borrow().clone(),
            v => v.clone(),
        };
        let rhs = match &self.rhs {
            LegacyValue::MutableReference(v) => v.borrow().clone(),
            v => v.clone(),
        };
        *self.out.borrow_mut() = lhs == rhs;
        Ok(())
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

impl MechFunctionFactory for StrictEqValue {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, lhs, rhs) => Ok(Box::new(Self {
                lhs,
                rhs,
                out: out.try_function_ref(FunctionArgumentRole::Output)?,
            })),
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for StrictEqValue {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.out.to_value();
        let destination = compile_value_register(&output, self.out.addr(), ctx)?;
        let lhs = compile_value_register(&self.lhs, core::ptr::from_ref(&self.lhs).addr(), ctx)?;
        let rhs = compile_value_register(&self.rhs, core::ptr::from_ref(&self.rhs).addr(), ctx)?;
        ctx.emit_binop(hash_str("compare/seq"), destination, lhs, rhs);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
fn impl_seq_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    Ok(Box::new(StrictEqValue {
        lhs: lhs_value,
        rhs: rhs_value,
        out: Ref::new(false),
    }))
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(CompareStrictEqual, impl_seq_fxn, "compare/seq");
