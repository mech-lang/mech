use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;
use std::sync::LazyLock;

static PURE_SET_INSERT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

// Insert ------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SetInsertFxn {
    arg1: Ref<MechSet>,
    arg2: LegacyValue,
    out: Ref<MechSet>,
}
impl MechFunctionFactory for SetInsertFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::AnyValue,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let arg1: Ref<MechSet> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let arg2 = normalize_set_element(arg2);
                let out: Ref<MechSet> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetInsertFxn { arg1, arg2, out }))
            }
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

fn match_types(type1: ValueKind, type2: ValueKind) -> (bool, bool) {
    match (type1, type2) {
        (ValueKind::Set(k1, s1), ValueKind::Set(k2, s2)) => {
            let (types_match, _) = match_types(*k1, *k2);
            (types_match, s1 == s2)
        }
        (ValueKind::Set(_, _), _) => (false, false),
        (_, ValueKind::Set(_, _)) => (false, false),
        (k1, k2) => (k1 == k2, k1 == k2),
    }
}

impl MechFunctionImpl for SetInsertFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            // Get mutable reference to the output set
            let mut out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

            // Get references to arg1 and arg2 sets
            let set_ptr: &MechSet = &*(self.arg1.as_ptr());
            let elem_ptr = &self.arg2;

            // Clear the output set first (optional, depending on semantics)
            out_ptr.set.clear();

            let (types_match, sizes_match) =
                match_types(set_ptr.kind.clone(), elem_ptr.kind().clone());
            // Insert arg2 into arg1
            if (types_match) {
                out_ptr.set = set_ptr.set.clone();
                out_ptr.set.insert(elem_ptr.clone());
                if (!sizes_match) {
                    out_ptr.kind = match out_ptr.kind.clone() {
                        ValueKind::Set(k1, _) => ValueKind::Set(k1, None),
                        _ => ValueKind::Empty,
                    }
                }
            }
            // Update metadata
            out_ptr.sync_cardinality_from_contents();
            if (types_match && sizes_match) {
                out_ptr.kind = set_ptr.kind.clone();
            }
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Set(self.out.clone())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_INSERT_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetInsertFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_register_brrw!(self.out, ctx);
        let set = compile_register_brrw!(self.arg1, ctx);
        let element =
            compile_value_register(&self.arg2, core::ptr::from_ref(&self.arg2).addr(), ctx)?;
        ctx.emit_binop(hash_str("SetInsertFxn"), destination, set, element);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
fn set_insert_fxn(arg1: LegacyValue, arg2: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (arg1, arg2) {
        (LegacyValue::Set(arg1), arg2) => Ok(Box::new(SetInsertFxn {
            arg1: arg1.clone(),
            arg2: normalize_set_element(arg2),
            out: Ref::new(MechSet::new(
                arg1.borrow().kind.clone(),
                arg1.borrow().num_elements + 1,
            )),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/insert".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetInsert {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetInsert {
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
        let arg1 = arguments[0].clone();
        let arg2 = arguments[1].clone();
        match set_insert_fxn(arg1.clone(), arg2.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match (arg1, arg2) {
                (LegacyValue::MutableReference(arg1), LegacyValue::MutableReference(arg2)) => {
                    set_insert_fxn(arg1.borrow().clone(), arg2.borrow().clone())
                }
                (arg1, LegacyValue::MutableReference(arg2)) => {
                    set_insert_fxn(arg1.clone(), arg2.borrow().clone())
                }
                (LegacyValue::MutableReference(arg1), arg2) => {
                    set_insert_fxn(arg1.borrow().clone(), arg2.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/insert".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_contract_matches_the_runtime_no_alias_policy() {
        assert_eq!(
            PURE_SET_INSERT_CONTRACT.outputs[0].alias,
            AliasPolicy::NoAlias
        );
    }
}
