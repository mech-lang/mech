use crate::{
    ContextBase, ContextDeclaration, ContextSend, ExecutionResourceRequest,
    ExternalResourceWriteFunction, GenericError, InitialSolvePolicy, InterpreterExecution, MResult,
    MechError, Ref, ResourceDelivery, ResourceIntent, Value, execute_specialized_function,
    expression,
};

// Interpreter-local context bindings are for direct interpreter execution.
// Host runtime resource bindings are owned by MechRuntime.resource_bindings.
pub fn context_declaration(
    ctx: &ContextDeclaration,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    match &ctx.base {
        ContextBase::ResourceUri(uri) => {
            p.bind_context(&ctx.name, uri.chars.iter().collect::<String>());
            Ok(Value::Empty)
        }
        ContextBase::Context(base) => match p.context_binding(base) {
            Some(binding) => {
                p.bind_context_with_name(&ctx.name, binding.context_name, binding.base_uri);
                Ok(Value::Empty)
            }
            None => Err(MechError::new(
                GenericError {
                    msg: format!("Context `@{}` is not defined", base.to_string()),
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(base.tokens())),
        },
    }
}

/// Lower a direct source send into the same external resource node used by
/// decoded bytecode. Runtime capability admission remains outside the engine;
/// this path only resolves an interpreter-local context binding.
pub fn context_send(send: &ContextSend, p: &InterpreterExecution<'_>) -> MResult<Value> {
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
    let binding = p.context_binding(context).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!("Context `@{}` is not defined", context.to_string()),
            },
            None,
        )
        .with_compiler_loc()
        .with_tokens(context.tokens())
    })?;
    let input = expression(&send.expression, None, p)?;
    let arguments = vec![input.clone()];
    let function = ExternalResourceWriteFunction {
        request: ExecutionResourceRequest {
            base_uri: binding.base_uri,
            path: send.target.name.to_string(),
            context_name: binding.context_name,
            operation: "write".to_string(),
            intent: ResourceIntent::Send,
            delivery: ResourceDelivery::Snapshot,
        },
        input,
        output: Ref::new(Value::Empty),
        initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
    };
    execute_specialized_function(Box::new(function), &arguments, p)
}
