#[cfg(feature = "variable_assign")]
use super::{NotMutableError, UndefinedVariableError};
#[cfg(all(feature = "subscript", feature = "assign", feature = "access"))]
use crate::SchemaBody;
#[cfg(any(
    feature = "variable_assign",
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign",
    all(feature = "subscript", feature = "assign")
))]
use crate::ValueCell;
#[cfg(feature = "subscript_formula")]
use crate::factor;
#[cfg(feature = "subscript_range")]
use crate::range;
#[cfg(any(
    feature = "variable_assign",
    all(
        feature = "access",
        feature = "subscript",
        any(
            feature = "math_add_assign",
            feature = "math_sub_assign",
            feature = "math_div_assign",
            feature = "math_mul_assign",
            feature = "assign"
        )
    )
))]
use crate::{Environment, InterpreterExecution, MResult, MechError};
#[cfg(all(
    feature = "access",
    feature = "subscript",
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign",
        feature = "assign"
    )
))]
use crate::{
    FunctionMatrixElement, FunctionValueRepresentation, OperationId, SpecializationInput,
    SpecializationInvocation, Subscript, execute_bound_specialized_function,
};
#[cfg(feature = "variable_assign")]
use crate::{
    VariableAssign, execute_catalog_operation_with_registration_arguments, expression_cell,
};
#[cfg(all(feature = "subscript", feature = "assign"))]
use mech_core::snapshot::ValueDataDraft;

#[cfg(any(
    feature = "variable_assign",
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
pub(super) fn assignment_registration_operand(value: &ValueCell) -> ValueCell {
    value.clone()
}

#[cfg(any(
    all(feature = "access", feature = "subscript", feature = "math_add_assign"),
    all(feature = "access", feature = "subscript", feature = "math_sub_assign"),
    all(feature = "access", feature = "subscript", feature = "math_div_assign"),
    all(feature = "access", feature = "subscript", feature = "math_mul_assign"),
    all(feature = "access", feature = "subscript", feature = "assign")
))]
pub(super) fn execute_assignment_invocation(
    canonical_name: &str,
    inputs: Vec<SpecializationInput>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let invocation = SpecializationInvocation::new(inputs.clone().into_boxed_slice());
    let specialized = interpreter.specialize_visible_invocation_named(
        OperationId::from_name(canonical_name),
        Some(canonical_name),
        &invocation,
    )?;
    execute_bound_specialized_function(specialized, &inputs, interpreter)
}

#[cfg(any(
    all(feature = "access", feature = "subscript", feature = "math_add_assign"),
    all(feature = "access", feature = "subscript", feature = "math_sub_assign"),
    all(feature = "access", feature = "subscript", feature = "math_div_assign"),
    all(feature = "access", feature = "subscript", feature = "math_mul_assign"),
    all(feature = "access", feature = "subscript", feature = "assign")
))]
pub(super) fn assignment_selector(
    selector: &Subscript,
    environment: Option<&Environment>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<SpecializationInput> {
    match selector {
        #[cfg(feature = "subscript_formula")]
        Subscript::Formula(formula) => {
            let value = factor(formula, environment, interpreter)?;
            let already_selector = matches!(
                value.representation(),
                FunctionValueRepresentation::Bool
                    | FunctionValueRepresentation::Index
                    | FunctionValueRepresentation::Matrix {
                        element: FunctionMatrixElement::Bool | FunctionMatrixElement::Index,
                        ..
                    }
            );
            if already_selector {
                Ok(SpecializationInput::Cell(value))
            } else if matches!(
                value.representation(),
                FunctionValueRepresentation::Matrix { .. }
            ) {
                crate::intrinsics::access::matrix::canonical_reactive_index_matrix(
                    value,
                    interpreter,
                )
                .map(SpecializationInput::Cell)
            } else {
                crate::intrinsics::access::matrix::canonical_reactive_scalar_index(
                    value,
                    interpreter,
                )
                .map(SpecializationInput::Cell)
            }
        }
        #[cfg(feature = "subscript_range")]
        Subscript::Range(range_expression) => {
            let value = range(range_expression, environment, interpreter)?;
            crate::intrinsics::access::matrix::canonical_reactive_index_matrix(value, interpreter)
                .map(SpecializationInput::Cell)
        }
        Subscript::All => Ok(SpecializationInput::MatrixAllSelection),
        Subscript::DotInt(number) => {
            let value = crate::real(number, interpreter)?;
            crate::intrinsics::access::matrix::canonical_reactive_scalar_index(value, interpreter)
                .map(SpecializationInput::Cell)
        }
        _ => Err(MechError::new(super::AddressedAssignmentUnsupported, None)
            .with_compiler_loc()
            .with_tokens(selector.tokens())),
    }
}

#[cfg(all(feature = "subscript", feature = "assign", feature = "access"))]
pub fn subscript_ref(
    subscript: &Subscript,
    sink: &ValueCell,
    source: &ValueCell,
    environment: Option<&Environment>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match subscript {
        Subscript::Dot(identifier) => {
            let key =
                ValueCell::from_schema_data(SchemaBody::Id, ValueDataDraft::Id(identifier.hash()))?;
            execute_assignment_invocation(
                "assign/column",
                vec![sink.clone(), source.clone(), key]
                    .into_iter()
                    .map(SpecializationInput::Cell)
                    .collect(),
                interpreter,
            )
        }
        Subscript::DotInt(_) => execute_assignment_invocation(
            "assign",
            vec![
                SpecializationInput::Cell(sink.clone()),
                SpecializationInput::Cell(source.clone()),
                assignment_selector(subscript, environment, interpreter)?,
            ],
            interpreter,
        ),
        Subscript::Bracket(selectors) => {
            let mut inputs = vec![
                SpecializationInput::Cell(sink.clone()),
                SpecializationInput::Cell(source.clone()),
            ];
            for selector in selectors {
                inputs.push(assignment_selector(selector, environment, interpreter)?);
            }
            execute_assignment_invocation("assign", inputs, interpreter)
        }
        _ => Err(MechError::new(super::AddressedAssignmentUnsupported, None)
            .with_compiler_loc()
            .with_tokens(subscript.tokens())),
    }
}

#[cfg(all(
    feature = "variable_assign",
    feature = "subscript",
    feature = "assign",
    feature = "access"
))]
pub(crate) fn assign_source_subscripts(
    subscripts: &[Subscript],
    sink: ValueCell,
    source: ValueCell,
    environment: Option<&Environment>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let Some(first) = subscripts.first() else {
        return execute_assignment_invocation(
            "assign",
            vec![
                SpecializationInput::Cell(sink),
                SpecializationInput::Cell(source),
            ],
            interpreter,
        );
    };
    subscript_ref(first, &sink, &source, environment, interpreter)
}

#[cfg(feature = "variable_assign")]
pub fn variable_assign(
    assignment: &VariableAssign,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    if assignment.target.context.is_some() {
        return super::context_assign(assignment, env, p);
    }
    let source = expression_cell(&assignment.expression, env, p)?;
    let id = assignment.target.name.hash();
    let sink = {
        let symbols = p.symbols();
        let symbols = symbols.borrow();
        match symbols.get_mutable(id) {
            Some(value) => value,
            None if !symbols.contains(id) => {
                return Err(MechError::new(
                    UndefinedVariableError {
                        id,
                        name: assignment.target.name.to_string(),
                    },
                    Some(
                        "(!)> Variables are defined with the `:=` operator. *e.g.*: {x := 123}"
                            .to_owned(),
                    ),
                )
                .with_compiler_loc()
                .with_tokens(assignment.target.name.tokens()));
            }
            None => {
                return Err(MechError::new(
                    NotMutableError { id },
                    Some(
                        "(!)> Mutable variables are defined with the `~` operator. *e.g.*: {~x := 123}"
                            .to_owned(),
                    ),
                )
                .with_compiler_loc()
                .with_tokens(assignment.target.name.tokens()));
            }
        }
    };

    if assignment.target.subscript.is_some() {
        #[cfg(all(feature = "subscript", feature = "access"))]
        {
            return assign_source_subscripts(
                assignment.target.subscript.as_deref().unwrap_or_default(),
                sink,
                source,
                env,
                p,
            );
        }
        #[cfg(not(all(feature = "subscript", feature = "access")))]
        {
            return Err(MechError::new(super::AddressedAssignmentUnsupported, None)
                .with_compiler_loc()
                .with_tokens(assignment.target.tokens()));
        }
    }

    let registration_source = assignment_registration_operand(&source);
    execute_catalog_operation_with_registration_arguments(
        p,
        &p.plan(),
        "assign",
        vec![sink, source],
        vec![registration_source],
    )
}
