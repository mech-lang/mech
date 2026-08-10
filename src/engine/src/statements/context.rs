use crate::{
    ContextBase, ContextDeclaration, ContextSend, ExecutionResourceRequest,
    ExternalResourceReadFunction, ExternalResourceWriteFunction, GenericError, Identifier,
    InitialSolvePolicy, InterpreterExecution, LegacyValue, MResult, MechError, Ref,
    ResourceDelivery, ResourceIntent, UndefinedContextError, ValRef, Var,
    execute_specialized_function, expression,
};

// Interpreter-local context bindings are for direct interpreter execution.
// Host runtime resource bindings are owned by MechRuntime.resource_bindings.
pub fn context_declaration(
    ctx: &ContextDeclaration,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    match &ctx.base {
        ContextBase::ResourceUri(uri) => {
            p.bind_context(&ctx.name, uri.chars.iter().collect::<String>());
            Ok(LegacyValue::Empty)
        }
        ContextBase::Context(base) => match p.context_binding(base) {
            Some(binding) => {
                p.bind_context_with_name(&ctx.name, binding.context_name, binding.base_uri);
                Ok(LegacyValue::Empty)
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
) -> MResult<ValRef> {
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
    let output = Ref::new(LegacyValue::Empty);
    let function = ExternalResourceReadFunction {
        interpreter_id: interpreter.id,
        request,
        output: output.clone(),
        initial_solve_policy: InitialSolvePolicy::Solve,
        semantic_contract: None,
    };
    let arguments = Vec::<LegacyValue>::new();
    execute_specialized_function(Box::new(function), &arguments, interpreter)?;
    Ok(output)
}

/// Lower a direct source send into the same external resource node used by
/// decoded bytecode. Runtime capability admission remains outside the engine;
/// this path only resolves an interpreter-local context binding.
pub fn context_send(send: &ContextSend, p: &InterpreterExecution<'_>) -> MResult<LegacyValue> {
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
    let input = expression(&send.expression, None, p)?;
    let arguments = vec![input.clone()];
    let function = ExternalResourceWriteFunction {
        request,
        input,
        output: Ref::new(LegacyValue::Empty),
        initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
        semantic_contract: None,
    };
    execute_specialized_function(Box::new(function), &arguments, p)
}
