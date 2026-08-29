use super::variables::{addressed_identifier_hash, addressed_identifier_name};
use super::{Environment, factor, range};
use crate::{
    FunctionMatrixElement, FunctionValueRepresentation, InterpreterExecution, MResult, MechError,
    OperationId, Slice, SpecializationInput, SpecializationInvocation, SpecializedFunction,
    Subscript, ValueCell,
};

#[cfg(all(feature = "subscript", feature = "access"))]
mod brace;
#[cfg(all(feature = "subscript", feature = "access", feature = "subscript_slice"))]
mod bracket;
#[cfg(all(feature = "subscript", feature = "access"))]
mod dot;
#[cfg(feature = "subscript_formula")]
mod string;

#[cfg(all(feature = "subscript", feature = "access"))]
fn catalog_access_function(
    p: &InterpreterExecution<'_>,
    canonical_name: &str,
    arguments: &[SpecializationInput],
) -> MResult<SpecializedFunction> {
    let invocation = SpecializationInvocation::new(arguments.to_vec().into_boxed_slice());
    p.specialize_visible_invocation_named(
        OperationId::from_name(canonical_name),
        Some(canonical_name),
        &invocation,
    )
}

#[cfg(all(feature = "subscript", feature = "access"))]
fn execute_access_function(
    p: &InterpreterExecution<'_>,
    canonical_name: &str,
    arguments: Vec<SpecializationInput>,
) -> MResult<ValueCell> {
    let specialized = catalog_access_function(p, canonical_name, &arguments)?;
    crate::execute_bound_specialized_function(specialized, &arguments, p)
}

#[cfg(feature = "subscript_formula")]
pub(crate) use string::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, reset_current_string_access_expression_live,
    string_access_input_is_live, string_access_value_is_marked_live,
    take_current_string_access_expression_live,
};

#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub fn slice(
    slc: &Slice,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let id = addressed_identifier_hash(&slc.name, &slc.context);
    let name = addressed_identifier_name(&slc.name, &slc.context);
    let mut value = if let Some(value) = env.and_then(|environment| environment.get(&id)) {
        value.clone()
    } else {
        let symbols = p.symbols();
        symbols.borrow().get(id).ok_or_else(|| {
            MechError::new(super::UndefinedVariableError { id, name }, None)
                .with_compiler_loc()
                .with_tokens(slc.tokens())
        })?
    };
    for selector in &slc.subscript {
        value = subscript(selector, &value, env, p)?;
    }
    Ok(value)
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula(
    subscript: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match subscript {
        Subscript::Formula(factor_expression) => factor(factor_expression, env, p),
        _ => unreachable!(),
    }
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula_ix(
    subscript: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let value = subscript_formula(subscript, env, p)?;
    subscript_formula_index(&value, p)
}

#[cfg(feature = "subscript_range")]
pub fn subscript_range(
    subscript: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match subscript {
        Subscript::Range(range_expression) => {
            let value = range(range_expression, env, p)?;
            #[cfg(feature = "matrix")]
            return crate::intrinsics::access::matrix::canonical_reactive_index_matrix(value, p);
            #[cfg(not(feature = "matrix"))]
            return subscript_formula_index(&value, p);
        }
        _ => unreachable!(),
    }
}

#[cfg(all(feature = "subscript", feature = "access"))]
pub fn subscript(
    subscript: &Subscript,
    value: &ValueCell,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match subscript {
        #[cfg(feature = "table")]
        Subscript::Dot(_) => dot::access(subscript, value, p),
        Subscript::DotInt(_) => dot::access(subscript, value, p),
        #[cfg(feature = "swizzle")]
        Subscript::Swizzle(_) => dot::access(subscript, value, p),
        Subscript::Brace(_) => brace::access(subscript, value, env, p),
        #[cfg(feature = "subscript_slice")]
        Subscript::Bracket(_) => bracket::access(subscript, value, env, p),
        _ => unreachable!(),
    }
}

#[cfg(feature = "subscript_formula")]
fn subscript_formula_index(
    value: &ValueCell,
    execution: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    #[cfg(feature = "matrix")]
    {
        if let FunctionValueRepresentation::Matrix { element, .. } = value.representation() {
            if matches!(
                element,
                FunctionMatrixElement::Bool | FunctionMatrixElement::Index
            ) {
                return Ok(value.clone());
            }
            return crate::intrinsics::access::matrix::canonical_reactive_index_matrix(
                value.clone(),
                execution,
            );
        }
        return crate::intrinsics::access::matrix::canonical_reactive_scalar_index(
            value.clone(),
            execution,
        );
    }
    #[cfg(not(feature = "matrix"))]
    return Ok(value.clone());
}
