use crate::*;

use mech_core::set::MechSet;

// Superset ------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SetSupersetFxn {
    lhs: Ref<MechSet>,
    rhs: Ref<MechSet>,
    out: Ref<bool>,
}
impl MechFunctionFactory for SetSupersetFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs: Ref<MechSet> = lhs.try_ref()?;
        let rhs: Ref<MechSet> = rhs.try_ref()?;
        let out: Ref<bool> = out.try_ref()?;
        Ok(Box::new(Self { lhs, rhs, out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
impl MechFunctionImpl for SetSupersetFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            // Get mutable reference to the output set
            let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());

            // Get references to lhs and rhs sets
            let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
            let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

            // Check if lhs is superset of rhs
            *out_ptr = lhs_ptr.set.is_superset(&(rhs_ptr.set));
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Bool(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetSupersetFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("SetSupersetFxn");
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn set_superset_fxn(lhs: LegacyValue, rhs: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (lhs, rhs) {
        (LegacyValue::Set(lhs), LegacyValue::Set(rhs)) => Ok(Box::new(SetSupersetFxn {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: Ref::new(false),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/superset".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetSuperset {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetSuperset {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = arguments[0].clone();
        let rhs = arguments[1].clone();
        match set_superset_fxn(lhs.clone(), rhs.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (lhs, rhs) {
                (LegacyValue::MutableReference(lhs), LegacyValue::MutableReference(rhs)) => {
                    set_superset_fxn(lhs.borrow().clone(), rhs.borrow().clone())
                }
                (lhs, LegacyValue::MutableReference(rhs)) => {
                    set_superset_fxn(lhs.clone(), rhs.borrow().clone())
                }
                (LegacyValue::MutableReference(lhs), rhs) => {
                    set_superset_fxn(lhs.borrow().clone(), rhs.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/superset".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
