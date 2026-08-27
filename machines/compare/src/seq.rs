use crate::*;

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
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.out))
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(crate::compare_full_write_contract(
            FunctionValueRepresentation::Bool,
        ))
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

#[cfg(feature = "semantic-compiler")]
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

#[cfg(all(test, feature = "runtime", feature = "bool", feature = "sneq"))]
mod state_port_tests {
    use super::*;
    use crate::StrictNotEqValue;

    #[test]
    fn strict_comparison_outputs_use_typed_identity_and_checkpoint_state() {
        let equal_out = Ref::new(false);
        let equal = StrictEqValue {
            lhs: LegacyValue::Index(Ref::new(1)),
            rhs: LegacyValue::Index(Ref::new(1)),
            out: equal_out.clone(),
        };
        equal.solve_result().unwrap();
        assert_eq!(
            equal.reactive_output_cell_ids(),
            equal.out().reactive_root_cell_ids(),
        );

        let not_equal_out = Ref::new(false);
        let not_equal = StrictNotEqValue {
            lhs: LegacyValue::Index(Ref::new(1)),
            rhs: LegacyValue::Index(Ref::new(2)),
            out: not_equal_out.clone(),
        };
        not_equal.solve_result().unwrap();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&equal)?;
            participant.capture_function_state(&not_equal)?;
            *equal_out.borrow_mut() = false;
            *not_equal_out.borrow_mut() = false;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(*equal_out.borrow());
        assert!(*not_equal_out.borrow());
    }
}
