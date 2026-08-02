use crate::{
    ContextBase, ContextDeclaration, GenericError, InterpreterExecution, MResult, MechError, Value,
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
                p.bind_context(&ctx.name, binding.base_uri);
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
