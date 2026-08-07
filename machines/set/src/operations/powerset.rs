use std::cell::RefCell;

use crate::*;

use indexmap::set::IndexSet;
use mech_core::set::MechSet;

// Powerset ------------------------------------------------------------------------

/// Powersets grow exponentially and materialize every subset as a Mech value.
/// Sixteen inputs already produce 65,536 sets and 524,288 memberships, so this
/// is the deterministic bytecode-v1 work boundary for both initial and reactive
/// execution.
const MAX_POWERSET_INPUT_CARDINALITY: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetPowersetLimitExceeded {
    pub found: usize,
    pub maximum: usize,
}

impl MechErrorKind for SetPowersetLimitExceeded {
    fn name(&self) -> &str {
        "SetPowersetLimitExceeded"
    }

    fn message(&self) -> String {
        format!(
            "set/powerset input has {} elements; the maximum supported cardinality is {}",
            self.found, self.maximum,
        )
    }
}

fn powerset_output_len(input_cardinality: usize) -> MResult<usize> {
    if input_cardinality > MAX_POWERSET_INPUT_CARDINALITY {
        return Err(MechError::new(
            SetPowersetLimitExceeded {
                found: input_cardinality,
                maximum: MAX_POWERSET_INPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc());
    }
    1usize.checked_shl(input_cardinality as u32).ok_or_else(|| {
        MechError::new(
            SetPowersetLimitExceeded {
                found: input_cardinality,
                maximum: MAX_POWERSET_INPUT_CARDINALITY,
            },
            None,
        )
        .with_compiler_loc()
    })
}

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
    fn solve_result(&self) -> MResult<()> {
        unsafe {
            // Get mutable reference to the output set
            let out_ptr: &mut MechSet = &mut *(self.out.as_mut_ptr());

            // Get references to lhs and rhs sets
            let input_ptr: &MechSet = &*(self.input.as_ptr());

            // Revalidate on every solve: live inputs can grow after the
            // function was planned and initially executed.
            let output_len = powerset_output_len(input_ptr.set.len())?;

            // Build the complete next value before replacing the reactive
            // output so a rejected solve leaves the previous value intact.
            let vec_set = powerset_recursive(&(input_ptr.set.clone().into_iter().collect()));
            debug_assert_eq!(vec_set.len(), output_len);
            let mut next = MechSet::new(
                ValueKind::Set(Box::new(input_ptr.kind.clone()), None),
                output_len,
            );
            for set in vec_set {
                next.set
                    .insert(Value::Set(Ref::new(MechSet::from_vec(set))));
            }
            next.sync_cardinality_from_contents();
            next.kind = ValueKind::Set(Box::new(input_ptr.kind.clone()), None);

            *out_ptr = next;
        };
        Ok(())
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
        (Value::Set(input)) => {
            let output_len = powerset_output_len(input.borrow().set.len())?;
            Ok(Box::new(SetPowersetFxn {
                input: input.clone(),
                out: Ref::new(MechSet::new(input.borrow().kind.clone(), output_len)),
            }))
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn index_set(cardinality: usize) -> MechSet {
        MechSet::from_vec(
            (0..cardinality)
                .map(|index| Value::Index(Ref::new(index)))
                .collect(),
        )
    }

    #[test]
    fn powerset_rejects_unbounded_initial_and_reactive_inputs() {
        let initial_error = powerset_output_len(MAX_POWERSET_INPUT_CARDINALITY + 1).unwrap_err();
        assert_eq!(initial_error.kind_name(), "SetPowersetLimitExceeded");

        let input = Ref::new(index_set(2));
        let out = Ref::new(MechSet::new(ValueKind::Set(Box::new(ValueKind::Index), None), 1));
        let function = SetPowersetFxn {
            input: input.clone(),
            out: out.clone(),
        };
        function.solve_result().unwrap();
        assert_eq!(out.borrow().set.len(), 4);
        let previous = out.borrow().clone();

        *input.borrow_mut() = index_set(MAX_POWERSET_INPUT_CARDINALITY + 1);
        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "SetPowersetLimitExceeded");
        assert_eq!(*out.borrow(), previous);
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
