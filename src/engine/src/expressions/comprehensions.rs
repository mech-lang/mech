use super::{
    ComprehensionGeneratorError, Environment, ReactiveComprehensionStructureUnsupported,
    expression_cell,
};
#[cfg(feature = "matrix_comprehensions")]
use crate::MatrixComprehension;
#[cfg(feature = "set_comprehensions")]
use crate::SetComprehension;
#[cfg(feature = "matrix_comprehensions")]
use crate::execute_function_instance;
#[cfg(feature = "matrix_comprehensions")]
pub use crate::intrinsics::constructors::ValueMatrixComprehension;
#[cfg(feature = "set_comprehensions")]
pub use crate::intrinsics::constructors::ValueSetComprehension;
use crate::patterns::PatternBindingSink;
use crate::{
    CanonicalFunctionSpecializer, ComprehensionQualifier, Expression, ExternalInteraction,
    FunctionInstance, FunctionInvocation, Interpreter, InterpreterExecution, MResult, MechError,
    MechFunctionFactory, ReactiveNodeKind, SchemaBody, SpecializationContext,
    SpecializationInvocation, SpecializedFunction, ValueCell, ValueData, execute_catalog_operation,
    hash_str,
};
use std::collections::{HashMap, HashSet};

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn value_depends_on_reactive_turn(value: &ValueCell, p: &InterpreterExecution<'_>) -> bool {
    let mut turn_cells = {
        let state = p.state.borrow();
        let symbols = state.symbol_table.borrow();
        symbols
            .mutable_variables
            .values()
            .map(ValueCell::reactive_cell_id)
            .collect::<HashSet<_>>()
    };
    let plan = p.plan();
    let plan = plan.borrow();
    for node in &plan.nodes {
        let external = node
            .function
            .semantic_operation_contract()
            .is_some_and(|contract| contract.interaction != ExternalInteraction::Pure);
        let depends_on_turn = node.kind == ReactiveNodeKind::Register
            || external
            || node
                .inputs
                .iter()
                .any(|input| turn_cells.contains(&input.cell));
        if depends_on_turn {
            turn_cells.extend(node.outputs.iter().copied());
        }
    }
    turn_cells.contains(&value.reactive_cell_id())
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn reject_reactive_structure(
    value: &ValueCell,
    qualifier: &'static str,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    if value_depends_on_reactive_turn(value, p) {
        return Err(MechError::new(
            ReactiveComprehensionStructureUnsupported { qualifier },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_environments(
    qualifiers: &[ComprehensionQualifier],
    comprehension_id: u64,
    p: &InterpreterExecution<'_>,
) -> MResult<(Vec<Environment>, Interpreter, Option<Environment>)> {
    let mut envs: Vec<Environment> = vec![HashMap::new()];
    let mut schema_environment = None;
    let mut new_p: Interpreter = (**p).clone();
    new_p.id = comprehension_id;
    // A comprehension has its own lexical environment, not its own reactive
    // lifetime. Keep the enclosing plan shared so operations compiled inside
    // generators, filters, lets, and the result expression remain live and
    // precede the final comprehension node in the same dependency graph.
    for qual in qualifiers {
        envs = match qual {
            ComprehensionQualifier::Generator((pttrn, expr)) => {
                let compiled = crate::patterns::compile_pattern(pttrn, None, &new_p)?;
                let mut new_envs = Vec::new();
                for env in &envs {
                    let collection = p.with_interpreter(&new_p, |execution| {
                        expression_cell(expr, Some(env), execution)
                    })?;
                    reject_reactive_structure(&collection, "generator", p)?;
                    for elmnt in comprehension_generator_values(&collection)? {
                        let mut new_env = env.clone();
                        let pattern_match = p.with_interpreter(&new_p, |execution| {
                            crate::patterns::match_compiled_pattern_with_environment_constraints(
                                &compiled, &elmnt, &new_env, execution,
                            )
                        })?;
                        if pattern_match.matched {
                            crate::patterns::EnvironmentBindingSink::new(&mut new_env)
                                .commit(&pattern_match)?;
                            new_envs.push(new_env);
                        }
                    }
                }
                new_envs
            }
            ComprehensionQualifier::Filter(expr) => {
                let mut filtered = Vec::new();
                for env in envs {
                    let result = p.with_interpreter(&new_p, |execution| {
                        expression_cell(expr, Some(&env), execution)
                    })?;
                    reject_reactive_structure(&result, "filter", p)?;
                    if matches!(result.snapshot()?.data(), ValueData::Bool(true)) {
                        filtered.push(env);
                    }
                }
                filtered
            }
            ComprehensionQualifier::Let(var_def) => envs
                .into_iter()
                .map(|mut env| -> MResult<_> {
                    let val = p.with_interpreter(&new_p, |execution| {
                        expression_cell(&var_def.expression, Some(&env), execution)
                    })?;
                    env.insert(var_def.var.name.hash(), val);
                    Ok(env)
                })
                .collect::<MResult<Vec<_>>>()?,
        };
        if let Some(environment) = envs.first() {
            schema_environment = Some(environment.clone());
        }
    }
    Ok((envs, new_p, schema_environment))
}

#[cfg(feature = "matrix_comprehensions")]
fn empty_comprehension_element(
    expression: &Expression,
    environment: Option<&Environment>,
) -> MResult<Option<ValueCell>> {
    let Expression::Var(variable) = expression else {
        return Ok(None);
    };
    Ok(environment
        .and_then(|environment| environment.get(&variable.name.hash()))
        .map(ValueCell::detached_clone)
        .transpose()?)
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_generator_values(collection: &ValueCell) -> MResult<Vec<ValueCell>> {
    if let Some(values) = collection.set_element_cells()? {
        return Ok(values);
    }
    if let Some(values) = collection.matrix_elements()? {
        return Ok(values);
    }
    Err(MechError::new(
        ComprehensionGeneratorError {
            found: collection.resolved_type()?,
        },
        None,
    )
    .with_compiler_loc())
}

#[cfg(feature = "set_comprehensions")]
pub struct SetComprehensionDefine {}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl CanonicalFunctionSpecializer for SetComprehensionDefine {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let arguments = invocation
            .inputs()
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?;
        let element = arguments
            .first()
            .map(ValueCell::closed_schema_body)
            .transpose()?
            .unwrap_or_else(|| SchemaBody::Tuple(Box::new([])));
        for argument in &arguments {
            if argument.closed_schema_body()? != element {
                return Err(MechError::new(
                    ComprehensionGeneratorError {
                        found: argument.resolved_type()?,
                    },
                    None,
                )
                .with_compiler_loc());
            }
        }
        let output = ValueCell::empty_dynamic_set(element)?
            .with_resolved_output_type(context.resolved_output(0)?)?;
        let invocation = FunctionInvocation::variadic(output, arguments.into_boxed_slice());
        let implementation = ValueSetComprehension::new_invocation(invocation.clone())?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            implementation,
            invocation,
        )))
    }
}
#[cfg(feature = "matrix_comprehensions")]
pub struct MatrixComprehensionDefine {}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl CanonicalFunctionSpecializer for MatrixComprehensionDefine {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let arguments = invocation
            .inputs()
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?;
        let output = crate::intrinsics::constructors::matrix_comprehension_output(&arguments)?;
        let invocation = FunctionInvocation::variadic(output, arguments.into_boxed_slice());
        let implementation = ValueMatrixComprehension::new_invocation(invocation.clone())?;
        Ok(SpecializedFunction::new(FunctionInstance::new(
            implementation,
            invocation,
        )))
    }
}
#[cfg(feature = "set_comprehensions")]
pub fn set_comprehension(
    set_comp: &SetComprehension,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let comprehension_id = hash_str(&format!("{:?}", set_comp));
    let (envs, new_p, _schema_environment) =
        comprehension_environments(&set_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        let val = p.with_interpreter(&new_p, |execution| {
            expression_cell(&set_comp.expression, Some(&env), execution)
        })?;
        values.push(val);
    }
    let plan = p.plan();
    execute_catalog_operation(p, &plan, "set/comprehension", values)
}

#[cfg(feature = "matrix_comprehensions")]
pub fn matrix_comprehension(
    matrix_comp: &MatrixComprehension,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let comprehension_id = hash_str(&format!("{:?}", matrix_comp));
    let (envs, new_p, schema_environment) =
        comprehension_environments(&matrix_comp.qualifiers, comprehension_id, p)?;
    let mut values = Vec::new();
    for env in envs {
        values.push(p.with_interpreter(&new_p, |execution| {
            expression_cell(&matrix_comp.expression, Some(&env), execution)
        })?);
    }
    let plan = p.plan();
    if values.is_empty()
        && let Some(element) =
            empty_comprehension_element(&matrix_comp.expression, schema_environment.as_ref())?
    {
        let element_schema = match element.closed_schema_body()? {
            SchemaBody::Matrix { element, .. } => *element,
            schema => schema,
        };
        let output =
            ValueCell::dynamic_matrix(element_schema, vec![0, 0].into_boxed_slice(), Box::new([]))?;
        let invocation = FunctionInvocation::variadic(output.clone(), Box::new([]));
        let implementation = ValueMatrixComprehension::new_invocation(invocation.clone())?;
        return execute_function_instance(
            p,
            &plan,
            FunctionInstance::new(implementation, invocation),
        );
    }
    execute_catalog_operation(p, &plan, "matrix/comprehension", values)
}
