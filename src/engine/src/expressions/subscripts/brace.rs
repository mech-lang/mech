use super::{Environment, execute_access_function, subscript_formula, subscript_range};
use crate::{InterpreterExecution, MResult, SpecializationInput, Subscript, ValueCell};

pub(super) fn access(
    subscript: &Subscript,
    value: &ValueCell,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let Subscript::Brace(subscripts) = subscript else {
        unreachable!()
    };
    let mut inputs = vec![SpecializationInput::Cell(value.clone())];
    let operation = match subscripts.as_slice() {
        #[cfg(feature = "subscript_formula")]
        [selector @ Subscript::Formula(_)] => {
            inputs.push(SpecializationInput::Cell(subscript_formula(
                selector, env, p,
            )?));
            "access/scalar"
        }
        #[cfg(feature = "subscript_range")]
        [selector @ Subscript::Range(_)] => {
            inputs.push(SpecializationInput::Cell(subscript_range(
                selector, env, p,
            )?));
            "access/range"
        }
        _ => unimplemented!("brace access layout"),
    };
    execute_access_function(p, operation, inputs)
}
