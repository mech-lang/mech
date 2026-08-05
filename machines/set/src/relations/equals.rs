use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Equals ------------------------------------------------------------------------
//
// Returns true if lhs and rhs contain exactly the same elements.
//

#[derive(Debug)]
pub(crate) struct SetEqualsFxn {
    lhs: Ref<MechSet>,
    rhs: Ref<MechSet>,
    out: Ref<bool>,
}

impl MechFunctionFactory for SetEqualsFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechSet> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let rhs: Ref<MechSet> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<bool> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetEqualsFxn { lhs, rhs, out }))
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

impl MechFunctionImpl for SetEqualsFxn {
    fn solve(&self) {
        unsafe {
            let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
            let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
            let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());

            // Uses the implementation of PartialEq for IndexSet (== operator)
            *out_ptr = lhs_ptr.set == rhs_ptr.set;
        }
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
impl MechFunctionCompiler for SetEqualsFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = "SetEqualsFxn".to_string();
        // Custom feature route: set/equals
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn set_equals_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
    match (lhs, rhs) {
        (Value::Set(lhs), Value::Set(rhs)) => Ok(Box::new(SetEqualsFxn {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: Ref::new(false),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/equals".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetEquals {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetEquals {
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
        let lhs = arguments[0].clone();
        let rhs = arguments[1].clone();
        match set_equals_fxn(lhs.clone(), rhs.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (lhs, rhs) {
                (Value::MutableReference(lhs), Value::MutableReference(rhs)) => {
                    set_equals_fxn(lhs.borrow().clone(), rhs.borrow().clone())
                }
                (lhs, Value::MutableReference(rhs)) => {
                    set_equals_fxn(lhs.clone(), rhs.borrow().clone())
                }
                (Value::MutableReference(lhs), rhs) => {
                    set_equals_fxn(lhs.borrow().clone(), rhs.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/equals".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
