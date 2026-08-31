#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
use super::variable_assign::assignment_registration_operand;
#[cfg(any(
    all(feature = "access", feature = "subscript", feature = "math_add_assign"),
    all(feature = "access", feature = "subscript", feature = "math_sub_assign"),
    all(feature = "access", feature = "subscript", feature = "math_div_assign"),
    all(feature = "access", feature = "subscript", feature = "math_mul_assign")
))]
use super::variable_assign::{assignment_selector, execute_assignment_invocation};
#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
use super::{AddressedAssignmentUnsupported, NotMutableError, UndefinedVariableError};
#[cfg(feature = "math_add_assign")]
use crate::SchemaBody;
use crate::{Environment, InterpreterExecution, MResult};
#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
use crate::{
    MechError, OpAssign, OpAssignOp, ValueCell,
    execute_catalog_operation_with_registration_arguments, expression_cell,
};
#[cfg(all(
    feature = "access",
    feature = "subscript",
    any(
        feature = "math_add_assign",
        feature = "math_sub_assign",
        feature = "math_div_assign",
        feature = "math_mul_assign"
    )
))]
use crate::{SpecializationInput, Subscript};

#[cfg(any(
    feature = "math_add_assign",
    feature = "math_sub_assign",
    feature = "math_div_assign",
    feature = "math_mul_assign"
))]
pub fn op_assign(
    assignment: &OpAssign,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let source = expression_cell(&assignment.expression, env, p)?;
    let target = &assignment.target;
    if target.context.is_some() {
        return Err(MechError::new(AddressedAssignmentUnsupported, None)
            .with_compiler_loc()
            .with_tokens(target.tokens()));
    }
    let id = target.name.hash();
    let sink = {
        let state = p.state.borrow();
        match state.get_mutable_symbol(id) {
            Some(value) => value,
            None if state.contains_symbol(id) => {
                return Err(MechError::new(
                    NotMutableError { id },
                    Some(
                        "(!)> Mutable variables are defined with the `~` operator. *e.g.*: {~x := 123}"
                            .to_owned(),
                    ),
                )
                .with_compiler_loc()
                .with_tokens(target.name.tokens()));
            }
            None => {
                return Err(MechError::new(
                    UndefinedVariableError {
                        id,
                        name: target.name.to_string(),
                    },
                    Some(
                        "(!)> Variables are defined with the `:=` operator. *e.g.*: {x := 123}"
                            .to_owned(),
                    ),
                )
                .with_compiler_loc()
                .with_tokens(target.name.tokens()));
            }
        }
    };

    let operation = match assignment.op {
        #[cfg(feature = "math_add_assign")]
        OpAssignOp::Add if matches!(sink.closed_schema_body()?, SchemaBody::Table { .. }) => {
            "assign/add"
        }
        #[cfg(feature = "math_add_assign")]
        OpAssignOp::Add => "math/add-assign",
        #[cfg(feature = "math_sub_assign")]
        OpAssignOp::Sub => "math/sub-assign",
        #[cfg(feature = "math_div_assign")]
        OpAssignOp::Div => "math/div-assign",
        #[cfg(feature = "math_mul_assign")]
        OpAssignOp::Mul => "math/mul-assign",
        _ => unreachable!(),
    };

    if let Some(_subscripts) = target.subscript.as_deref() {
        #[cfg(all(feature = "subscript", feature = "access"))]
        {
            return op_assign_source_subscripts(operation, _subscripts, sink, source, env, p);
        }
        #[cfg(not(all(feature = "subscript", feature = "access")))]
        {
            return Err(MechError::new(AddressedAssignmentUnsupported, None)
                .with_compiler_loc()
                .with_tokens(target.tokens()));
        }
    }

    let registration_source = assignment_registration_operand(&source);
    execute_catalog_operation_with_registration_arguments(
        p,
        &p.plan(),
        operation,
        vec![sink, source],
        vec![registration_source],
    )
}

#[cfg(any(
    all(feature = "access", feature = "subscript", feature = "math_add_assign"),
    all(feature = "access", feature = "subscript", feature = "math_sub_assign"),
    all(feature = "access", feature = "subscript", feature = "math_div_assign"),
    all(feature = "access", feature = "subscript", feature = "math_mul_assign")
))]
fn selector_is_range(selector: &SpecializationInput) -> MResult<bool> {
    let SpecializationInput::Cell(cell) = selector else {
        return Ok(false);
    };
    Ok(cell
        .matrix_elements()?
        .is_some_and(|elements| elements.len() != 1))
}

#[cfg(any(
    all(feature = "access", feature = "subscript", feature = "math_add_assign"),
    all(feature = "access", feature = "subscript", feature = "math_sub_assign"),
    all(feature = "access", feature = "subscript", feature = "math_div_assign"),
    all(feature = "access", feature = "subscript", feature = "math_mul_assign")
))]
pub(crate) fn op_assign_source_subscripts(
    operation: &str,
    subscripts: &[Subscript],
    sink: ValueCell,
    source: ValueCell,
    environment: Option<&Environment>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let Some(Subscript::Bracket(selectors)) = subscripts.first() else {
        return Err(MechError::new(super::AddressedAssignmentUnsupported, None).with_compiler_loc());
    };
    let mut lowered = Vec::with_capacity(selectors.len());
    for selector in selectors {
        lowered.push(assignment_selector(selector, environment, interpreter)?);
    }
    if let [
        SpecializationInput::Cell(selector),
        SpecializationInput::MatrixAllSelection,
    ] = lowered.as_slice()
        && selector.matrix_elements()?.is_none()
    {
        lowered[0] = SpecializationInput::Cell(ValueCell::dynamic_matrix_from_cells(
            1,
            1,
            core::slice::from_ref(&selector),
        )?);
    }
    let selected_operation = match lowered.as_slice() {
        [selector] if selector_is_range(selector)? => format!("{operation}/range"),
        [SpecializationInput::MatrixAllSelection] => "assign".to_owned(),
        [_] => "assign".to_owned(),
        [_, SpecializationInput::MatrixAllSelection] => format!("{operation}/range-all"),
        _ => operation.to_owned(),
    };
    let mut inputs = vec![
        SpecializationInput::Cell(sink),
        SpecializationInput::Cell(source),
    ];
    inputs.extend(lowered);
    execute_assignment_invocation(&selected_operation, inputs, interpreter)
}

macro_rules! indexed_op_assignment {
    ($name:ident, $operation:literal, $feature:literal) => {
        #[cfg(all(
                                    feature = "access",
                                    feature = "subscript",
                                    feature = $feature,
                                    any(feature = "subscript_formula", feature = "subscript_range")
                                ))]
        pub fn $name(
            subscript: &Subscript,
            sink: &ValueCell,
            source: &ValueCell,
            environment: Option<&Environment>,
            interpreter: &InterpreterExecution<'_>,
        ) -> MResult<ValueCell> {
            op_assign_source_subscripts(
                $operation,
                core::slice::from_ref(subscript),
                sink.clone(),
                source.clone(),
                environment,
                interpreter,
            )
        }
    };
}

indexed_op_assignment!(add_assign, "math/add-assign", "math_add_assign");
indexed_op_assignment!(sub_assign, "math/sub-assign", "math_sub_assign");
indexed_op_assignment!(mul_assign, "math/mul-assign", "math_mul_assign");
indexed_op_assignment!(div_assign, "math/div-assign", "math_div_assign");
