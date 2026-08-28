use crate::*;

use mech_core::set::MechSet;

// Size --------------------------------------------------------------------------
//
// Returns the cardinality |A| as a u64.
//

#[derive(Debug)]
pub(crate) struct SetSizeFxn {
    input: Ref<MechSet>,
    out: Ref<u64>,
}

impl MechFunctionFactory for SetSizeFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::U64,
        FunctionValueRepresentation::Set,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, input) = invocation.expect_unary()?;
        let input: Ref<MechSet> = input.try_ref()?;
        let out: Ref<u64> = out.try_ref()?;
        Ok(Box::new(Self { input, out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}

impl MechFunctionImpl for SetSizeFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr: &mut u64 = &mut *(self.out.as_mut_ptr());
            let input_ptr: &MechSet = &*(self.input.as_ptr());
            // Uses the internal IndexSet length
            *out_ptr = input_ptr.set.len() as u64;
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::U64(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetSizeFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = "SetSizeFxn".to_string();
        // Custom feature route: set/size
        compile_unop!(name, self.out, self.input, ctx);
    }
}

#[cfg(feature = "source")]
fn set_size_fxn(input: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match input {
        LegacyValue::Set(s) => Ok(Box::new(SetSizeFxn {
            input: s.clone(),
            out: Ref::new(0u64),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 {
                arg: x.kind(),
                fxn_name: "set/size".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetSize {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetSize {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = arguments[0].clone();
        match set_size_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match input {
                LegacyValue::MutableReference(r) => set_size_fxn(r.borrow().clone()),
                input => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: input.kind(),
                        fxn_name: "set/size".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
