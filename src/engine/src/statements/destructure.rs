#[cfg(feature = "tuple")]
use super::VariableAlreadyDefinedError;
#[cfg(feature = "tuple")]
use crate::{
    FunctionValueRepresentation, InterpreterExecution, MResult, MechError, MechErrorKind,
    SchemaBody, TupleDestructure, ValueCell, expression_cell,
};

#[cfg(feature = "tuple")]
#[derive(Debug, Clone)]
struct CanonicalDestructureExpectedTuple {
    representation: FunctionValueRepresentation,
}

#[cfg(feature = "tuple")]
impl MechErrorKind for CanonicalDestructureExpectedTuple {
    fn name(&self) -> &str {
        "DestructureExpectedTuple"
    }

    fn message(&self) -> String {
        format!(
            "Expected a tuple value for destructuring, found: {:?}",
            self.representation
        )
    }
}

#[cfg(feature = "tuple")]
#[derive(Debug, Clone)]
struct CanonicalTupleDestructureTooManyVars {
    representation: FunctionValueRepresentation,
}

#[cfg(feature = "tuple")]
impl MechErrorKind for CanonicalTupleDestructureTooManyVars {
    fn name(&self) -> &str {
        "TupleDestructureTooManyVars"
    }

    fn message(&self) -> String {
        format!(
            "Attempted to destructure tuple into too many variables: {:?}",
            self.representation
        )
    }
}

#[cfg(feature = "tuple")]
pub fn tuple_destructure(
    destructure: &TupleDestructure,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let source = expression_cell(&destructure.expression, None, interpreter)?;
    let representation = source.representation();
    let SchemaBody::Tuple(_) = source.closed_schema_body()? else {
        return Err(
            MechError::new(CanonicalDestructureExpectedTuple { representation }, None)
                .with_compiler_loc()
                .with_tokens(destructure.expression.tokens()),
        );
    };
    let element_values = source
        .reactive_tuple_elements()?
        .expect("a validated tuple schema retains tuple values");

    if destructure.vars.len() > element_values.len() {
        return Err(MechError::new(
            CanonicalTupleDestructureTooManyVars { representation },
            None,
        )
        .with_compiler_loc()
        .with_tokens(destructure.expression.tokens()));
    }

    let symbols = interpreter.symbols();
    let mut symbols = symbols.borrow_mut();
    for variable in &destructure.vars {
        let id = variable.hash();
        if symbols.contains(id) {
            return Err(MechError::new(VariableAlreadyDefinedError { id }, None)
                .with_compiler_loc()
                .with_tokens(variable.tokens()));
        }
    }
    for (variable, element) in destructure.vars.iter().zip(element_values) {
        let id = variable.hash();
        symbols.insert_cell(id, element, true);
        symbols
            .dictionary
            .borrow_mut()
            .insert(id, variable.name.to_string());
    }
    Ok(source)
}
