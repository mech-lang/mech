#[cfg(feature = "subscript_formula")]
use super::string::{
    current_string_access_expression_live, string_access_argument_is_live,
    string_access_index_argument,
};
use super::{
    Environment, execute_access_function, subscript_formula, subscript_formula_ix, subscript_range,
};
use crate::{
    FunctionValueRepresentation, InterpreterExecution, MResult, SchemaBody, SpecializationInput,
    Subscript, ValueCell,
};

fn selector_is_scalar(selector: &SpecializationInput) -> MResult<bool> {
    let Some(cell) = selector.cell().ok() else {
        return Ok(false);
    };
    if !matches!(
        cell.representation(),
        FunctionValueRepresentation::Matrix { .. }
    ) {
        return Ok(true);
    }
    Ok(selector
        .matrix_descriptor()?
        .is_some_and(|matrix| matrix.rows.checked_mul(matrix.cols) == Some(1)))
}

fn operation_for(selectors: &[SpecializationInput]) -> MResult<&'static str> {
    if matches!(
        selectors,
        [
            SpecializationInput::MatrixAllSelection,
            SpecializationInput::Cell(_)
        ]
    ) {
        return Ok("access/columns");
    }
    if matches!(
        selectors,
        [
            SpecializationInput::Cell(_),
            SpecializationInput::MatrixAllSelection
        ]
    ) {
        return Ok("access/rows");
    }
    if selectors.len() == 2
        && selectors
            .iter()
            .any(|selector| !selector_is_scalar(selector).unwrap_or(false))
    {
        return Ok("access/rectangle");
    }
    if selectors
        .iter()
        .all(|selector| selector_is_scalar(selector).unwrap_or(false))
    {
        Ok("access/scalar")
    } else {
        Ok("access/range")
    }
}

pub(super) fn access(
    subscript: &Subscript,
    value: &ValueCell,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let Subscript::Bracket(subscripts) = subscript else {
        unreachable!()
    };
    let map_key_selection = matches!(value.closed_schema_body()?, SchemaBody::Map { .. });
    let mut selectors = Vec::with_capacity(subscripts.len());
    for selector in subscripts {
        match selector {
            #[cfg(feature = "subscript_formula")]
            Subscript::Formula(_) => {
                if map_key_selection {
                    selectors.push(SpecializationInput::Cell(subscript_formula(
                        selector, env, p,
                    )?));
                    continue;
                }
                let index = subscript_formula_ix(selector, env, p)?;
                selectors.push(SpecializationInput::Cell(string_access_index_argument(
                    index, selector, env, p,
                )?));
            }
            #[cfg(feature = "subscript_range")]
            Subscript::Range(_) => {
                selectors.push(SpecializationInput::Cell(subscript_range(
                    selector, env, p,
                )?));
            }
            Subscript::All => selectors.push(SpecializationInput::MatrixAllSelection),
            _ => unreachable!("invalid bracket selector"),
        }
    }
    #[cfg(feature = "subscript_formula")]
    let _source_is_live =
        current_string_access_expression_live(p) || string_access_argument_is_live(value, p);
    let operation = operation_for(&selectors)?;
    let mut inputs = Vec::with_capacity(selectors.len() + 1);
    inputs.push(SpecializationInput::Cell(value.clone()));
    inputs.extend(selectors);
    execute_access_function(p, operation, inputs)
}
