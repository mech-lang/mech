use crate::*;

use mech_core::set::MechSet;

// Union ------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SetUnionFxn {
    lhs: Ref<MechSet>,
    rhs: Ref<MechSet>,
    out: Ref<MechSet>,
}
impl MechFunctionFactory for SetUnionFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, lhs, rhs) = invocation.expect_binary()?;
        let lhs: Ref<MechSet> = lhs.try_ref()?;
        let rhs: Ref<MechSet> = rhs.try_ref()?;
        let out: Ref<MechSet> = out.try_ref()?;
        Ok(Box::new(Self { lhs, rhs, out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
impl MechFunctionImpl for SetUnionFxn {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
    }
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            // Get mutable reference to the output set
            let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

            // Get references to lhs and rhs sets
            let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
            let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

            // Clear the output set first (optional, depending on semantics)
            out_ptr.set.clear();

            // Intersection lhs and rhs sets into output
            out_ptr.set = lhs_ptr.set.union(&(rhs_ptr.set)).cloned().collect();

            // Update metadata
            out_ptr.sync_cardinality_from_contents();
            out_ptr.kind = if out_ptr.set.len() > 0 {
                out_ptr.set.iter().next().unwrap().kind()
            } else {
                ValueKind::Empty
            };
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Set(self.out.clone())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_BINARY_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetUnionFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("SetUnionFxn");
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn set_union_fxn(lhs: LegacyValue, rhs: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (lhs, rhs) {
        (LegacyValue::Set(lhs), LegacyValue::Set(rhs)) => Ok(Box::new(SetUnionFxn {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: Ref::new(MechSet::new(
                lhs.borrow().kind.clone(),
                lhs.borrow().num_elements + rhs.borrow().num_elements,
            )),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/union".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetUnion {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetUnion {
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
        match set_union_fxn(lhs.clone(), rhs.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (lhs, rhs) {
                (LegacyValue::MutableReference(lhs), LegacyValue::MutableReference(rhs)) => {
                    set_union_fxn(lhs.borrow().clone(), rhs.borrow().clone())
                }
                (lhs, LegacyValue::MutableReference(rhs)) => {
                    set_union_fxn(lhs.clone(), rhs.borrow().clone())
                }
                (LegacyValue::MutableReference(lhs), rhs) => {
                    set_union_fxn(lhs.borrow().clone(), rhs.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/union".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
