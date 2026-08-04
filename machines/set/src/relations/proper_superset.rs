use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Proper Superset ----------------------------------------------------------------
//
// Returns true iff lhs ⊃ rhs, i.e. lhs is a superset of rhs and strictly larger.
//

#[derive(Debug)]
pub(crate) struct SetProperSupersetFxn {
    lhs: Ref<MechSet>,
    rhs: Ref<MechSet>,
    out: Ref<bool>,
}

impl MechFunctionFactory for SetProperSupersetFxn {
    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechSet> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let rhs: Ref<MechSet> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<bool> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetProperSupersetFxn { lhs, rhs, out }))
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

impl MechFunctionImpl for SetProperSupersetFxn {
    fn solve(&self) {
        unsafe {
            let out_ptr: &mut bool = &mut *(self.out.as_mut_ptr());
            let lhs_ptr: &MechSet = &*(self.lhs.as_ptr());
            let rhs_ptr: &MechSet = &*(self.rhs.as_ptr());
            // Proper superset: lhs ⊃ rhs  <=>  lhs ⊇ rhs and |lhs| > |rhs|
            *out_ptr =
                lhs_ptr.set.is_superset(&rhs_ptr.set) && (lhs_ptr.set.len() > rhs_ptr.set.len());
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
impl MechFunctionCompiler for SetProperSupersetFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = "SetProperSupersetFxn".to_string();
        // Builtin operator ⊋
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn set_proper_superset_fxn(lhs: Value, rhs: Value) -> MResult<Box<dyn MechFunction>> {
    match (lhs, rhs) {
        (Value::Set(lhs), Value::Set(rhs)) => Ok(Box::new(SetProperSupersetFxn {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: Ref::new(false),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (x.0.kind(), x.1.kind()),
                fxn_name: "set/proper-superset".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetProperSuperset {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetProperSuperset {
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
        match set_proper_superset_fxn(lhs.clone(), rhs.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (lhs, rhs) {
                (Value::MutableReference(lhs), Value::MutableReference(rhs)) => {
                    set_proper_superset_fxn(lhs.borrow().clone(), rhs.borrow().clone())
                }
                (lhs, Value::MutableReference(rhs)) => {
                    set_proper_superset_fxn(lhs.clone(), rhs.borrow().clone())
                }
                (Value::MutableReference(lhs), rhs) => {
                    set_proper_superset_fxn(lhs.borrow().clone(), rhs.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/proper-superset".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
