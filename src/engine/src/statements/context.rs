use crate::{
    ContextBase, ContextDeclaration, ContextSend, ExecutionResourceRequest,
    ExternalResourceReadFunction, ExternalResourceWriteFunction, FunctionInstance,
    FunctionInvocation, GenericError, Identifier, InitialSolvePolicy, InterpreterExecution,
    MResult, MechError, ResourceDelivery, ResourceIntent, SpecializationInput, SpecializedFunction,
    UndefinedContextError, ValueCell, Var, execute_bound_specialized_function, expression_cell,
};
#[cfg(feature = "variable_assign")]
use crate::{Environment, VariableAssign};

// Interpreter-local context bindings are for direct interpreter execution.
pub fn context_declaration(
    ctx: &ContextDeclaration,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    match &ctx.base {
        ContextBase::ResourceUri(uri) => {
            p.bind_context(&ctx.name, uri.chars.iter().collect::<String>());
            Ok(ValueCell::unit())
        }
        ContextBase::Context(base) => match p.context_binding(base) {
            Some(binding) => {
                p.bind_context_with_name(&ctx.name, binding.context_name, binding.base_uri);
                Ok(ValueCell::unit())
            }
            None => Err(MechError::new(
                UndefinedContextError {
                    context: base.to_string(),
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(base.tokens())),
        },
    }
}

fn resource_request(
    context: &Identifier,
    path: &Identifier,
    operation: &'static str,
    intent: ResourceIntent,
    delivery: ResourceDelivery,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ExecutionResourceRequest> {
    let binding = interpreter.context_binding(context).ok_or_else(|| {
        MechError::new(
            UndefinedContextError {
                context: context.to_string(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(context.tokens())
    })?;

    Ok(ExecutionResourceRequest {
        base_uri: binding.base_uri,
        path: path.to_string(),
        context_name: binding.context_name,
        operation: operation.to_owned(),
        intent,
        delivery,
    })
}

pub(crate) fn context_read(
    variable: &Var,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let context = variable.context.as_ref().ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: "context_read requires a context-addressed variable".to_string(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(variable.tokens())
    })?;
    let request = resource_request(
        context,
        &variable.name,
        "read",
        ResourceIntent::Read,
        ResourceDelivery::Live,
        interpreter,
    )?;
    let representative =
        interpreter.with_services(|services| services.plan_resource_read_output(&request))?;
    let output = ValueCell::from_snapshot(representative)?;
    let function = ExternalResourceReadFunction::new(
        interpreter.id,
        request,
        output.clone(),
        false,
        InitialSolvePolicy::Solve,
        None,
    );
    let arguments = Vec::<SpecializationInput>::new();
    let invocation = FunctionInvocation::nullary(output.clone());
    execute_bound_specialized_function(
        SpecializedFunction::syntax_directed(
            FunctionInstance::new(Box::new(function), invocation),
            mech_core::ResolvedOperationDescriptor::from_name(
                "context/read",
                crate::function::external::RESOURCE_OBSERVATION_CONTRACT.clone(),
            )?,
            mech_core::RuntimeFunctionId::from_name("ExternalResourceReadFunction"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )?,
        &arguments,
        interpreter,
    )?;
    Ok(output)
}

#[cfg(feature = "variable_assign")]
pub(crate) fn context_assign(
    assignment: &VariableAssign,
    environment: Option<&Environment>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let context = assignment.target.context.as_ref().ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: "context_assign requires a context-addressed target".to_string(),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(assignment.target.tokens())
    })?;
    let request = resource_request(
        context,
        &assignment.target.name,
        "write",
        ResourceIntent::Assign,
        ResourceDelivery::Snapshot,
        interpreter,
    )?;
    let input_cell = expression_cell(&assignment.expression, environment, interpreter)?;
    let arguments = vec![SpecializationInput::Cell(input_cell.clone())];
    let output = ValueCell::unit();
    let function = ExternalResourceWriteFunction {
        request,
        input: input_cell.clone(),
        output: output.clone(),
        initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
        semantic_contract: None,
    };
    let invocation = FunctionInvocation::unary(output, input_cell);
    execute_bound_specialized_function(
        SpecializedFunction::syntax_directed(
            FunctionInstance::new(Box::new(function), invocation),
            mech_core::ResolvedOperationDescriptor::from_name(
                "context/write",
                crate::function::external::RESOURCE_EFFECT_CONTRACT.clone(),
            )?,
            mech_core::RuntimeFunctionId::from_name("ExternalResourceWriteFunction"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )?,
        &arguments,
        interpreter,
    )
}

/// Lower a direct source send into the same external resource node used by
/// decoded bytecode. Runtime capability admission remains outside the engine;
/// this path only resolves an interpreter-local context binding.
pub fn context_send(send: &ContextSend, p: &InterpreterExecution<'_>) -> MResult<ValueCell> {
    let context = send.target.context.as_ref().ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!(
                    "Context send target `{}` is not context-addressed",
                    send.target.name.to_string(),
                ),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(send.target.tokens())
    })?;
    let request = resource_request(
        context,
        &send.target.name,
        "write",
        ResourceIntent::Send,
        ResourceDelivery::Snapshot,
        p,
    )?;
    let input_cell = expression_cell(&send.expression, None, p)?;
    let arguments = vec![SpecializationInput::Cell(input_cell.clone())];
    let output = ValueCell::unit();
    let function = ExternalResourceWriteFunction {
        request,
        input: input_cell.clone(),
        output: output.clone(),
        initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
        semantic_contract: None,
    };
    let invocation = FunctionInvocation::unary(output, input_cell);
    execute_bound_specialized_function(
        SpecializedFunction::syntax_directed(
            FunctionInstance::new(Box::new(function), invocation),
            mech_core::ResolvedOperationDescriptor::from_name(
                "context/send",
                crate::function::external::RESOURCE_EFFECT_CONTRACT.clone(),
            )?,
            mech_core::RuntimeFunctionId::from_name("ExternalResourceWriteFunction"),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )?,
        &arguments,
        p,
    )
}
