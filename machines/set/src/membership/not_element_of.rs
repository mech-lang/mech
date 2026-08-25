use crate::*;

use mech_core::set::MechSet;

// Not Element Of ----------------------------------------------------------------
//
// Returns true iff elem ∉ set. Mirrors element_of with negated result.
//

#[derive(Debug)]
pub(crate) struct SetNotElementOfFxn {
    elem: LegacyValue,
    set: Ref<MechSet>,
    out: Ref<bool>,
}

impl MechFunctionFactory for SetNotElementOfFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::Set,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let elem = normalize_set_element(arg1);
                let set: Ref<MechSet> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<bool> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetNotElementOfFxn { elem, set, out }))
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

impl MechFunctionImpl for SetNotElementOfFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
            let elem_ptr = &self.elem;
            let set_ptr: &MechSet = &*(self.set.as_ptr());

            // Only true if kinds are incompatible or the set does not contain elem.
            if set_ptr.kind == elem_ptr.kind() {
                *out_ptr = !set_ptr.set.contains(elem_ptr);
            } else {
                *out_ptr = true;
            }
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Bool(self.out.clone())
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SET_MEMBERSHIP_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetNotElementOfFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_register_brrw!(self.out, ctx);
        let element =
            compile_value_register(&self.elem, core::ptr::from_ref(&self.elem).addr(), ctx)?;
        let set = compile_register_brrw!(self.set, ctx);
        // Builtin operator ∉
        ctx.emit_binop(hash_str("SetNotElementOfFxn"), destination, element, set);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
fn set_not_element_of_fxn(elem: LegacyValue, set: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (elem, set) {
        (elem, LegacyValue::Set(set)) => Ok(Box::new(SetNotElementOfFxn {
            elem: normalize_set_element(elem),
            set: set.clone(),
            out: Ref::new(false),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/not-element-of".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetNotElementOf {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetNotElementOf {
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
        let elem = arguments[0].clone();
        let set = arguments[1].clone();
        match set_not_element_of_fxn(elem.clone(), set.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (elem, set) {
                (LegacyValue::MutableReference(elem), LegacyValue::MutableReference(set)) => {
                    set_not_element_of_fxn(elem.borrow().clone(), set.borrow().clone())
                }
                (elem, LegacyValue::MutableReference(set)) => {
                    set_not_element_of_fxn(elem.clone(), set.borrow().clone())
                }
                (LegacyValue::MutableReference(elem), set) => {
                    set_not_element_of_fxn(elem.borrow().clone(), set.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/not-element-of".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
