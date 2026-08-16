use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;
use std::sync::LazyLock;

static PURE_SET_REMOVE_CONTRACT: LazyLock<OperationContractDeclaration> =
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

// Remove ------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SetRemoveFxn {
    arg1: Ref<MechSet>,
    arg2: LegacyValue,
    out: Ref<MechSet>,
}
impl MechFunctionFactory for SetRemoveFxn {
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
                Ok(Box::new(SetRemoveFxn { arg1, arg2, out }))
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
impl MechFunctionImpl for SetRemoveFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            // Get mutable reference to the output set
            let mut out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

            // Get references to arg1 and arg2 sets
            let set_ptr: &MechSet = &*(self.arg1.as_ptr());
            let elem_ptr = &self.arg2;

            // Clear the output set first (optional, depending on semantics)
            out_ptr.set.clear();

            // Remove arg2 into arg1
            if (set_ptr.kind == elem_ptr.kind()) {
                out_ptr.set = set_ptr.set.clone();
                out_ptr.set.shift_remove(elem_ptr);
            }
            // Update metadata
            out_ptr.sync_cardinality_from_contents();
            out_ptr.kind = set_ptr.kind.clone();
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Set(self.out.clone())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_REMOVE_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetRemoveFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_register_brrw!(self.out, ctx);
        let set = compile_register_brrw!(self.arg1, ctx);
        let element =
            compile_value_register(&self.arg2, core::ptr::from_ref(&self.arg2).addr(), ctx)?;
        ctx.emit_binop(hash_str("SetRemoveFxn"), destination, set, element);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
fn set_remove_fxn(arg1: LegacyValue, arg2: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (arg1, arg2) {
        (LegacyValue::Set(arg1), arg2) => Ok(Box::new(SetRemoveFxn {
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
                fxn_name: "set/remove".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetRemove {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetRemove {
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
        match set_remove_fxn(arg1.clone(), arg2.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match (arg1, arg2) {
                (LegacyValue::MutableReference(arg1), LegacyValue::MutableReference(arg2)) => {
                    set_remove_fxn(arg1.borrow().clone(), arg2.borrow().clone())
                }
                (arg1, LegacyValue::MutableReference(arg2)) => {
                    set_remove_fxn(arg1.clone(), arg2.borrow().clone())
                }
                (LegacyValue::MutableReference(arg1), arg2) => {
                    set_remove_fxn(arg1.borrow().clone(), arg2.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/remove".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
