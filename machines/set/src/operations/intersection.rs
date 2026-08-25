use crate::*;

use mech_core::set::MechSet;

// Intersection ------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SetIntersectionFxn {
    lhs: Ref<MechSet>,
    rhs: Ref<MechSet>,
    out: Ref<MechSet>,
}
impl MechFunctionFactory for SetIntersectionFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Binary(out, arg1, arg2) => {
                let lhs: Ref<MechSet> = arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                let rhs: Ref<MechSet> = arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                let out: Ref<MechSet> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetIntersectionFxn { lhs, rhs, out }))
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
impl MechFunctionImpl for SetIntersectionFxn {
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
            out_ptr.set = lhs_ptr.set.intersection(&(rhs_ptr.set)).cloned().collect();

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
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for SetIntersectionFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("SetIntersectionFxn");
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
    }
}

#[cfg(feature = "source")]
fn set_intersection_fxn(lhs: LegacyValue, rhs: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    match (lhs, rhs) {
        (LegacyValue::Set(lhs), LegacyValue::Set(rhs)) => Ok(Box::new(SetIntersectionFxn {
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
                fxn_name: "set/intersection".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetIntersection {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetIntersection {
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
        match set_intersection_fxn(lhs.clone(), rhs.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (lhs, rhs) {
                (LegacyValue::MutableReference(lhs), LegacyValue::MutableReference(rhs)) => {
                    set_intersection_fxn(lhs.borrow().clone(), rhs.borrow().clone())
                }
                (lhs, LegacyValue::MutableReference(rhs)) => {
                    set_intersection_fxn(lhs.clone(), rhs.borrow().clone())
                }
                (LegacyValue::MutableReference(lhs), rhs) => {
                    set_intersection_fxn(lhs.borrow().clone(), rhs.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (x.0.kind(), x.1.kind()),
                        fxn_name: "set/intersection".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
