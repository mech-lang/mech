use std::cell::RefCell;

use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Powerset ------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct SetPowersetFxn {
    input: Ref<MechSet>,
    out: Ref<MechSet>,
}
impl MechFunctionFactory for SetPowersetFxn {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        FunctionValueRepresentation::Set,
        FunctionValueRepresentation::Set,
    );

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Unary(out, input) => {
                let input: Ref<MechSet> = input.try_function_ref(FunctionArgumentRole::Input(0))?;
                let out: Ref<MechSet> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(SetPowersetFxn { input, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

fn powerset_recursive<T>(set: &Vec<T>) -> Vec<Vec<T>>
where
    T: std::fmt::Debug + Clone,
{
    if set.len() == 0 {
        return vec![vec![]];
    }
    let mut with_set = powerset_recursive_aux(set, vec![vec![set[0].clone()]], 1);
    let mut without_set = powerset_recursive_aux(set, vec![vec![]], 1);
    with_set.append(&mut without_set);
    with_set.sort_by(|a, b| a.len().cmp(&b.len()));
    with_set
}

fn powerset_recursive_aux<T>(
    set: &Vec<T>,
    mut unfinished_set: Vec<Vec<T>>,
    index: usize,
) -> Vec<Vec<T>>
where
    T: std::fmt::Debug + Clone,
{
    if index == set.len() {
        return unfinished_set;
    }
    let mut with_set = powerset_recursive_aux(
        set,
        unfinished_set
            .iter_mut()
            .map(|x| {
                let mut y = x.clone();
                y.push(set[index].clone());
                y
            })
            .collect(),
        index + 1,
    );
    let mut without_set = powerset_recursive_aux(set, unfinished_set, index + 1);
    with_set.append(&mut without_set);
    with_set
}

impl MechFunctionImpl for SetPowersetFxn {
    fn solve(&self) {
        unsafe {
            // Get mutable reference to the output set
            let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

            // Get references to lhs and rhs sets
            let input_ptr: &MechSet = &*(self.input.as_ptr());

            // Clear the output set first (optional, depending on semantics)
            out_ptr.set.clear();

            // Powerset input into output
            let vec_set = powerset_recursive(&(input_ptr.set.clone().into_iter().collect()));
            for set in vec_set {
                out_ptr
                    .set
                    .insert(Value::Set(Ref::new(MechSet::from_vec(set))));
            }

            // Update metadata
            out_ptr.sync_cardinality_from_contents();
            out_ptr.kind = ValueKind::Set(Box::new(input_ptr.kind.clone()), None);
        }
    }
    fn out(&self) -> Value {
        Value::Set(self.out.clone())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for SetPowersetFxn {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("SetPowersetFxn");
        compile_unop!(name, self.out, self.input, ctx);
    }
}

#[cfg(feature = "source")]
fn set_powerset_fxn(input: Value) -> MResult<Box<dyn MechFunction>> {
    match (input) {
        (Value::Set(input)) => Ok(Box::new(SetPowersetFxn {
            input: input.clone(),
            out: Ref::new(MechSet::new(
                input.borrow().kind.clone(),
                2_u32.pow(input.borrow().num_elements as u32) as usize,
            )),
        })),
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 {
                arg: x.kind(),
                fxn_name: "set/powerset".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "source")]
pub struct SetPowerset {}
#[cfg(feature = "source")]
impl FunctionSpecializer for SetPowerset {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = arguments[0].clone();
        match set_powerset_fxn(input.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match input {
                Value::MutableReference(input) => set_powerset_fxn(input.borrow().clone()),
                input => set_powerset_fxn(input.clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: "set/powerset".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}
