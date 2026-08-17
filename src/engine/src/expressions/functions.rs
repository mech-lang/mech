use super::{Environment, expression};
#[cfg(feature = "subscript_formula")]
use super::{
    current_string_access_expression_live, mark_current_string_access_expression_live,
    mark_string_access_value_live, string_access_input_is_live,
};
use crate::{
    FunctionCall, FunctionDefinition, FunctionExtensionEntry, FunctionResolver,
    FunctionSpecializerEntry, InterpreterExecution, LegacyValue, MResult, ResolvedNamedFunction,
    execute_specialized_function, format_trace, format_trace_args,
};

enum OwnedResolvedNamedFunction {
    User(FunctionDefinition),
    Catalog(FunctionSpecializerEntry),
    Extension(FunctionExtensionEntry),
}

fn evaluate_arguments(
    fxn_call: &FunctionCall,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Vec<LegacyValue>> {
    fxn_call
        .args
        .iter()
        .map(|(_, argument)| expression(argument, env, p))
        .collect()
}

// Dispatches a named function through the program's unified resolver. User
// definitions have precedence over the current catalog or extension binding.
pub fn function_call(
    fxn_call: &FunctionCall,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let fxn_name = fxn_call.name.to_string();

    let resolved = {
        let state = p.state.borrow();
        let resolver = FunctionResolver::new(
            p.function_catalog(),
            &state.function_environment,
            &state.function_extensions,
            &state.user_functions,
        );
        match resolver.resolve_named(&fxn_name) {
            Ok(ResolvedNamedFunction::User(definition)) => {
                OwnedResolvedNamedFunction::User(definition.clone())
            }
            Ok(ResolvedNamedFunction::Catalog(entry)) => {
                OwnedResolvedNamedFunction::Catalog(entry.clone())
            }
            Ok(ResolvedNamedFunction::Extension(entry)) => {
                OwnedResolvedNamedFunction::Extension(entry.clone())
            }
            Err(error) => return Err(error.with_tokens(fxn_call.name.tokens())),
        }
    };

    let input_arg_values = evaluate_arguments(fxn_call, env, p)?;
    match resolved {
        OwnedResolvedNamedFunction::User(definition) => {
            #[cfg(feature = "subscript_formula")]
            let output_is_live = current_string_access_expression_live(p)
                || input_arg_values
                    .iter()
                    .any(|value| string_access_input_is_live(value, p));
            let output = crate::function::execute_user_function(&definition, &input_arg_values, p)?;
            #[cfg(feature = "subscript_formula")]
            if output_is_live {
                mark_current_string_access_expression_live(p);
                mark_string_access_value_live(p, &output);
            }
            Ok(output)
        }
        OwnedResolvedNamedFunction::Catalog(entry) => {
            trace_println!(
                p,
                "{}",
                format_trace(
                    "fn",
                    format!(
                        "catalog {}({})",
                        fxn_name,
                        format_trace_args(&input_arg_values)
                    ),
                )
            );
            let function = mech_core::with_semantic_operation(
                entry.canonical_name,
                entry.specializer.specialize(&input_arg_values)?,
            );
            execute_specialized_function(function, &input_arg_values, p)
        }
        OwnedResolvedNamedFunction::Extension(entry) => {
            trace_println!(
                p,
                "{}",
                format_trace(
                    "fn",
                    format!(
                        "extension {}({})",
                        fxn_name,
                        format_trace_args(&input_arg_values)
                    ),
                )
            );
            let function = mech_core::with_semantic_operation(
                entry.canonical_name,
                entry.specializer.specialize(&input_arg_values)?,
            );
            execute_specialized_function(function, &input_arg_values, p)
        }
    }
}
