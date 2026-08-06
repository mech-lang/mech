use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Element Of --------------------------------------------------------------------
//
// Returns true iff elem ∈ set
//

#[derive(Debug)]
pub(crate) struct SetElementOfFxn {
    elem: Value,
    set: Ref<MechSet>,
    out: Ref<bool>,
}

impl MechFunctionFactory for SetElementOfFxn {
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
                Ok(Box::new(SetElementOfFxn { elem, set, out }))
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

impl MechFunctionImpl for SetElementOfFxn {
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
            let elem_ptr = &self.elem;
            let set_ptr: &MechSet = &*(self.set.as_ptr());

            // Only true if kinds are compatible and the set contains elem.
            if set_ptr.kind == elem_ptr.kind() {
                *out_ptr = set_ptr.set.contains(elem_ptr);
            } else {
                *out_ptr = false;
            }
        };
        Ok(())
    }
    fn out(&self) -> Value {
        Value::Bool(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetElementOfFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = compile_register_brrw!(self.out, ctx);
        let element = compile_value_register(
            &self.elem,
            core::ptr::from_ref(&self.elem).addr(),
            ctx,
        )?;
        let set = compile_register_brrw!(self.set, ctx);
        // Builtin operator ∈
        ctx.emit_binop(hash_str("SetElementOfFxn"), destination, element, set);
        Ok(destination)
    }
}

#[cfg(feature = "source")]
fn set_element_of_fxn(elem: Value, set: Value) -> MResult<Box<dyn MechFunction>> {
    match (elem, set) {
        (elem, Value::Set(set)) => Ok(Box::new(SetElementOfFxn {
            elem: normalize_set_element(elem),
            set: set.clone(),
            out: Ref::new(false),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/element-of".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetElementOf {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetElementOf {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
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
        match set_element_of_fxn(elem.clone(), set.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (elem, set) {
                (Value::MutableReference(elem), Value::MutableReference(set)) => {
                    set_element_of_fxn(elem.borrow().clone(), set.borrow().clone())
                }
                (elem, Value::MutableReference(set)) => {
                    set_element_of_fxn(elem.clone(), set.borrow().clone())
                }
                (Value::MutableReference(elem), set) => {
                    set_element_of_fxn(elem.borrow().clone(), set.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/element-of".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
